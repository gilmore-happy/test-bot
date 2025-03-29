use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::metrics::CoreMetricsSnapshot;

/// Performance snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Uptime in seconds
    pub uptime_seconds: u64,
    
    /// Core metrics
    pub core_metrics: CoreMetricsSnapshot,
    
    /// Module metrics
    pub module_metrics: HashMap<String, serde_json::Value>,
}

/// Performance monitor for tracking system performance
pub struct PerformanceMonitor {
    /// Start time
    start_time: std::time::Instant,
    
    /// Performance history
    history: Vec<PerformanceSnapshot>,
    
    /// Maximum history size
    max_history_size: usize,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(max_history_size: usize) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            history: Vec::with_capacity(max_history_size),
            max_history_size,
        }
    }
    
    /// Add a performance snapshot
    pub fn add_snapshot(&mut self, snapshot: PerformanceSnapshot) {
        self.history.push(snapshot);
        
        // Trim history if it exceeds the maximum size
        if self.history.len() > self.max_history_size {
            self.history.remove(0);
        }
    }
    
    /// Get the performance history
    pub fn get_history(&self) -> &[PerformanceSnapshot] {
        &self.history
    }
    
    /// Get the uptime in seconds
    pub fn get_uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
    
    /// Create a new performance snapshot
    pub fn create_snapshot(
        &self,
        core_metrics: CoreMetricsSnapshot,
        module_metrics: HashMap<String, serde_json::Value>,
    ) -> PerformanceSnapshot {
        PerformanceSnapshot {
            timestamp: chrono::Utc::now(),
            uptime_seconds: self.get_uptime_seconds(),
            core_metrics,
            module_metrics,
        }
    }
}