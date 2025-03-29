//! Rate limiter for RPC requests
//!
//! This module provides functionality for rate limiting RPC requests
//! to avoid overloading endpoints and hitting rate limits.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use tokio::sync::broadcast;
use tracing::{debug, error, info, trace, warn};
use anyhow::{anyhow, Result};

/// Rate limit strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitStrategy {
    /// Token bucket algorithm
    TokenBucket,
    
    /// Leaky bucket algorithm
    LeakyBucket,
    
    /// Fixed window counter
    FixedWindow,
    
    /// Sliding window counter
    SlidingWindow,
    
    /// Adaptive rate limiting based on response times
    Adaptive,
}

impl Default for RateLimitStrategy {
    fn default() -> Self {
        Self::TokenBucket
    }
}

/// Token bucket for rate limiting
struct TokenBucket {
    /// Maximum number of tokens
    max_tokens: f64,
    
    /// Current number of tokens
    tokens: f64,
    
    /// Tokens per second
    tokens_per_second: f64,
    
    /// Last refill time
    last_refill: Instant,
}

impl TokenBucket {
    /// Create a new token bucket
    fn new(tokens_per_second: f64, burst_size: f64) -> Self {
        Self {
            max_tokens: burst_size,
            tokens: burst_size,
            tokens_per_second,
            last_refill: Instant::now(),
        }
    }
    
    /// Try to consume tokens
    fn try_consume(&mut self, cost: f64) -> bool {
        self.refill();
        
        if self.tokens >= cost {
            self.tokens -= cost;
            true
        } else {
            false
        }
    }
    
    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.last_refill = now;
        
        self.tokens = (self.tokens + elapsed * self.tokens_per_second).min(self.max_tokens);
    }
}

/// Rate limiter statistics
#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    /// Total requests
    pub total_requests: u64,
    
    /// Allowed requests
    pub allowed_requests: u64,
    
    /// Limited requests
    pub limited_requests: u64,
    
    /// Requests per second
    pub requests_per_second: f64,
    
    /// Current token count
    pub current_tokens: f64,
    
    /// Maximum token count
    pub max_tokens: f64,
    
    /// Method-specific stats
    pub method_stats: HashMap<String, MethodStats>,
}

/// Method-specific rate limiter statistics
#[derive(Debug, Clone)]
pub struct MethodStats {
    /// Total requests
    pub total_requests: u64,
    
    /// Allowed requests
    pub allowed_requests: u64,
    
    /// Limited requests
    pub limited_requests: u64,
    
    /// Average cost
    pub average_cost: f64,
}

/// RPC rate limiter
pub struct RpcRateLimiter {
    /// Default tokens per second
    tokens_per_second: f64,
    
    /// Default burst size
    burst_size: f64,
    
    /// Token buckets for each endpoint
    buckets: DashMap<String, Mutex<TokenBucket>>,
    
    /// Method-specific costs
    method_costs: RwLock<HashMap<String, f64>>,
    
    /// Method-specific stats
    method_stats: DashMap<String, Mutex<MethodStats>>,
    
    /// Rate limit strategy
    strategy: RwLock<RateLimitStrategy>,
    
    /// Shutdown signal sender
    shutdown_tx: Mutex<Option<broadcast::Sender<()>>>,
}

impl RpcRateLimiter {
    /// Create a new RPC rate limiter
    pub fn new(tokens_per_second: f64, burst_size: f64) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        
        let mut method_costs = HashMap::new();
        
        // Set default costs for common methods
        method_costs.insert("getAccountInfo".to_string(), 1.0);
        method_costs.insert("getBalance".to_string(), 0.5);
        method_costs.insert("getBlockHeight".to_string(), 0.2);
        method_costs.insert("getLatestBlockhash".to_string(), 0.5);
        method_costs.insert("getMultipleAccounts".to_string(), 2.0);
        method_costs.insert("getProgramAccounts".to_string(), 5.0);
        method_costs.insert("getRecentBlockhash".to_string(), 0.5);
        method_costs.insert("getSignatureStatuses".to_string(), 1.0);
        method_costs.insert("getSlot".to_string(), 0.2);
        method_costs.insert("getTransaction".to_string(), 1.0);
        method_costs.insert("sendTransaction".to_string(), 3.0);
        
        Self {
            tokens_per_second,
            burst_size,
            buckets: DashMap::new(),
            method_costs: RwLock::new(method_costs),
            method_stats: DashMap::new(),
            strategy: RwLock::new(RateLimitStrategy::TokenBucket),
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
        }
    }
    
    /// Configure rate limiter from endpoints
    pub fn configure_from_endpoints(&self, endpoints: &[crate::endpoints::Endpoint]) {
        for endpoint in endpoints {
            self.buckets.entry(endpoint.url.clone()).or_insert_with(|| {
                Mutex::new(TokenBucket::new(self.tokens_per_second, self.burst_size))
            });
        }
    }
    
    /// Set the rate limit strategy
    pub fn set_strategy(&self, strategy: RateLimitStrategy) {
        *self.strategy.write() = strategy;
    }
    
    /// Set the cost for a specific method
    pub fn set_method_cost(&self, method: &str, cost: f64) {
        self.method_costs.write().insert(method.to_string(), cost);
    }
    
    /// Get the cost for a specific method
    pub fn get_method_cost(&self, method: &str) -> f64 {
        *self.method_costs.read().get(method).unwrap_or(&1.0)
    }
    
    /// Try to acquire a token for the given endpoint
    pub async fn try_acquire(&self, endpoint: &str) -> bool {
        self.try_acquire_with_cost(endpoint, "", 1.0).await
    }
    
    /// Try to acquire tokens for the given endpoint with a specific cost
    pub async fn try_acquire_with_cost(&self, endpoint: &str, method: &str, cost: f64) -> bool {
        // Get or create the token bucket for this endpoint
        let bucket = self.buckets
            .entry(endpoint.to_string())
            .or_insert_with(|| {
                Mutex::new(TokenBucket::new(self.tokens_per_second, self.burst_size))
            });
        
        // Calculate the actual cost based on method
        let actual_cost = if method.is_empty() {
            cost
        } else {
            let method_cost = self.get_method_cost(method);
            cost * method_cost
        };
        
        // Try to consume tokens
        let result = bucket.value().lock().try_consume(actual_cost);
        
        // Update stats
        if !method.is_empty() {
            let stats = self.method_stats
                .entry(method.to_string())
                .or_insert_with(|| {
                    Mutex::new(MethodStats {
                        total_requests: 0,
                        allowed_requests: 0,
                        limited_requests: 0,
                        average_cost: actual_cost,
                    })
                });
            
            let mut stats = stats.lock();
            stats.total_requests += 1;
            
            if result {
                stats.allowed_requests += 1;
            } else {
                stats.limited_requests += 1;
            }
            
            // Update average cost
            stats.average_cost = (stats.average_cost * (stats.total_requests - 1) as f64 + actual_cost) / stats.total_requests as f64;
        }
        
        result
    }
    
    /// Get rate limiter statistics
    pub fn get_stats(&self) -> RateLimiterStats {
        let mut total_requests = 0;
        let mut allowed_requests = 0;
        let mut limited_requests = 0;
        let mut method_stats = HashMap::new();
        
        // Collect method stats
        for entry in self.method_stats.iter() {
            let stats = entry.value().lock().clone();
            total_requests += stats.total_requests;
            allowed_requests += stats.allowed_requests;
            limited_requests += stats.limited_requests;
            method_stats.insert(entry.key().clone(), stats);
        }
        
        // Calculate requests per second
        let requests_per_second = if total_requests > 0 {
            // This is a crude approximation
            total_requests as f64 / 60.0
        } else {
            0.0
        };
        
        // Get a sample bucket to check token count
        let (current_tokens, max_tokens) = if let Some(bucket) = self.buckets.iter().next() {
            let bucket = bucket.value().lock();
            (bucket.tokens, bucket.max_tokens)
        } else {
            (self.burst_size, self.burst_size)
        };
        
        RateLimiterStats {
            total_requests,
            allowed_requests,
            limited_requests,
            requests_per_second,
            current_tokens,
            max_tokens,
            method_stats,
        }
    }
    
    /// Shutdown the rate limiter
    pub async fn shutdown(&self) {
        info!("Shutting down RPC rate limiter");
        
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.lock().take() {
            let _ = tx.send(());
        }
    }
}
