//! RPC endpoint monitoring
//! 
//! This module provides functionality for monitoring RPC endpoints,
//! including health checks, latency tracking, and alerting.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, broadcast};
use tokio::time::interval;
use tracing::{debug, error, info, warn, trace};
use anyhow::{anyhow, Result};

use crate::endpoints::EndpointManager;
use crate::metrics::RpcMetrics;

/// RPC monitor configuration
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Interval for checking endpoint health in milliseconds
    pub health_check_interval_ms: u64,
    
    /// Threshold for high latency in milliseconds
    pub high_latency_threshold_ms: u64,
    
    /// Threshold for consecutive errors before alerting
    pub consecutive_errors_threshold: u32,
    
    /// Whether to enable automatic endpoint disabling
    pub enable_auto_disable: bool,
    
    /// Threshold for consecutive errors before auto-disabling
    pub auto_disable_threshold: u32,
    
    /// Whether to enable automatic endpoint re-enabling
    pub enable_auto_reenable: bool,
    
    /// Interval for attempting to re-enable endpoints in milliseconds
    pub auto_reenable_interval_ms: u64,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            health_check_interval_ms: 10_000, // 10 seconds
            high_latency_threshold_ms: 1_000, // 1 second
            consecutive_errors_threshold: 5,
            enable_auto_disable: true,
            auto_disable_threshold: 10,
            enable_auto_reenable: true,
            auto_reenable_interval_ms: 300_000, // 5 minutes
        }
    }
}

/// Alert level for endpoint issues
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    /// Informational alert
    Info,
    
    /// Warning alert
    Warning,
    
    /// Error alert
    Error,
    
    /// Critical alert
    Critical,
}

/// Alert for endpoint issues
#[derive(Debug, Clone)]
pub struct EndpointAlert {
    /// Endpoint URL
    pub url: String,
    
    /// Alert level
    pub level: AlertLevel,
    
    /// Alert message
    pub message: String,
    
    /// Alert timestamp
    pub timestamp: Instant,
    
    /// Whether the alert has been acknowledged
    pub acknowledged: bool,
}

/// RPC monitor for tracking endpoint health
pub struct RpcMonitor {
    /// Endpoint manager
    endpoint_manager: Arc<EndpointManager>,
    
    /// Metrics collector
    metrics: Arc<RpcMetrics>,
    
    /// Monitor configuration
    config: MonitorConfig,
    
    /// Alert channel sender
    alert_tx: broadcast::Sender<EndpointAlert>,
    
    /// Command channel sender
    command_tx: mpsc::Sender<MonitorCommand>,
    
    /// Command channel receiver
    command_rx: Option<mpsc::Receiver<MonitorCommand>>,
    
    /// Active alerts
    alerts: Vec<EndpointAlert>,
    
    /// Disabled endpoints
    disabled_endpoints: Vec<(String, Instant)>,
    
    /// Monitor task handle
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Monitor command
enum MonitorCommand {
    /// Check endpoint health
    CheckHealth,
    
    /// Acknowledge alert
    AcknowledgeAlert {
        /// Alert ID (URL + timestamp)
        id: String,
    },
    
    /// Disable endpoint
    DisableEndpoint {
        /// Endpoint URL
        url: String,
    },
    
    /// Enable endpoint
    EnableEndpoint {
        /// Endpoint URL
        url: String,
    },
    
    /// Stop the monitor
    Stop {
        /// Response channel
        response_tx: oneshot::Sender<()>,
    },
}

impl RpcMonitor {
    /// Create a new RPC monitor
    pub fn new(
        endpoint_manager: Arc<EndpointManager>,
        metrics: Arc<RpcMetrics>,
        health_check_interval_ms: u64,
    ) -> Self {
        let config = MonitorConfig {
            health_check_interval_ms,
            ..Default::default()
        };
        
        let (alert_tx, _) = broadcast::channel(100);
        let (command_tx, command_rx) = mpsc::channel(100);
        
        Self {
            endpoint_manager,
            metrics,
            config,
            alert_tx,
            command_tx,
            command_rx: Some(command_rx),
            alerts: Vec::new(),
            disabled_endpoints: Vec::new(),
            task_handle: None,
        }
    }
    
    /// Start the monitor
    pub async fn start(&self) -> Result<()> {
        if self.task_handle.is_some() {
            return Err(anyhow!("Monitor already started"));
        }
        
        let endpoint_manager = self.endpoint_manager.clone();
        let metrics = self.metrics.clone();
        let config = self.config.clone();
        let alert_tx = self.alert_tx.clone();
        let mut command_rx = self.command_rx.take().ok_or_else(|| anyhow!("Command receiver already taken"))?;
        let command_tx = self.command_tx.clone();
        
        // Spawn the monitor task
        let handle = tokio::spawn(async move {
            let mut health_check_interval = interval(Duration::from_millis(config.health_check_interval_ms));
            let mut reenable_interval = interval(Duration::from_millis(config.auto_reenable_interval_ms));
            let mut alerts = Vec::new();
            let mut disabled_endpoints = Vec::new();
            
            // Send initial health check command
            let _ = command_tx.send(MonitorCommand::CheckHealth).await;
            
            loop {
                tokio::select! {
                    _ = health_check_interval.tick() => {
                        // Send health check command
                        let _ = command_tx.send(MonitorCommand::CheckHealth).await;
                    }
                    _ = reenable_interval.tick() => {
                        if config.enable_auto_reenable {
                            // Try to re-enable disabled endpoints
                            let now = Instant::now();
                            let mut to_reenable = Vec::new();
                            
                            for (i, (url, disabled_at)) in disabled_endpoints.iter().enumerate() {
                                if now.duration_since(*disabled_at).as_millis() >= config.auto_reenable_interval_ms as u128 {
                                    to_reenable.push((i, url.clone()));
                                }
                            }
                            
                            // Re-enable endpoints in reverse order to avoid index issues
                            for (i, url) in to_reenable.into_iter().rev() {
                                info!("Auto-reenabling endpoint: {}", url);
                                endpoint_manager.enable_endpoint(&url).await;
                                disabled_endpoints.remove(i);
                            }
                        }
                    }
                    Some(cmd) = command_rx.recv() => {
                        match cmd {
                            MonitorCommand::CheckHealth => {
                                // Check endpoint health
                                if let Err(e) = endpoint_manager.update_endpoint_health().await {
                                    error!("Failed to update endpoint health: {}", e);
                                }
                                
                                // Get endpoint status
                                let status = endpoint_manager.get_endpoint_status().await;
                                
                                // Check for issues
                                for endpoint in &status {
                                    // Check for high latency
                                    if endpoint.avg_latency_ms > config.high_latency_threshold_ms as f64 {
                                        let alert = EndpointAlert {
                                            url: endpoint.url.clone(),
                                            level: AlertLevel::Warning,
                                            message: format!("High latency: {:.2}ms", endpoint.avg_latency_ms),
                                            timestamp: Instant::now(),
                                            acknowledged: false,
                                        };
                                        
                                        alerts.push(alert.clone());
                                        let _ = alert_tx.send(alert);
                                    }
                                    
                                    // Check for consecutive errors
                                    if endpoint.consecutive_errors >= config.consecutive_errors_threshold {
                                        let alert = EndpointAlert {
                                            url: endpoint.url.clone(),
                                            level: AlertLevel::Error,
                                            message: format!("Consecutive errors: {}", endpoint.consecutive_errors),
                                            timestamp: Instant::now(),
                                            acknowledged: false,
                                        };
                                        
                                        alerts.push(alert.clone());
                                        let _ = alert_tx.send(alert);
                                    }
                                    
                                    // Check for auto-disable threshold
                                    if config.enable_auto_disable && 
                                       endpoint.consecutive_errors >= config.auto_disable_threshold {
                                        // Check if already disabled
                                        if !disabled_endpoints.iter().any(|(url, _)| url == &endpoint.url) {
                                            info!("Auto-disabling endpoint due to consecutive errors: {}", endpoint.url);
                                            endpoint_manager.disable_endpoint(&endpoint.url).await;
                                            disabled_endpoints.push((endpoint.url.clone(), Instant::now()));
                                            
                                            let alert = EndpointAlert {
                                                url: endpoint.url.clone(),
                                                level: AlertLevel::Critical,
                                                message: format!("Endpoint auto-disabled due to {} consecutive errors", endpoint.consecutive_errors),
                                                timestamp: Instant::now(),
                                                acknowledged: false,
                                            };
                                            
                                            alerts.push(alert.clone());
                                            let _ = alert_tx.send(alert);
                                        }
                                    }
                                }
                                
                                // Update metrics
                                metrics.update_endpoint_metrics(&status);
                            }
                            MonitorCommand::AcknowledgeAlert { id } => {
                                // Find and acknowledge alert
                                for alert in &mut alerts {
                                    let alert_id = format!("{}:{:?}", alert.url, alert.timestamp);
                                    if alert_id == id {
                                        alert.acknowledged = true;
                                        break;
                                    }
                                }
                            }
                            MonitorCommand::DisableEndpoint { url } => {
                                // Disable endpoint
                                endpoint_manager.disable_endpoint(&url).await;
                                
                                // Add to disabled list if not already there
                                if !disabled_endpoints.iter().any(|(u, _)| u == &url) {
                                    disabled_endpoints.push((url, Instant::now()));
                                }
                            }
                            MonitorCommand::EnableEndpoint { url } => {
                                // Enable endpoint
                                endpoint_manager.enable_endpoint(&url).await;
                                
                                // Remove from disabled list
                                disabled_endpoints.retain(|(u, _)| u != &url);
                            }
                            MonitorCommand::Stop { response_tx } => {
                                info!("Stopping RPC monitor");
                                let _ = response_tx.send(());
                                break;
                            }
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Stop the monitor
    pub async fn stop(&self) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(MonitorCommand::Stop { response_tx }).await
            .map_err(|e| anyhow!("Failed to send stop command: {}", e))?;
        
        response_rx.await
            .map_err(|e| anyhow!("Failed to receive stop response: {}", e))?;
        
        Ok(())
    }
    
    /// Subscribe to alerts
    pub fn subscribe_alerts(&self) -> broadcast::Receiver<EndpointAlert> {
        self.alert_tx.subscribe()
    }
    
    /// Acknowledge an alert
    pub async fn acknowledge_alert(&self, id: &str) -> Result<()> {
        self.command_tx.send(MonitorCommand::AcknowledgeAlert { id: id.to_string() }).await
            .map_err(|e| anyhow!("Failed to send acknowledge command: {}", e))?;
        
        Ok(())
    }
    
    /// Disable an endpoint
    pub async fn disable_endpoint(&self, url: &str) -> Result<()> {
        self.command_tx.send(MonitorCommand::DisableEndpoint { url: url.to_string() }).await
            .map_err(|e| anyhow!("Failed to send disable command: {}", e))?;
        
        Ok(())
    }
    
    /// Enable an endpoint
    pub async fn enable_endpoint(&self, url: &str) -> Result<()> {
        self.command_tx.send(MonitorCommand::EnableEndpoint { url: url.to_string() }).await
            .map_err(|e| anyhow!("Failed to send enable command: {}", e))?;
        
        Ok(())
    }
    
    /// Get active alerts
    pub fn get_alerts(&self) -> Vec<EndpointAlert> {
        self.alerts.clone()
    }
    
    /// Get disabled endpoints
    pub fn get_disabled_endpoints(&self) -> Vec<String> {
        self.disabled_endpoints.iter().map(|(url, _)| url.clone()).collect()
    }
}
