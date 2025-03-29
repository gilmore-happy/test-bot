//! Metrics for the arbitrage module
//!
//! This module provides metrics collection and reporting for the arbitrage engine.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::trace;

/// Snapshot of arbitrage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageMetricsSnapshot {
    /// Total arbitrage opportunities detected
    pub total_opportunities_detected: u64,
    
    /// Total arbitrage opportunities executed
    pub total_opportunities_executed: u64,
    
    /// Total arbitrage opportunities succeeded
    pub total_opportunities_succeeded: u64,
    
    /// Total arbitrage opportunities failed
    pub total_opportunities_failed: u64,
    
    /// Total profit in USD
    pub total_profit_usd: f64,
    
    /// Average profit per successful arbitrage in USD
    pub avg_profit_usd: f64,
    
    /// Average execution time in milliseconds
    pub avg_execution_time_ms: u64,
    
    /// Current queue size
    pub current_queue_size: u64,
    
    /// Arbitrage opportunities by strategy
    pub opportunities_by_strategy: HashMap<String, u64>,
    
    /// Profit by strategy in USD
    pub profit_by_strategy: HashMap<String, f64>,
}

/// Metrics for the arbitrage engine
pub struct ArbitrageMetrics {
    /// Total arbitrage opportunities detected
    total_opportunities_detected: AtomicU64,
    
    /// Total arbitrage opportunities executed
    total_opportunities_executed: AtomicU64,
    
    /// Total arbitrage opportunities succeeded
    total_opportunities_succeeded: AtomicU64,
    
    /// Total arbitrage opportunities failed
    total_opportunities_failed: AtomicU64,
    
    /// Total profit in microUSD (multiplied by 1M for atomic ops)
    total_profit_micro_usd: AtomicU64,
    
    /// Total execution time in milliseconds
    total_execution_time_ms: AtomicU64,
    
    /// Current queue size
    current_queue_size: AtomicU64,
    
    /// Opportunities by strategy
    opportunities_by_strategy: DashMap<String, u64>,
    
    /// Profit by strategy in microUSD
    profit_by_strategy: DashMap<String, u64>,
    
    /// Price update durations
    price_update_durations: DashMap<u64, Duration>,
    
    /// Pool update durations
    pool_update_durations: DashMap<u64, Duration>,
    
    /// Opportunity detection durations
    opportunity_detection_durations: DashMap<u64, Duration>,
    
    /// Strategy execution durations
    strategy_execution_durations: DashMap<String, Vec<Duration>>,
}

impl ArbitrageMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            total_opportunities_detected: AtomicU64::new(0),
            total_opportunities_executed: AtomicU64::new(0),
            total_opportunities_succeeded: AtomicU64::new(0),
            total_opportunities_failed: AtomicU64::new(0),
            total_profit_micro_usd: AtomicU64::new(0),
            total_execution_time_ms: AtomicU64::new(0),
            current_queue_size: AtomicU64::new(0),
            opportunities_by_strategy: DashMap::new(),
            profit_by_strategy: DashMap::new(),
            price_update_durations: DashMap::new(),
            pool_update_durations: DashMap::new(),
            opportunity_detection_durations: DashMap::new(),
            strategy_execution_durations: DashMap::new(),
        }
    }
    
    /// Record an opportunity being processed
    pub fn record_opportunity_processing(&self, id: String) {
        trace!("Recording opportunity processing: {}", id);
        self.total_opportunities_executed.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Record an opportunity succeeding
    pub fn record_opportunity_succeeded(&self, id: String, strategy: String, profit_usd: f64) {
        trace!("Recording opportunity success: {} with profit ${:.2}", id, profit_usd);
        self.total_opportunities_succeeded.fetch_add(1, Ordering::SeqCst);
        
        // Track profit (convert to microUSD for atomic operations)
        let micro_usd = (profit_usd * 1_000_000.0) as u64;
        self.total_profit_micro_usd.fetch_add(micro_usd, Ordering::SeqCst);
        
        // Track by strategy
        self.opportunities_by_strategy
            .entry(strategy.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        
        self.profit_by_strategy
            .entry(strategy)
            .and_modify(|profit| *profit += micro_usd)
            .or_insert(micro_usd);
    }
    
    /// Record an opportunity failing
    pub fn record_opportunity_failed(&self, id: String) {
        trace!("Recording opportunity failure: {}", id);
        self.total_opportunities_failed.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Record a price update
    pub fn record_price_update(&self, duration: Duration) {
        let id = chrono::Utc::now().timestamp_millis() as u64;
        self.price_update_durations.insert(id, duration);
        
        // Keep only recent data
        self.cleanup_durations(&self.price_update_durations, 100);
    }
    
    /// Record a pool update
    pub fn record_pool_update(&self, duration: Duration) {
        let id = chrono::Utc::now().timestamp_millis() as u64;
        self.pool_update_durations.insert(id, duration);
        
        // Keep only recent data
        self.cleanup_durations(&self.pool_update_durations, 100);
    }
    
    /// Record an opportunity detection
    pub fn record_opportunity_detection(&self, duration: Duration) {
        let id = chrono::Utc::now().timestamp_millis() as u64;
        self.opportunity_detection_durations.insert(id, duration);
        
        // Increment counter
        self.total_opportunities_detected.fetch_add(1, Ordering::SeqCst);
        
        // Keep only recent data
        self.cleanup_durations(&self.opportunity_detection_durations, 100);
    }
    
    /// Record a strategy execution
    pub fn record_strategy_execution(&self, strategy: &str, duration: Duration) {
        let mut durations = self.strategy_execution_durations
            .entry(strategy.to_string())
            .or_insert_with(Vec::new);
        
        durations.push(duration);
        
        // Keep only recent data
        while durations.len() > 100 {
            durations.remove(0);
        }
    }
    
    /// Record current queue size
    pub fn record_queue_size(&self, size: u64) {
        self.current_queue_size.store(size, Ordering::SeqCst);
    }
    
    /// Record execution time
    pub fn record_execution_time(&self, time_ms: u64) {
        self.total_execution_time_ms.fetch_add(time_ms, Ordering::SeqCst);
    }
    
    /// Clean up old durations
    fn cleanup_durations(&self, map: &DashMap<u64, Duration>, max_size: usize) {
        if map.len() > max_size {
            let keys: Vec<u64> = map.iter().map(|e| *e.key()).collect();
            
            // Sort keys (oldest first)
            let mut sorted_keys = keys.clone();
            sorted_keys.sort();
            
            // Remove oldest entries
            let to_remove = map.len() - max_size;
            for i in 0..to_remove {
                map.remove(&sorted_keys[i]);
            }
        }
    }
    
    /// Get a snapshot of the current metrics
    pub fn snapshot(&self) -> ArbitrageMetricsSnapshot {
        let total_opportunities_succeeded = self.total_opportunities_succeeded.load(Ordering::SeqCst);
        let total_profit_micro_usd = self.total_profit_micro_usd.load(Ordering::SeqCst);
        let total_execution_time_ms = self.total_execution_time_ms.load(Ordering::SeqCst);
        
        let avg_profit_usd = if total_opportunities_succeeded > 0 {
            (total_profit_micro_usd as f64 / 1_000_000.0) / total_opportunities_succeeded as f64
        } else {
            0.0
        };
        
        let avg_execution_time_ms = if total_opportunities_succeeded > 0 {
            total_execution_time_ms / total_opportunities_succeeded
        } else {
            0
        };
        
        let opportunities_by_strategy = self.opportunities_by_strategy
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect();
        
        let profit_by_strategy = self.profit_by_strategy
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value() as f64 / 1_000_000.0))
            .collect();
        
        ArbitrageMetricsSnapshot {
            total_opportunities_detected: self.total_opportunities_detected.load(Ordering::SeqCst),
            total_opportunities_executed: self.total_opportunities_executed.load(Ordering::SeqCst),
            total_opportunities_succeeded,
            total_opportunities_failed: self.total_opportunities_failed.load(Ordering::SeqCst),
            total_profit_usd: total_profit_micro_usd as f64 / 1_000_000.0,
            avg_profit_usd,
            avg_execution_time_ms,
            current_queue_size: self.current_queue_size.load(Ordering::SeqCst),
            opportunities_by_strategy,
            profit_by_strategy,
        }
    }
}

impl Default for ArbitrageMetrics {
    fn default() -> Self {
        Self::new()
    }
}