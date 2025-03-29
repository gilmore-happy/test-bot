//! Endpoint management for RPC client
//!
//! This module provides functionality for managing RPC endpoints,
//! including health checks, latency tracking, and endpoint selection.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use futures::{future::join_all, StreamExt};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use solana_client::{
    client_error::ClientError,
    nonblocking::rpc_client::RpcClient,
};
use solana_sdk::clock::Slot;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{debug, error, info, trace, warn};
use anyhow::{anyhow, Result};
use rand::{thread_rng, Rng};

use crate::config::{EndpointConfig, RpcConfig};
use crate::metrics::RpcMetrics;

/// Endpoint selection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointSelectionStrategy {
    /// Round-robin selection
    RoundRobin,
    
    /// Random selection
    Random,
    
    /// Weighted random selection based on latency
    WeightedRandom,
    
    /// Lowest latency selection
    LowestLatency,
    
    /// Highest success rate selection
    HighestSuccessRate,
    
    /// Lowest load selection
    LowestLoad,
    
    /// Geographically closest selection
    GeographicallyClosest,
}

impl Default for EndpointSelectionStrategy {
    fn default() -> Self {
        Self::WeightedRandom
    }
}

/// RPC endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    /// URL of the endpoint
    pub url: String,
    
    /// Weight for load balancing (higher = more traffic)
    pub weight: u32,
    
    /// Geographic region of the endpoint
    pub region: String,
    
    /// Whether the endpoint is enabled
    pub enabled: bool,
    
    /// Whether the endpoint is healthy
    pub healthy: bool,
    
    /// Endpoint features
    pub features: Vec<String>,
    
    /// Endpoint tier (free, paid, etc.)
    pub tier: String,
}

/// Status of an endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointStatus {
    /// URL of the endpoint
    pub url: String,
    
    /// Whether the endpoint is enabled
    pub enabled: bool,
    
    /// Whether the endpoint is healthy
    pub healthy: bool,
    
    /// Current latency in milliseconds
    pub avg_latency_ms: f64,
    
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
    
    /// Requests per second
    pub requests_per_second: f64,
    
    /// Current slot
    pub current_slot: Option<Slot>,
    
    /// Slot lag behind the network
    pub slot_lag: Option<u64>,
    
    /// Consecutive errors
    pub consecutive_errors: u32,
    
    /// Last error message
    pub last_error: Option<String>,
    
    /// Last checked timestamp
    pub last_checked: chrono::DateTime<chrono::Utc>,
}

/// Endpoint statistics
#[derive(Debug, Default)]
struct EndpointStats {
    /// Total requests
    requests: u64,
    
    /// Successful requests
    successes: u64,
    
    /// Failed requests
    failures: u64,
    
    /// Timeout requests
    timeouts: u64,
    
    /// Recent latencies (microseconds)
    recent_latencies: VecDeque<u64>,
    
    /// Current slot
    current_slot: Option<Slot>,
    
    /// Consecutive errors
    consecutive_errors: u32,
    
    /// Last error message
    last_error: Option<String>,
    
    /// Last checked timestamp
    last_checked: chrono::DateTime<chrono::Utc>,
}

/// Endpoint manager for dynamic endpoint selection
pub struct EndpointManager {
    /// List of available endpoints
    endpoints: RwLock<Vec<Endpoint>>,
    
    /// Endpoint statistics
    stats: DashMap<String, EndpointStats>,
    
    /// Configuration
    config: RpcConfig,
    
    /// Current selection strategy
    selection_strategy: RwLock<EndpointSelectionStrategy>,
    
    /// Current round-robin index
    round_robin_index: Mutex<usize>,
    
    /// Metrics collector
    metrics: Arc<RpcMetrics>,
    
    /// Shutdown signal sender
    shutdown_tx: Mutex<Option<broadcast::Sender<()>>>,
}

impl EndpointManager {
    /// Create a new endpoint manager
    pub async fn new(config: RpcConfig, metrics: Arc<RpcMetrics>) -> Result<Self> {
        let mut endpoints = Vec::new();
        
        // Add configured endpoints
        for endpoint_config in &config.endpoints {
            endpoints.push(Endpoint {
                url: endpoint_config.url.clone(),
                weight: endpoint_config.weight,
                region: endpoint_config.region.clone(),
                enabled: true,
                healthy: true,
                features: endpoint_config.features.clone(),
                tier: endpoint_config.tier.clone(),
            });
        }
        
        if endpoints.is_empty() {
            return Err(anyhow!("No endpoints configured"));
        }
        
        let (shutdown_tx, _) = broadcast::channel(1);
        
        let manager = Self {
            endpoints: RwLock::new(endpoints),
            stats: DashMap::new(),
            config,
            selection_strategy: RwLock::new(EndpointSelectionStrategy::WeightedRandom),
            round_robin_index: Mutex::new(0),
            metrics,
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
        };
        
        // Initialize stats for all endpoints
        let endpoints = manager.endpoints.read().clone();
        for endpoint in &endpoints {
            manager.stats.insert(endpoint.url.clone(), EndpointStats {
                last_checked: chrono::Utc::now(),
                ..Default::default()
            });
        }
        
        // Perform initial health check
        manager.update_endpoint_health().await?;
        
        Ok(manager)
    }
    
    /// Get the list of endpoints
    pub async fn get_endpoints(&self) -> Vec<Endpoint> {
        self.endpoints.read().clone()
    }
    
    /// Get the current selection strategy
    pub async fn get_selection_strategy(&self) -> EndpointSelectionStrategy {
        *self.selection_strategy.read()
    }
    
    /// Set the selection strategy
    pub async fn set_selection_strategy(&self, strategy: EndpointSelectionStrategy) {
        *self.selection_strategy.write() = strategy;
    }
    
    /// Update the health status of all endpoints
    pub async fn update_endpoint_health(&self) -> Result<()> {
        debug!("Updating endpoint health");
        
        let endpoints = self.endpoints.read().clone();
        let mut futures = Vec::new();
        
        // Check health of all endpoints concurrently
        for endpoint in &endpoints {
            if !endpoint.enabled {
                continue;
            }
            
            let url = endpoint.url.clone();
            let future = tokio::spawn(async move {
                let client = RpcClient::new(url.clone());
                let start = Instant::now();
                let result = client.get_version().await;
                let latency = start.elapsed();
                (url, result, latency)
            });
            
            futures.push(future);
        }
        
        // Process results
        let results = join_all(futures).await;
        let mut updates = Vec::new();
        
        for result in results {
            match result {
                Ok((url, Ok(_version), latency)) => {
                    // Endpoint is healthy
                    debug!("Endpoint {} is healthy, latency: {:?}", url, latency);
                    
                    if let Some(mut stats) = self.stats.get_mut(&url) {
                        stats.successes += 1;
                        stats.consecutive_errors = 0;
                        stats.last_checked = chrono::Utc::now();
                        
                        // Add latency to recent_latencies
                        let latency_us = latency.as_micros() as u64;
                        stats.recent_latencies.push_back(latency_us);
                        
                        // Keep only the most recent latencies
                        while stats.recent_latencies.len() > 100 {
                            stats.recent_latencies.pop_front();
                        }
                    }
                    
                    updates.push((url, true));
                },
                Ok((url, Err(err), _)) => {
                    // Endpoint is unhealthy
                    warn!("Endpoint {} is unhealthy: {}", url, err);
                    
                    if let Some(mut stats) = self.stats.get_mut(&url) {
                        stats.failures += 1;
                        stats.consecutive_errors += 1;
                        stats.last_checked = chrono::Utc::now();
                        stats.last_error = Some(err.to_string());
                    }
                    
                    updates.push((url, false));
                },
                Err(err) => {
                    error!("Failed to check endpoint health: {}", err);
                },
            }
        }
        
        // Update endpoint status
        {
            let mut endpoints = self.endpoints.write();
            
            for (url, healthy) in updates {
                if let Some(endpoint) = endpoints.iter_mut().find(|e| e.url == url) {
                    endpoint.healthy = healthy;
                }
            }
        }
        
        Ok(())
    }
    
    /// Get the best endpoint for a request
    pub async fn get_best_endpoint(&self) -> Option<Endpoint> {
        let endpoints = self.endpoints.read();
        
        // Filter enabled and healthy endpoints
        let active_endpoints: Vec<_> = endpoints.iter()
            .filter(|e| e.enabled && e.healthy)
            .collect();
        
        if active_endpoints.is_empty() {
            // Fall back to any enabled endpoint
            let enabled_endpoints: Vec<_> = endpoints.iter()
                .filter(|e| e.enabled)
                .collect();
            
            if enabled_endpoints.is_empty() {
                return None;
            }
            
            return Some(enabled_endpoints[0].clone());
        }
        
        // Select based on strategy
        match *self.selection_strategy.read() {
            EndpointSelectionStrategy::RoundRobin => {
                let mut index = self.round_robin_index.lock();
                let endpoint = active_endpoints[*index % active_endpoints.len()].clone();
                *index = (*index + 1) % active_endpoints.len();
                Some(endpoint)
            },
            EndpointSelectionStrategy::Random => {
                let mut rng = thread_rng();
                let index = rng.gen_range(0..active_endpoints.len());
                Some(active_endpoints[index].clone())
            },
            EndpointSelectionStrategy::WeightedRandom => {
                // Weighted selection based on latency and success rate
                let mut candidates = Vec::new();
                let mut total_score = 0.0;
                
                for endpoint in &active_endpoints {
                    if let Some(stats) = self.stats.get(&endpoint.url) {
                        // Calculate average latency
                        let avg_latency = if stats.recent_latencies.is_empty() {
                            100_000.0 // Default to 100ms if no data
                        } else {
                            stats.recent_latencies.iter().sum::<u64>() as f64 / stats.recent_latencies.len() as f64
                        };
                        
                        // Calculate success rate
                        let total = stats.successes + stats.failures + stats.timeouts;
                        let success_rate = if total == 0 {
                            0.5 // Default to 50% if no data
                        } else {
                            stats.successes as f64 / total as f64
                        };
                        
                        // Calculate score (higher is better)
                        // We want lower latency and higher success rate
                        let latency_score = 1.0 / (1.0 + avg_latency / 1000.0); // Normalize to 0-1 range
                        let score = latency_score * success_rate * endpoint.weight as f64;
                        
                        candidates.push((endpoint, score));
                        total_score += score;
                    }
                }
                
                if candidates.is_empty() {
                    // Fall back to first active endpoint
                    active_endpoints.first().map(|e| e.clone())
                } else {
                    // Weighted random selection
                    let mut rng = thread_rng();
                    let threshold = rng.gen::<f64>() * total_score;
                    
                    let mut cumulative_score = 0.0;
                    for (endpoint, score) in candidates {
                        cumulative_score += score;
                        if cumulative_score >= threshold {
                            return Some(endpoint.clone());
                        }
                    }
                    
                    // Fallback to first candidate
                    candidates.first().map(|(e, _)| (*e).clone())
                }
            },
            EndpointSelectionStrategy::LowestLatency => {
                // Select endpoint with lowest latency
                let mut best_endpoint = None;
                let mut best_latency = f64::MAX;
                
                for endpoint in &active_endpoints {
                    if let Some(stats) = self.stats.get(&endpoint.url) {
                        if stats.recent_latencies.is_empty() {
                            continue;
                        }
                        
                        let avg_latency = stats.recent_latencies.iter().sum::<u64>() as f64 / stats.recent_latencies.len() as f64;
                        
                        if avg_latency < best_latency {
                            best_latency = avg_latency;
                            best_endpoint = Some(endpoint.clone());
                        }
                    }
                }
                
                best_endpoint.or_else(|| active_endpoints.first().map(|e| e.clone()))
            },
            EndpointSelectionStrategy::HighestSuccessRate => {
                // Select endpoint with highest success rate
                let mut best_endpoint = None;
                let mut best_success_rate = 0.0;
                
                for endpoint in &active_endpoints {
                    if let Some(stats) = self.stats.get(&endpoint.url) {
                        let total = stats.successes + stats.failures + stats.timeouts;
                        if total == 0 {
                            continue;
                        }
                        
                        let success_rate = stats.successes as f64 / total as f64;
                        
                        if success_rate > best_success_rate {
                            best_success_rate = success_rate;
                            best_endpoint = Some(endpoint.clone());
                        }
                    }
                }
                
                best_endpoint.or_else(|| active_endpoints.first().map(|e| e.clone()))
            },
            EndpointSelectionStrategy::LowestLoad => {
                // Select endpoint with lowest load (requests per second)
                let mut best_endpoint = None;
                let mut best_load = f64::MAX;
                
                for endpoint in &active_endpoints {
                    if let Some(stats) = self.stats.get(&endpoint.url) {
                        // Calculate requests per second (crude approximation)
                        let load = (stats.successes + stats.failures + stats.timeouts) as f64 / 60.0;
                        
                        if load < best_load {
                            best_load = load;
                            best_endpoint = Some(endpoint.clone());
                        }
                    }
                }
                
                best_endpoint.or_else(|| active_endpoints.first().map(|e| e.clone()))
            },
            EndpointSelectionStrategy::GeographicallyClosest => {
                // In a real implementation, this would use geolocation
                // For now, just use the first endpoint in the preferred region
                let preferred_region = "us-east"; // This would be configurable
                
                active_endpoints.iter()
                    .find(|e| e.region == preferred_region)
                    .map(|e| e.clone())
                    .or_else(|| active_endpoints.first().map(|e| e.clone()))
            },
        }
    }
    
    /// Record a successful request to an endpoint
    pub async fn record_success(&self, url: &str, latency: Duration) {
        if let Some(mut stats) = self.stats.get_mut(url) {
            stats.successes += 1;
            stats.consecutive_errors = 0;
            
            // Add latency to recent_latencies
            let latency_us = latency.as_micros() as u64;
            stats.recent_latencies.push_back(latency_us);
            
            // Keep only the most recent latencies
            while stats.recent_latencies.len() > 100 {
                stats.recent_latencies.pop_front();
            }
        }
    }
    
    /// Record an error from an endpoint
    pub async fn record_error(&self, url: &str, error: Option<String>) {
        if let Some(mut stats) = self.stats.get_mut(url) {
            stats.failures += 1;
            stats.consecutive_errors += 1;
            
            if let Some(error) = error {
                stats.last_error = Some(error);
            }
        }
    }
    
    /// Record a timeout from an endpoint
    pub async fn record_timeout(&self, url: &str) {
        if let Some(mut stats) = self.stats.get_mut(url) {
            stats.timeouts += 1;
            stats.consecutive_errors += 1;
        }
    }
    
    /// Update the current slot for an endpoint
    pub async fn update_endpoint_slot(&self, url: &str, slot: Slot) {
        if let Some(mut stats) = self.stats.get_mut(url) {
            stats.current_slot = Some(slot);
        }
    }
    
    /// Enable an endpoint
    pub async fn enable_endpoint(&self, url: &str) {
        let mut endpoints = self.endpoints.write();
        
        if let Some(endpoint) = endpoints.iter_mut().find(|e| e.url == url) {
            endpoint.enabled = true;
            info!("Enabled endpoint: {}", url);
        }
    }
    
    /// Disable an endpoint
    pub async fn disable_endpoint(&self, url: &str) {
        let mut endpoints = self.endpoints.write();
        
        if let Some(endpoint) = endpoints.iter_mut().find(|e| e.url == url) {
            endpoint.enabled = false;
            info!("Disabled endpoint: {}", url);
        }
    }
    
    /// Add a new endpoint
    pub async fn add_endpoint(&self, config: EndpointConfig) -> Result<()> {
        let url = config.url.clone();
        
        // Check if endpoint already exists
        {
            let endpoints = self.endpoints.read();
            if endpoints.iter().any(|e| e.url == url) {
                return Err(anyhow!("Endpoint already exists: {}", url));
            }
        }
        
        // Add the endpoint
        {
            let mut endpoints = self.endpoints.write();
            endpoints.push(Endpoint {
                url: url.clone(),
                weight: config.weight,
                region: config.region.clone(),
                enabled: true,
                healthy: true,
                features: config.features.clone(),
                tier: config.tier.clone(),
            });
        }
        
        // Initialize stats
        self.stats.insert(url.clone(), EndpointStats {
            last_checked: chrono::Utc::now(),
            ..Default::default()
        });
        
        info!("Added endpoint: {}", url);
        
        Ok(())
    }
    
    /// Remove an endpoint
    pub async fn remove_endpoint(&self, url: &str) -> Result<()> {
        // Remove the endpoint
        {
            let mut endpoints = self.endpoints.write();
            let initial_len = endpoints.len();
            endpoints.retain(|e| e.url != url);
            
            if endpoints.len() == initial_len {
                return Err(anyhow!("Endpoint not found: {}", url));
            }
        }
        
        // Remove stats
        self.stats.remove(url);
        
        info!("Removed endpoint: {}", url);
        
        Ok(())
    }
    
    /// Get the current status of all endpoints
    pub async fn get_endpoint_status(&self) -> Vec<EndpointStatus> {
        let endpoints = self.endpoints.read();
        let mut status = Vec::with_capacity(endpoints.len());
        
        for endpoint in endpoints.iter() {
            if let Some(stats) = self.stats.get(&endpoint.url) {
                // Calculate average latency
                let avg_latency = if stats.recent_latencies.is_empty() {
                    0.0
                } else {
                    stats.recent_latencies.iter().sum::<u64>() as f64 / stats.recent_latencies.len() as f64 / 1000.0 // Convert to ms
                };
                
                // Calculate success rate
                let total = stats.successes + stats.failures + stats.timeouts;
                let success_rate = if total == 0 {
                    1.0
                } else {
                    stats.successes as f64 / total as f64
                };
                
                // Calculate requests per second
                let requests_per_second = (stats.successes + stats.failures + stats.timeouts) as f64 / 60.0;
                
                // Calculate slot lag
                let slot_lag = if let Some(current_slot) = stats.current_slot {
                    // Find the highest slot across all endpoints
                    let highest_slot = self.stats.iter()
                        .filter_map(|entry| entry.current_slot)
                        .max()
                        .unwrap_or(current_slot);
                    
                    if highest_slot > current_slot {
                        Some(highest_slot - current_slot)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                status.push(EndpointStatus {
                    url: endpoint.url.clone(),
                    enabled: endpoint.enabled,
                    healthy: endpoint.healthy,
                    avg_latency_ms: avg_latency,
                    success_rate,
                    requests_per_second,
                    current_slot: stats.current_slot,
                    slot_lag,
                    consecutive_errors: stats.consecutive_errors,
                    last_error: stats.last_error.clone(),
                    last_checked: stats.last_checked,
                });
            }
        }
        
        status
    }
    
    /// Shutdown the endpoint manager
    pub async fn shutdown(&self) {
        info!("Shutting down endpoint manager");
        
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.lock().take() {
            let _ = tx.send(());
        }
    }
}
