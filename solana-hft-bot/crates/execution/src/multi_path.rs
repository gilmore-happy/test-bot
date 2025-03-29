//! Multi-path transaction submission for Solana HFT Bot
//!
//! This module provides parallel transaction submission capabilities with:
//! - Multiple endpoint support
//! - Dynamic endpoint selection based on performance
//! - Automatic failover and recovery
//! - Transaction status tracking

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::Signature,
    transaction::Transaction,
};
use tokio::sync::{mpsc, oneshot, Semaphore};
use tokio::sync::broadcast;
use tokio::time::timeout;
use tracing::{debug, error, info, instrument, trace, warn};

use crate::ExecutionError;

/// Endpoint configuration for multi-path submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// Endpoint URL
    pub url: String,
    
    /// Endpoint type
    pub endpoint_type: EndpointType,
    
    /// Weight for selection (higher = more likely to be selected)
    pub weight: u32,
    
    /// Whether the endpoint is enabled
    pub enabled: bool,
    
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
    
    /// Authentication token (if required)
    pub auth_token: Option<String>,
    
    /// Whether to use this endpoint for high-value transactions
    pub use_for_high_value: bool,
    
    /// Whether to use this endpoint for time-sensitive transactions
    pub use_for_time_sensitive: bool,
}

/// Endpoint type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EndpointType {
    /// Self-hosted endpoint
    SelfHosted,
    
    /// Premium provider (e.g., QuickNode, Alchemy)
    Premium,
    
    /// Public endpoint
    Public,
}

/// Endpoint status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EndpointStatus {
    /// Endpoint is healthy
    Healthy,
    
    /// Endpoint is degraded
    Degraded,
    
    /// Endpoint is down
    Down,
    
    /// Endpoint status is unknown
    Unknown,
}

/// Endpoint metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointMetrics {
    /// Endpoint URL
    pub url: String,
    
    /// Endpoint type
    pub endpoint_type: EndpointType,
    
    /// Current status
    pub status: EndpointStatus,
    
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
    
    /// Number of requests sent
    pub requests_sent: u64,
    
    /// Number of successful responses
    pub successful_responses: u64,
    
    /// Number of failed responses
    pub failed_responses: u64,
    
    /// Number of timeouts
    pub timeouts: u64,
    
    /// Last updated timestamp
    pub last_updated: Instant,
}

/// Submission result
#[derive(Debug, Clone)]
pub struct SubmissionResult {
    /// Transaction signature
    pub signature: Signature,
    
    /// Endpoint URL that succeeded
    pub endpoint_url: String,
    
    /// Response time in milliseconds
    pub response_time_ms: u64,
    
    /// Whether the transaction was confirmed
    pub confirmed: bool,
    
    /// Error message (if any)
    pub error: Option<String>,
}

/// Multi-path submitter for parallel transaction submission
pub struct MultiPathSubmitter {
    /// Endpoints
    endpoints: Arc<RwLock<Vec<EndpointConfig>>>,
    
    /// RPC clients
    clients: Arc<DashMap<String, Arc<RpcClient>>>,
    
    /// Endpoint metrics
    metrics: Arc<DashMap<String, EndpointMetrics>>,
    
    /// Semaphores for limiting concurrent requests per endpoint
    semaphores: Arc<DashMap<String, Arc<Semaphore>>>,
    
    /// Maximum number of parallel endpoints to use
    max_parallel_endpoints: usize,
    
    /// Default timeout
    default_timeout: Duration,
    
    /// Whether to cancel in-flight transactions after first success
    cancel_after_first_success: bool,
}

impl MultiPathSubmitter {
    /// Create a new multi-path submitter
    pub fn new(
        endpoints: Vec<EndpointConfig>,
        max_parallel_endpoints: usize,
        default_timeout: Duration,
        cancel_after_first_success: bool,
    ) -> Result<Self> {
        if endpoints.is_empty() {
            return Err(anyhow!("No endpoints provided"));
        }
        
        let clients = Arc::new(DashMap::new());
        let metrics = Arc::new(DashMap::new());
        let semaphores = Arc::new(DashMap::new());
        
        // Initialize clients and metrics
        for endpoint in &endpoints {
            let client = RpcClient::new_with_timeout(
                endpoint.url.clone(),
                Duration::from_millis(endpoint.timeout_ms),
            );
            
            clients.insert(endpoint.url.clone(), Arc::new(client));
            
            metrics.insert(endpoint.url.clone(), EndpointMetrics {
                url: endpoint.url.clone(),
                endpoint_type: endpoint.endpoint_type,
                status: EndpointStatus::Unknown,
                avg_response_time_ms: 0.0,
                success_rate: 1.0, // Start optimistic
                requests_sent: 0,
                successful_responses: 0,
                failed_responses: 0,
                timeouts: 0,
                last_updated: Instant::now(),
            });
            
            semaphores.insert(
                endpoint.url.clone(),
                Arc::new(Semaphore::new(endpoint.max_concurrent_requests)),
            );
        }
        
        Ok(Self {
            endpoints: Arc::new(RwLock::new(endpoints)),
            clients,
            metrics,
            semaphores,
            max_parallel_endpoints,
            default_timeout,
            cancel_after_first_success,
        })
    }
    
    /// Submit a transaction to multiple endpoints in parallel
    pub async fn submit_transaction(
        &self,
        transaction: &Transaction,
        timeout: Option<Duration>,
    ) -> Result<SubmissionResult, ExecutionError> {
        let timeout = timeout.unwrap_or(self.default_timeout);
        
        // Select endpoints
        let selected_endpoints = self.select_endpoints(self.max_parallel_endpoints);
        
        if selected_endpoints.is_empty() {
            return Err(ExecutionError::Internal("No endpoints available".to_string()));
        }
        
        // Create channels for coordination
        let (result_tx, result_rx) = oneshot::channel();
        let (cancel_tx, _) = broadcast::channel(selected_endpoints.len());
        
        // Submit to each endpoint in parallel
        for endpoint in selected_endpoints {
            let transaction = transaction.clone();
            let client = self.clients.get(&endpoint.url).unwrap().clone();
            let semaphore = self.semaphores.get(&endpoint.url).unwrap().clone();
            let metrics = self.metrics.clone();
            let endpoint_url = endpoint.url.clone();
            let result_tx = result_tx.clone();
            let cancel_rx = cancel_tx.subscribe();
            let timeout = timeout;
            
            tokio::spawn(async move {
                // Acquire semaphore permit
                let _permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        // Semaphore closed
                        return;
                    }
                };
                
                let start = Instant::now();
                
                // Submit transaction with timeout
                let result = tokio::select! {
                    result = client.send_transaction(&transaction) => {
                        match result {
                            Ok(signature) => {
                                // Update metrics
                                if let Some(mut metric) = metrics.get_mut(&endpoint_url) {
                                    metric.requests_sent += 1;
                                    metric.successful_responses += 1;
                                    metric.avg_response_time_ms = (metric.avg_response_time_ms * (metric.requests_sent - 1) as f64 + start.elapsed().as_millis() as f64) / metric.requests_sent as f64;
                                    metric.success_rate = metric.successful_responses as f64 / metric.requests_sent as f64;
                                    metric.status = EndpointStatus::Healthy;
                                    metric.last_updated = Instant::now();
                                }
                                
                                // Send result
                                let _ = result_tx.send(Ok(SubmissionResult {
                                    signature,
                                    endpoint_url: endpoint_url.clone(),
                                    response_time_ms: start.elapsed().as_millis() as u64,
                                    confirmed: false,
                                    error: None,
                                }));
                            }
                            Err(e) => {
                                // Update metrics
                                if let Some(mut metric) = metrics.get_mut(&endpoint_url) {
                                    metric.requests_sent += 1;
                                    metric.failed_responses += 1;
                                    metric.avg_response_time_ms = (metric.avg_response_time_ms * (metric.requests_sent - 1) as f64 + start.elapsed().as_millis() as f64) / metric.requests_sent as f64;
                                    metric.success_rate = metric.successful_responses as f64 / metric.requests_sent as f64;
                                    
                                    if metric.success_rate < 0.5 {
                                        metric.status = EndpointStatus::Degraded;
                                    }
                                    
                                    metric.last_updated = Instant::now();
                                }
                                
                                // Don't send error result yet, let other endpoints try
                                debug!("Endpoint {} failed: {}", endpoint_url, e);
                            }
                        }
                    }
                    _ = tokio::time::sleep(timeout) => {
                        // Update metrics
                        if let Some(mut metric) = metrics.get_mut(&endpoint_url) {
                            metric.requests_sent += 1;
                            metric.timeouts += 1;
                            metric.avg_response_time_ms = (metric.avg_response_time_ms * (metric.requests_sent - 1) as f64 + timeout.as_millis() as f64) / metric.requests_sent as f64;
                            metric.success_rate = metric.successful_responses as f64 / metric.requests_sent as f64;
                            
                            if metric.timeouts > 3 {
                                metric.status = EndpointStatus::Down;
                            } else {
                                metric.status = EndpointStatus::Degraded;
                            }
                            
                            metric.last_updated = Instant::now();
                        }
                        
                        debug!("Endpoint {} timed out", endpoint_url);
                    }
                    _ = cancel_rx.recv() => {
                        // Another endpoint succeeded, cancel this one
                        debug!("Cancelling submission to endpoint {}", endpoint_url);
                        return;
                    }
                };
            });
        }
        
        // Wait for first success or all failures
        match result_rx.await {
            Ok(result) => {
                // Cancel other in-flight submissions if configured
                if self.cancel_after_first_success {
                    let _ = cancel_tx.send(());
                }
                
                result.map_err(|e| ExecutionError::Transaction(format!("Transaction failed: {}", e)))
            }
            Err(_) => {
                Err(ExecutionError::Internal("All endpoints failed".to_string()))
            }
        }
    }
    
    /// Select endpoints for submission
    fn select_endpoints(&self, max_count: usize) -> Vec<EndpointConfig> {
        let endpoints = self.endpoints.read();
        
        // Filter enabled endpoints
        let mut enabled_endpoints: Vec<_> = endpoints.iter()
            .filter(|e| e.enabled)
            .cloned()
            .collect();
        
        if enabled_endpoints.is_empty() {
            return Vec::new();
        }
        
        // Sort by weight and success rate
        enabled_endpoints.sort_by(|a, b| {
            let a_metric = self.metrics.get(&a.url);
            let b_metric = self.metrics.get(&b.url);
            
            let a_score = a_metric.map_or(0.0, |m| m.success_rate * a.weight as f64);
            let b_score = b_metric.map_or(0.0, |m| m.success_rate * b.weight as f64);
            
            b_score.partial_cmp(&a_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Take top N
        enabled_endpoints.truncate(max_count);
        
        enabled_endpoints
    }
    
    /// Get metrics for all endpoints
    pub fn get_metrics(&self) -> Vec<EndpointMetrics> {
        self.metrics.iter().map(|m| m.clone()).collect()
    }
    
    /// Get metrics for a specific endpoint
    pub fn get_endpoint_metrics(&self, url: &str) -> Option<EndpointMetrics> {
        self.metrics.get(url).map(|m| m.clone())
    }
    
    /// Enable an endpoint
    pub fn enable_endpoint(&self, url: &str) -> bool {
        let mut endpoints = self.endpoints.write();
        
        for endpoint in endpoints.iter_mut() {
            if endpoint.url == url {
                endpoint.enabled = true;
                return true;
            }
        }
        
        false
    }
    
    /// Disable an endpoint
    pub fn disable_endpoint(&self, url: &str) -> bool {
        let mut endpoints = self.endpoints.write();
        
        for endpoint in endpoints.iter_mut() {
            if endpoint.url == url {
                endpoint.enabled = false;
                return true;
            }
        }
        
        false
    }
    
    /// Add a new endpoint
    pub fn add_endpoint(&self, config: EndpointConfig) -> Result<()> {
        let mut endpoints = self.endpoints.write();
        
        // Check if endpoint already exists
        for endpoint in endpoints.iter() {
            if endpoint.url == config.url {
                return Err(anyhow!("Endpoint already exists"));
            }
        }
        
        // Create client
        let client = RpcClient::new_with_timeout(
            config.url.clone(),
            Duration::from_millis(config.timeout_ms),
        );
        
        self.clients.insert(config.url.clone(), Arc::new(client));
        
        // Initialize metrics
        self.metrics.insert(config.url.clone(), EndpointMetrics {
            url: config.url.clone(),
            endpoint_type: config.endpoint_type,
            status: EndpointStatus::Unknown,
            avg_response_time_ms: 0.0,
            success_rate: 1.0, // Start optimistic
            requests_sent: 0,
            successful_responses: 0,
            failed_responses: 0,
            timeouts: 0,
            last_updated: Instant::now(),
        });
        
        // Initialize semaphore
        self.semaphores.insert(
            config.url.clone(),
            Arc::new(Semaphore::new(config.max_concurrent_requests)),
        );
        
        // Add to endpoints
        endpoints.push(config);
        
        Ok(())
    }
    
    /// Remove an endpoint
    pub fn remove_endpoint(&self, url: &str) -> bool {
        let mut endpoints = self.endpoints.write();
        
        let initial_len = endpoints.len();
        endpoints.retain(|e| e.url != url);
        
        if endpoints.len() < initial_len {
            // Remove client, metrics, and semaphore
            self.clients.remove(url);
            self.metrics.remove(url);
            self.semaphores.remove(url);
            
            true
        } else {
            false
        }
    }
}

/// Submission recovery manager
pub struct SubmissionRecoveryManager {
    /// Multi-path submitter
    submitter: Arc<MultiPathSubmitter>,
    
    /// Maximum age of transactions before they're considered stale
    max_age: Duration,
    
    /// Check interval
    check_interval: Duration,
    
    /// Pending transactions
    pending_transactions: Arc<DashMap<Signature, (Transaction, Instant)>>,
    
    /// Shutdown signal
    shutdown: Arc<Mutex<bool>>,
}

impl SubmissionRecoveryManager {
    /// Create a new submission recovery manager
    pub fn new(
        submitter: Arc<MultiPathSubmitter>,
        max_age: Duration,
        check_interval: Duration,
    ) -> Self {
        Self {
            submitter,
            max_age,
            check_interval,
            pending_transactions: Arc::new(DashMap::new()),
            shutdown: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Start the recovery manager
    pub fn start(&self) {
        let pending_transactions = self.pending_transactions.clone();
        let submitter = self.submitter.clone();
        let max_age = self.max_age;
        let check_interval = self.check_interval;
        let shutdown = self.shutdown.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);
            
            loop {
                interval.tick().await;
                
                // Check if shutdown requested
                if *shutdown.lock() {
                    break;
                }
                
                // Get current time
                let now = Instant::now();
                
                // Check pending transactions
                let mut to_retry = Vec::new();
                
                for entry in pending_transactions.iter() {
                    let (signature, (transaction, timestamp)) = entry.pair();
                    
                    // Check if transaction is stale
                    if now.duration_since(*timestamp) > max_age {
                        // Remove stale transaction
                        pending_transactions.remove(signature);
                        continue;
                    }
                    
                    // Add to retry list
                    to_retry.push((signature.clone(), transaction.clone()));
                }
                
                // Retry transactions
                for (signature, transaction) in to_retry {
                    match submitter.submit_transaction(&transaction, None).await {
                        Ok(_) => {
                            // Transaction succeeded, remove from pending
                            pending_transactions.remove(&signature);
                        }
                        Err(_) => {
                            // Transaction still failing, keep in pending
                        }
                    }
                }
            }
        });
    }
    
    /// Stop the recovery manager
    pub fn stop(&self) {
        let mut shutdown = self.shutdown.lock();
        *shutdown = true;
    }
    
    /// Add a transaction to the recovery manager
    pub fn add_transaction(&self, transaction: Transaction) -> Signature {
        let signature = transaction.signatures[0];
        
        self.pending_transactions.insert(signature, (transaction, Instant::now()));
        
        signature
    }
    
    /// Remove a transaction from the recovery manager
    pub fn remove_transaction(&self, signature: &Signature) -> bool {
        self.pending_transactions.remove(signature).is_some()
    }
    
    /// Get the number of pending transactions
    pub fn pending_count(&self) -> usize {
        self.pending_transactions.len()
    }
}

/// Endpoint selector for dynamic endpoint selection
pub struct EndpointSelector {
    /// Multi-path submitter
    submitter: Arc<MultiPathSubmitter>,
    
    /// Update interval
    update_interval: Duration,
    
    /// Endpoint rankings
    rankings: Arc<RwLock<Vec<(String, f64)>>>,
    
    /// Shutdown signal
    shutdown: Arc<Mutex<bool>>,
}

impl EndpointSelector {
    /// Create a new endpoint selector
    pub fn new(
        submitter: Arc<MultiPathSubmitter>,
        update_interval: Duration,
    ) -> Self {
        let rankings = Arc::new(RwLock::new(Vec::new()));
        let shutdown = Arc::new(Mutex::new(false));
        
        let selector = Self {
            submitter,
            update_interval,
            rankings,
            shutdown,
        };
        
        selector.start();
        
        selector
    }
    
    /// Start the endpoint selector
    fn start(&self) {
        let submitter = self.submitter.clone();
        let rankings = self.rankings.clone();
        let update_interval = self.update_interval;
        let shutdown = self.shutdown.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(update_interval);
            
            loop {
                interval.tick().await;
                
                // Check if shutdown requested
                if *shutdown.lock() {
                    break;
                }
                
                // Get metrics
                let metrics = submitter.get_metrics();
                
                // Calculate rankings
                let mut new_rankings = Vec::new();
                
                for metric in metrics {
                    // Skip down endpoints
                    if metric.status == EndpointStatus::Down {
                        continue;
                    }
                    
                    // Calculate score based on response time and success rate
                    let response_time_score = if metric.avg_response_time_ms > 0.0 {
                        1000.0 / metric.avg_response_time_ms
                    } else {
                        0.0
                    };
                    
                    let success_score = metric.success_rate * 10.0;
                    
                    let score = response_time_score + success_score;
                    
                    new_rankings.push((metric.url, score));
                }
                
                // Sort by score (descending)
                new_rankings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                
                // Update rankings
                *rankings.write() = new_rankings;
            }
        });
    }
    
    /// Stop the endpoint selector
    pub fn stop(&self) {
        let mut shutdown = self.shutdown.lock();
        *shutdown = true;
    }
    
    /// Get the best endpoint
    pub fn get_best_endpoint(&self) -> Option<String> {
        let rankings = self.rankings.read();
        
        rankings.first().map(|(url, _)| url.clone())
    }
    
    /// Get the top N endpoints
    pub fn get_top_endpoints(&self, n: usize) -> Vec<String> {
        let rankings = self.rankings.read();
        
        rankings.iter()
            .take(n)
            .map(|(url, _)| url.clone())
            .collect()
    }
    
    /// Get all ranked endpoints
    pub fn get_all_ranked_endpoints(&self) -> Vec<(String, f64)> {
        let rankings = self.rankings.read();
        
        rankings.clone()
    }
}