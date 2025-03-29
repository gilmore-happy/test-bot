use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use crate::types::SignalType;

/// Snapshot of core metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreMetricsSnapshot {
    /// Number of signals processed
    pub signals_processed: u64,
    
    /// Number of signals rejected
    pub signals_rejected: u64,
    
    /// Number of strategies run
    pub strategies_run: u64,
    
    /// Number of strategy errors
    pub strategy_errors: u64,
    
    /// Average strategy execution time in milliseconds
    pub avg_strategy_time_ms: u64,
    
    /// Current signal queue size
    pub signal_queue_size: usize,
    
    /// Signals by type
    pub signals_by_type: HashMap<SignalType, u64>,
    
    /// Signals by strategy
    pub signals_by_strategy: HashMap<String, u64>,
}

/// Core metrics
pub struct CoreMetrics {
    /// Number of signals processed
    signals_processed: AtomicU64,
    
    /// Number of signals rejected
    signals_rejected: AtomicU64,
    
    /// Number of strategies run
    strategies_run: AtomicU64,
    
    /// Number of strategy errors
    strategy_errors: AtomicU64,
    
    /// Total strategy execution time in milliseconds
    strategy_time_ms: AtomicU64,
    
    /// Number of strategy executions for average calculation
    strategy_count: AtomicU64,
    
    /// Signals by type
    signals_by_type: DashMap<SignalType, u64>,
    
    /// Signals by strategy
    signals_by_strategy: DashMap<String, u64>,
}

impl CoreMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            signals_processed: AtomicU64::new(0),
            signals_rejected: AtomicU64::new(0),
            strategies_run: AtomicU64::new(0),
            strategy_errors: AtomicU64::new(0),
            strategy_time_ms: AtomicU64::new(0),
            strategy_count: AtomicU64::new(0),
            signals_by_type: DashMap::new(),
            signals_by_strategy: DashMap::new(),
        }
    }

    /// Record a signal being processed
    pub fn record_signal_processed(&self, id: String, success: bool) {
        self.signals_processed.fetch_add(1, Ordering::SeqCst);
        
        // Update signal type stats if successful
        if success {
            // This would typically be updated from the signal data in a real implementation
        }
    }
    
    /// Record a signal being rejected
    pub fn record_signal_rejected(&self, id: String) {
        self.signals_rejected.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Record a strategy run
    pub fn record_strategy_run(&self, strategy_id: &str, duration: Duration, signal_count: usize) {
        self.strategies_run.fetch_add(1, Ordering::SeqCst);
        self.strategy_time_ms.fetch_add(duration.as_millis() as u64, Ordering::SeqCst);
        self.strategy_count.fetch_add(1, Ordering::SeqCst);
        
        // Update signals by strategy
        *self.signals_by_strategy.entry(strategy_id.to_string()).or_insert(0) += signal_count as u64;
    }
    
    /// Record a strategy error
    pub fn record_strategy_error(&self, strategy_id: &str) {
        self.strategy_errors.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Get a snapshot of the current metrics
    pub fn snapshot(&self) -> CoreMetricsSnapshot {
        let strategy_count = self.strategy_count.load(Ordering::SeqCst);
        let strategy_time_ms = self.strategy_time_ms.load(Ordering::SeqCst);
        
        let avg_strategy_time_ms = if strategy_count > 0 {
            strategy_time_ms / strategy_count
        } else {
            0
        };
        
        let signals_by_type = self.signals_by_type
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect();
        
        let signals_by_strategy = self.signals_by_strategy
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect();
        
        CoreMetricsSnapshot {
            signals_processed: self.signals_processed.load(Ordering::SeqCst),
            signals_rejected: self.signals_rejected.load(Ordering::SeqCst),
            strategies_run: self.strategies_run.load(Ordering::SeqCst),
            strategy_errors: self.strategy_errors.load(Ordering::SeqCst),
            avg_strategy_time_ms,
            signal_queue_size: 0, // This would need to be updated from the queue
            signals_by_type,
            signals_by_strategy,
        }
    }
}