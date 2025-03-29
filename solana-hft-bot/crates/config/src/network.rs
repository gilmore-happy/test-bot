//! Network optimization and endpoint management
//!
//! This module provides functionality for network optimization
//! and RPC endpoint management.

use crate::error::{ConfigError, ConfigResult};
use crate::hardware::HardwareFeatureFlags;
use crate::schema::{
    EndpointConfig, EndpointSelectionStrategy, KernelBypassMode, LatencyOptimizationConfig,
    NetworkConfig, RpcEndpoint, SocketBufferConfig, TcpOptimizations, ZeroCopyConfig,
};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Network optimizer
#[derive(Debug)]
pub struct NetworkOptimizer {
    /// Network configuration
    config: NetworkConfig,
    
    /// Hardware feature flags
    feature_flags: HardwareFeatureFlags,
}

impl NetworkOptimizer {
    /// Create a new network optimizer
    pub fn new(config: NetworkConfig, feature_flags: HardwareFeatureFlags) -> Self {
        Self {
            config,
            feature_flags,
        }
    }
    
    /// Optimize network configuration based on hardware capabilities
    pub fn optimize(&mut self) -> ConfigResult<()> {
        info!("Optimizing network configuration...");
        
        // Optimize kernel bypass mode
        self.optimize_kernel_bypass();
        
        // Optimize TCP settings
        self.optimize_tcp_settings();
        
        // Optimize socket buffers
        self.optimize_socket_buffers();
        
        // Optimize zero-copy settings
        self.optimize_zero_copy();
        
        info!("Network optimization complete");
        
        Ok(())
    }
    
    /// Optimize kernel bypass mode
    fn optimize_kernel_bypass(&mut self) {
        if self.feature_flags.kernel_bypass_networking {
            if self.feature_flags.dpdk_support && self.feature_flags.io_uring_support {
                info!("Enabling DPDK and io_uring kernel bypass");
                self.config.kernel_bypass_mode = KernelBypassMode::DpdkAndIoUring;
                
                // Enable DPDK
                self.config.dpdk.enabled = true;
                
                // Enable io_uring
                self.config.io_uring.enabled = true;
            } else if self.feature_flags.dpdk_support {
                info!("Enabling DPDK kernel bypass");
                self.config.kernel_bypass_mode = KernelBypassMode::Dpdk;
                
                // Enable DPDK
                self.config.dpdk.enabled = true;
            } else if self.feature_flags.io_uring_support {
                info!("Enabling io_uring kernel bypass");
                self.config.kernel_bypass_mode = KernelBypassMode::IoUring;
                
                // Enable io_uring
                self.config.io_uring.enabled = true;
            } else {
                warn!("Kernel bypass networking is enabled in feature flags, but neither DPDK nor io_uring is supported");
                self.config.kernel_bypass_mode = KernelBypassMode::None;
            }
        } else {
            debug!("Kernel bypass networking is disabled");
            self.config.kernel_bypass_mode = KernelBypassMode::None;
        }
    }
    
    /// Optimize TCP settings
    fn optimize_tcp_settings(&mut self) {
        // Always enable TCP_NODELAY for low latency
        self.config.tcp_optimizations.tcp_nodelay = true;
        
        // Enable TCP_QUICKACK for low latency
        self.config.tcp_optimizations.tcp_quickack = true;
        
        // Enable TCP_FASTOPEN if high-frequency networking is available
        self.config.tcp_optimizations.tcp_fastopen = self.feature_flags.high_frequency_networking;
        
        // Disable TCP_CORK as it can increase latency
        self.config.tcp_optimizations.tcp_cork = false;
        
        // Enable TCP_DEFER_ACCEPT if high-frequency networking is available
        self.config.tcp_optimizations.tcp_defer_accept = self.feature_flags.high_frequency_networking;
        
        // Set keepalive interval based on network speed
        if self.feature_flags.high_frequency_networking {
            self.config.tcp_optimizations.keepalive_interval_secs = 5;
        } else {
            self.config.tcp_optimizations.keepalive_interval_secs = 15;
        }
        
        // Set connection backlog based on network speed
        if self.feature_flags.high_frequency_networking {
            self.config.tcp_optimizations.connection_backlog = 1024;
        } else {
            self.config.tcp_optimizations.connection_backlog = 128;
        }
    }
    
    /// Optimize socket buffers
    fn optimize_socket_buffers(&mut self) {
        if self.feature_flags.high_frequency_networking {
            // Large buffers for high-frequency networking
            self.config.socket_buffers.send_buffer_size = 8 * 1024 * 1024; // 8MB
            self.config.socket_buffers.recv_buffer_size = 8 * 1024 * 1024; // 8MB
            self.config.socket_buffers.dynamic_buffer_sizing = true;
            self.config.socket_buffers.max_buffer_size = 32 * 1024 * 1024; // 32MB
        } else {
            // Moderate buffers for normal networking
            self.config.socket_buffers.send_buffer_size = 2 * 1024 * 1024; // 2MB
            self.config.socket_buffers.recv_buffer_size = 2 * 1024 * 1024; // 2MB
            self.config.socket_buffers.dynamic_buffer_sizing = false;
            self.config.socket_buffers.max_buffer_size = 8 * 1024 * 1024; // 8MB
        }
    }
    
    /// Optimize zero-copy settings
    fn optimize_zero_copy(&mut self) {
        // Enable zero-copy if kernel bypass is available
        self.config.zero_copy.enabled = self.config.kernel_bypass_mode != KernelBypassMode::None;
        
        // Set memory map size based on available memory
        if self.feature_flags.high_frequency_networking {
            self.config.zero_copy.memory_map_size = 256 * 1024 * 1024; // 256MB
        } else {
            self.config.zero_copy.memory_map_size = 64 * 1024 * 1024; // 64MB
        }
        
        // Enable hardware offloading if available
        self.config.zero_copy.hardware_offloading = self.feature_flags.dpdk_support;
        
        // Enable direct memory access if available
        self.config.zero_copy.direct_memory_access = self.feature_flags.direct_memory_access;
    }
    
    /// Get optimized network configuration
    pub fn get_config(&self) -> NetworkConfig {
        self.config.clone()
    }
}

/// Endpoint health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointHealth {
    /// Endpoint is healthy
    Healthy,
    
    /// Endpoint is degraded (high latency or errors)
    Degraded,
    
    /// Endpoint is unhealthy (not responding)
    Unhealthy,
}

/// Endpoint statistics
#[derive(Debug, Clone)]
pub struct EndpointStats {
    /// Endpoint URL
    pub url: String,
    
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    
    /// Minimum latency in milliseconds
    pub min_latency_ms: f64,
    
    /// Maximum latency in milliseconds
    pub max_latency_ms: f64,
    
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
    
    /// Request count
    pub request_count: u64,
    
    /// Error count
    pub error_count: u64,
    
    /// Last check time
    pub last_check: Instant,
    
    /// Health status
    pub health: EndpointHealth,
    
    /// Geographic region
    pub region: String,
}

impl EndpointStats {
    /// Create new endpoint statistics
    pub fn new(url: String, region: String) -> Self {
        Self {
            url,
            avg_latency_ms: 0.0,
            min_latency_ms: f64::MAX,
            max_latency_ms: 0.0,
            success_rate: 1.0,
            request_count: 0,
            error_count: 0,
            last_check: Instant::now(),
            health: EndpointHealth::Healthy,
            region,
        }
    }
    
    /// Update statistics with a new latency measurement
    pub fn update_latency(&mut self, latency_ms: f64, success: bool) {
        self.last_check = Instant::now();
        
        if success {
            // Update latency statistics
            self.min_latency_ms = self.min_latency_ms.min(latency_ms);
            self.max_latency_ms = self.max_latency_ms.max(latency_ms);
            
            // Update average latency using weighted average
            if self.request_count > 0 {
                self.avg_latency_ms = (self.avg_latency_ms * 0.9) + (latency_ms * 0.1);
            } else {
                self.avg_latency_ms = latency_ms;
            }
            
            self.request_count += 1;
        } else {
            self.error_count += 1;
            self.request_count += 1;
        }
        
        // Update success rate
        self.success_rate = 1.0 - (self.error_count as f64 / self.request_count as f64);
        
        // Update health status
        self.update_health();
    }
    
    /// Update health status based on statistics
    fn update_health(&mut self) {
        if self.success_rate < 0.5 {
            self.health = EndpointHealth::Unhealthy;
        } else if self.success_rate < 0.9 || self.avg_latency_ms > 1000.0 {
            self.health = EndpointHealth::Degraded;
        } else {
            self.health = EndpointHealth::Healthy;
        }
    }
    
    /// Check if the endpoint is healthy
    pub fn is_healthy(&self) -> bool {
        self.health == EndpointHealth::Healthy
    }
    
    /// Check if the endpoint is usable (healthy or degraded)
    pub fn is_usable(&self) -> bool {
        self.health != EndpointHealth::Unhealthy
    }
    
    /// Get quality score (0.0 - 1.0)
    pub fn quality_score(&self) -> f64 {
        // Calculate score based on latency and success rate
        let latency_score = if self.avg_latency_ms <= 100.0 {
            1.0
        } else if self.avg_latency_ms >= 1000.0 {
            0.0
        } else {
            1.0 - ((self.avg_latency_ms - 100.0) / 900.0)
        };
        
        // Combine scores with weights
        (latency_score * 0.7) + (self.success_rate * 0.3)
    }
}

/// Endpoint manager
#[derive(Debug)]
pub struct EndpointManager {
    /// Endpoint configuration
    config: EndpointConfig,
    
    /// Endpoint statistics
    stats: Arc<Mutex<HashMap<String, EndpointStats>>>,
    
    /// Current primary endpoint
    primary_endpoint: Arc<Mutex<Option<String>>>,
    
    /// Latency optimization configuration
    latency_optimization: LatencyOptimizationConfig,
    
    /// Validator proximity map
    validator_proximity: Arc<Mutex<HashMap<String, f64>>>,
}

impl EndpointManager {
    /// Create a new endpoint manager
    pub fn new(config: EndpointConfig, latency_optimization: LatencyOptimizationConfig) -> Self {
        let mut stats = HashMap::new();
        
        // Initialize statistics for all endpoints
        for endpoint in &config.rpc_endpoints {
            stats.insert(
                endpoint.url.clone(),
                EndpointStats::new(endpoint.url.clone(), endpoint.region.clone()),
            );
        }
        
        Self {
            config,
            stats: Arc::new(Mutex::new(stats)),
            primary_endpoint: Arc::new(Mutex::new(None)),
            latency_optimization,
            validator_proximity: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Get all endpoints
    pub fn get_endpoints(&self) -> Vec<RpcEndpoint> {
        self.config.rpc_endpoints.clone()
    }
    
    /// Get all endpoint statistics
    pub fn get_stats(&self) -> HashMap<String, EndpointStats> {
        self.stats.lock().unwrap().clone()
    }
    
    /// Get the best endpoint based on the selection strategy
    pub fn get_best_endpoint(&self) -> ConfigResult<RpcEndpoint> {
        let stats = self.stats.lock().unwrap();
        
        // Filter out unhealthy endpoints
        let healthy_endpoints: Vec<_> = self.config.rpc_endpoints.iter()
            .filter(|endpoint| {
                if let Some(endpoint_stats) = stats.get(&endpoint.url) {
                    endpoint_stats.is_usable()
                } else {
                    true // Assume healthy if no stats
                }
            })
            .collect();
        
        if healthy_endpoints.is_empty() {
            return Err(ConfigError::NetworkError(
                "No healthy endpoints available".to_string(),
            ));
        }
        
        // Select endpoint based on strategy
        match self.config.selection_strategy {
            EndpointSelectionStrategy::RoundRobin => {
                // Simple round-robin: just pick the first healthy endpoint
                // In a real implementation, we would maintain a counter
                Ok(healthy_endpoints[0].clone())
            }
            EndpointSelectionStrategy::Random => {
                // Random selection: pick a random healthy endpoint
                use std::time::{SystemTime, UNIX_EPOCH};
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                let index = (now % healthy_endpoints.len() as u64) as usize;
                Ok(healthy_endpoints[index].clone())
            }
            EndpointSelectionStrategy::WeightedRandom => {
                // Weighted random selection
                let total_weight: u32 = healthy_endpoints.iter().map(|e| e.weight).sum();
                if total_weight == 0 {
                    return Ok(healthy_endpoints[0].clone());
                }
                
                use std::time::{SystemTime, UNIX_EPOCH};
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                let mut random = (now % total_weight as u64) as u32;
                
                for endpoint in healthy_endpoints {
                    if random < endpoint.weight {
                        return Ok(endpoint.clone());
                    }
                    random -= endpoint.weight;
                }
                
                // Fallback
                Ok(healthy_endpoints[0].clone())
            }
            EndpointSelectionStrategy::LowestLatency => {
                // Find endpoint with lowest latency
                let mut best_endpoint = healthy_endpoints[0];
                let mut best_latency = f64::MAX;
                
                for endpoint in healthy_endpoints {
                    if let Some(endpoint_stats) = stats.get(&endpoint.url) {
                        if endpoint_stats.avg_latency_ms < best_latency {
                            best_latency = endpoint_stats.avg_latency_ms;
                            best_endpoint = endpoint;
                        }
                    }
                }
                
                Ok(best_endpoint.clone())
            }
            EndpointSelectionStrategy::GeographicProximity => {
                // Use geographic proximity if validator proximity map is available
                let validator_proximity = self.validator_proximity.lock().unwrap();
                
                if !validator_proximity.is_empty() {
                    // Group endpoints by region
                    let mut regions = HashMap::new();
                    for endpoint in &healthy_endpoints {
                        regions.entry(endpoint.region.clone())
                            .or_insert_with(Vec::new)
                            .push(endpoint);
                    }
                    
                    // Find region with best proximity
                    let mut best_region = "";
                    let mut best_proximity = f64::MAX;
                    
                    for (region, _) in &regions {
                        if let Some(&proximity) = validator_proximity.get(region) {
                            if proximity < best_proximity {
                                best_proximity = proximity;
                                best_region = region;
                            }
                        }
                    }
                    
                    if !best_region.is_empty() {
                        if let Some(endpoints) = regions.get(best_region) {
                            // Find best endpoint in the region
                            let mut best_endpoint = endpoints[0];
                            let mut best_score = 0.0;
                            
                            for endpoint in endpoints {
                                if let Some(endpoint_stats) = stats.get(&endpoint.url) {
                                    let score = endpoint_stats.quality_score();
                                    if score > best_score {
                                        best_score = score;
                                        best_endpoint = endpoint;
                                    }
                                }
                            }
                            
                            return Ok(best_endpoint.clone());
                        }
                    }
                }
                
                // Fallback to lowest latency if no proximity data
                self.get_best_endpoint_by_latency()
            }
        }
    }
    
    /// Get the best endpoint by latency
    fn get_best_endpoint_by_latency(&self) -> ConfigResult<RpcEndpoint> {
        let stats = self.stats.lock().unwrap();
        
        // Filter out unhealthy endpoints
        let healthy_endpoints: Vec<_> = self.config.rpc_endpoints.iter()
            .filter(|endpoint| {
                if let Some(endpoint_stats) = stats.get(&endpoint.url) {
                    endpoint_stats.is_usable()
                } else {
                    true // Assume healthy if no stats
                }
            })
            .collect();
        
        if healthy_endpoints.is_empty() {
            return Err(ConfigError::NetworkError(
                "No healthy endpoints available".to_string(),
            ));
        }
        
        // Find endpoint with lowest latency
        let mut best_endpoint = healthy_endpoints[0];
        let mut best_latency = f64::MAX;
        
        for endpoint in healthy_endpoints {
            if let Some(endpoint_stats) = stats.get(&endpoint.url) {
                if endpoint_stats.avg_latency_ms < best_latency {
                    best_latency = endpoint_stats.avg_latency_ms;
                    best_endpoint = endpoint;
                }
            }
        }
        
        Ok(best_endpoint.clone())
    }
    
    /// Update endpoint statistics
    pub fn update_endpoint_stats(&self, url: &str, latency_ms: f64, success: bool) {
        let mut stats = self.stats.lock().unwrap();
        
        if let Some(endpoint_stats) = stats.get_mut(url) {
            endpoint_stats.update_latency(latency_ms, success);
        }
    }
    
    /// Update validator proximity map
    pub fn update_validator_proximity(&self, region: &str, proximity: f64) {
        let mut validator_proximity = self.validator_proximity.lock().unwrap();
        validator_proximity.insert(region.to_string(), proximity);
    }
    
    /// Get the primary endpoint
    pub fn get_primary_endpoint(&self) -> ConfigResult<RpcEndpoint> {
        let primary_endpoint = self.primary_endpoint.lock().unwrap();
        
        if let Some(url) = &*primary_endpoint {
            // Find the endpoint with this URL
            for endpoint in &self.config.rpc_endpoints {
                if &endpoint.url == url {
                    return Ok(endpoint.clone());
                }
            }
        }
        
        // If no primary endpoint or not found, get the best endpoint
        drop(primary_endpoint);
        self.get_best_endpoint()
    }
    
    /// Set the primary endpoint
    pub fn set_primary_endpoint(&self, url: &str) -> ConfigResult<()> {
        // Check if the endpoint exists
        let endpoint_exists = self.config.rpc_endpoints.iter()
            .any(|endpoint| endpoint.url == url);
        
        if !endpoint_exists {
            return Err(ConfigError::NetworkError(
                format!("Endpoint {} not found", url),
            ));
        }
        
        let mut primary_endpoint = self.primary_endpoint.lock().unwrap();
        *primary_endpoint = Some(url.to_string());
        
        Ok(())
    }
    
    /// Reset the primary endpoint
    pub fn reset_primary_endpoint(&self) {
        let mut primary_endpoint = self.primary_endpoint.lock().unwrap();
        *primary_endpoint = None;
    }
    
    /// Check if latency optimization is enabled
    pub fn is_latency_optimization_enabled(&self) -> bool {
        self.latency_optimization.enabled
    }
    
    /// Get connection timeout
    pub fn get_connection_timeout(&self) -> Duration {
        Duration::from_millis(self.config.connection_timeout_ms)
    }
    
    /// Get keepalive interval
    pub fn get_keepalive_interval(&self) -> Duration {
        Duration::from_millis(self.config.keepalive_interval_ms)
    }
    
    /// Get connection pool size
    pub fn get_connection_pool_size(&self) -> usize {
        self.config.connection_pool_size
    }
    
    /// Get maximum connections
    pub fn get_max_connections(&self) -> usize {
        self.config.max_connections
    }
    
    /// Get health check interval
    pub fn get_health_check_interval(&self) -> Duration {
        Duration::from_millis(self.config.health_check_interval_ms)
    }
    
    /// Get base timeout
    pub fn get_base_timeout(&self) -> Duration {
        Duration::from_millis(self.latency_optimization.base_timeout_ms)
    }
    
    /// Get maximum timeout
    pub fn get_max_timeout(&self) -> Duration {
        Duration::from_millis(self.latency_optimization.max_timeout_ms)
    }
    
    /// Calculate adaptive timeout based on endpoint statistics
    pub fn calculate_adaptive_timeout(&self, url: &str) -> Duration {
        if !self.latency_optimization.adaptive_timeouts {
            return self.get_base_timeout();
        }
        
        let stats = self.stats.lock().unwrap();
        
        if let Some(endpoint_stats) = stats.get(url) {
            // Calculate timeout based on average latency and jitter
            let jitter = endpoint_stats.max_latency_ms - endpoint_stats.min_latency_ms;
            let timeout_ms = endpoint_stats.avg_latency_ms * 2.0 + jitter;
            
            // Clamp to base and max timeout
            let base_ms = self.latency_optimization.base_timeout_ms as f64;
            let max_ms = self.latency_optimization.max_timeout_ms as f64;
            let clamped_ms = timeout_ms.max(base_ms).min(max_ms);
            
            Duration::from_millis(clamped_ms as u64)
        } else {
            self.get_base_timeout()
        }
    }
    
    /// Build validator proximity map
    pub fn build_validator_proximity_map(&self) -> ConfigResult<()> {
        if !self.latency_optimization.build_validator_proximity_map {
            return Ok(());
        }
        
        // In a real implementation, this would:
        // 1. Get a list of validators
        // 2. Measure latency to each validator
        // 3. Group validators by region
        // 4. Calculate average latency for each region
        // 5. Update the validator proximity map
        
        // For now, just create a dummy map
        let mut validator_proximity = self.validator_proximity.lock().unwrap();
        validator_proximity.insert("us-east".to_string(), 50.0);
        validator_proximity.insert("us-west".to_string(), 100.0);
        validator_proximity.insert("eu-central".to_string(), 150.0);
        validator_proximity.insert("ap-southeast".to_string(), 200.0);
        
        Ok(())
    }
    
    /// Optimize routes
    pub fn optimize_routes(&self) -> ConfigResult<()> {
        if !self.latency_optimization.optimize_routes {
            return Ok(());
        }
        
        // In a real implementation, this would:
        // 1. Trace network paths to important endpoints
        // 2. Detect suboptimal routing
        // 3. Recommend network configuration changes
        
        // For now, just log that route optimization is enabled
        info!("Route optimization is enabled");
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_optimizer_kernel_bypass() {
        let config = NetworkConfig::default();
        let mut feature_flags = HardwareFeatureFlags::default();
        
        // Test with kernel bypass disabled
        let mut optimizer = NetworkOptimizer::new(config.clone(), feature_flags.clone());
        optimizer.optimize_kernel_bypass();
        assert_eq!(optimizer.config.kernel_bypass_mode, KernelBypassMode::None);
        
        // Test with kernel bypass enabled but no support
        feature_flags.kernel_bypass_networking = true;
        let mut optimizer = NetworkOptimizer::new(config.clone(), feature_flags.clone());
        optimizer.optimize_kernel_bypass();
        assert_eq!(optimizer.config.kernel_bypass_mode, KernelBypassMode::None);
        
        // Test with DPDK support
        feature_flags.dpdk_support = true;
        let mut optimizer = NetworkOptimizer::new(config.clone(), feature_flags.clone());
        optimizer.optimize_kernel_bypass();
        assert_eq!(optimizer.config.kernel_bypass_mode, KernelBypassMode::Dpdk);
        
        // Test with io_uring support
        feature_flags.dpdk_support = false;
        feature_flags.io_uring_support = true;
        let mut optimizer = NetworkOptimizer::new(config.clone(), feature_flags.clone());
        optimizer.optimize_kernel_bypass();
        assert_eq!(optimizer.config.kernel_bypass_mode, KernelBypassMode::IoUring);
        
        // Test with both DPDK and io_uring support
        feature_flags.dpdk_support = true;
        let mut optimizer = NetworkOptimizer::new(config.clone(), feature_flags.clone());
        optimizer.optimize_kernel_bypass();
        assert_eq!(optimizer.config.kernel_bypass_mode, KernelBypassMode::DpdkAndIoUring);
    }
    
    #[test]
    fn test_endpoint_stats() {
        let mut stats = EndpointStats::new("https://example.com".to_string(), "us-east".to_string());
        
        // Test initial state
        assert_eq!(stats.url, "https://example.com");
        assert_eq!(stats.region, "us-east");
        assert_eq!(stats.avg_latency_ms, 0.0);
        assert_eq!(stats.min_latency_ms, f64::MAX);
        assert_eq!(stats.max_latency_ms, 0.0);
        assert_eq!(stats.success_rate, 1.0);
        assert_eq!(stats.request_count, 0);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.health, EndpointHealth::Healthy);
        
        // Test successful request
        stats.update_latency(100.0, true);
        assert_eq!(stats.avg_latency_ms, 100.0);
        assert_eq!(stats.min_latency_ms, 100.0);
        assert_eq!(stats.max_latency_ms, 100.0);
        assert_eq!(stats.success_rate, 1.0);
        assert_eq!(stats.request_count, 1);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.health, EndpointHealth::Healthy);
        
        // Test failed request
        stats.update_latency(0.0, false);
        assert_eq!(stats.avg_latency_ms, 100.0); // Latency unchanged for failed requests
        assert_eq!(stats.min_latency_ms, 100.0);
        assert_eq!(stats.max_latency_ms, 100.0);
        assert_eq!(stats.success_rate, 0.5);
        assert_eq!(stats.request_count, 2);
        assert_eq!(stats.error_count, 1);
        assert_eq!(stats.health, EndpointHealth::Degraded);
        
        // Test multiple failed requests
        stats.update_latency(0.0, false);
        stats.update_latency(0.0, false);
        assert_eq!(stats.success_rate, 0.25);
        assert_eq!(stats.request_count, 4);
        assert_eq!(stats.error_count, 3);
        assert_eq!(stats.health, EndpointHealth::Unhealthy);
        
        // Test quality score
        let score = stats.quality_score();
        assert!(score >= 0.0 && score <= 1.0);
    }
}