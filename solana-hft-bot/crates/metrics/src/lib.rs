//! Metrics module for the Solana HFT Bot
//!
//! This module provides metrics collection, aggregation, and reporting
//! functionality for the Solana HFT Bot system.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use prometheus::{
    Counter, Gauge, Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge,
    Registry,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use solana_hft_core::CoreError;

/// Configuration for the metrics system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Whether to enable Prometheus metrics
    pub enable_prometheus: bool,
    
    /// Prometheus metrics endpoint (if enabled)
    pub prometheus_endpoint: String,
    
    /// Whether to log metrics periodically
    pub log_metrics: bool,
    
    /// Interval for logging metrics (in seconds)
    pub log_interval_seconds: u64,
    
    /// Whether to collect detailed metrics
    pub detailed_metrics: bool,
    
    /// Maximum number of historical data points to keep
    pub history_size: usize,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_prometheus: true,
            prometheus_endpoint: "0.0.0.0:9090".to_string(),
            log_metrics: true,
            log_interval_seconds: 60,
            detailed_metrics: true,
            history_size: 1000,
        }
    }
}

/// Metrics system for collecting and reporting metrics
pub struct MetricsSystem {
    /// Configuration
    pub config: MetricsConfig,
    
    /// Whether the system is running
    pub running: bool,
    
    /// Prometheus registry
    registry: Option<Registry>,
    
    /// System metrics
    system_metrics: SystemMetrics,
    
    /// Network metrics
    network_metrics: NetworkMetrics,
    
    /// RPC metrics
    rpc_metrics: RpcMetrics,
    
    /// Execution metrics
    execution_metrics: ExecutionMetrics,
    
    /// Trading metrics
    trading_metrics: TradingMetrics,
}

impl MetricsSystem {
    /// Create a new metrics system with the given configuration
    pub fn new(config: MetricsConfig) -> Self {
        let registry = if config.enable_prometheus {
            Some(Registry::new())
        } else {
            None
        };
        
        let system_metrics = SystemMetrics::new(registry.as_ref());
        let network_metrics = NetworkMetrics::new(registry.as_ref());
        let rpc_metrics = RpcMetrics::new(registry.as_ref());
        let execution_metrics = ExecutionMetrics::new(registry.as_ref());
        let trading_metrics = TradingMetrics::new(registry.as_ref());
        
        Self {
            config,
            running: false,
            registry,
            system_metrics,
            network_metrics,
            rpc_metrics,
            execution_metrics,
            trading_metrics,
        }
    }
    
    /// Start the metrics system
    pub fn start(&mut self) -> Result<(), MetricsError> {
        if self.running {
            return Err(MetricsError::AlreadyRunning);
        }
        
        info!("Starting metrics system with config: {:?}", self.config);
        
        // Start Prometheus server if enabled
        if self.config.enable_prometheus {
            self.start_prometheus_server()?;
        }
        
        // Start metrics logging if enabled
        if self.config.log_metrics {
            self.start_metrics_logging()?;
        }
        
        self.running = true;
        info!("Metrics system started");
        Ok(())
    }
    
    /// Stop the metrics system
    pub fn stop(&mut self) -> Result<(), MetricsError> {
        if !self.running {
            return Err(MetricsError::NotRunning);
        }
        
        info!("Stopping metrics system");
        
        // Stop Prometheus server if enabled
        if self.config.enable_prometheus {
            self.stop_prometheus_server()?;
        }
        
        self.running = false;
        info!("Metrics system stopped");
        Ok(())
    }
    
    /// Start the Prometheus server
    fn start_prometheus_server(&self) -> Result<(), MetricsError> {
        // This is a placeholder - in a real implementation, this would start a Prometheus server
        info!("Starting Prometheus server on {}", self.config.prometheus_endpoint);
        Ok(())
    }
    
    /// Stop the Prometheus server
    fn stop_prometheus_server(&self) -> Result<(), MetricsError> {
        // This is a placeholder - in a real implementation, this would stop the Prometheus server
        info!("Stopping Prometheus server");
        Ok(())
    }
    
    /// Start metrics logging
    fn start_metrics_logging(&self) -> Result<(), MetricsError> {
        // This is a placeholder - in a real implementation, this would start a metrics logging task
        info!("Starting metrics logging with interval {} seconds", self.config.log_interval_seconds);
        Ok(())
    }
    
    /// Get a snapshot of all metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            timestamp: Utc::now(),
            system: self.system_metrics.snapshot(),
            network: self.network_metrics.snapshot(),
            rpc: self.rpc_metrics.snapshot(),
            execution: self.execution_metrics.snapshot(),
            trading: self.trading_metrics.snapshot(),
        }
    }
    
    /// Get the Prometheus registry
    pub fn registry(&self) -> Option<&Registry> {
        self.registry.as_ref()
    }
    
    /// Get system metrics
    pub fn system_metrics(&self) -> &SystemMetrics {
        &self.system_metrics
    }
    
    /// Get network metrics
    pub fn network_metrics(&self) -> &NetworkMetrics {
        &self.network_metrics
    }
    
    /// Get RPC metrics
    pub fn rpc_metrics(&self) -> &RpcMetrics {
        &self.rpc_metrics
    }
    
    /// Get execution metrics
    pub fn execution_metrics(&self) -> &ExecutionMetrics {
        &self.execution_metrics
    }
    
    /// Get trading metrics
    pub fn trading_metrics(&self) -> &TradingMetrics {
        &self.trading_metrics
    }
}

/// System metrics
pub struct SystemMetrics {
    /// CPU usage (percentage)
    pub cpu_usage: Gauge,
    
    /// Memory usage (bytes)
    pub memory_usage: Gauge,
    
    /// Disk usage (bytes)
    pub disk_usage: Gauge,
    
    /// Network usage (bytes/sec)
    pub network_usage: Gauge,
    
    /// Uptime (seconds)
    pub uptime: Counter,
    
    /// Start time
    pub start_time: Instant,
}

impl SystemMetrics {
    /// Create new system metrics
    fn new(registry: Option<&Registry>) -> Self {
        let cpu_usage = Gauge::new("system_cpu_usage", "CPU usage percentage").unwrap();
        let memory_usage = Gauge::new("system_memory_usage", "Memory usage in bytes").unwrap();
        let disk_usage = Gauge::new("system_disk_usage", "Disk usage in bytes").unwrap();
        let network_usage = Gauge::new("system_network_usage", "Network usage in bytes per second").unwrap();
        let uptime = Counter::new("system_uptime", "System uptime in seconds").unwrap();
        
        if let Some(registry) = registry {
            registry.register(Box::new(cpu_usage.clone())).unwrap();
            registry.register(Box::new(memory_usage.clone())).unwrap();
            registry.register(Box::new(disk_usage.clone())).unwrap();
            registry.register(Box::new(network_usage.clone())).unwrap();
            registry.register(Box::new(uptime.clone())).unwrap();
        }
        
        Self {
            cpu_usage,
            memory_usage,
            disk_usage,
            network_usage,
            uptime,
            start_time: Instant::now(),
        }
    }
    
    /// Get a snapshot of system metrics
    fn snapshot(&self) -> SystemMetricsSnapshot {
        SystemMetricsSnapshot {
            cpu_usage: self.cpu_usage.get(),
            memory_usage: self.memory_usage.get() as u64,
            disk_usage: self.disk_usage.get() as u64,
            network_usage: self.network_usage.get() as u64,
            uptime: self.uptime.get() as u64,
        }
    }
}

/// Network metrics
pub struct NetworkMetrics {
    /// Total bytes sent
    pub bytes_sent: Counter,
    
    /// Total bytes received
    pub bytes_received: Counter,
    
    /// Active connections
    pub active_connections: IntGauge,
    
    /// Connection latency
    pub connection_latency: Histogram,
    
    /// Connection errors
    pub connection_errors: IntCounter,
    
    /// Packets dropped
    pub packets_dropped: IntCounter,
}

impl NetworkMetrics {
    /// Create new network metrics
    fn new(registry: Option<&Registry>) -> Self {
        let bytes_sent = Counter::new("network_bytes_sent", "Total bytes sent").unwrap();
        let bytes_received = Counter::new("network_bytes_received", "Total bytes received").unwrap();
        let active_connections = IntGauge::new("network_active_connections", "Number of active connections").unwrap();
        
        let connection_latency = Histogram::with_opts(
            HistogramOpts::new(
                "network_connection_latency",
                "Connection latency in milliseconds"
            )
            .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0])
        ).unwrap();
        
        let connection_errors = IntCounter::new("network_connection_errors", "Number of connection errors").unwrap();
        let packets_dropped = IntCounter::new("network_packets_dropped", "Number of packets dropped").unwrap();
        
        if let Some(registry) = registry {
            registry.register(Box::new(bytes_sent.clone())).unwrap();
            registry.register(Box::new(bytes_received.clone())).unwrap();
            registry.register(Box::new(active_connections.clone())).unwrap();
            registry.register(Box::new(connection_latency.clone())).unwrap();
            registry.register(Box::new(connection_errors.clone())).unwrap();
            registry.register(Box::new(packets_dropped.clone())).unwrap();
        }
        
        Self {
            bytes_sent,
            bytes_received,
            active_connections,
            connection_latency,
            connection_errors,
            packets_dropped,
        }
    }
    
    /// Get a snapshot of network metrics
    fn snapshot(&self) -> NetworkMetricsSnapshot {
        NetworkMetricsSnapshot {
            bytes_sent: self.bytes_sent.get() as u64,
            bytes_received: self.bytes_received.get() as u64,
            active_connections: self.active_connections.get() as u64,
            connection_errors: self.connection_errors.get() as u64,
            packets_dropped: self.packets_dropped.get() as u64,
        }
    }
}

/// RPC metrics
pub struct RpcMetrics {
    /// Total requests
    pub total_requests: IntCounter,
    
    /// Successful requests
    pub successful_requests: IntCounter,
    
    /// Failed requests
    pub failed_requests: IntCounter,
    
    /// Request latency
    pub request_latency: Histogram,
    
    /// Request rate
    pub request_rate: Gauge,
    
    /// Cache hits
    pub cache_hits: IntCounter,
    
    /// Cache misses
    pub cache_misses: IntCounter,
    
    /// Rate limited requests
    pub rate_limited_requests: IntCounter,
}

impl RpcMetrics {
    /// Create new RPC metrics
    fn new(registry: Option<&Registry>) -> Self {
        let total_requests = IntCounter::new("rpc_total_requests", "Total number of RPC requests").unwrap();
        let successful_requests = IntCounter::new("rpc_successful_requests", "Number of successful RPC requests").unwrap();
        let failed_requests = IntCounter::new("rpc_failed_requests", "Number of failed RPC requests").unwrap();
        
        let request_latency = Histogram::with_opts(
            HistogramOpts::new(
                "rpc_request_latency",
                "RPC request latency in milliseconds"
            )
            .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0])
        ).unwrap();
        
        let request_rate = Gauge::new("rpc_request_rate", "RPC requests per second").unwrap();
        let cache_hits = IntCounter::new("rpc_cache_hits", "Number of RPC cache hits").unwrap();
        let cache_misses = IntCounter::new("rpc_cache_misses", "Number of RPC cache misses").unwrap();
        let rate_limited_requests = IntCounter::new("rpc_rate_limited_requests", "Number of rate-limited RPC requests").unwrap();
        
        if let Some(registry) = registry {
            registry.register(Box::new(total_requests.clone())).unwrap();
            registry.register(Box::new(successful_requests.clone())).unwrap();
            registry.register(Box::new(failed_requests.clone())).unwrap();
            registry.register(Box::new(request_latency.clone())).unwrap();
            registry.register(Box::new(request_rate.clone())).unwrap();
            registry.register(Box::new(cache_hits.clone())).unwrap();
            registry.register(Box::new(cache_misses.clone())).unwrap();
            registry.register(Box::new(rate_limited_requests.clone())).unwrap();
        }
        
        Self {
            total_requests,
            successful_requests,
            failed_requests,
            request_latency,
            request_rate,
            cache_hits,
            cache_misses,
            rate_limited_requests,
        }
    }
    
    /// Get a snapshot of RPC metrics
    fn snapshot(&self) -> RpcMetricsSnapshot {
        RpcMetricsSnapshot {
            total_requests: self.total_requests.get() as u64,
            successful_requests: self.successful_requests.get() as u64,
            failed_requests: self.failed_requests.get() as u64,
            cache_hits: self.cache_hits.get() as u64,
            cache_misses: self.cache_misses.get() as u64,
            rate_limited_requests: self.rate_limited_requests.get() as u64,
        }
    }
}

/// Execution metrics
pub struct ExecutionMetrics {
    /// Total transactions
    pub total_transactions: IntCounter,
    
    /// Successful transactions
    pub successful_transactions: IntCounter,
    
    /// Failed transactions
    pub failed_transactions: IntCounter,
    
    /// Transaction latency
    pub transaction_latency: Histogram,
    
    /// Transaction rate
    pub transaction_rate: Gauge,
    
    /// Transaction fees
    pub transaction_fees: Counter,
    
    /// Transaction timeouts
    pub transaction_timeouts: IntCounter,
}

impl ExecutionMetrics {
    /// Create new execution metrics
    fn new(registry: Option<&Registry>) -> Self {
        let total_transactions = IntCounter::new("execution_total_transactions", "Total number of transactions").unwrap();
        let successful_transactions = IntCounter::new("execution_successful_transactions", "Number of successful transactions").unwrap();
        let failed_transactions = IntCounter::new("execution_failed_transactions", "Number of failed transactions").unwrap();
        
        let transaction_latency = Histogram::with_opts(
            HistogramOpts::new(
                "execution_transaction_latency",
                "Transaction latency in milliseconds"
            )
            .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0])
        ).unwrap();
        
        let transaction_rate = Gauge::new("execution_transaction_rate", "Transactions per second").unwrap();
        let transaction_fees = Counter::new("execution_transaction_fees", "Total transaction fees in lamports").unwrap();
        let transaction_timeouts = IntCounter::new("execution_transaction_timeouts", "Number of transaction timeouts").unwrap();
        
        if let Some(registry) = registry {
            registry.register(Box::new(total_transactions.clone())).unwrap();
            registry.register(Box::new(successful_transactions.clone())).unwrap();
            registry.register(Box::new(failed_transactions.clone())).unwrap();
            registry.register(Box::new(transaction_latency.clone())).unwrap();
            registry.register(Box::new(transaction_rate.clone())).unwrap();
            registry.register(Box::new(transaction_fees.clone())).unwrap();
            registry.register(Box::new(transaction_timeouts.clone())).unwrap();
        }
        
        Self {
            total_transactions,
            successful_transactions,
            failed_transactions,
            transaction_latency,
            transaction_rate,
            transaction_fees,
            transaction_timeouts,
        }
    }
    
    /// Get a snapshot of execution metrics
    fn snapshot(&self) -> ExecutionMetricsSnapshot {
        ExecutionMetricsSnapshot {
            total_transactions: self.total_transactions.get() as u64,
            successful_transactions: self.successful_transactions.get() as u64,
            failed_transactions: self.failed_transactions.get() as u64,
            transaction_fees: self.transaction_fees.get() as u64,
            transaction_timeouts: self.transaction_timeouts.get() as u64,
        }
    }
}

/// Trading metrics
pub struct TradingMetrics {
    /// Total trades
    pub total_trades: IntCounter,
    
    /// Successful trades
    pub successful_trades: IntCounter,
    
    /// Failed trades
    pub failed_trades: IntCounter,
    
    /// Trade volume
    pub trade_volume: Counter,
    
    /// Trade profit
    pub trade_profit: Counter,
    
    /// Trade loss
    pub trade_loss: Counter,
    
    /// Net profit
    pub net_profit: Gauge,
}

impl TradingMetrics {
    /// Create new trading metrics
    fn new(registry: Option<&Registry>) -> Self {
        let total_trades = IntCounter::new("trading_total_trades", "Total number of trades").unwrap();
        let successful_trades = IntCounter::new("trading_successful_trades", "Number of successful trades").unwrap();
        let failed_trades = IntCounter::new("trading_failed_trades", "Number of failed trades").unwrap();
        let trade_volume = Counter::new("trading_volume", "Total trade volume in USD").unwrap();
        let trade_profit = Counter::new("trading_profit", "Total trade profit in USD").unwrap();
        let trade_loss = Counter::new("trading_loss", "Total trade loss in USD").unwrap();
        let net_profit = Gauge::new("trading_net_profit", "Net profit in USD").unwrap();
        
        if let Some(registry) = registry {
            registry.register(Box::new(total_trades.clone())).unwrap();
            registry.register(Box::new(successful_trades.clone())).unwrap();
            registry.register(Box::new(failed_trades.clone())).unwrap();
            registry.register(Box::new(trade_volume.clone())).unwrap();
            registry.register(Box::new(trade_profit.clone())).unwrap();
            registry.register(Box::new(trade_loss.clone())).unwrap();
            registry.register(Box::new(net_profit.clone())).unwrap();
        }
        
        Self {
            total_trades,
            successful_trades,
            failed_trades,
            trade_volume,
            trade_profit,
            trade_loss,
            net_profit,
        }
    }
    
    /// Get a snapshot of trading metrics
    fn snapshot(&self) -> TradingMetricsSnapshot {
        TradingMetricsSnapshot {
            total_trades: self.total_trades.get() as u64,
            successful_trades: self.successful_trades.get() as u64,
            failed_trades: self.failed_trades.get() as u64,
            trade_volume: self.trade_volume.get() as f64,
            trade_profit: self.trade_profit.get() as f64,
            trade_loss: self.trade_loss.get() as f64,
            net_profit: self.net_profit.get() as f64,
        }
    }
}

/// Metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    
    /// System metrics
    pub system: SystemMetricsSnapshot,
    
    /// Network metrics
    pub network: NetworkMetricsSnapshot,
    
    /// RPC metrics
    pub rpc: RpcMetricsSnapshot,
    
    /// Execution metrics
    pub execution: ExecutionMetricsSnapshot,
    
    /// Trading metrics
    pub trading: TradingMetricsSnapshot,
}

/// System metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsSnapshot {
    /// CPU usage (percentage)
    pub cpu_usage: f64,
    
    /// Memory usage (bytes)
    pub memory_usage: u64,
    
    /// Disk usage (bytes)
    pub disk_usage: u64,
    
    /// Network usage (bytes/sec)
    pub network_usage: u64,
    
    /// Uptime (seconds)
    pub uptime: u64,
}

/// Network metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetricsSnapshot {
    /// Total bytes sent
    pub bytes_sent: u64,
    
    /// Total bytes received
    pub bytes_received: u64,
    
    /// Active connections
    pub active_connections: u64,
    
    /// Connection errors
    pub connection_errors: u64,
    
    /// Packets dropped
    pub packets_dropped: u64,
}

/// RPC metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcMetricsSnapshot {
    /// Total requests
    pub total_requests: u64,
    
    /// Successful requests
    pub successful_requests: u64,
    
    /// Failed requests
    pub failed_requests: u64,
    
    /// Cache hits
    pub cache_hits: u64,
    
    /// Cache misses
    pub cache_misses: u64,
    
    /// Rate limited requests
    pub rate_limited_requests: u64,
}

/// Execution metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetricsSnapshot {
    /// Total transactions
    pub total_transactions: u64,
    
    /// Successful transactions
    pub successful_transactions: u64,
    
    /// Failed transactions
    pub failed_transactions: u64,
    
    /// Transaction fees
    pub transaction_fees: u64,
    
    /// Transaction timeouts
    pub transaction_timeouts: u64,
}

/// Trading metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingMetricsSnapshot {
    /// Total trades
    pub total_trades: u64,
    
    /// Successful trades
    pub successful_trades: u64,
    
    /// Failed trades
    pub failed_trades: u64,
    
    /// Trade volume
    pub trade_volume: f64,
    
    /// Trade profit
    pub trade_profit: f64,
    
    /// Trade loss
    pub trade_loss: f64,
    
    /// Net profit
    pub net_profit: f64,
}

/// Metrics error types
#[derive(thiserror::Error, Debug)]
pub enum MetricsError {
    #[error("Metrics system already running")]
    AlreadyRunning,
    
    #[error("Metrics system not running")]
    NotRunning,
    
    #[error("Prometheus error: {0}")]
    PrometheusError(String),
    
    #[error("Core error: {0}")]
    Core(#[from] CoreError),
}

/// Module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the module
pub fn init() {
    info!("Initializing metrics module");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();
        assert!(config.enable_prometheus);
        assert_eq!(config.prometheus_endpoint, "0.0.0.0:9090");
        assert!(config.log_metrics);
        assert_eq!(config.log_interval_seconds, 60);
        assert!(config.detailed_metrics);
        assert_eq!(config.history_size, 1000);
    }
    
    #[test]
    fn test_metrics_system_start_stop() {
        let config = MetricsConfig::default();
        let mut system = MetricsSystem::new(config);
        
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