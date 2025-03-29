//! Metrics collection for RPC client
//!
//! This module provides functionality for collecting and reporting metrics
//! about RPC client performance and behavior.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, trace, warn};

/// Snapshot of RPC metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcMetricsSnapshot {
    /// Total requests
    pub total_requests: u64,
    
    /// Successful requests
    pub successful_requests: u64,
    
    /// Failed requests
    pub failed_requests: u64,
    
    /// Timed out requests
    pub timeout_requests: u64,
    
    /// Rate limited requests
    pub rate_limited_requests: u64,
    
    /// Cache hits
    pub cache_hits: u64,
    
    /// Average request latency in microseconds
    pub avg_latency_us: u64,
    
    /// Request counts by method
    pub requests_by_method: HashMap<String, u64>,
    
    /// Error counts by type
    pub errors_by_type: HashMap<String, u64>,
    
    /// Latency percentiles (50th, 90th, 95th, 99th)
    pub latency_percentiles: HashMap<String, u64>,
    
    /// Requests per second
    pub requests_per_second: f64,
    
    /// Timestamp of the snapshot
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// RPC metrics collector
pub struct RpcMetrics {
    /// Next request ID
    next_request_id: AtomicU64,
    
    /// Total requests
    total_requests: AtomicU64,
    
    /// Successful requests
    successful_requests: AtomicU64,
    
    /// Failed requests
    failed_requests: AtomicU64,
    
    /// Timed out requests
    timeout_requests: AtomicU64,
    
    /// Rate limited requests
    rate_limited_requests: AtomicU64,
    
    /// Cache hits
    cache_hits: AtomicU64,
    
    /// Total latency in microseconds
    total_latency_us: AtomicU64,
    
    /// Request counts by method
    requests_by_method: DashMap<String, u64>,
    
    /// Error counts by type
    errors_by_type: DashMap<String, u64>,
    
    /// Request start times
    request_starts: DashMap<u64, (Instant, String)>,
    
    /// Recent latencies for percentile calculation
    recent_latencies: Mutex<Vec<u64>>,
    
    /// Start time for requests per second calculation
    start_time: Instant,
}

impl RpcMetrics {
    /// Create a new RPC metrics collector
    pub fn new() -> Self {
        Self {
            next_request_id: AtomicU64::new(1),
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            timeout_requests: AtomicU64::new(0),
            rate_limited_requests: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            requests_by_method: DashMap::new(),
            errors_by_type: DashMap::new(),
            request_starts: DashMap::new(),
            recent_latencies: Mutex::new(Vec::with_capacity(1000)),
            start_time: Instant::now(),
        }
    }
    
    /// Generate a unique request ID
    pub fn generate_request_id(&self) -> u64 {
        self.next_request_id.fetch_add(1, Ordering::SeqCst)
    }
    
    /// Record the start of a request
    pub fn record_request_start(&self, request_id: u64, method: &str) {
        self.total_requests.fetch_add(1, Ordering::SeqCst);
        self.request_starts.insert(request_id, (Instant::now(), method.to_string()));
        
        *self.requests_by_method.entry(method.to_string()).or_insert(0) += 1;
    }
    
    /// Record a successful request
    pub fn record_request_success(&self, request_id: u64, latency: Duration) {
        self.successful_requests.fetch_add(1, Ordering::SeqCst);
        
        let latency_us = latency.as_micros() as u64;
        self.total_latency_us.fetch_add(latency_us, Ordering::SeqCst);
        
        // Add to recent latencies for percentile calculation
        {
            let mut recent_latencies = self.recent_latencies.lock();
            recent_latencies.push(latency_us);
            
            // Keep only the most recent latencies
            if recent_latencies.len() > 1000 {
                recent_latencies.remove(0);
            }
        }
        
        self.request_starts.remove(&request_id);
    }
    
    /// Record a request error
    pub fn record_request_error(&self, request_id: u64, error: &str) {
        self.failed_requests.fetch_add(1, Ordering::SeqCst);
        
        // Extract error type
        let error_type = if error.contains("timeout") {
            "timeout"
        } else if error.contains("rate limit") {
            "rate_limit"
        } else if error.contains("not found") {
            "not_found"
        } else {
            "other"
        };
        
        *self.errors_by_type.entry(error_type.to_string()).or_insert(0) += 1;
        
        self.request_starts.remove(&request_id);
    }
    
    /// Record a request timeout
    pub fn record_request_timeout(&self, request_id: u64) {
        self.timeout_requests.fetch_add(1, Ordering::SeqCst);
        *self.errors_by_type.entry("timeout".to_string()).or_insert(0) += 1;
        
        self.request_starts.remove(&request_id);
    }
    
    /// Record a rate limited request
    pub fn record_rate_limited(&self, request_id: u64) {
        self.rate_limited_requests.fetch_add(1, Ordering::SeqCst);
        *self.errors_by_type.entry("rate_limit".to_string()).or_insert(0) += 1;
        
        self.request_starts.remove(&request_id);
    }
    
    /// Record a cache hit
    pub fn record_cache_hit(&self, request_id: u64) {
        self.cache_hits.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Update endpoint metrics
    pub fn update_endpoint_metrics(&self, endpoints: &[crate::endpoints::EndpointStatus]) {
        // This method can be used to update metrics based on endpoint status
        // For example, tracking the number of healthy endpoints, average latency, etc.
    }
    
    /// Calculate latency percentiles
    fn calculate_latency_percentiles(&self) -> HashMap<String, u64> {
        let mut percentiles = HashMap::new();
        let recent_latencies = self.recent_latencies.lock();
        
        if recent_latencies.is_empty() {
            return percentiles;
        }
        
        // Sort latencies for percentile calculation
        let mut sorted_latencies = recent_latencies.clone();
        sorted_latencies.sort();
        
        let len = sorted_latencies.len();
        
        // Calculate percentiles
        percentiles.insert("p50".to_string(), sorted_latencies[len * 50 / 100]);
        percentiles.insert("p90".to_string(), sorted_latencies[len * 90 / 100]);
        percentiles.insert("p95".to_string(), sorted_latencies[len * 95 / 100]);
        percentiles.insert("p99".to_string(), sorted_latencies[len * 99 / 100]);
        
        percentiles
    }
    
    /// Get a snapshot of the current metrics
    pub fn snapshot(&self) -> RpcMetricsSnapshot {
        let total = self.total_requests.load(Ordering::SeqCst);
        let successful = self.successful_requests.load(Ordering::SeqCst);
        let total_latency = self.total_latency_us.load(Ordering::SeqCst);
        
        let avg_latency = if successful > 0 {
            total_latency / successful
        } else {
            0
        };
        
        let requests_by_method = self.requests_by_method
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect();
        
        let errors_by_type = self.errors_by_type
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect();
        
        let latency_percentiles = self.calculate_latency_percentiles();
        
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let requests_per_second = if elapsed > 0.0 {
            total as f64 / elapsed
        } else {
            0.0
        };
        
        RpcMetricsSnapshot {
            total_requests: total,
            successful_requests: successful,
            failed_requests: self.failed_requests.load(Ordering::SeqCst),
            timeout_requests: self.timeout_requests.load(Ordering::SeqCst),
            rate_limited_requests: self.rate_limited_requests.load(Ordering::SeqCst),
            cache_hits: self.cache_hits.load(Ordering::SeqCst),
            avg_latency_us: avg_latency,
            requests_by_method,
            errors_by_type,
            latency_percentiles,
            requests_per_second,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Reset metrics
    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::SeqCst);
        self.successful_requests.store(0, Ordering::SeqCst);
        self.failed_requests.store(0, Ordering::SeqCst);
        self.timeout_requests.store(0, Ordering::SeqCst);
        self.rate_limited_requests.store(0, Ordering::SeqCst);
        self.cache_hits.store(0, Ordering::SeqCst);
        self.total_latency_us.store(0, Ordering::SeqCst);
        
        self.requests_by_method.clear();
        self.errors_by_type.clear();
        self.recent_latencies.lock().clear();
        
        self.start_time = Instant::now();
    }
}

impl Default for RpcMetrics {
    fn default() -> Self {
        Self::new()
    }
}
