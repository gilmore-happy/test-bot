//! Jito ShredStream integration for Solana HFT Bot
//!
//! This module provides integration with Jito's ShredStream service for low-latency
//! block updates and transaction monitoring.

#[cfg(feature = "jito-full")]
use std::collections::HashMap;
#[cfg(feature = "jito-full")]
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
#[cfg(feature = "jito-full")]
use std::path::PathBuf;
#[cfg(feature = "jito-full")]
use std::sync::Arc;
#[cfg(feature = "jito-full")]
use std::time::Duration;

#[cfg(feature = "jito-full")]
use anyhow::{anyhow, Context, Result};
#[cfg(feature = "jito-full")]
use jito_shredstream_proxy::proxy::ShredstreamProxy;
#[cfg(feature = "jito-full")]
use parking_lot::RwLock;
#[cfg(feature = "jito-full")]
use solana_sdk::signature::Keypair;
#[cfg(feature = "jito-full")]
use tokio::sync::{mpsc, oneshot};
#[cfg(feature = "jito-full")]
use tracing::{debug, error, info, instrument, trace, warn};

#[cfg(feature = "jito-full")]
use crate::ExecutionError;

/// Configuration for Jito ShredStream integration
#[cfg(feature = "jito-full")]
#[derive(Debug, Clone)]
pub struct ShredStreamConfig {
    /// Block engine URL
    pub block_engine_url: String,
    
    /// Auth keypair path
    pub auth_keypair_path: Option<String>,
    
    /// Desired regions
    pub desired_regions: Vec<String>,
    
    /// Destination IP:Port combinations
    pub dest_ip_ports: Vec<String>,
    
    /// Source bind port
    pub src_bind_port: Option<u16>,
    
    /// Whether to enable gRPC service for transaction decoding
    pub enable_grpc_service: bool,
    
    /// gRPC service port
    pub grpc_service_port: Option<u16>,
    
    /// Whether to enable metrics
    pub enable_metrics: bool,
    
    /// Metrics port
    pub metrics_port: Option<u16>,
}

#[cfg(feature = "jito-full")]
impl Default for ShredStreamConfig {
    fn default() -> Self {
        Self {
            block_engine_url: "https://mainnet.block-engine.jito.wtf".to_string(),
            auth_keypair_path: None,
            desired_regions: vec!["amsterdam".to_string(), "ny".to_string()],
            dest_ip_ports: vec!["127.0.0.1:8001".to_string()],
            src_bind_port: Some(20000),
            enable_grpc_service: false,
            grpc_service_port: Some(50051),
            enable_metrics: false,
            metrics_port: Some(8080),
        }
    }
}

/// Jito ShredStream client for low-latency block updates
#[cfg(feature = "jito-full")]
pub struct ShredStreamClient {
    /// Configuration
    config: ShredStreamConfig,
    
    /// Proxy instance
    proxy: Option<Arc<ShredstreamProxy>>,
    
    /// Shutdown channel
    shutdown_tx: Option<oneshot::Sender<()>>,
    
    /// Status
    status: Arc<RwLock<ShredStreamStatus>>,
    
    /// Metrics
    metrics: Arc<RwLock<ShredStreamMetrics>>,
}

/// ShredStream status
#[cfg(feature = "jito-full")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShredStreamStatus {
    /// Not started
    NotStarted,
    
    /// Starting
    Starting,
    
    /// Running
    Running,
    
    /// Stopped
    Stopped,
    
    /// Error
    Error(String),
}

/// ShredStream metrics
#[cfg(feature = "jito-full")]
#[derive(Debug, Clone, Default)]
pub struct ShredStreamMetrics {
    /// Total shreds received
    pub total_shreds: u64,
    
    /// Shreds received per second
    pub shreds_per_second: f64,
    
    /// Shreds received per region
    pub shreds_per_region: HashMap<String, u64>,
    
    /// Transactions decoded
    pub transactions_decoded: u64,
    
    /// Blocks processed
    pub blocks_processed: u64,
    
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
}

#[cfg(feature = "jito-full")]
impl ShredStreamClient {
    /// Create a new ShredStream client
    pub fn new(config: ShredStreamConfig) -> Self {
        Self {
            config,
            proxy: None,
            shutdown_tx: None,
            status: Arc::new(RwLock::new(ShredStreamStatus::NotStarted)),
            metrics: Arc::new(RwLock::new(ShredStreamMetrics::default())),
        }
    }
    
    /// Start the ShredStream client
    pub async fn start(&mut self) -> Result<(), ExecutionError> {
        // Update status
        *self.status.write() = ShredStreamStatus::Starting;
        
        // Parse keypair if provided
        let keypair = if let Some(keypair_path) = &self.config.auth_keypair_path {
            let keypair_path = PathBuf::from(keypair_path);
            match read_keypair_file(&keypair_path) {
                Ok(keypair) => Some(keypair),
                Err(e) => {
                    let error = format!("Failed to read keypair file: {}", e);
                    *self.status.write() = ShredStreamStatus::Error(error.clone());
                    return Err(ExecutionError::InvalidConfig(error));
                }
            }
        } else {
            None
        };
        
        // Parse destination IP:Port combinations
        let mut dest_addrs = Vec::new();
        for dest in &self.config.dest_ip_ports {
            match parse_socket_addr(dest) {
                Ok(addr) => dest_addrs.push(addr),
                Err(e) => {
                    let error = format!("Failed to parse destination address {}: {}", dest, e);
                    *self.status.write() = ShredStreamStatus::Error(error.clone());
                    return Err(ExecutionError::InvalidConfig(error));
                }
            }
        }
        
        if dest_addrs.is_empty() {
            let error = "No valid destination addresses provided".to_string();
            *self.status.write() = ShredStreamStatus::Error(error.clone());
            return Err(ExecutionError::InvalidConfig(error));
        }
        
        // Create proxy options
        let mut proxy_options = jito_shredstream::proxy::Options {
            block_engine_url: self.config.block_engine_url.clone(),
            auth_keypair: keypair,
            desired_regions: self.config.desired_regions.clone(),
            dest_addrs,
            src_bind_addr: if let Some(port) = self.config.src_bind_port {
                Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port))
            } else {
                None
            },
            grpc_service_addr: if self.config.enable_grpc_service {
                if let Some(port) = self.config.grpc_service_port {
                    Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port))
                } else {
                    None
                }
            } else {
                None
            },
            metrics_addr: if self.config.enable_metrics {
                if let Some(port) = self.config.metrics_port {
                    Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port))
                } else {
                    None
                }
            } else {
                None
            },
        };
        
        // Create proxy
        let proxy = match ShredstreamProxy::new(proxy_options) {
            Ok(proxy) => Arc::new(proxy),
            Err(e) => {
                let error = format!("Failed to create ShredStream proxy: {}", e);
                *self.status.write() = ShredStreamStatus::Error(error.clone());
                return Err(ExecutionError::Internal(error));
            }
        };
        
        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        
        // Start proxy in a separate task
        let proxy_clone = proxy.clone();
        let status_clone = self.status.clone();
        let metrics_clone = self.metrics.clone();
        
        tokio::spawn(async move {
            info!("Starting ShredStream proxy");
            
            // Update status
            *status_clone.write() = ShredStreamStatus::Running;
            
            // Create metrics channel
            let (metrics_tx, mut metrics_rx) = mpsc::channel(100);
            
            // Start metrics collection
            let metrics_clone2 = metrics_clone.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(1));
                
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            // Update metrics
                            let mut metrics = metrics_clone2.write();
                            metrics.shreds_per_second = metrics.total_shreds as f64;
                            metrics.total_shreds = 0;
                        }
                        
                        Some(metric) = metrics_rx.recv() => {
                            match metric {
                                ShredMetric::Received { region } => {
                                    let mut metrics = metrics_clone2.write();
                                    metrics.total_shreds += 1;
                                    
                                    let entry = metrics.shreds_per_region.entry(region).or_insert(0);
                                    *entry += 1;
                                }
                                ShredMetric::Decoded => {
                                    let mut metrics = metrics_clone2.write();
                                    metrics.transactions_decoded += 1;
                                }
                                ShredMetric::BlockProcessed { latency_ms } => {
                                    let mut metrics = metrics_clone2.write();
                                    metrics.blocks_processed += 1;
                                    
                                    // Update average latency
                                    let total_blocks = metrics.blocks_processed;
                                    let current_avg = metrics.avg_latency_ms;
                                    
                                    metrics.avg_latency_ms = (current_avg * (total_blocks - 1) as f64 + latency_ms) / total_blocks as f64;
                                }
                            }
                        }
                    }
                }
            });
            
            // Run proxy with shutdown signal
            match proxy_clone.run_with_shutdown(shutdown_rx).await {
                Ok(_) => {
                    info!("ShredStream proxy stopped gracefully");
                    *status_clone.write() = ShredStreamStatus::Stopped;
                }
                Err(e) => {
                    error!("ShredStream proxy error: {}", e);
                    *status_clone.write() = ShredStreamStatus::Error(e.to_string());
                }
            }
        });
        
        // Store proxy and shutdown channel
        self.proxy = Some(proxy);
        self.shutdown_tx = Some(shutdown_tx);
        
        Ok(())
    }
    
    /// Stop the ShredStream client
    pub async fn stop(&mut self) -> Result<(), ExecutionError> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            // Send shutdown signal
            if let Err(e) = shutdown_tx.send(()) {
                warn!("Failed to send shutdown signal: {}", e);
            }
            
            // Wait for proxy to stop
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Update status
            *self.status.write() = ShredStreamStatus::Stopped;
        }
        
        self.proxy = None;
        
        Ok(())
    }
    
    /// Get the current status
    pub fn get_status(&self) -> ShredStreamStatus {
        self.status.read().clone()
    }
    
    /// Get the current metrics
    pub fn get_metrics(&self) -> ShredStreamMetrics {
        self.metrics.read().clone()
    }
    
    /// Check if the client is running
    pub fn is_running(&self) -> bool {
        matches!(*self.status.read(), ShredStreamStatus::Running)
    }
}

/// Shred metric type for internal metrics collection
#[cfg(feature = "jito-full")]
enum ShredMetric {
    /// Shred received
    Received { region: String },
    
    /// Shred decoded
    Decoded,
    
    /// Block processed
    BlockProcessed { latency_ms: f64 },
}

/// Parse a socket address from a string
#[cfg(feature = "jito-full")]
fn parse_socket_addr(addr: &str) -> Result<SocketAddr> {
    addr.parse::<SocketAddr>()
        .map_err(|e| anyhow!("Invalid socket address: {}", e))
}

/// Read a keypair from a file
#[cfg(feature = "jito-full")]
fn read_keypair_file(path: &PathBuf) -> Result<Keypair> {
    solana_sdk::signature::read_keypair_file(path)
        .map_err(|e| anyhow!("Failed to read keypair file: {}", e))
}

// Mock implementation when jito-full feature is not enabled
#[cfg(not(feature = "jito-full"))]
pub struct ShredStreamConfig;

#[cfg(not(feature = "jito-full"))]
impl Default for ShredStreamConfig {
    fn default() -> Self {
        Self
    }
}

#[cfg(not(feature = "jito-full"))]
pub struct ShredStreamClient;

#[cfg(not(feature = "jito-full"))]
impl ShredStreamClient {
    pub fn new(_config: ShredStreamConfig) -> Self {
        Self
    }
    
    pub async fn start(&mut self) -> Result<(), crate::ExecutionError> {
        Err(crate::ExecutionError::InvalidConfig(
            "Jito ShredStream support is not enabled. Enable the 'jito-full' feature to use ShredStream.".to_string()
        ))
    }
    
    pub async fn stop(&mut self) -> Result<(), crate::ExecutionError> {
        Ok(())
    }
}

#[cfg(not(feature = "jito-full"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShredStreamStatus {
    NotStarted,
    Error(String),
}

#[cfg(not(feature = "jito-full"))]
#[derive(Debug, Clone, Default)]
pub struct ShredStreamMetrics;