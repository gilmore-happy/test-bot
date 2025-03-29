//! Performance metrics module
//! 
//! This module provides utilities for collecting and reporting performance metrics.

mod latency;
mod throughput;

pub use latency::{LatencyMetrics, LatencyHistogram};
pub use throughput::{ThroughputMetrics, ThroughputSample};

use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tracing::{debug, error, info, warn};

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// Latency metric
    Latency,
    
    /// Throughput metric
    Throughput,
    
    /// Error rate metric
    ErrorRate,
    
    /// Custom metric
    Custom,
}

/// Metric value
#[derive(Debug, Clone, Copy)]
pub enum MetricValue {
    /// Counter value
    Counter(u64),
    
    /// Gauge value
    Gauge(f64),
    
    /// Histogram value
    Histogram(Vec<(f64, u64)>),
    
    /// Duration value
    Duration(Duration),
}

/// Metric
#[derive(Debug, Clone)]
pub struct Metric {
    /// Metric name
    pub name: String,
    
    /// Metric type
    pub metric_type: MetricType,
    
    /// Metric value
    pub value: MetricValue,
    
    /// Metric timestamp
    pub timestamp: Instant,
    
    /// Metric tags
    pub tags: Vec<(String, String)>,
}

impl Metric {
    /// Create a new metric
    pub fn new(name: String, metric_type: MetricType, value: MetricValue) -> Self {
        Self {
            name,
            metric_type,
            value,
            timestamp: Instant::now(),
            tags: Vec::new(),
        }
    }
    
    /// Add a tag to the metric
    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.push((key, value));
        self
    }
    
    /// Add multiple tags to the metric
    pub fn with_tags(mut self, tags: Vec<(String, String)>) -> Self {
        self.tags.extend(tags);
        self
    }
}

/// Metrics collector
pub struct MetricsCollector {
    /// Metrics
    metrics: RwLock<Vec<Metric>>,
    
    /// Latency metrics
    latency_metrics: RwLock<LatencyMetrics>,
    
    /// Throughput metrics
    throughput_metrics: RwLock<ThroughputMetrics>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            metrics: RwLock::new(Vec::new()),
            latency_metrics: RwLock::new(LatencyMetrics::new()),
            throughput_metrics: RwLock::new(ThroughputMetrics::new()),
        }
    }
    
    /// Record a metric
    pub fn record(&self, metric: Metric) {
        self.metrics.write().push(metric);
    }
    
    /// Record a latency sample
    pub fn record_latency(&self, name: &str, latency: Duration) {
        self.latency_metrics.write().record(name, latency);
        
        self.record(Metric::new(
            name.to_string(),
            MetricType::Latency,
            MetricValue::Duration(latency),
        ));
    }
    
    /// Record a throughput sample
    pub fn record_throughput(&self, name: &str, bytes: usize) {
        self.throughput_metrics.write().record(name, bytes);
        
        self.record(Metric::new(
            name.to_string(),
            MetricType::Throughput,
            MetricValue::Counter(bytes as u64),
        ));
    }
    
    /// Record an error
    pub fn record_error(&self, name: &str) {
        self.record(Metric::new(
            name.to_string(),
            MetricType::ErrorRate,
            MetricValue::Counter(1),
        ));
    }
    
    /// Get all metrics
    pub fn get_metrics(&self) -> Vec<Metric> {
        self.metrics.read().clone()
    }
    
    /// Get latency metrics
    pub fn get_latency_metrics(&self) -> LatencyMetrics {
        self.latency_metrics.read().clone()
    }
    
    /// Get throughput metrics
    pub fn get_throughput_metrics(&self) -> ThroughputMetrics {
        self.throughput_metrics.read().clone()
    }
    
    /// Reset all metrics
    pub fn reset(&self) {
        self.metrics.write().clear();
        self.latency_metrics.write().reset();
        self.throughput_metrics.write().reset();
    }
}

/// Default metrics collector
pub fn default_collector() -> Arc<MetricsCollector> {
    static COLLECTOR: once_cell::sync::Lazy<Arc<MetricsCollector>> = once_cell::sync::Lazy::new(|| {
        Arc::new(MetricsCollector::new())
    });
    
    COLLECTOR.clone()
}

/// Record a latency sample
pub fn record_latency(name: &str, latency: Duration) {
    default_collector().record_latency(name, latency);
}

/// Record a throughput sample
pub fn record_throughput(name: &str, bytes: usize) {
    default_collector().record_throughput(name, bytes);
}

/// Record an error
pub fn record_error(name: &str) {
    default_collector().record_error(name);
}

/// Get all metrics
pub fn get_metrics() -> Vec<Metric> {
    default_collector().get_metrics()
}

/// Get latency metrics
pub fn get_latency_metrics() -> LatencyMetrics {
    default_collector().get_latency_metrics()
}

/// Get throughput metrics
pub fn get_throughput_metrics() -> ThroughputMetrics {
    default_collector().get_throughput_metrics()
}

/// Reset all metrics
pub fn reset() {
    default_collector().reset();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric() {
        let metric = Metric::new(
            "test".to_string(),
            MetricType::Latency,
            MetricValue::Duration(Duration::from_micros(100)),
        )
        .with_tag("key1".to_string(), "value1".to_string())
        .with_tag("key2".to_string(), "value2".to_string());
        
        assert_eq!(metric.name, "test");
        assert_eq!(metric.metric_type, MetricType::Latency);
        assert_eq!(metric.tags.len(), 2);
        assert_eq!(metric.tags[0].0, "key1");
        assert_eq!(metric.tags[0].1, "value1");
        assert_eq!(metric.tags[1].0, "key2");
        assert_eq!(metric.tags[1].1, "value2");
    }

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        
        // Record metrics
        collector.record_latency("latency", Duration::from_micros(100));
        collector.record_throughput("throughput", 1000);
        collector.record_error("error");
        
        // Get metrics
        let metrics = collector.get_metrics();
        assert_eq!(metrics.len(), 3);
        
        // Get latency metrics
        let latency_metrics = collector.get_latency_metrics();
        assert!(latency_metrics.get("latency").is_some());
        
        // Get throughput metrics
        let throughput_metrics = collector.get_throughput_metrics();
        assert!(throughput_metrics.get("throughput").is_some());
        
        // Reset metrics
        collector.reset();
        
        // Check if metrics are reset
        let metrics = collector.get_metrics();
        assert_eq!(metrics.len(), 0);
        
        let latency_metrics = collector.get_latency_metrics();
        assert!(latency_metrics.get("latency").is_none());
        
        let throughput_metrics = collector.get_throughput_metrics();
        assert!(throughput_metrics.get("throughput").is_none());
    }
}