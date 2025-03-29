//! High-performance RPC client for Solana
//!
//! This module provides optimized RPC access to Solana validators with features like:
//! - Dynamic endpoint selection based on latency and health
//! - Connection pooling and keep-alive optimization
//! - Request batching and pipelining
//! - Automatic failover and load balancing
//! - Advanced caching with prefetching
//! - Adaptive rate limiting
//! - Request prioritization
//! - Comprehensive metrics and monitoring

#![allow(unused_imports)]
#![feature(async_fn_in_trait)]

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::{Duration, Instant};
use std::net::{IpAddr, SocketAddr};
use std::fmt;

use dashmap::DashMap;
use futures::{future::{Either, join_all}, stream::FuturesUnordered, StreamExt};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_client::{
    client_error::ClientError,
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcSendTransactionConfig, RpcTransactionConfig, RpcContextConfig, 
                RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_request::{RpcRequest, RpcResponseErrorData},
    rpc_response::{Response as RpcResponse, RpcResponseContext},
};
use solana_sdk::{
    account::Account,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    hash::Hash,
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
    clock::Slot,
    program_pack::Pack,
};
use solana_transaction_status::{TransactionStatus, UiTransactionEncoding};
use tokio::sync::{mpsc, oneshot, RwLock as TokioRwLock, Semaphore, broadcast};
use tokio::time::{interval, sleep, timeout};
use tracing::{debug, error, info, instrument, trace, warn, span, Level};
use anyhow::{anyhow, Context as AnyhowContext, Result};
use rand::{thread_rng, Rng};
use backoff::{ExponentialBackoff, backoff::Backoff};

mod cache;
mod config;
mod endpoints;
mod metrics;
mod monitor;
mod proxy;
mod rate_limiter;
mod request;
mod websocket;
mod batch;

pub use config::{RpcConfig, EndpointConfig, BatchingConfig, RetryConfig};
pub use endpoints::{Endpoint, EndpointManager, EndpointStatus, EndpointSelectionStrategy};
pub use proxy::RpcProxy;
pub use rate_limiter::{RpcRateLimiter, RateLimitStrategy, RateLimiterStats};
pub use cache::{ResponseCache, CacheStats};
pub use metrics::{RpcMetrics, RpcMetricsSnapshot};
pub use monitor::RpcMonitor;
pub use websocket::{WebsocketSubscription, WebsocketManager};
pub use batch::{BatchProcessor, BatchRequest, BatchResponse};
pub use request::{RequestPriority, RequestContext, RequestOptions};

/// Result type for the RPC module
pub type RpcResult<T> = std::result::Result<T, RpcError>;

/// Error types for the RPC module
#[derive(thiserror::Error, Debug, Clone)]
pub enum RpcError {
    #[error("RPC call failed: {0}")]
    Call(String),
    
    #[error("Client error: {0}")]
    Client(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("IO error: {0}")]
    Io(String),
    
    #[error("Connection timeout")]
    Timeout,
    
    #[error("Rate limited")]
    RateLimited,
    
    #[error("No available endpoints")]
    NoEndpoints,
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Cache error: {0}")]
    Cache(String),
    
    #[error("Request cancelled")]
    Cancelled,
    
    #[error("Quorum not reached: {0}")]
    QuorumNotReached(String),
    
    #[error("Batch processing error: {0}")]
    BatchProcessing(String),
    
    #[error("Websocket error: {0}")]
    Websocket(String),
    
    #[error("Retry limit exceeded: {0}")]
    RetryLimitExceeded(String),
}

impl From<ClientError> for RpcError {
    fn from(err: ClientError) -> Self {
        RpcError::Client(err.to_string())
    }
}

impl From<serde_json::Error> for RpcError {
    fn from(err: serde_json::Error) -> Self {
        RpcError::Serialization(err.to_string())
    }
}

impl From<std::io::Error> for RpcError {
    fn from(err: std::io::Error) -> Self {
        RpcError::Io(err.to_string())
    }
}

/// Enhanced RPC client with high-performance optimizations
#[derive(Clone)]
pub struct EnhancedRpcClient {
    /// Endpoint manager for dynamic endpoint selection
    endpoint_manager: Arc<EndpointManager>,
    
    /// Cache for RPC responses
    cache: Arc<ResponseCache>,
    
    /// Rate limiter to avoid overloading endpoints
    rate_limiter: Arc<RpcRateLimiter>,
    
    /// Request queue for batching and pipelining
    request_queue: Arc<TokioRwLock<HashMap<RequestPriority, VecDeque<BatchRequest>>>>,
    
    /// Connection pool
    connection_pool: Arc<DashMap<String, Vec<RpcClient>>>,
    
    /// Default commitment level
    commitment: CommitmentConfig,
    
    /// Configuration
    config: RpcConfig,
    
    /// Metrics collector
    metrics: Arc<RpcMetrics>,
    
    /// Request semaphore to limit concurrent requests
    request_semaphore: Arc<Semaphore>,
    
    /// Batch processor
    batch_processor: Arc<BatchProcessor>,
    
    /// Websocket manager
    websocket_manager: Option<Arc<WebsocketManager>>,
    
    /// Monitor
    monitor: Option<Arc<RpcMonitor>>,
    
    /// Shutdown signal sender
    shutdown_tx: Arc<Mutex<Option<broadcast::Sender<()>>>>,
    
    /// Next request ID
    next_request_id: Arc<AtomicU64>,
}

impl EnhancedRpcClient {
    /// Create a new enhanced RPC client with the provided configuration
    pub async fn new(config: RpcConfig) -> Result<Self> {
        info!("Initializing EnhancedRpcClient with config: {:?}", config);
        
        let metrics = Arc::new(RpcMetrics::new());
        
        let endpoint_manager = Arc::new(EndpointManager::new(config.clone(), metrics.clone()).await?);
        
        let cache = Arc::new(ResponseCache::new(config.cache_config.clone()));
        
        let rate_limiter = Arc::new(RpcRateLimiter::new(
            config.rate_limit_requests_per_second,
            config.rate_limit_burst_size,
        ));
        
        // Configure rate limiter from endpoints
        rate_limiter.configure_from_endpoints(&endpoint_manager.get_endpoints().await);
        
        let request_queue = Arc::new(TokioRwLock::new(HashMap::new()));
        
        let connection_pool = Arc::new(DashMap::new());
        
        let request_semaphore = Arc::new(Semaphore::new(config.max_concurrent_requests));
        
        let batch_processor = Arc::new(BatchProcessor::new(
            config.batching_config.clone(),
            metrics.clone(),
        ));
        
        let (shutdown_tx, _) = broadcast::channel(16);
        
        let client = Self {
            endpoint_manager,
            cache,
            rate_limiter,
            request_queue,
            connection_pool,
            commitment: config.commitment_config,
            config: config.clone(),
            metrics,
            request_semaphore,
            batch_processor,
            websocket_manager: None,
            monitor: None,
            shutdown_tx: Arc::new(Mutex::new(Some(shutdown_tx))),
            next_request_id: Arc::new(AtomicU64::new(1)),
        };
        
        // Initialize the connection pool
        client.initialize_connection_pool().await?;
        
        // Initialize websocket manager if enabled
        if config.use_websocket {
            let websocket_manager = WebsocketManager::new(
                config.websocket_endpoints.clone(),
                config.commitment_config,
                client.metrics.clone(),
            ).await?;
            
            client.websocket_manager = Some(Arc::new(websocket_manager));
        }
        
        // Initialize monitor
        let monitor = monitor::RpcMonitor::new(
            client.endpoint_manager.clone(),
            client.metrics.clone(),
            config.endpoint_health_check_interval_ms,
        );
        
        client.monitor = Some(Arc::new(monitor));
        
        // Spawn background workers
        client.spawn_workers();
        
        // Start the monitor
        if let Some(monitor) = &client.monitor {
            monitor.start().await?;
        }
        
        Ok(client)
    }
    
    /// Initialize the connection pool for all endpoints
    async fn initialize_connection_pool(&self) -> Result<()> {
        info!("Initializing connection pool");
        
        let endpoints = self.endpoint_manager.get_endpoints().await;
        
        for endpoint in endpoints {
            let pool_size = self.config.connection_pool_size_per_endpoint;
            let mut pool = Vec::with_capacity(pool_size);
            
            for _ in 0..pool_size {
                let client = RpcClient::new_with_commitment(
                    endpoint.url.clone(),
                    self.commitment,
                );
                pool.push(client);
            }
            
            self.connection_pool.insert(endpoint.url.clone(), pool);
            info!("Added {} connections to pool for endpoint {}", pool_size, endpoint.url);
        }
        
        Ok(())
    }
    
    /// Spawn background workers for various tasks
    fn spawn_workers(&self) {
        self.spawn_request_processor();
        self.spawn_connection_refresher();
        self.spawn_cache_cleaner();
        self.spawn_prefetch_worker();
    }
    
    /// Spawn a worker to process the request queue
    fn spawn_request_processor(&self) {
        let this = self.clone();
        let shutdown_rx = self.shutdown_tx.lock().as_ref().unwrap().subscribe();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(1));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Process requests by priority
                        for priority in [
                            RequestPriority::Critical,
                            RequestPriority::High,
                            RequestPriority::Normal,
                            RequestPriority::Low,
                            RequestPriority::Background,
                        ] {
                            if let Err(e) = this.process_priority_queue(priority).await {
                                error!("Failed to process request queue for priority {:?}: {}", priority, e);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Request processor shutting down");
                        break;
                    }
                }
            }
        });
    }
    
    /// Process requests for a specific priority level
    async fn process_priority_queue(&self, priority: RequestPriority) -> Result<()> {
        // Process up to batch_size requests at once
        let batch_size = self.config.batching_config.max_batch_size;
        let mut requests = Vec::with_capacity(batch_size);
        
        // Get requests from the queue
        {
            let mut queue = self.request_queue.write().await;
            let priority_queue = queue.entry(priority).or_insert_with(VecDeque::new);
            
            while !priority_queue.is_empty() && requests.len() < batch_size {
                if let Some(request) = priority_queue.pop_front() {
                    requests.push(request);
                }
            }
        }
        
        if !requests.is_empty() {
            // Process the batch
            let batch_future = self.batch_processor.process_batch(
                requests,
                self.clone(),
            );
            
            // Spawn the batch processing task
            tokio::spawn(batch_future);
        }
        
        Ok(())
    }
    
    /// Spawn a worker to refresh connections periodically
    fn spawn_connection_refresher(&self) {
        let this = self.clone();
        let interval_ms = this.config.connection_refresh_interval_ms;
        let shutdown_rx = self.shutdown_tx.lock().as_ref().unwrap().subscribe();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = this.refresh_connections().await {
                            error!("Failed to refresh connections: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Connection refresher shutting down");
                        break;
                    }
                }
            }
        });
    }
    
    /// Spawn a worker to clean the cache periodically
    fn spawn_cache_cleaner(&self) {
        let cache = self.cache.clone();
        let shutdown_rx = self.shutdown_tx.lock().as_ref().unwrap().subscribe();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        cache.clean_expired().await;
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Cache cleaner shutting down");
                        break;
                    }
                }
            }
        });
    }
    
    /// Spawn a worker to prefetch frequently accessed data
    fn spawn_prefetch_worker(&self) {
        let this = self.clone();
        let shutdown_rx = self.shutdown_tx.lock().as_ref().unwrap().subscribe();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if this.config.cache_config.enable_prefetching {
                            if let Some(key) = this.cache.get_next_prefetch_key().await {
                                // Handle different prefetch key types
                                if key.starts_with("account:") {
                                    if let Some(pubkey_str) = key.strip_prefix("account:") {
                                        if let Ok(pubkey) = Pubkey::try_from_str(pubkey_str) {
                                            let options = RequestOptions {
                                                priority: RequestPriority::Background,
                                                bypass_cache: true,
                                                ..Default::default()
                                            };
                                            
                                            // Prefetch the account
                                            let _ = this.get_account_with_options(&pubkey, options).await;
                                        }
                                    }
                                } else if key == "latest_blockhash" {
                                    let options = RequestOptions {
                                        priority: RequestPriority::Background,
                                        bypass_cache: true,
                                        ..Default::default()
                                    };
                                    
                                    // Prefetch the latest blockhash
                                    let _ = this.get_latest_blockhash_with_options(options).await;
                                }
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Prefetch worker shutting down");
                        break;
                    }
                }
            }
        });
    }
    
    /// Refresh connections in the pool
    async fn refresh_connections(&self) -> Result<()> {
        debug!("Refreshing connection pool");
        
        // Refresh a portion of connections in each pool
        for mut entry in self.connection_pool.iter_mut() {
            let url = entry.key().clone();
            let pool = entry.value_mut();
            
            // Refresh 25% of connections
            let refresh_count = (pool.len() / 4).max(1);
            
            for i in 0..refresh_count {
                let client = RpcClient::new_with_commitment(
                    url.clone(),
                    self.commitment,
                );
                
                // Replace an existing connection
                if i < pool.len() {
                    pool[i] = client;
                }
            }
        }
        
        Ok(())
    }
    
    /// Get a connection from the pool for the given endpoint
    fn get_connection(&self, endpoint: &str) -> Option<RpcClient> {
        let pool = self.connection_pool.get(endpoint)?;
        
        // Simple round-robin selection
        let index = rand::random::<usize>() % pool.len();
        Some(pool[index].clone())
    }
    
    /// Generate a new request ID
    fn generate_request_id(&self) -> u64 {
        self.next_request_id.fetch_add(1, Ordering)
    }
    
    /// Create a new request context
    fn create_request_context(&self, method: &str, options: &RequestOptions) -> RequestContext {
        let id = self.generate_request_id();
        
        RequestContext {
            id,
            method: method.to_string(),
            priority: options.priority,
            start_time: Instant::now(),
            cacheable: !options.bypass_cache,
            batchable: options.allow_batching,
            requires_quorum: options.require_quorum,
            retry_count: 0,
            max_retries: options.max_retries.unwrap_or(self.config.retry_config.max_retries as u32),
        }
    }
    
    /// Get the best endpoint for a request
    pub async fn get_best_endpoint_for_request(
        &self,
        method: &str,
        options: &RequestOptions,
    ) -> RpcResult<Endpoint> {
        // Get the best endpoint for this request
        match &options.specific_endpoint {
            Some(url) => {
                // Find the endpoint with the specified URL
                let endpoints = self.endpoint_manager.get_endpoints().await;
                endpoints.iter()
                    .find(|e| e.url == *url && e.enabled)
                    .cloned()
                    .ok_or(RpcError::NoEndpoints)
            },
            None => {
                // Use the endpoint selection strategy
                let strategy = if let Some(s) = options.selection_strategy {
                    s
                } else {
                    self.endpoint_manager.get_selection_strategy().await
                };
                
                self.endpoint_manager.get_best_endpoint().await
                    .ok_or(RpcError::NoEndpoints)
            }
        }
    }
    
    /// Execute a batch request
    pub async fn execute_batch_request(
        &self,
        endpoint_url: &str,
        method: &str,
        params: Vec<Value>,
        options: RequestOptions,
    ) -> RpcResult<Vec<Value>> {
        let start = Instant::now();
        let request_id = self.generate_request_id();
        
        self.metrics.record_request_start(request_id, method);
        
        // Get a connection from the pool
        let client = self.get_connection(endpoint_url)
            .ok_or_else(|| RpcError::Call(format!("No connection available for {}", endpoint_url)))?;
        
        // Build batch request
        let mut batch_requests = Vec::with_capacity(params.len());
        
        for (i, param) in params.iter().enumerate() {
            let request = json!({
                "jsonrpc": "2.0",
                "id": i + 1,
                "method": method,
                "params": param,
            });
            
            batch_requests.push(request);
        }
        
        // Send batch request
        let timeout_ms = options.timeout_ms.unwrap_or(self.config.request_timeout_ms);
        
        let result = match timeout(
            Duration::from_millis(timeout_ms),
            client.send_batch(batch_requests)
        ).await {
            Ok(Ok(responses)) => {
                // Update endpoint stats
                self.endpoint_manager.record_success(endpoint_url, start.elapsed()).await;
                
                // Extract results
                let mut results = Vec::with_capacity(responses.len());
                
                for response in responses {
                    if let Some(result) = response.get("result") {
                        results.push(result.clone());
                    } else if let Some(error) = response.get("error") {
                        return Err(RpcError::Call(format!("Batch request error: {}", error)));
                    } else {
                        return Err(RpcError::Call("Invalid batch response".to_string()));
                    }
                }
                
                self.metrics.record_request_success(request_id, start.elapsed());
                Ok(results)
            },
            Ok(Err(err)) => {
                // Update endpoint stats
                self.endpoint_manager.record_error(endpoint_url, Some(err.to_string())).await;
                
                self.metrics.record_request_error(request_id, &err.to_string());
                Err(RpcError::Client(err.to_string()))
            },
            Err(_) => {
                // Timeout
                self.endpoint_manager.record_timeout(endpoint_url).await;
                
                self.metrics.record_request_timeout(request_id);
                Err(RpcError::Timeout)
            },
        };
        
        result
    }
    
    /// Execute a request with retries and error handling
    async fn execute_request<T, F, Fut>(&self, 
        method: &str, 
        options: RequestOptions,
        request_fn: F
    ) -> RpcResult<T> 
    where
        F: Fn(RequestContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = RpcResult<T>> + Send + 'static,
        T: Send + Sync + 'static + Clone,
    {
        let context = self.create_request_context(method, &options);
        let max_retries = context.max_retries;
        
        let mut backoff = ExponentialBackoff {
            initial_interval: Duration::from_millis(self.config.retry_config.base_delay_ms),
            max_interval: Duration::from_millis(self.config.retry_config.max_delay_ms),
            max_elapsed_time: Some(Duration::from_millis(self.config.request_timeout_ms * 2)),
            ..Default::default()
        };
        
        let mut context = context;
        let mut last_error = None;
        
        loop {
            match request_fn(context.clone()).await {
                Ok(result) => {
                    return Ok(result);
                }
                Err(err) => {
                    // Check if we should retry based on error type
                    let should_retry = match &err {
                        RpcError::Timeout => self.config.retry_config.retry_on_timeout,
                        RpcError::RateLimited => self.config.retry_config.retry_on_rate_limit,
                        RpcError::Client(_) => self.config.retry_config.retry_on_connection_error,
                        _ => false,
                    };
                    
                    if !should_retry || context.retry_count >= max_retries {
                        if context.retry_count > 0 {
                            return Err(RpcError::RetryLimitExceeded(format!(
                                "Failed after {} retries: {}", context.retry_count, err
                            )));
                        }
                        return Err(err);
                    }
                    
                    // Increment retry count
                    context.retry_count += 1;
                    last_error = Some(err);
                    
                    // Calculate backoff duration
                    let backoff_duration = if self.config.retry_config.use_exponential_backoff {
                        match backoff.next_backoff() {
                            Some(duration) => duration,
                            None => break,
                        }
                    } else {
                        Duration::from_millis(self.config.retry_config.base_delay_ms)
                    };
                    
                    // Log retry attempt
                    debug!(
                        "Retrying request {} ({}) after error, attempt {}/{}, backoff: {:?}",
                        context.id, method, context.retry_count, max_retries, backoff_duration
                    );
                    
                    // Wait before retrying
                    sleep(backoff_duration).await;
                }
            }
        }
        
        // If we get here, we've exhausted retries
        Err(last_error.unwrap_or_else(|| RpcError::RetryLimitExceeded(
            format!("Failed after {} retries", max_retries)
        )))
    }
    
    /// Send a transaction with optimized settings
    pub async fn send_transaction(&self, transaction: &Transaction) -> RpcResult<Signature> {
        self.send_transaction_with_options(transaction, RequestOptions::default()).await
    }
    
    /// Send a transaction with custom options
    pub async fn send_transaction_with_options(
        &self, 
        transaction: &Transaction,
        options: RequestOptions,
    ) -> RpcResult<Signature> {
        let method = "sendTransaction";
        let transaction = transaction.clone();
        
        self.execute_request(method, options.clone(), move |context| {
            let this = self.clone();
            let transaction = transaction.clone();
            
            async move {
                let start = context.start_time;
                let request_id = context.id;
                
                this.metrics.record_request_start(request_id, method);
                
                // Get the best endpoint for this request
                let endpoint = this.get_best_endpoint_for_request(method, &options).await?;
                
                // Apply rate limiting unless bypassed
                if !options.bypass_rate_limit {
                    if !this.rate_limiter.try_acquire_with_cost(&endpoint.url, method, 3.0).await {
                        this.metrics.record_rate_limited(request_id);
                        return Err(RpcError::RateLimited);
                    }
                }
                
                // Acquire a request permit
                let timeout_ms = options.timeout_ms.unwrap_or(this.config.request_timeout_ms);
                let permit_timeout = timeout(
                    Duration::from_millis(timeout_ms),
                    this.request_semaphore.acquire()
                ).await;
                
                let _permit = match permit_timeout {
                    Ok(permit) => permit.map_err(|e| RpcError::Call(format!("Failed to acquire permit: {}", e)))?,
                    Err(_) => {
                        this.metrics.record_request_timeout(request_id);
                        return Err(RpcError::Timeout);
                    }
                };
                
                // Get a connection from the pool
                let client = this.get_connection(&endpoint.url)
                    .ok_or_else(|| RpcError::Call(format!("No connection available for {}", endpoint.url)))?;
                
                // Configure optimized transaction settings
                let config = RpcSendTransactionConfig {
                    skip_preflight: this.config.skip_preflight,
                    preflight_commitment: Some(this.commitment.commitment),
                    encoding: Some(UiTransactionEncoding::Base64),
                    max_retries: Some(this.config.send_transaction_retry_count),
                    min_context_slot: None,
                };
                
                // Send the transaction
                let result = match timeout(
                    Duration::from_millis(timeout_ms),
                    client.send_transaction_with_config(&transaction, config)
                ).await {
                    Ok(Ok(signature)) => {
                        // Update endpoint stats
                        this.endpoint_manager.record_success(&endpoint.url, start.elapsed()).await;
                        
                        this.metrics.record_request_success(request_id, start.elapsed());
                        Ok(signature)
                    },
                    Ok(Err(err)) => {
                        // Update endpoint stats
                        this.endpoint_manager.record_error(&endpoint.url, Some(err.to_string())).await;
                        
                        this.metrics.record_request_error(request_id, &err.to_string());
                        Err(RpcError::Client(err.to_string()))
                    },
                    Err(_) => {
                        // Timeout
                        this.endpoint_manager.record_timeout(&endpoint.url).await;
                        
                        this.metrics.record_request_timeout(request_id);
                        Err(RpcError::Timeout)
                    },
                };
                
                result
            }
        }).await
    }
    
    /// Get account information with caching
    pub async fn get_account(&self, pubkey: &Pubkey) -> RpcResult<Option<Account>> {
        self.get_account_with_options(pubkey, RequestOptions::default()).await
    }
    
    /// Get account information with custom options
    pub async fn get_account_with_options(
        &self, 
        pubkey: &Pubkey,
        options: RequestOptions,
    ) -> RpcResult<Option<Account>> {
        let method = "getAccountInfo";
        let pubkey = *pubkey;
        
        self.execute_request(method, options.clone(), move |context| {
            let this = self.clone();
            
            async move {
                let start = context.start_time;
                let request_id = context.id;
                
                this.metrics.record_request_start(request_id, method);
                
                // Check cache first if enabled and not bypassed
                if this.config.cache_config.enable_cache && !options.bypass_cache && context.cacheable {
                    let cache_key = format!("account:{}", pubkey);
                    if let Some(cached) = this.cache.get(&cache_key).await {
                        if let Ok(account) = serde_json::from_value(cached) {
                            this.metrics.record_cache_hit(request_id);
                            return Ok(account);
                        }
                    }
                }
                
                // Get the best endpoint for this request
                let endpoint = this.get_best_endpoint_for_request(method, &options).await?;
                
                // Apply rate limiting unless bypassed
                if !options.bypass_rate_limit {
                    if !this.rate_limiter.try_acquire_with_cost(&endpoint.url, method, 1.0).await {
                        this.metrics.record_rate_limited(request_id);
                        return Err(RpcError::RateLimited);
                    }
                }
                
                // Acquire a request permit
                let timeout_ms = options.timeout_ms.unwrap_or(this.config.request_timeout_ms);
                let permit_timeout = timeout(
                    Duration::from_millis(timeout_ms),
                    this.request_semaphore.acquire()
                ).await;
                
                let _permit = match permit_timeout {
                    Ok(permit) => permit.map_err(|e| RpcError::Call(format!("Failed to acquire permit: {}", e)))?,
                    Err(_) => {
                        this.metrics.record_request_timeout(request_id);
                        return Err(RpcError::Timeout);
                    }
                };
                
                // Get a connection from the pool
                let client = this.get_connection(&endpoint.url)
                    .ok_or_else(|| RpcError::Call(format!("No connection available for {}", endpoint.url)))?;
                
                // Configure commitment
                let commitment = match options.min_commitment {
                    Some(level) => CommitmentConfig { commitment: level },
                    None => this.commitment,
                };
                
                // Create account config
                let config = RpcAccountInfoConfig {
                    encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                    commitment: Some(commitment),
                    data_slice: None,
                    min_context_slot: None,
                };
                
                // Send the request
                let result = match timeout(
                    Duration::from_millis(timeout_ms),
                    client.get_account_with_config(&pubkey, config)
                ).await {
                    Ok(Ok(account)) => {
                        // Update endpoint stats
                        this.endpoint_manager.record_success(&endpoint.url, start.elapsed()).await;
                        
                        // Cache the result if cacheable
                        if this.config.cache_config.enable_cache && context.cacheable {
                            let cache_key = format!("account:{}", pubkey);
                            if let Ok(json_value) = serde_json::to_value(&Some(account.clone())) {
                                this.cache.set(&cache_key, json_value, this.config.cache_config.account_ttl_ms).await;
                            }
                        }
                        
                        this.metrics.record_request_success(request_id, start.elapsed());
                        Ok(Some(account))
                    },
                    Ok(Err(err)) => {
                        if err.to_string().contains("AccountNotFound") {
                            // Account not found is a valid response
                            this.endpoint_manager.record_success(&endpoint.url, start.elapsed()).await;
                            
                            // Cache the negative result
                            if this.config.cache_config.enable_cache && context.cacheable {
                                let cache_key = format!("account:{}", pubkey);
                                this.cache.set(&cache_key, json!(null), this.config.cache_config.account_ttl_ms).await;
                            }
                            
                            this.metrics.record_request_success(request_id, start.elapsed());
                            Ok(None)
                        } else {
                            // Other error
                            this.endpoint_manager.record_error(&endpoint.url, Some(err.to_string())).await;
                            this.metrics.record_request_error(request_id, &err.to_string());
                            Err(RpcError::Client(err.to_string()))
                        }
                    },
                    Err(_) => {
                        // Timeout
                        this.endpoint_manager.record_timeout(&endpoint.url).await;
                        this.metrics.record_request_timeout(request_id);
                        Err(RpcError::Timeout)
                    },
                };
                
                result
            }
        }).await
    }
    
    /// Get multiple accounts in a single request
    pub async fn get_multiple_accounts(&self, pubkeys: &[Pubkey]) -> RpcResult<Vec<Option<Account>>> {
        self.get_multiple_accounts_with_options(pubkeys, RequestOptions::default()).await
    }
    
    /// Get multiple accounts with custom options
    pub async fn get_multiple_accounts_with_options(
        &self, 
        pubkeys: &[Pubkey],
        options: RequestOptions,
    ) -> RpcResult<Vec<Option<Account>>> {
        let method = "getMultipleAccounts";
        let pubkeys = pubkeys.to_vec();
        
        self.execute_request(method, options.clone(), move |context| {
            let this = self.clone();
            
            async move {
                let start = context.start_time;
                let request_id = context.id;
                
                this.metrics.record_request_start(request_id, method);
                
                // Get the best endpoint for this request
                let endpoint = this.get_best_endpoint_for_request(method, &options).await?;
                
                // Apply rate limiting unless bypassed
                if !options.bypass_rate_limit {
                    // Cost is proportional to number of accounts
                    let cost = (pubkeys.len() as f64 * 0.2).max(1.0);
                    if !this.rate_limiter.try_acquire_with_cost(&endpoint.url, method, cost).await {
                        this.metrics.record_rate_limited(request_id);
                        return Err(RpcError::RateLimited);
                    }
                }
                
                // Acquire a request permit
                let timeout_ms = options.timeout_ms.unwrap_or(this.config.request_timeout_ms);
                let permit_timeout = timeout(
                    Duration::from_millis(timeout_ms),
                    this.request_semaphore.acquire()
                ).await;
                
                let _permit = match permit_timeout {
                    Ok(permit) => permit.map_err(|e| RpcError::Call(format!("Failed to acquire permit: {}", e)))?,
                    Err(_) => {
                        this.metrics.record_request_timeout(request_id);
                        return Err(RpcError::Timeout);
                    }
                };
                
                // Get a connection from the pool
                let client = this.get_connection(&endpoint.url)
                    .ok_or_else(|| RpcError::Call(format!("No connection available for {}", endpoint.url)))?;
                
                // Configure commitment
                let commitment = match options.min_commitment {
                    Some(level) => CommitmentConfig { commitment: level },
                    None => this.commitment,
                };
                
                // Send the request
                let result = match timeout(
                    Duration::from_millis(timeout_ms),
                    client.get_multiple_accounts_with_commitment(&pubkeys, commitment)
                ).await {
                    Ok(Ok(response)) => {
                        // Update endpoint stats
                        this.endpoint_manager.record_success(&endpoint.url, start.elapsed()).await;
                        
                        // Cache individual accounts if cacheable
                        if this.config.cache_config.enable_cache && context.cacheable {
                            for (i, account_opt) in response.value.iter().enumerate() {
                                if i < pubkeys.len() {
                                    let cache_key = format!("account:{}", pubkeys[i]);
                                    if let Ok(json_value) = serde_json::to_value(account_opt) {
                                        this.cache.set(&cache_key, json_value, this.config.cache_config.account_ttl_ms).await;
                                    }
                                }
                            }
                        }
                        
                        this.metrics.record_request_success(request_id, start.elapsed());
                        Ok(response.value)
                    },
                    Ok(Err(err)) => {
                        // Update endpoint stats
                        this.endpoint_manager.record_error(&endpoint.url, Some(err.to_string())).await;
                        
                        this.metrics.record_request_error(request_id, &err.to_string());
                        Err(RpcError::Client(err.to_string()))
                    },
                    Err(_) => {
                        // Timeout
                        this.endpoint_manager.record_timeout(&endpoint.url).await;
                        
                        this.metrics.record_request_timeout(request_id);
                        Err(RpcError::Timeout)
                    },
                };
                
                result
            }
        }).await
    }
    
    /// Get the latest blockhash
    pub async fn get_latest_blockhash(&self) -> RpcResult<Hash> {
        self.get_latest_blockhash_with_options(RequestOptions::default()).await
    }
    
    /// Get the latest blockhash with custom options
    pub async fn get_latest_blockhash_with_options(
        &self,
        options: RequestOptions,
    ) -> RpcResult<Hash> {
        let method = "getLatestBlockhash";
        
        self.execute_request(method, options.clone(), move |context| {
            let this = self.clone();
            
            async move {
                let start = context.start_time;
                let request_id = context.id;
                
                this.metrics.record_request_start(request_id, method);
                
                // Check cache first if enabled and not bypassed
                if this.config.cache_config.enable_cache && !options.bypass_cache && context.cacheable {
                    let cache_key = "latest_blockhash";
                    if let Some(cached) = this.cache.get(cache_key).await {
                        if let Ok(blockhash) = serde_json::from_value(cached) {
                            this.metrics.record_cache_hit(request_id);
                            return Ok(blockhash);
                        }
                    }
                }
                
                // Get the best endpoint for this request
                let endpoint = this.get_best_endpoint_for_request(method, &options).await?;
                
                // Apply rate limiting unless bypassed
                if !options.bypass_rate_limit {
                    if !this.rate_limiter.try_acquire_with_cost(&endpoint.url, method, 0.5).await {
                        this.metrics.record_rate_limited(request_id);
                        return Err(RpcError::RateLimited);
                    }
                }
                
                // Acquire a request permit
                let timeout_ms = options.timeout_ms.unwrap_or(this.config.request_timeout_ms);
                let permit_timeout = timeout(
                    Duration::from_millis(timeout_ms),
                    this.request_semaphore.acquire()
                ).await;
                
                let _permit = match permit_timeout {
                    Ok(permit) => permit.map_err(|e| RpcError::Call(format!("Failed to acquire permit: {}", e)))?,
                    Err(_) => {
                        this.metrics.record_request_timeout(request_id);
                        return Err(RpcError::Timeout);
                    }
                };
                
                // Get a connection from the pool
                let client = this.get_connection(&endpoint.url)
                    .ok_or_else(|| RpcError::Call(format!("No connection available for {}", endpoint.url)))?;
                
                // Configure commitment
                let commitment = match options.min_commitment {
                    Some(level) => CommitmentConfig { commitment: level },
                    None => this.commitment,
                };
                
                // Send the request
                let result = match timeout(
                    Duration::from_millis(timeout_ms),
                    client.get_latest_blockhash_with_commitment(commitment)
                ).await {
                    Ok(Ok(response)) => {
                        // Update endpoint stats
                        this.endpoint_manager.record_success(&endpoint.url, start.elapsed()).await;
                        
                        // Cache the result
                        if this.config.cache_config.enable_cache && context.cacheable {
                            if let Ok(json_value) = serde_json::to_value(&response.0) {
                                this.cache.set("latest_blockhash", json_value, this.config.cache_config.blockhash_ttl_ms).await;
                            }
                        }
                        
                        this.metrics.record_request_success(request_id, start.elapsed());
                        Ok(response.0)
                    },
                    Ok(Err(err)) => {
                        // Update endpoint stats
                        this.endpoint_manager.record_error(&endpoint.url, Some(err.to_string())).await;
                        
                        this.metrics.record_request_error(request_id, &err.to_string());
                        Err(RpcError::Client(err.to_string()))
                    },
                    Err(_) => {
                        // Timeout
                        this.endpoint_manager.record_timeout(&endpoint.url).await;
                        
                        this.metrics.record_request_timeout(request_id);
                        Err(RpcError::Timeout)
                    },
                };
                
                result
            }
        }).await
    }
    
    /// Get transaction status
    pub async fn get_transaction_status(&self, signature: &Signature) -> RpcResult<Option<TransactionStatus>> {
        self.get_transaction_status_with_options(signature, RequestOptions::default()).await
    }
    
    /// Get transaction status with custom options
    pub async fn get_transaction_status_with_options(
        &self,
        signature: &Signature,
        options: RequestOptions,
    ) -> RpcResult<Option<TransactionStatus>> {
        let method = "getSignatureStatuses";
        let signature = *signature;
        
        self.execute_request(method, options.clone(), move |context| {
            let this = self.clone();
            
            async move {
                let start = context.start_time;
                let request_id = context.id;
                
                this.metrics.record_request_start(request_id, method);
                
                // Get the best endpoint for this request
                let endpoint = this.get_best_endpoint_for_request(method, &options).await?;
                
                // Apply rate limiting unless bypassed
                if !options.bypass_rate_limit {
                    if !this.rate_limiter.try_acquire_with_cost(&endpoint.url, method, 1.0).await {
                        this.metrics.record_rate_limited(request_id);
                        return Err(RpcError::RateLimited);
                    }
                }
                
                // Acquire a request permit
                let timeout_ms = options.timeout_ms.unwrap_or(this.config.request_timeout_ms);
                let permit_timeout = timeout(
                    Duration::from_millis(timeout_ms),
                    this.request_semaphore.acquire()
                ).await;
                
                let _permit = match permit_timeout {
                    Ok(permit) => permit.map_err(|e| RpcError::Call(format!("Failed to acquire permit: {}", e)))?,
                    Err(_) => {
                        this.metrics.record_request_timeout(request_id);
                        return Err(RpcError::Timeout);
                    }
                };
                
                // Get a connection from the pool
                let client = this.get_connection(&endpoint.url)
                    .ok_or_else(|| RpcError::Call(format!("No connection available for {}", endpoint.url)))?;
                
                // Send the request
                let result = match timeout(
                    Duration::from_millis(timeout_ms),
                    client.get_signature_statuses(&[signature])
                ).await {
                    Ok(Ok(response)) => {
                        // Update endpoint stats
                        this.endpoint_manager.record_success(&endpoint.url, start.elapsed()).await;
                        
                        this.metrics.record_request_success(request_id, start.elapsed());
                        
                        // Return the first status (there's only one)
                        Ok(response.value.into_iter().next().flatten())
                    },
                    Ok(Err(err)) => {
                        // Update endpoint stats
                        this.endpoint_manager.record_error(&endpoint.url, Some(err.to_string())).await;
                        
                        this.metrics.record_request_error(request_id, &err.to_string());
                        Err(RpcError::Client(err.to_string()))
                    },
                    Err(_) => {
                        // Timeout
                        this.endpoint_manager.record_timeout(&endpoint.url).await;
                        
                        this.metrics.record_request_timeout(request_id);
                        Err(RpcError::Timeout)
                    },
                };
                
                result
            }
        }).await
    }
    
    /// Get the current slot number
    pub async fn get_slot(&self) -> RpcResult<u64> {
        self.get_slot_with_options(RequestOptions::default()).await
    }
    
    /// Get the current slot with custom options
    pub async fn get_slot_with_options(
        &self,
        options: RequestOptions,
    ) -> RpcResult<u64> {
        let method = "getSlot";
        
        self.execute_request(method, options.clone(), move |context| {
            let this = self.clone();
            
            async move {
                let start = context.start_time;
                let request_id = context.id;
                
                this.metrics.record_request_start(request_id, method);
                
                // Check cache first if enabled and not bypassed
                if this.config.cache_config.enable_cache && !options.bypass_cache && context.cacheable {
                    let cache_key = "current_slot";
                    if let Some(cached) = this.cache.get(cache_key).await {
                        if let Ok(slot) = serde_json::from_value(cached) {
                            this.metrics.record_cache_hit(request_id);
                            return Ok(slot);
                        }
                    }
                }
                
                // Get the best endpoint for this request
                let endpoint = this.get_best_endpoint_for_request(method, &options).await?;
                
                // Apply rate limiting unless bypassed
                if !options.bypass_rate_limit {
                    if !this.rate_limiter.try_acquire_with_cost(&endpoint.url, method, 0.2).await {
                        this.metrics.record_rate_limited(request_id);
                        return Err(RpcError::RateLimited);
                    }
                }
                
                // Acquire a request permit
                let timeout_ms = options.timeout_ms.unwrap_or(this.config.request_timeout_ms);
                let permit_timeout = timeout(
                    Duration::from_millis(timeout_ms),
                    this.request_semaphore.acquire()
                ).await;
                
                let _permit = match permit_timeout {
                    Ok(permit) => permit.map_err(|e| RpcError::Call(format!("Failed to acquire permit: {}", e)))?,
                    Err(_) => {
                        this.metrics.record_request_timeout(request_id);
                        return Err(RpcError::Timeout);
                    }
                };
                
                // Get a connection from the pool
                let client = this.get_connection(&endpoint.url)
                    .ok_or_else(|| RpcError::Call(format!("No connection available for {}", endpoint.url)))?;
                
                // Configure commitment
                let commitment = match options.min_commitment {
                    Some(level) => CommitmentConfig { commitment: level },
                    None => this.commitment,
                };
                
                // Send the request
                let result = match timeout(
                    Duration::from_millis(timeout_ms),
                    client.get_slot_with_commitment(commitment)
                ).await {
                    Ok(Ok(slot)) => {
                        // Update endpoint stats
                        this.endpoint_manager.record_success(&endpoint.url, start.elapsed()).await;
                        
                        // Update endpoint slot information
                        this.endpoint_manager.update_endpoint_slot(&endpoint.url, slot).await;
                        
                        // Cache the result
                        if this.config.cache_config.enable_cache && context.cacheable {
                            if let Ok(json_value) = serde_json::to_value(&slot) {
                                this.cache.set("current_slot", json_value, this.config.cache_config.slot_ttl_ms).await;
                            }
                        }
                        
                        this.metrics.record_request_success(request_id, start.elapsed());
                        Ok(slot)
                    },
                    Ok(Err(err)) => {
                        // Update endpoint stats
                        this.endpoint_manager.record_error(&endpoint.url, Some(err.to_string())).await;
                        
                        this.metrics.record_request_error(request_id, &err.to_string());
                        Err(RpcError::Client(err.to_string()))
                    },
                    Err(_) => {
                        // Timeout
                        this.endpoint_manager.record_timeout(&endpoint.url).await;
                        
                        this.metrics.record_request_timeout(request_id);
                        Err(RpcError::Timeout)
                    },
                };
                
                result
            }
        }).await
    }
    
    /// Get metrics for the RPC client
    pub fn get_metrics(&self) -> RpcMetricsSnapshot {
        self.metrics.snapshot()
    }
    
    /// Get the current status of all endpoints
    pub async fn get_endpoint_status(&self) -> Vec<EndpointStatus> {
        self.endpoint_manager.get_endpoint_status().await
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> CacheStats {
        self.cache.get_stats().await
    }
    
    /// Get rate limiter statistics
    pub fn get_rate_limiter_stats(&self) -> RateLimiterStats {
        self.rate_limiter.get_stats()
    }
    
    /// Subscribe to account updates
    pub async fn subscribe_account(
        &self,
        pubkey: &Pubkey,
        commitment: Option<CommitmentConfig>,
    ) -> RpcResult<WebsocketSubscription> {
        if let Some(websocket_manager) = &self.websocket_manager {
            websocket_manager.subscribe_account(pubkey, commitment)
                .await
                .map_err(|e| RpcError::Websocket(e.to_string()))
        } else {
            Err(RpcError::Call("WebSocket support not enabled".to_string()))
        }
    }
    
    /// Subscribe to program updates
    pub async fn subscribe_program(
        &self,
        program_id: &Pubkey,
        commitment: Option<CommitmentConfig>,
    ) -> RpcResult<WebsocketSubscription> {
        if let Some(websocket_manager) = &self.websocket_manager {
            websocket_manager.subscribe_program(program_id, commitment)
                .await
                .map_err(|e| RpcError::Websocket(e.to_string()))
        } else {
            Err(RpcError::Call("WebSocket support not enabled".to_string()))
        }
    }
    
    /// Subscribe to signature updates
    pub async fn subscribe_signature(
        &self,
        signature: &Signature,
        commitment: Option<CommitmentConfig>,
    ) -> RpcResult<WebsocketSubscription> {
        if let Some(websocket_manager) = &self.websocket_manager {
            websocket_manager.subscribe_signature(signature, commitment)
                .await
                .map_err(|e| RpcError::Websocket(e.to_string()))
        } else {
            Err(RpcError::Call("WebSocket support not enabled".to_string()))
        }
    }
    
    /// Subscribe to slot updates
    pub async fn subscribe_slot(&self) -> RpcResult<WebsocketSubscription> {
        if let Some(websocket_manager) = &self.websocket_manager {
            websocket_manager.subscribe_slot()
                .await
                .map_err(|e| RpcError::Websocket(e.to_string()))
        } else {
            Err(RpcError::Call("WebSocket support not enabled".to_string()))
        }
    }
    
    /// Shutdown the client
    pub async fn shutdown(&self) {
        info!("Shutting down EnhancedRpcClient");
        
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.lock().take() {
            let _ = tx.send(());
        }
        
        // Shutdown websocket manager
        if let Some(websocket_manager) = &self.websocket_manager {
            websocket_manager.shutdown().await;
        }
        
        // Shutdown monitor
        if let Some(monitor) = &self.monitor {
            if let Err(e) = monitor.stop().await {
                error!("Failed to stop monitor: {}", e);
            }
        }
        
        // Shutdown rate limiter
        self.rate_limiter.shutdown().await;
        
        // Shutdown cache
        self.cache.stop().await;
        
        // Shutdown endpoint manager
        self.endpoint_manager.shutdown().await;
    }
}

// Initialize the module
pub fn init() {
    info!("Initializing RPC module");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_endpoint_manager() {
        let config = RpcConfig::default();
        let metrics = Arc::new(RpcMetrics::new());
        let manager = EndpointManager::new(config.clone(), metrics).await.unwrap();
        
        assert!(!manager.get_endpoints().await.is_empty());
    }
    
    #[tokio::test]
    async fn test_cache() {
        let config = cache::CacheConfig::default();
        let cache = cache::ResponseCache::new(config);
        
        let key = "test_key";
        let value = json!("test_value");
        
        cache.set(key, value.clone(), 1000).await;
        
        let cached = cache.get(key).await;
        assert_eq!(cached, Some(value));
    }
}
