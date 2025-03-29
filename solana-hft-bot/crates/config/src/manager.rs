//! Configuration management
//!
//! This module provides a central configuration manager that
//! combines all configuration components.

use crate::error::{ConfigError, ConfigResult};
use crate::hardware::{HardwareCapabilities, HardwareFeatureFlags};
use crate::loader::ConfigLoader;
use crate::network::{EndpointManager, NetworkOptimizer};
use crate::profiles::{HardwareProfile, ProfileManager};
use crate::schema::BotConfig;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

/// Configuration manager
#[derive(Debug)]
pub struct ConfigurationManager {
    /// Current configuration
    config: Arc<RwLock<BotConfig>>,
    
    /// Configuration loader
    loader: ConfigLoader,
    
    /// Profile manager
    profile_manager: ProfileManager,
    
    /// Hardware capabilities
    hardware_capabilities: Option<HardwareCapabilities>,
    
    /// Network optimizer
    network_optimizer: Option<NetworkOptimizer>,
    
    /// Endpoint manager
    endpoint_manager: Option<EndpointManager>,
    
    /// Configuration file path
    config_path: Option<PathBuf>,
}

impl ConfigurationManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(BotConfig::default())),
            loader: ConfigLoader::default(),
            profile_manager: ProfileManager::default(),
            hardware_capabilities: None,
            network_optimizer: None,
            endpoint_manager: None,
            config_path: None,
        }
    }
    
    /// Initialize the configuration manager
    pub fn initialize(&mut self, cli_config_path: Option<&Path>) -> ConfigResult<()> {
        info!("Initializing configuration manager...");
        
        // Set CLI config path
        self.loader = self.loader.with_cli_config_path(cli_config_path);
        self.config_path = cli_config_path.map(|p| p.to_path_buf());
        
        // Load configuration
        let config = self.loader.load()?;
        
        // Detect hardware capabilities
        self.detect_hardware_capabilities()?;
        
        // Select hardware profile
        self.select_hardware_profile(&config)?;
        
        // Optimize network configuration
        self.optimize_network_configuration()?;
        
        // Initialize endpoint manager
        self.initialize_endpoint_manager()?;
        
        // Update configuration
        *self.config.write().unwrap() = config;
        
        info!("Configuration manager initialized");
        
        Ok(())
    }
    
    /// Detect hardware capabilities
    fn detect_hardware_capabilities(&mut self) -> ConfigResult<()> {
        info!("Detecting hardware capabilities...");
        
        let capabilities = HardwareCapabilities::detect()?;
        
        // Store capabilities
        self.hardware_capabilities = Some(capabilities);
        
        Ok(())
    }
    
    /// Select hardware profile
    fn select_hardware_profile(&mut self, config: &BotConfig) -> ConfigResult<()> {
        info!("Selecting hardware profile...");
        
        let capabilities = self.hardware_capabilities.as_ref().ok_or_else(|| {
            ConfigError::General("Hardware capabilities not detected".to_string())
        })?;
        
        // Check if a profile is specified in the configuration
        let profile = if let Some(profile_name) = &config.hardware.profile {
            if let Some(profile) = self.profile_manager.get_profile(profile_name) {
                info!("Using specified profile: {}", profile_name);
                profile
            } else {
                warn!("Specified profile not found: {}", profile_name);
                self.profile_manager.select_best_profile(capabilities)
            }
        } else {
            // Auto-select profile
            info!("Auto-selecting hardware profile...");
            self.profile_manager.select_best_profile(capabilities)
        };
        
        info!("Selected profile: {}", profile.name);
        
        // Generate configuration from profile
        let profile_config = profile.generate_config(capabilities)?;
        
        // Merge with existing configuration
        let mut config_guard = self.config.write().unwrap();
        
        // Only update hardware and network configuration from profile
        config_guard.hardware = profile_config.hardware;
        config_guard.network = profile_config.network;
        config_guard.feature_flags = profile_config.feature_flags;
        
        Ok(())
    }
    
    /// Optimize network configuration
    fn optimize_network_configuration(&mut self) -> ConfigResult<()> {
        info!("Optimizing network configuration...");
        
        let capabilities = self.hardware_capabilities.as_ref().ok_or_else(|| {
            ConfigError::General("Hardware capabilities not detected".to_string())
        })?;
        
        let config = self.config.read().unwrap();
        
        // Create network optimizer
        let mut optimizer = NetworkOptimizer::new(
            config.network.clone(),
            capabilities.feature_flags.clone(),
        );
        
        // Optimize network configuration
        optimizer.optimize()?;
        
        // Store optimizer
        self.network_optimizer = Some(optimizer);
        
        // Update configuration
        drop(config);
        let mut config_guard = self.config.write().unwrap();
        config_guard.network = optimizer.get_config();
        
        Ok(())
    }
    
    /// Initialize endpoint manager
    fn initialize_endpoint_manager(&mut self) -> ConfigResult<()> {
        info!("Initializing endpoint manager...");
        
        let config = self.config.read().unwrap();
        
        // Create endpoint manager
        let endpoint_manager = EndpointManager::new(
            config.network.endpoints.clone(),
            config.network.latency_optimization.clone(),
        );
        
        // Store endpoint manager
        self.endpoint_manager = Some(endpoint_manager);
        
        Ok(())
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> BotConfig {
        self.config.read().unwrap().clone()
    }
    
    /// Get hardware capabilities
    pub fn get_hardware_capabilities(&self) -> Option<HardwareCapabilities> {
        self.hardware_capabilities.clone()
    }
    
    /// Get feature flags
    pub fn get_feature_flags(&self) -> HardwareFeatureFlags {
        if let Some(capabilities) = &self.hardware_capabilities {
            capabilities.feature_flags.clone()
        } else {
            HardwareFeatureFlags::default()
        }
    }
    
    /// Get endpoint manager
    pub fn get_endpoint_manager(&self) -> Option<&EndpointManager> {
        self.endpoint_manager.as_ref()
    }
    
    /// Update configuration
    pub fn update_config(&self, config: BotConfig) -> ConfigResult<()> {
        // Validate configuration
        self.validate_config(&config)?;
        
        // Update configuration
        *self.config.write().unwrap() = config;
        
        Ok(())
    }
    
    /// Save configuration to file
    pub fn save_config(&self) -> ConfigResult<()> {
        let config = self.config.read().unwrap();
        
        // Determine path
        let path = if let Some(path) = &self.config_path {
            path.clone()
        } else {
            // Default to user config path
            let mut path = dirs::home_dir().ok_or_else(|| {
                ConfigError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Home directory not found",
                ))
            })?;
            path.push(".solana-hft");
            path.push("config.json");
            path
        };
        
        // Save configuration
        self.loader.save_config(&config, &path)?;
        
        Ok(())
    }
    
    /// Save configuration to a specific file
    pub fn save_config_to(&self, path: &Path) -> ConfigResult<()> {
        let config = self.config.read().unwrap();
        
        // Save configuration
        self.loader.save_config(&config, path)?;
        
        Ok(())
    }
    
    /// Validate configuration
    fn validate_config(&self, config: &BotConfig) -> ConfigResult<()> {
        // Validate RPC URL
        if config.rpc_url.is_empty() {
            return Err(ConfigError::ValidationError(
                "RPC URL cannot be empty".to_string(),
            ));
        }
        
        // Validate websocket URL
        if config.websocket_url.is_empty() {
            return Err(ConfigError::ValidationError(
                "Websocket URL cannot be empty".to_string(),
            ));
        }
        
        // Validate endpoints
        if config.network.endpoints.rpc_endpoints.is_empty() {
            return Err(ConfigError::ValidationError(
                "At least one RPC endpoint must be configured".to_string(),
            ));
        }
        
        Ok(())
    }
    
    /// Enable a feature
    pub fn enable_feature(&self, feature_name: &str) -> ConfigResult<()> {
        let mut config = self.config.write().unwrap();
        
        // Update feature flags
        match feature_name {
            // Networking features
            "kernel_bypass_networking" => config.feature_flags.networking.kernel_bypass_networking = true,
            "hardware_timestamping" => config.feature_flags.networking.hardware_timestamping = true,
            "high_frequency_networking" => config.feature_flags.networking.high_frequency_networking = true,
            "dpdk_support" => config.feature_flags.networking.dpdk_support = true,
            "io_uring_support" => config.feature_flags.networking.io_uring_support = true,
            
            // CPU features
            "cpu_pinning" => config.feature_flags.cpu.cpu_pinning = true,
            "simd_optimizations" => config.feature_flags.cpu.simd_optimizations = true,
            "avx_support" => config.feature_flags.cpu.avx_support = true,
            "avx2_support" => config.feature_flags.cpu.avx2_support = true,
            "avx512_support" => config.feature_flags.cpu.avx512_support = true,
            
            // Memory features
            "huge_pages" => config.feature_flags.memory.huge_pages = true,
            "huge_pages_1gb" => config.feature_flags.memory.huge_pages_1gb = true,
            
            // Additional capabilities
            "numa_awareness" => config.feature_flags.capabilities.numa_awareness = true,
            "direct_memory_access" => config.feature_flags.capabilities.direct_memory_access = true,
            "isolated_cpus" => config.feature_flags.capabilities.isolated_cpus = true,
            
            _ => return Err(ConfigError::ValidationError(
                format!("Unknown feature: {}", feature_name),
            )),
        }
        
        Ok(())
    }
    
    /// Disable a feature
    pub fn disable_feature(&self, feature_name: &str) -> ConfigResult<()> {
        let mut config = self.config.write().unwrap();
        
        // Update feature flags
        match feature_name {
            // Networking features
            "kernel_bypass_networking" => config.feature_flags.networking.kernel_bypass_networking = false,
            "hardware_timestamping" => config.feature_flags.networking.hardware_timestamping = false,
            "high_frequency_networking" => config.feature_flags.networking.high_frequency_networking = false,
            "dpdk_support" => config.feature_flags.networking.dpdk_support = false,
            "io_uring_support" => config.feature_flags.networking.io_uring_support = false,
            
            // CPU features
            "cpu_pinning" => config.feature_flags.cpu.cpu_pinning = false,
            "simd_optimizations" => config.feature_flags.cpu.simd_optimizations = false,
            "avx_support" => config.feature_flags.cpu.avx_support = false,
            "avx2_support" => config.feature_flags.cpu.avx2_support = false,
            "avx512_support" => config.feature_flags.cpu.avx512_support = false,
            
            // Memory features
            "huge_pages" => config.feature_flags.memory.huge_pages = false,
            "huge_pages_1gb" => config.feature_flags.memory.huge_pages_1gb = false,
            
            // Additional capabilities
            "numa_awareness" => config.feature_flags.capabilities.numa_awareness = false,
            "direct_memory_access" => config.feature_flags.capabilities.direct_memory_access = false,
            "isolated_cpus" => config.feature_flags.capabilities.isolated_cpus = false,
            
            _ => return Err(ConfigError::ValidationError(
                format!("Unknown feature: {}", feature_name),
            )),
        }
        
        Ok(())
    }
    
    /// Get all enabled features
    pub fn get_enabled_features(&self) -> Vec<String> {
        let config = self.config.read().unwrap();
        let mut features = Vec::new();
        
        // Networking features
        if config.feature_flags.networking.kernel_bypass_networking {
            features.push("kernel_bypass_networking".to_string());
        }
        if config.feature_flags.networking.hardware_timestamping {
            features.push("hardware_timestamping".to_string());
        }
        if config.feature_flags.networking.high_frequency_networking {
            features.push("high_frequency_networking".to_string());
        }
        if config.feature_flags.networking.dpdk_support {
            features.push("dpdk_support".to_string());
        }
        if config.feature_flags.networking.io_uring_support {
            features.push("io_uring_support".to_string());
        }
        
        // CPU features
        if config.feature_flags.cpu.cpu_pinning {
            features.push("cpu_pinning".to_string());
        }
        if config.feature_flags.cpu.simd_optimizations {
            features.push("simd_optimizations".to_string());
        }
        if config.feature_flags.cpu.avx_support {
            features.push("avx_support".to_string());
        }
        if config.feature_flags.cpu.avx2_support {
            features.push("avx2_support".to_string());
        }
        if config.feature_flags.cpu.avx512_support {
            features.push("avx512_support".to_string());
        }
        
        // Memory features
        if config.feature_flags.memory.huge_pages {
            features.push("huge_pages".to_string());
        }
        if config.feature_flags.memory.huge_pages_1gb {
            features.push("huge_pages_1gb".to_string());
        }
        
        // Additional capabilities
        if config.feature_flags.capabilities.numa_awareness {
            features.push("numa_awareness".to_string());
        }
        if config.feature_flags.capabilities.direct_memory_access {
            features.push("direct_memory_access".to_string());
        }
        if config.feature_flags.capabilities.isolated_cpus {
            features.push("isolated_cpus".to_string());
        }
        
        features
    }
    
    /// Check if a feature is enabled
    pub fn is_feature_enabled(&self, feature_name: &str) -> bool {
        let config = self.config.read().unwrap();
        
        match feature_name {
            // Networking features
            "kernel_bypass_networking" => config.feature_flags.networking.kernel_bypass_networking,
            "hardware_timestamping" => config.feature_flags.networking.hardware_timestamping,
            "high_frequency_networking" => config.feature_flags.networking.high_frequency_networking,
            "dpdk_support" => config.feature_flags.networking.dpdk_support,
            "io_uring_support" => config.feature_flags.networking.io_uring_support,
            
            // CPU features
            "cpu_pinning" => config.feature_flags.cpu.cpu_pinning,
            "simd_optimizations" => config.feature_flags.cpu.simd_optimizations,
            "avx_support" => config.feature_flags.cpu.avx_support,
            "avx2_support" => config.feature_flags.cpu.avx2_support,
            "avx512_support" => config.feature_flags.cpu.avx512_support,
            
            // Memory features
            "huge_pages" => config.feature_flags.memory.huge_pages,
            "huge_pages_1gb" => config.feature_flags.memory.huge_pages_1gb,
            
            // Additional capabilities
            "numa_awareness" => config.feature_flags.capabilities.numa_awareness,
            "direct_memory_access" => config.feature_flags.capabilities.direct_memory_access,
            "isolated_cpus" => config.feature_flags.capabilities.isolated_cpus,
            
            _ => false,
        }
    }
}

impl Default for ConfigurationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_configuration_manager_default() {
        let manager = ConfigurationManager::default();
        assert!(manager.hardware_capabilities.is_none());
        assert!(manager.network_optimizer.is_none());
        assert!(manager.endpoint_manager.is_none());
        assert!(manager.config_path.is_none());
    }
    
    #[test]
    fn test_feature_flags() {
        let manager = ConfigurationManager::default();
        
        // Enable a feature
        manager.enable_feature("kernel_bypass_networking").unwrap();
        assert!(manager.is_feature_enabled("kernel_bypass_networking"));
        
        // Disable a feature
        manager.disable_feature("kernel_bypass_networking").unwrap();
        assert!(!manager.is_feature_enabled("kernel_bypass_networking"));
        
        // Get enabled features
        let features = manager.get_enabled_features();
        assert!(!features.contains(&"kernel_bypass_networking".to_string()));
    }
    
    #[test]
    fn test_save_config() -> ConfigResult<()> {
        // Create a temporary directory
        let dir = tempdir()?;
        let config_path = dir.path().join("config.json");
        
        // Create a manager
        let manager = ConfigurationManager::default();
        
        // Save configuration
        manager.save_config_to(&config_path)?;
        
        // Check that the file exists
        assert!(config_path.exists());
        
        // Clean up
        dir.close()?;
        
        Ok(())
    }
}