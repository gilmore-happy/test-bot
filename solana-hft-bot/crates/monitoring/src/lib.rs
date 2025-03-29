//! Monitoring module for the Solana HFT Bot
//!
//! This module provides monitoring, metrics collection, and alerting
//! functionality for the Solana HFT Bot system.

use solana_hft_core::CoreError;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use prometheus::{Registry, Counter, Gauge, Histogram, IntCounter, IntGauge};

/// Configuration for the monitoring system
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Whether to enable Prometheus metrics
    pub enable_prometheus: bool,
    /// Prometheus metrics endpoint (if enabled)
    pub prometheus_endpoint: String,
    /// Whether to enable alerting
    pub enable_alerting: bool,
    /// Alert threshold for latency (ms)
    pub latency_threshold_ms: u64,
    /// Alert threshold for error rate (percentage)
    pub error_rate_threshold: f64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_prometheus: true,
            prometheus_endpoint: "0.0.0.0:9090".to_string(),
            enable_alerting: true,
            latency_threshold_ms: 100,
            error_rate_threshold: 5.0,
        }
    }
}

/// Monitoring system state
pub struct MonitoringSystem {
    /// Configuration
    pub config: MonitoringConfig,
    /// Whether the system is running
    pub running: bool,
    /// Prometheus registry
    registry: Option<Registry>,
}

impl MonitoringSystem {
    /// Create a new monitoring system with the given configuration
    pub fn new(config: MonitoringConfig) -> Self {
        let registry = if config.enable_prometheus {
            Some(Registry::new())
        } else {
            None
        };
        
        Self {
            config,
            running: false,
            registry,
        }
    }

    /// Start the monitoring system
    pub fn start(&mut self) -> Result<(), MonitoringError> {
        if self.running {
            return Err(MonitoringError::AlreadyRunning);
        }
        
        info!("Starting monitoring system with config: {:?}", self.config);
        
        // Initialize monitoring system here
        
        self.running = true;
        info!("Monitoring system started");
        Ok(())
    }
    
    /// Stop the monitoring system
    pub fn stop(&mut self) -> Result<(), MonitoringError> {
        if !self.running {
            return Err(MonitoringError::NotRunning);
        }
        
        info!("Stopping monitoring system");
        
        // Cleanup monitoring system here
        
        self.running = false;
        info!("Monitoring system stopped");
        Ok(())
    }
    
    /// Get the Prometheus registry
    pub fn registry(&self) -> Option<&Registry> {
        self.registry.as_ref()
    }
}

/// Monitoring error types
#[derive(Debug, thiserror::Error)]
pub enum MonitoringError {
    #[error("Monitoring system already running")]
    AlreadyRunning,
    
    #[error("Monitoring system not running")]
    NotRunning,
    
    #[error("Prometheus error: {0}")]
    PrometheusError(String),
    
    #[error("Alerting error: {0}")]
    AlertingError(String),
    
    #[error("Core error: {0}")]
    Core(#[from] CoreError),
}

/// Module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Metrics collection for the HFT bot
pub struct Metrics {
    /// Transaction count
    pub tx_count: IntCounter,
    /// Transaction latency
    pub tx_latency: Histogram,
    /// Error count
    pub error_count: IntCounter,
    /// Current connections
    pub connections: IntGauge,
}

impl Metrics {
    /// Create a new metrics collection
    pub fn new(registry: &Registry) -> Result<Self, MonitoringError> {
        let tx_count = IntCounter::new(
            "solana_hft_transactions_total",
            "Total number of transactions sent"
        ).map_err(|e| MonitoringError::PrometheusError(e.to_string()))?;
        
        let tx_latency = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "solana_hft_transaction_latency_ms",
                "Transaction latency in milliseconds"
            )
            .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0])
        ).map_err(|e| MonitoringError::PrometheusError(e.to_string()))?;
        
        let error_count = IntCounter::new(
            "solana_hft_errors_total",
            "Total number of errors encountered"
        ).map_err(|e| MonitoringError::PrometheusError(e.to_string()))?;
        
        let connections = IntGauge::new(
            "solana_hft_connections",
            "Current number of active connections"
        ).map_err(|e| MonitoringError::PrometheusError(e.to_string()))?;
        
        registry.register(Box::new(tx_count.clone()))
            .map_err(|e| MonitoringError::PrometheusError(e.to_string()))?;
        registry.register(Box::new(tx_latency.clone()))
            .map_err(|e| MonitoringError::PrometheusError(e.to_string()))?;
        registry.register(Box::new(error_count.clone()))
            .map_err(|e| MonitoringError::PrometheusError(e.to_string()))?;
        registry.register(Box::new(connections.clone()))
            .map_err(|e| MonitoringError::PrometheusError(e.to_string()))?;
        
        Ok(Self {
            tx_count,
            tx_latency,
            error_count,
            connections,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_monitoring_config_default() {
        let config = MonitoringConfig::default();
        assert!(config.enable_prometheus);
        assert_eq!(config.prometheus_endpoint, "0.0.0.0:9090");
        assert!(config.enable_alerting);
        assert_eq!(config.latency_threshold_ms, 100);
        assert_eq!(config.error_rate_threshold, 5.0);
    }
    
    #[test]
    fn test_monitoring_system_start_stop() {
        let config = MonitoringConfig::default();
        let mut system = MonitoringSystem::new(config);
        
        assert!(!system.running);
        
        let result = system.start();
        assert!(result.is_ok());
        assert!(system.running);
        
        // Trying to start again should fail
        let result = system.start();
        assert!(result.is_err());
        
        let result = system.stop();
        assert!(result.is_ok());
        assert!(!system.running);
        
        // Trying to stop again should fail
        let result = system.stop();
        assert!(result.is_err());
    }
}