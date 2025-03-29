//! Throughput metrics
//! 
//! This module provides utilities for collecting and reporting throughput metrics.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Throughput sample
#[derive(Debug, Clone)]
pub struct ThroughputSample {
    /// Timestamp
    pub timestamp: Instant,
    
    /// Bytes
    pub bytes: usize,
}

/// Throughput metrics
#[derive(Debug, Clone)]
pub struct ThroughputMetrics {
    /// Samples by name
    samples: HashMap<String, Vec<ThroughputSample>>,
    
    /// Window duration
    window: Duration,
}

impl ThroughputMetrics {
    /// Create new throughput metrics
    pub fn new() -> Self {
        Self {
            samples: HashMap::new(),
            window: Duration::from_secs(60), // Default window: 60 seconds
        }
    }
    
    /// Create new throughput metrics with a custom window
    pub fn with_window(window: Duration) -> Self {
        Self {
            samples: HashMap::new(),
            window,
        }
    }
    
    /// Record a throughput sample
    pub fn record(&mut self, name: &str, bytes: usize) {
        let sample = ThroughputSample {
            timestamp: Instant::now(),
            bytes,
        };
        
        let samples = self.samples
            .entry(name.to_string())
            .or_insert_with(Vec::new);
        
        samples.push(sample);
        
        // Prune old samples
        self.prune(name);
    }
    
    /// Prune old samples
    fn prune(&mut self, name: &str) {
        if let Some(samples) = self.samples.get_mut(name) {
            let now = Instant::now();
            let cutoff = now - self.window;
            
            samples.retain(|sample| sample.timestamp >= cutoff);
        }
    }
    
    /// Get samples by name
    pub fn get(&self, name: &str) -> Option<&Vec<ThroughputSample>> {
        self.samples.get(name)
    }
    
    /// Get all samples
    pub fn get_all(&self) -> &HashMap<String, Vec<ThroughputSample>> {
        &self.samples
    }
    
    /// Get the throughput in bytes per second
    pub fn get_throughput_bps(&self, name: &str) -> f64 {
        if let Some(samples) = self.samples.get(name) {
            if samples.is_empty() {
                return 0.0;
            }
            
            let now = Instant::now();
            let cutoff = now - self.window;
            
            let mut total_bytes = 0;
            let mut oldest_timestamp = now;
            
            for sample in samples {
                if sample.timestamp >= cutoff {
                    total_bytes += sample.bytes;
                    if sample.timestamp < oldest_timestamp {
                        oldest_timestamp = sample.timestamp;
                    }
                }
            }
            
            let duration = now.duration_since(oldest_timestamp);
            if duration.as_secs_f64() > 0.0 {
                total_bytes as f64 / duration.as_secs_f64()
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
    
    /// Get the throughput in megabits per second
    pub fn get_throughput_mbps(&self, name: &str) -> f64 {
        self.get_throughput_bps(name) * 8.0 / 1_000_000.0
    }
    
    /// Get the total bytes
    pub fn get_total_bytes(&self, name: &str) -> usize {
        if let Some(samples) = self.samples.get(name) {
            samples.iter().map(|sample| sample.bytes).sum()
        } else {
            0
        }
    }
    
    /// Get the window duration
    pub fn get_window(&self) -> Duration {
        self.window
    }
    
    /// Set the window duration
    pub fn set_window(&mut self, window: Duration) {
        self.window = window;
        
        // Prune all samples
        for name in self.samples.keys().cloned().collect::<Vec<_>>() {
            self.prune(&name);
        }
    }
    
    /// Reset all samples
    pub fn reset(&mut self) {
        self.samples.clear();
    }
}

impl Default for ThroughputMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate throughput in bytes per second
pub fn calculate_throughput_bps(bytes: usize, duration: Duration) -> f64 {
    if duration.as_secs_f64() > 0.0 {
        bytes as f64 / duration.as_secs_f64()
    } else {
        0.0
    }
}

/// Calculate throughput in megabits per second
pub fn calculate_throughput_mbps(bytes: usize, duration: Duration) -> f64 {
    calculate_throughput_bps(bytes, duration) * 8.0 / 1_000_000.0
}

/// Format throughput in a human-readable format
pub fn format_throughput(bps: f64) -> String {
    if bps >= 1_000_000_000.0 {
        format!("{:.2} GB/s", bps / 1_000_000_000.0)
    } else if bps >= 1_000_000.0 {
        format!("{:.2} MB/s", bps / 1_000_000.0)
    } else if bps >= 1_000.0 {
        format!("{:.2} KB/s", bps / 1_000.0)
    } else {
        format!("{:.2} B/s", bps)
    }
}

/// Format throughput in bits per second
pub fn format_throughput_bits(bps: f64) -> String {
    let bits_per_second = bps * 8.0;
    
    if bits_per_second >= 1_000_000_000.0 {
        format!("{:.2} Gbps", bits_per_second / 1_000_000_000.0)
    } else if bits_per_second >= 1_000_000.0 {
        format!("{:.2} Mbps", bits_per_second / 1_000_000.0)
    } else if bits_per_second >= 1_000.0 {
        format!("{:.2} Kbps", bits_per_second / 1_000.0)
    } else {
        format!("{:.2} bps", bits_per_second)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_throughput_metrics() {
        let mut metrics = ThroughputMetrics::with_window(Duration::from_secs(1));
        
        // Test initial state
        assert!(metrics.get("test").is_none());
        assert_eq!(metrics.get_throughput_bps("test"), 0.0);
        assert_eq!(metrics.get_throughput_mbps("test"), 0.0);
        assert_eq!(metrics.get_total_bytes("test"), 0);
        
        // Record some samples
        metrics.record("test", 1000);
        sleep(Duration::from_millis(100));
        metrics.record("test", 2000);
        sleep(Duration::from_millis(100));
        metrics.record("test", 3000);
        
        // Test metrics
        assert!(metrics.get("test").is_some());
        assert_eq!(metrics.get("test").unwrap().len(), 3);
        assert!(metrics.get_throughput_bps("test") > 0.0);
        assert!(metrics.get_throughput_mbps("test") > 0.0);
        assert_eq!(metrics.get_total_bytes("test"), 6000);
        
        // Test window
        assert_eq!(metrics.get_window(), Duration::from_secs(1));
        
        // Test reset
        metrics.reset();
        assert!(metrics.get("test").is_none());
        assert_eq!(metrics.get_throughput_bps("test"), 0.0);
        assert_eq!(metrics.get_throughput_mbps("test"), 0.0);
        assert_eq!(metrics.get_total_bytes("test"), 0);
    }

    #[test]
    fn test_calculate_throughput() {
        assert_eq!(calculate_throughput_bps(1000, Duration::from_secs(1)), 1000.0);
        assert_eq!(calculate_throughput_bps(1000, Duration::from_secs(2)), 500.0);
        assert_eq!(calculate_throughput_bps(1000, Duration::from_millis(500)), 2000.0);
        
        assert_eq!(calculate_throughput_mbps(125000, Duration::from_secs(1)), 1.0);
        assert_eq!(calculate_throughput_mbps(125000, Duration::from_secs(2)), 0.5);
        assert_eq!(calculate_throughput_mbps(125000, Duration::from_millis(500)), 2.0);
    }

    #[test]
    fn test_format_throughput() {
        assert_eq!(format_throughput(500.0), "500.00 B/s");
        assert_eq!(format_throughput(1500.0), "1.50 KB/s");
        assert_eq!(format_throughput(1500000.0), "1.50 MB/s");
        assert_eq!(format_throughput(1500000000.0), "1.50 GB/s");
        
        assert_eq!(format_throughput_bits(500.0), "4000.00 Kbps");
        assert_eq!(format_throughput_bits(1500.0), "12.00 Kbps");
        assert_eq!(format_throughput_bits(1500000.0), "12.00 Mbps");
        assert_eq!(format_throughput_bits(1500000000.0), "12.00 Gbps");
    }
}