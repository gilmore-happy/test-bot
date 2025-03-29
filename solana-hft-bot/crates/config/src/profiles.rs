//! Hardware profiles for different environments
//!
//! This module provides predefined hardware profiles for different
//! environments and hardware configurations.

use crate::error::ConfigResult;
use crate::hardware::HardwareCapabilities;
use crate::schema::{
    BotConfig, CpuConfig, EnvironmentType, FeatureFlags, HardwareConfig, MemoryAllocationStrategy,
    MemoryConfig, NetworkConfig, KernelBypassMode, TcpOptimizations, SocketBufferConfig,
    ZeroCopyConfig, EndpointConfig, LatencyOptimizationConfig,
};
use std::collections::HashMap;
use tracing::{debug, info};

/// Hardware profile
#[derive(Debug, Clone)]
pub struct HardwareProfile {
    /// Profile name
    pub name: String,
    
    /// Profile description
    pub description: String,
    
    /// Minimum requirements
    pub min_requirements: ProfileRequirements,
    
    /// Recommended requirements
    pub recommended_requirements: ProfileRequirements,
    
    /// Configuration generator function
    pub config_generator: fn(&HardwareCapabilities) -> ConfigResult<BotConfig>,
}

/// Profile requirements
#[derive(Debug, Clone)]
pub struct ProfileRequirements {
    /// Minimum CPU cores
    pub min_cpu_cores: usize,
    
    /// Minimum RAM in MB
    pub min_ram_mb: usize,
    
    /// Minimum network speed in Mbps
    pub min_network_speed_mbps: u32,
    
    /// Required features
    pub required_features: Vec<String>,
    
    /// Recommended features
    pub recommended_features: Vec<String>,
}

impl HardwareProfile {
    /// Check if the hardware meets the minimum requirements
    pub fn meets_minimum_requirements(&self, capabilities: &HardwareCapabilities) -> bool {
        // Check CPU cores
        if capabilities.cpu.physical_cores < self.min_requirements.min_cpu_cores {
            return false;
        }
        
        // Check RAM
        if capabilities.memory.total_mb < self.min_requirements.min_ram_mb {
            return false;
        }
        
        // Check network speed
        let has_required_network_speed = capabilities.network_interfaces.iter()
            .any(|iface| iface.speed_mbps.unwrap_or(0) >= self.min_requirements.min_network_speed_mbps);
        
        if !has_required_network_speed {
            return false;
        }
        
        // Check required features
        for feature in &self.min_requirements.required_features {
            if !capabilities.feature_flags.is_feature_enabled(feature) {
                return false;
            }
        }
        
        true
    }
    
    /// Check if the hardware meets the recommended requirements
    pub fn meets_recommended_requirements(&self, capabilities: &HardwareCapabilities) -> bool {
        // First check minimum requirements
        if !self.meets_minimum_requirements(capabilities) {
            return false;
        }
        
        // Check CPU cores
        if capabilities.cpu.physical_cores < self.recommended_requirements.min_cpu_cores {
            return false;
        }
        
        // Check RAM
        if capabilities.memory.total_mb < self.recommended_requirements.min_ram_mb {
            return false;
        }
        
        // Check network speed
        let has_recommended_network_speed = capabilities.network_interfaces.iter()
            .any(|iface| iface.speed_mbps.unwrap_or(0) >= self.recommended_requirements.min_network_speed_mbps);
        
        if !has_recommended_network_speed {
            return false;
        }
        
        // Check recommended features
        for feature in &self.recommended_requirements.recommended_features {
            if !capabilities.feature_flags.is_feature_enabled(feature) {
                return false;
            }
        }
        
        true
    }
    
    /// Generate configuration based on hardware capabilities
    pub fn generate_config(&self, capabilities: &HardwareCapabilities) -> ConfigResult<BotConfig> {
        (self.config_generator)(capabilities)
    }
}

/// Profile manager
#[derive(Debug)]
pub struct ProfileManager {
    /// Available profiles
    profiles: HashMap<String, HardwareProfile>,
}

impl ProfileManager {
    /// Create a new profile manager with default profiles
    pub fn new() -> Self {
        let mut profiles = HashMap::new();
        
        // Add default profiles
        profiles.insert("high_end_bare_metal".to_string(), high_end_bare_metal_profile());
        profiles.insert("mid_range_server".to_string(), mid_range_server_profile());
        profiles.insert("cloud_optimized".to_string(), cloud_optimized_profile());
        profiles.insert("standard_vps".to_string(), standard_vps_profile());
        profiles.insert("minimal".to_string(), minimal_profile());
        
        Self { profiles }
    }
    
    /// Get a profile by name
    pub fn get_profile(&self, name: &str) -> Option<&HardwareProfile> {
        self.profiles.get(name)
    }
    
    /// Add a custom profile
    pub fn add_profile(&mut self, name: String, profile: HardwareProfile) {
        self.profiles.insert(name, profile);
    }
    
    /// Get all profile names
    pub fn profile_names(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }
    
    /// Select the best profile for the given hardware capabilities
    pub fn select_best_profile(&self, capabilities: &HardwareCapabilities) -> &HardwareProfile {
        // Try profiles in order of preference
        let profile_order = [
            "high_end_bare_metal",
            "mid_range_server",
            "cloud_optimized",
            "standard_vps",
            "minimal",
        ];
        
        for profile_name in profile_order.iter() {
            if let Some(profile) = self.profiles.get(*profile_name) {
                if profile.meets_minimum_requirements(capabilities) {
                    info!("Selected profile: {}", profile_name);
                    return profile;
                }
            }
        }
        
        // If no profile matches, return minimal profile
        info!("No matching profile found, using minimal profile");
        self.profiles.get("minimal").unwrap()
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

/// High-end bare metal profile
fn high_end_bare_metal_profile() -> HardwareProfile {
    HardwareProfile {
        name: "High-End Bare Metal".to_string(),
        description: "Optimized for high-end dedicated servers with 64+ CPU cores, 128GB+ RAM, and high-end NICs".to_string(),
        min_requirements: ProfileRequirements {
            min_cpu_cores: 32,
            min_ram_mb: 64 * 1024, // 64GB
            min_network_speed_mbps: 10000, // 10Gbps
            required_features: vec![
                "high_frequency_networking".to_string(),
            ],
            recommended_features: vec![
                "kernel_bypass_networking".to_string(),
                "hardware_timestamping".to_string(),
                "cpu_pinning".to_string(),
                "huge_pages".to_string(),
            ],
        },
        recommended_requirements: ProfileRequirements {
            min_cpu_cores: 64,
            min_ram_mb: 128 * 1024, // 128GB
            min_network_speed_mbps: 25000, // 25Gbps
            required_features: vec![
                "high_frequency_networking".to_string(),
                "kernel_bypass_networking".to_string(),
                "hardware_timestamping".to_string(),
            ],
            recommended_features: vec![
                "cpu_pinning".to_string(),
                "simd_optimizations".to_string(),
                "avx2_support".to_string(),
                "huge_pages".to_string(),
                "huge_pages_1gb".to_string(),
                "numa_awareness".to_string(),
                "direct_memory_access".to_string(),
                "isolated_cpus".to_string(),
            ],
        },
        config_generator: generate_high_end_bare_metal_config,
    }
}

/// Mid-range server profile
fn mid_range_server_profile() -> HardwareProfile {
    HardwareProfile {
        name: "Mid-Range Server".to_string(),
        description: "Optimized for mid-range servers with 16-32 CPU cores and 64GB RAM".to_string(),
        min_requirements: ProfileRequirements {
            min_cpu_cores: 8,
            min_ram_mb: 32 * 1024, // 32GB
            min_network_speed_mbps: 1000, // 1Gbps
            required_features: vec![],
            recommended_features: vec![
                "cpu_pinning".to_string(),
                "simd_optimizations".to_string(),
            ],
        },
        recommended_requirements: ProfileRequirements {
            min_cpu_cores: 16,
            min_ram_mb: 64 * 1024, // 64GB
            min_network_speed_mbps: 10000, // 10Gbps
            required_features: vec![],
            recommended_features: vec![
                "cpu_pinning".to_string(),
                "simd_optimizations".to_string(),
                "avx_support".to_string(),
                "huge_pages".to_string(),
                "high_frequency_networking".to_string(),
            ],
        },
        config_generator: generate_mid_range_server_config,
    }
}

/// Cloud-optimized profile
fn cloud_optimized_profile() -> HardwareProfile {
    HardwareProfile {
        name: "Cloud Optimized".to_string(),
        description: "Optimized for cloud environments like AWS, GCP, and Azure".to_string(),
        min_requirements: ProfileRequirements {
            min_cpu_cores: 4,
            min_ram_mb: 16 * 1024, // 16GB
            min_network_speed_mbps: 1000, // 1Gbps
            required_features: vec![],
            recommended_features: vec![],
        },
        recommended_requirements: ProfileRequirements {
            min_cpu_cores: 8,
            min_ram_mb: 32 * 1024, // 32GB
            min_network_speed_mbps: 10000, // 10Gbps
            required_features: vec![],
            recommended_features: vec![
                "simd_optimizations".to_string(),
            ],
        },
        config_generator: generate_cloud_optimized_config,
    }
}

/// Standard VPS profile
fn standard_vps_profile() -> HardwareProfile {
    HardwareProfile {
        name: "Standard VPS".to_string(),
        description: "Optimized for standard VPS environments with limited resources".to_string(),
        min_requirements: ProfileRequirements {
            min_cpu_cores: 2,
            min_ram_mb: 4 * 1024, // 4GB
            min_network_speed_mbps: 100, // 100Mbps
            required_features: vec![],
            recommended_features: vec![],
        },
        recommended_requirements: ProfileRequirements {
            min_cpu_cores: 4,
            min_ram_mb: 8 * 1024, // 8GB
            min_network_speed_mbps: 1000, // 1Gbps
            required_features: vec![],
            recommended_features: vec![],
        },
        config_generator: generate_standard_vps_config,
    }
}

/// Minimal profile
fn minimal_profile() -> HardwareProfile {
    HardwareProfile {
        name: "Minimal".to_string(),
        description: "Minimal configuration for testing and development".to_string(),
        min_requirements: ProfileRequirements {
            min_cpu_cores: 1,
            min_ram_mb: 2 * 1024, // 2GB
            min_network_speed_mbps: 10, // 10Mbps
            required_features: vec![],
            recommended_features: vec![],
        },
        recommended_requirements: ProfileRequirements {
            min_cpu_cores: 2,
            min_ram_mb: 4 * 1024, // 4GB
            min_network_speed_mbps: 100, // 100Mbps
            required_features: vec![],
            recommended_features: vec![],
        },
        config_generator: generate_minimal_config,
    }
}

/// Generate high-end bare metal configuration
fn generate_high_end_bare_metal_config(capabilities: &HardwareCapabilities) -> ConfigResult<BotConfig> {
    let mut config = BotConfig::default();
    
    // Set environment type
    config.environment = EnvironmentType::BareMetal;
    
    // Configure hardware
    config.hardware = HardwareConfig {
        cpu: CpuConfig {
            physical_cores: Some(capabilities.cpu.physical_cores),
            logical_threads: Some(capabilities.cpu.logical_threads),
            frequency_mhz: Some(capabilities.cpu.frequency_mhz),
            use_cpu_pinning: true,
            isolated_cores: Vec::new(), // Would need to be configured manually
            numa_aware: capabilities.feature_flags.numa_awareness,
            numa_node: None, // Would need to be configured manually
        },
        memory: MemoryConfig {
            total_mb: Some(capabilities.memory.total_mb),
            use_huge_pages: capabilities.feature_flags.huge_pages,
            huge_page_size_kb: if capabilities.feature_flags.huge_pages_1gb { 1048576 } else { 2048 },
            huge_pages_count: None, // Would need to be configured manually
            allocation_strategy: MemoryAllocationStrategy::Aggressive,
        },
        storage: Default::default(),
        network_interfaces: capabilities.network_interfaces
            .iter()
            .map(|iface| iface.into())
            .collect(),
        profile: Some("high_end_bare_metal".to_string()),
    };
    
    // Configure network
    config.network = NetworkConfig {
        kernel_bypass_mode: if capabilities.feature_flags.dpdk_support && capabilities.feature_flags.io_uring_support {
            KernelBypassMode::DpdkAndIoUring
        } else if capabilities.feature_flags.dpdk_support {
            KernelBypassMode::Dpdk
        } else if capabilities.feature_flags.io_uring_support {
            KernelBypassMode::IoUring
        } else {
            KernelBypassMode::None
        },
        dpdk: Default::default(),
        io_uring: Default::default(),
        tcp_optimizations: TcpOptimizations {
            tcp_nodelay: true,
            tcp_quickack: true,
            tcp_fastopen: true,
            tcp_cork: false,
            tcp_defer_accept: true,
            keepalive_interval_secs: 5,
            connection_backlog: 1024,
        },
        socket_buffers: SocketBufferConfig {
            send_buffer_size: 8 * 1024 * 1024, // 8MB
            recv_buffer_size: 8 * 1024 * 1024, // 8MB
            dynamic_buffer_sizing: true,
            max_buffer_size: 32 * 1024 * 1024, // 32MB
        },
        zero_copy: ZeroCopyConfig {
            enabled: true,
            memory_map_size: 256 * 1024 * 1024, // 256MB
            hardware_offloading: true,
            direct_memory_access: capabilities.feature_flags.direct_memory_access,
        },
        endpoints: EndpointConfig::default(),
        latency_optimization: LatencyOptimizationConfig {
            enabled: true,
            build_validator_proximity_map: true,
            optimize_routes: true,
            connection_keepalive_management: true,
            request_coalescing: true,
            adaptive_timeouts: true,
            base_timeout_ms: 1000,
            max_timeout_ms: 30000,
        },
    };
    
    // Configure feature flags
    config.feature_flags = capabilities.to_feature_flags();
    
    Ok(config)
}

/// Generate mid-range server configuration
fn generate_mid_range_server_config(capabilities: &HardwareCapabilities) -> ConfigResult<BotConfig> {
    let mut config = BotConfig::default();
    
    // Set environment type
    config.environment = EnvironmentType::BareMetal;
    
    // Configure hardware
    config.hardware = HardwareConfig {
        cpu: CpuConfig {
            physical_cores: Some(capabilities.cpu.physical_cores),
            logical_threads: Some(capabilities.cpu.logical_threads),
            frequency_mhz: Some(capabilities.cpu.frequency_mhz),
            use_cpu_pinning: capabilities.feature_flags.cpu_pinning,
            isolated_cores: Vec::new(),
            numa_aware: capabilities.feature_flags.numa_awareness,
            numa_node: None,
        },
        memory: MemoryConfig {
            total_mb: Some(capabilities.memory.total_mb),
            use_huge_pages: capabilities.feature_flags.huge_pages,
            huge_page_size_kb: if capabilities.feature_flags.huge_pages_1gb { 1048576 } else { 2048 },
            huge_pages_count: None,
            allocation_strategy: MemoryAllocationStrategy::Balanced,
        },
        storage: Default::default(),
        network_interfaces: capabilities.network_interfaces
            .iter()
            .map(|iface| iface.into())
            .collect(),
        profile: Some("mid_range_server".to_string()),
    };
    
    // Configure network
    config.network = NetworkConfig {
        kernel_bypass_mode: if capabilities.feature_flags.io_uring_support {
            KernelBypassMode::IoUring
        } else {
            KernelBypassMode::None
        },
        dpdk: Default::default(),
        io_uring: Default::default(),
        tcp_optimizations: TcpOptimizations {
            tcp_nodelay: true,
            tcp_quickack: true,
            tcp_fastopen: true,
            tcp_cork: false,
            tcp_defer_accept: true,
            keepalive_interval_secs: 10,
            connection_backlog: 512,
        },
        socket_buffers: SocketBufferConfig {
            send_buffer_size: 4 * 1024 * 1024, // 4MB
            recv_buffer_size: 4 * 1024 * 1024, // 4MB
            dynamic_buffer_sizing: true,
            max_buffer_size: 16 * 1024 * 1024, // 16MB
        },
        zero_copy: ZeroCopyConfig {
            enabled: capabilities.feature_flags.io_uring_support,
            memory_map_size: 128 * 1024 * 1024, // 128MB
            hardware_offloading: false,
            direct_memory_access: capabilities.feature_flags.direct_memory_access,
        },
        endpoints: EndpointConfig::default(),
        latency_optimization: LatencyOptimizationConfig {
            enabled: true,
            build_validator_proximity_map: false,
            optimize_routes: true,
            connection_keepalive_management: true,
            request_coalescing: true,
            adaptive_timeouts: true,
            base_timeout_ms: 2000,
            max_timeout_ms: 30000,
        },
    };
    
    // Configure feature flags
    config.feature_flags = capabilities.to_feature_flags();
    
    Ok(config)
}

/// Generate cloud-optimized configuration
fn generate_cloud_optimized_config(capabilities: &HardwareCapabilities) -> ConfigResult<BotConfig> {
    let mut config = BotConfig::default();
    
    // Set environment type
    config.environment = EnvironmentType::Cloud;
    
    // Configure hardware
    config.hardware = HardwareConfig {
        cpu: CpuConfig {
            physical_cores: Some(capabilities.cpu.physical_cores),
            logical_threads: Some(capabilities.cpu.logical_threads),
            frequency_mhz: Some(capabilities.cpu.frequency_mhz),
            use_cpu_pinning: false, // Usually not available in cloud environments
            isolated_cores: Vec::new(),
            numa_aware: false, // Usually not available in cloud environments
            numa_node: None,
        },
        memory: MemoryConfig {
            total_mb: Some(capabilities.memory.total_mb),
            use_huge_pages: false, // Usually not available in cloud environments
            huge_page_size_kb: 2048,
            huge_pages_count: None,
            allocation_strategy: MemoryAllocationStrategy::Balanced,
        },
        storage: Default::default(),
        network_interfaces: capabilities.network_interfaces
            .iter()
            .map(|iface| iface.into())
            .collect(),
        profile: Some("cloud_optimized".to_string()),
    };
    
    // Configure network
    config.network = NetworkConfig {
        kernel_bypass_mode: KernelBypassMode::None, // Usually not available in cloud environments
        dpdk: Default::default(),
        io_uring: Default::default(),
        tcp_optimizations: TcpOptimizations {
            tcp_nodelay: true,
            tcp_quickack: true,
            tcp_fastopen: true,
            tcp_cork: false,
            tcp_defer_accept: false,
            keepalive_interval_secs: 15,
            connection_backlog: 256,
        },
        socket_buffers: SocketBufferConfig {
            send_buffer_size: 2 * 1024 * 1024, // 2MB
            recv_buffer_size: 2 * 1024 * 1024, // 2MB
            dynamic_buffer_sizing: false,
            max_buffer_size: 8 * 1024 * 1024, // 8MB
        },
        zero_copy: ZeroCopyConfig {
            enabled: false,
            memory_map_size: 64 * 1024 * 1024, // 64MB
            hardware_offloading: false,
            direct_memory_access: false,
        },
        endpoints: EndpointConfig::default(),
        latency_optimization: LatencyOptimizationConfig {
            enabled: true,
            build_validator_proximity_map: false,
            optimize_routes: true,
            connection_keepalive_management: true,
            request_coalescing: false,
            adaptive_timeouts: true,
            base_timeout_ms: 5000,
            max_timeout_ms: 60000,
        },
    };
    
    // Configure feature flags
    config.feature_flags = FeatureFlags::default();
    
    Ok(config)
}

/// Generate standard VPS configuration
fn generate_standard_vps_config(capabilities: &HardwareCapabilities) -> ConfigResult<BotConfig> {
    let mut config = BotConfig::default();
    
    // Set environment type
    config.environment = EnvironmentType::Vps;
    
    // Configure hardware
    config.hardware = HardwareConfig {
        cpu: CpuConfig {
            physical_cores: Some(capabilities.cpu.physical_cores),
            logical_threads: Some(capabilities.cpu.logical_threads),
            frequency_mhz: Some(capabilities.cpu.frequency_mhz),
            use_cpu_pinning: false,
            isolated_cores: Vec::new(),
            numa_aware: false,
            numa_node: None,
        },
        memory: MemoryConfig {
            total_mb: Some(capabilities.memory.total_mb),
            use_huge_pages: false,
            huge_page_size_kb: 2048,
            huge_pages_count: None,
            allocation_strategy: MemoryAllocationStrategy::Conservative,
        },
        storage: Default::default(),
        network_interfaces: capabilities.network_interfaces
            .iter()
            .map(|iface| iface.into())
            .collect(),
        profile: Some("standard_vps".to_string()),
    };
    
    // Configure network
    config.network = NetworkConfig {
        kernel_bypass_mode: KernelBypassMode::None,
        dpdk: Default::default(),
        io_uring: Default::default(),
        tcp_optimizations: TcpOptimizations {
            tcp_nodelay: true,
            tcp_quickack: true,
            tcp_fastopen: false,
            tcp_cork: false,
            tcp_defer_accept: false,
            keepalive_interval_secs: 30,
            connection_backlog: 128,
        },
        socket_buffers: SocketBufferConfig {
            send_buffer_size: 1 * 1024 * 1024, // 1MB
            recv_buffer_size: 1 * 1024 * 1024, // 1MB
            dynamic_buffer_sizing: false,
            max_buffer_size: 4 * 1024 * 1024, // 4MB
        },
        zero_copy: ZeroCopyConfig {
            enabled: false,
            memory_map_size: 32 * 1024 * 1024, // 32MB
            hardware_offloading: false,
            direct_memory_access: false,
        },
        endpoints: EndpointConfig::default(),
        latency_optimization: LatencyOptimizationConfig {
            enabled: false,
            build_validator_proximity_map: false,
            optimize_routes: false,
            connection_keepalive_management: true,
            request_coalescing: false,
            adaptive_timeouts: false,
            base_timeout_ms: 10000,
            max_timeout_ms: 60000,
        },
    };
    
    // Configure feature flags
    config.feature_flags = FeatureFlags::default();
    
    Ok(config)
}

/// Generate minimal configuration
fn generate_minimal_config(capabilities: &HardwareCapabilities) -> ConfigResult<BotConfig> {
    let mut config = BotConfig::default();
    
    // Set environment type
    config.environment = EnvironmentType::Development;
    
    // Configure hardware
    config.hardware = HardwareConfig {
        cpu: CpuConfig {
            physical_cores: Some(capabilities.cpu.physical_cores),
            logical_threads: Some(capabilities.cpu.logical_threads),
            frequency_mhz: Some(capabilities.cpu.frequency_mhz),
            use_cpu_pinning: false,
            isolated_cores: Vec::new(),
            numa_aware: false,
            numa_node: None,
        },
        memory: MemoryConfig {
            total_mb: Some(capabilities.memory.total_mb),
            use_huge_pages: false,
            huge_page_size_kb: 2048,
            huge_pages_count: None,
            allocation_strategy: MemoryAllocationStrategy::Conservative,
        },
        storage: Default::default(),
        network_interfaces: capabilities.network_interfaces
            .iter()
            .map(|iface| iface.into())
            .collect(),
        profile: Some("minimal".to_string()),
    };
    
    // Configure network
    config.network = NetworkConfig {
        kernel_bypass_mode: KernelBypassMode::None,
        dpdk: Default::default(),
        io_uring: Default::default(),
        tcp_optimizations: TcpOptimizations {
            tcp_nodelay: true,
            tcp_quickack: false,
            tcp_fastopen: false,
            tcp_cork: false,
            tcp_defer_accept: false,
            keepalive_interval_secs: 60,
            connection_backlog: 64,
        },
        socket_buffers: SocketBufferConfig {
            send_buffer_size: 512 * 1024, // 512KB
            recv_buffer_size: 512 * 1024, // 512KB
            dynamic_buffer_sizing: false,
            max_buffer_size: 2 * 1024 * 1024, // 2MB
        },
        zero_copy: ZeroCopyConfig {
            enabled: false,
            memory_map_size: 16 * 1024 * 1024, // 16MB
            hardware_offloading: false,
            direct_memory_access: false,
        },
        endpoints: EndpointConfig::default(),
        latency_optimization: LatencyOptimizationConfig {
            enabled: false,
            build_validator_proximity_map: false,
            optimize_routes: false,
            connection_keepalive_management: false,
            request_coalescing: false,
            adaptive_timeouts: false,
            base_timeout_ms: 30000,
            max_timeout_ms: 120000,
        },
    };
    
    // Configure feature flags
    config.feature_flags = FeatureFlags::default();
    
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_profile_manager_default() {
        let manager = ProfileManager::default();
        
        // Check that all default profiles are available
        assert!(manager.get_profile("high_end_bare_metal").is_some());
        assert!(manager.get_profile("mid_range_server").is_some());
        assert!(manager.get_profile("cloud_optimized").is_some());
        assert!(manager.get_profile("standard_vps").is_some());
        assert!(manager.get_profile("minimal").is_some());
        
        // Check profile names
        let names = manager.profile_names();
        assert!(names.contains(&"high_end_bare_metal".to_string()));
        assert!(names.contains(&"mid_range_server".to_string()));
        assert!(names.contains(&"cloud_optimized".to_string()));
        assert!(names.contains(&"standard_vps".to_string()));
        assert!(names.contains(&"minimal".to_string()));
    }
}