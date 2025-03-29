//! Latency metrics
//! 
//! This module provides utilities for collecting and reporting latency metrics.

use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Latency histogram
#[derive(Debug, Clone)]
pub struct LatencyHistogram {
    /// Buckets (in microseconds)
    buckets: Vec<u64>,
    
    /// Counts
    counts: Vec<u64>,
    
    /// Total count
    total_count: u64,
    
    /// Minimum latency
    min: u64,
    
    /// Maximum latency
    max: u64,
    
    /// Sum of latencies
    sum: u64,
}

impl LatencyHistogram {
    /// Create a new latency histogram
    pub fn new() -> Self {
        // Default buckets in microseconds: 10, 50, 100, 250, 500, 1000, 2500, 5000, 10000, 25000, 50000, 100000
        let buckets = vec![10, 50, 100, 250, 500, 1000, 2500, 5000, 10000, 25000, 50000, 100000];
        let counts = vec![0; buckets.len() + 1]; // +1 for overflow bucket
        
        Self {
            buckets,
            counts,
            total_count: 0,
            min: u64::MAX,
            max: 0,
            sum: 0,
        }
    }
    
    /// Create a new latency histogram with custom buckets
    pub fn with_buckets(buckets: Vec<u64>) -> Self {
        let counts = vec![0; buckets.len() + 1]; // +1 for overflow bucket
        
        Self {
            buckets,
            counts,
            total_count: 0,
            min: u64::MAX,
            max: 0,
            sum: 0,
        }
    }
    
    /// Record a latency sample
    pub fn record(&mut self, latency: Duration) {
        let latency_us = latency.as_micros() as u64;
        
        // Update min/max/sum
        self.min = self.min.min(latency_us);
        self.max = self.max.max(latency_us);
        self.sum += latency_us;
        self.total_count += 1;
        
        // Find the bucket
        let mut bucket_idx = self.buckets.len(); // Default to overflow bucket
        
        for (i, &bucket) in self.buckets.iter().enumerate() {
            if latency_us <= bucket {
                bucket_idx = i;
                break;
            }
        }
        
        // Increment the bucket count
        self.counts[bucket_idx] += 1;
    }
    
    /// Get the number of samples
    pub fn count(&self) -> u64 {
        self.total_count
    }
    
    /// Get the minimum latency
    pub fn min(&self) -> Duration {
        if self.total_count == 0 {
            Duration::from_micros(0)
        } else {
            Duration::from_micros(self.min)
        }
    }
    
    /// Get the maximum latency
    pub fn max(&self) -> Duration {
        Duration::from_micros(self.max)
    }
    
    /// Get the average latency
    pub fn avg(&self) -> Duration {
        if self.total_count == 0 {
            Duration::from_micros(0)
        } else {
            Duration::from_micros(self.sum / self.total_count)
        }
    }
    
    /// Get the percentile latency
    pub fn percentile(&self, p: f64) -> Duration {
        if self.total_count == 0 {
            return Duration::from_micros(0);
        }
        
        if p <= 0.0 {
            return self.min();
        }
        
        if p >= 1.0 {
            return self.max();
        }
        
        let target_count = (self.total_count as f64 * p).ceil() as u64;
        let mut cumulative_count = 0;
        
        for (i, &count) in self.counts.iter().enumerate() {
            cumulative_count += count;
            
            if cumulative_count >= target_count {
                if i < self.buckets.len() {
                    return Duration::from_micros(self.buckets[i]);
                } else {
                    return Duration::from_micros(self.max);
                }
            }
        }
        
        Duration::from_micros(self.max)
    }
    
    /// Get the 50th percentile (median) latency
    pub fn p50(&self) -> Duration {
        self.percentile(0.5)
    }
    
    /// Get the 90th percentile latency
    pub fn p90(&self) -> Duration {
        self.percentile(0.9)
    }
    
    /// Get the 95th percentile latency
    pub fn p95(&self) -> Duration {
        self.percentile(0.95)
    }
    
    /// Get the 99th percentile latency
    pub fn p99(&self) -> Duration {
        self.percentile(0.99)
    }
    
    /// Get the 99.9th percentile latency
    pub fn p999(&self) -> Duration {
        self.percentile(0.999)
    }
    
    /// Get the buckets and counts
    pub fn buckets_and_counts(&self) -> Vec<(u64, u64)> {
        let mut result = Vec::with_capacity(self.buckets.len() + 1);
        
        for (i, &bucket) in self.buckets.iter().enumerate() {
            result.push((bucket, self.counts[i]));
        }
        
        result.push((u64::MAX, self.counts[self.buckets.len()]));
        
        result
    }
    
    /// Reset the histogram
    pub fn reset(&mut self) {
        for count in &mut self.counts {
            *count = 0;
        }
        
        self.total_count = 0;
        self.min = u64::MAX;
        self.max = 0;
        self.sum = 0;
    }
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        Self::new()
    }
}

/// Latency metrics
#[derive(Debug, Clone)]
pub struct LatencyMetrics {
    /// Histograms by name
    histograms: HashMap<String, LatencyHistogram>,
}

impl LatencyMetrics {
    /// Create new latency metrics
    pub fn new() -> Self {
        Self {
            histograms: HashMap::new(),
        }
    }
    
    /// Record a latency sample
    pub fn record(&mut self, name: &str, latency: Duration) {
        let histogram = self.histograms
            .entry(name.to_string())
            .or_insert_with(LatencyHistogram::new);
        
        histogram.record(latency);
    }
    
    /// Get a histogram by name
    pub fn get(&self, name: &str) -> Option<&LatencyHistogram> {
        self.histograms.get(name)
    }
    
    /// Get all histograms
    pub fn get_all(&self) -> &HashMap<String, LatencyHistogram> {
        &self.histograms
    }
    
    /// Reset all histograms
    pub fn reset(&mut self) {
        self.histograms.clear();
    }
}

impl Default for LatencyMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_histogram() {
        let mut histogram = LatencyHistogram::new();
        
        // Test initial state
        assert_eq!(histogram.count(), 0);
        assert_eq!(histogram.min(), Duration::from_micros(0));
        assert_eq!(histogram.max(), Duration::from_micros(0));
        assert_eq!(histogram.avg(), Duration::from_micros(0));
        assert_eq!(histogram.p50(), Duration::from_micros(0));
        assert_eq!(histogram.p90(), Duration::from_micros(0));
        assert_eq!(histogram.p95(), Duration::from_micros(0));
        assert_eq!(histogram.p99(), Duration::from_micros(0));
        assert_eq!(histogram.p999(), Duration::from_micros(0));
        
        // Record some samples
        histogram.record(Duration::from_micros(10));
        histogram.record(Duration::from_micros(20));
        histogram.record(Duration::from_micros(30));
        histogram.record(Duration::from_micros(40));
        histogram.record(Duration::from_micros(50));
        histogram.record(Duration::from_micros(60));
        histogram.record(Duration::from_micros(70));
        histogram.record(Duration::from_micros(80));
        histogram.record(Duration::from_micros(90));
        histogram.record(Duration::from_micros(100));
        
        // Test metrics
        assert_eq!(histogram.count(), 10);
        assert_eq!(histogram.min(), Duration::from_micros(10));
        assert_eq!(histogram.max(), Duration::from_micros(100));
        assert_eq!(histogram.avg(), Duration::from_micros(55));
        assert_eq!(histogram.p50(), Duration::from_micros(50));
        assert_eq!(histogram.p90(), Duration::from_micros(100));
        assert_eq!(histogram.p95(), Duration::from_micros(100));
        assert_eq!(histogram.p99(), Duration::from_micros(100));
        assert_eq!(histogram.p999(), Duration::from_micros(100));
        
        // Test reset
        histogram.reset();
        
        assert_eq!(histogram.count(), 0);
        assert_eq!(histogram.min(), Duration::from_micros(0));
        assert_eq!(histogram.max(), Duration::from_micros(0));
        assert_eq!(histogram.avg(), Duration::from_micros(0));
    }

    #[test]
    fn test_latency_metrics() {
        let mut metrics = LatencyMetrics::new();
        
        // Test initial state
        assert!(metrics.get("test").is_none());
        
        // Record some samples
        metrics.record("test", Duration::from_micros(10));
        metrics.record("test", Duration::from_micros(20));
        metrics.record("test", Duration::from_micros(30));
        
        // Test metrics
        let histogram = metrics.get("test").unwrap();
        assert_eq!(histogram.count(), 3);
        assert_eq!(histogram.min(), Duration::from_micros(10));
        assert_eq!(histogram.max(), Duration::from_micros(30));
        assert_eq!(histogram.avg(), Duration::from_micros(20));
        
        // Test reset
        metrics.reset();
        assert!(metrics.get("test").is_none());
    }
}