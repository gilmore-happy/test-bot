//! Configuration loading and validation
//!
//! This module provides functionality for loading configuration
//! from various sources and validating it.

use crate::error::{ConfigError, ConfigResult};
use crate::schema::BotConfig;
use config::{Config, ConfigBuilder, Environment, File, FileFormat};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Configuration loader
#[derive(Debug)]
pub struct ConfigLoader {
    /// Command-line specified config path
    cli_config_path: Option<PathBuf>,
    
    /// Environment variable prefix
    env_prefix: String,
    
    /// Search paths for configuration files
    search_paths: Vec<PathBuf>,
    
    /// Default configuration
    default_config: BotConfig,
    
    /// Configuration sources that were used
    used_sources: Vec<String>,
}

impl ConfigLoader {
    /// Create a new configuration loader
    pub fn new() -> Self {
        Self {
            cli_config_path: None,
            env_prefix: "SOLANA_HFT".to_string(),
            search_paths: vec![
                PathBuf::from("./config.json"),
                PathBuf::from("./config.yaml"),
                Self::get_user_config_path(),
                Self::get_system_config_path(),
            ],
            default_config: BotConfig::default(),
            used_sources: Vec::new(),
        }
    }
    
    /// Set command-line specified config path
    pub fn with_cli_config_path<P: AsRef<Path>>(mut self, path: Option<P>) -> Self {
        self.cli_config_path = path.map(|p| p.as_ref().to_path_buf());
        self
    }
    
    /// Set environment variable prefix
    pub fn with_env_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.env_prefix = prefix.into();
        self
    }
    
    /// Add a search path
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }
    
    /// Set default configuration
    pub fn with_default_config(mut self, config: BotConfig) -> Self {
        self.default_config = config;
        self
    }
    
    /// Load configuration
    pub fn load(&mut self) -> ConfigResult<BotConfig> {
        info!("Loading configuration...");
        
        // Start with default configuration
        let mut builder = Config::builder();
        
        // Convert default config to a HashMap for the builder
        let default_values = self.config_to_map(&self.default_config)?;
        builder = builder.add_source(config::Config::try_from(&default_values)?);
        self.used_sources.push("default configuration".to_string());
        
        // Try to load configuration from search paths
        let mut found_config_file = false;
        
        // First, try command-line specified config
        if let Some(path) = &self.cli_config_path {
            if path.exists() {
                debug!("Loading configuration from CLI-specified path: {:?}", path);
                builder = self.add_file_source(builder, path)?;
                self.used_sources.push(format!("CLI-specified config: {:?}", path));
                found_config_file = true;
            } else {
                warn!("CLI-specified config file not found: {:?}", path);
                return Err(ConfigError::LoadError(
                    format!("CLI-specified config file not found: {:?}", path),
                ));
            }
        }
        
        // If no CLI config, try search paths
        if !found_config_file {
            for path in &self.search_paths {
                if path.exists() {
                    debug!("Loading configuration from: {:?}", path);
                    builder = self.add_file_source(builder, path)?;
                    self.used_sources.push(format!("config file: {:?}", path));
                    found_config_file = true;
                    break;
                }
            }
        }
        
        // Load environment variables
        let env_source = Environment::with_prefix(&self.env_prefix)
            .separator("__")
            .try_parsing(true);
        
        builder = builder.add_source(env_source);
        self.used_sources.push(format!("environment variables with prefix {}", self.env_prefix));
        
        // Build the configuration
        let config_values = builder.build()?;
        
        // Convert back to BotConfig
        let config: BotConfig = config_values.try_deserialize()?;
        
        // Validate the configuration
        self.validate_config(&config)?;
        
        info!("Configuration loaded from: {}", self.used_sources.join(", "));
        
        Ok(config)
    }
    
    /// Add a file source to the config builder
    fn add_file_source(&self, builder: ConfigBuilder<config::builder::DefaultState>, path: &Path) -> ConfigResult<ConfigBuilder<config::builder::DefaultState>> {
        let format = match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => FileFormat::Json,
            Some("yaml") | Some("yml") => FileFormat::Yaml,
            _ => {
                // Try to detect format from content
                if let Ok(content) = std::fs::read_to_string(path) {
                    if content.trim().starts_with('{') {
                        FileFormat::Json
                    } else {
                        FileFormat::Yaml
                    }
                } else {
                    return Err(ConfigError::LoadError(
                        format!("Could not determine file format for: {:?}", path),
                    ));
                }
            }
        };
        
        Ok(builder.add_source(File::from(path).format(format).required(false)))
    }
    
    /// Convert BotConfig to a HashMap for the config builder
    fn config_to_map(&self, config: &BotConfig) -> ConfigResult<HashMap<String, config::Value>> {
        // Serialize to JSON Value
        let json_value = serde_json::to_value(config)?;
        
        // Convert to config::Value
        let config_value = config::Value::try_from(json_value)?;
        
        // Convert to HashMap
        match config_value {
            config::Value::Table(table) => Ok(table),
            _ => Err(ConfigError::ParseError(
                "Failed to convert config to table".to_string(),
            )),
        }
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
        
        // Validate hardware configuration
        if let Some(cores) = config.hardware.cpu.physical_cores {
            if cores == 0 {
                return Err(ConfigError::ValidationError(
                    "Physical cores must be greater than 0".to_string(),
                ));
            }
        }
        
        if let Some(threads) = config.hardware.cpu.logical_threads {
            if threads == 0 {
                return Err(ConfigError::ValidationError(
                    "Logical threads must be greater than 0".to_string(),
                ));
            }
        }
        
        // Validate socket buffer sizes
        if config.network.socket_buffers.send_buffer_size == 0 {
            return Err(ConfigError::ValidationError(
                "Send buffer size must be greater than 0".to_string(),
            ));
        }
        
        if config.network.socket_buffers.recv_buffer_size == 0 {
            return Err(ConfigError::ValidationError(
                "Receive buffer size must be greater than 0".to_string(),
            ));
        }
        
        // Validate connection pool size
        if config.network.endpoints.connection_pool_size == 0 {
            return Err(ConfigError::ValidationError(
                "Connection pool size must be greater than 0".to_string(),
            ));
        }
        
        // Validate timeouts
        if config.network.endpoints.connection_timeout_ms == 0 {
            return Err(ConfigError::ValidationError(
                "Connection timeout must be greater than 0".to_string(),
            ));
        }
        
        if config.network.latency_optimization.base_timeout_ms == 0 {
            return Err(ConfigError::ValidationError(
                "Base timeout must be greater than 0".to_string(),
            ));
        }
        
        if config.network.latency_optimization.max_timeout_ms == 0 {
            return Err(ConfigError::ValidationError(
                "Maximum timeout must be greater than 0".to_string(),
            ));
        }
        
        if config.network.latency_optimization.max_timeout_ms < config.network.latency_optimization.base_timeout_ms {
            return Err(ConfigError::ValidationError(
                "Maximum timeout must be greater than or equal to base timeout".to_string(),
            ));
        }
        
        Ok(())
    }
    
    /// Get user configuration path
    fn get_user_config_path() -> PathBuf {
        if let Some(mut path) = dirs::home_dir() {
            path.push(".solana-hft");
            path.push("config.json");
            path
        } else {
            PathBuf::from("~/.solana-hft/config.json")
        }
    }
    
    /// Get system configuration path
    fn get_system_config_path() -> PathBuf {
        if cfg!(unix) {
            PathBuf::from("/etc/solana-hft/config.json")
        } else if cfg!(windows) {
            PathBuf::from("C:\\ProgramData\\solana-hft\\config.json")
        } else {
            PathBuf::from("/etc/solana-hft/config.json")
        }
    }
    
    /// Save configuration to a file
    pub fn save_config(&self, config: &BotConfig, path: &Path) -> ConfigResult<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Determine file format
        let format = match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => "json",
            Some("yaml") | Some("yml") => "yaml",
            _ => "json", // Default to JSON
        };
        
        // Serialize and save
        match format {
            "json" => {
                let json = serde_json::to_string_pretty(config)?;
                std::fs::write(path, json)?;
            }
            "yaml" => {
                let yaml = serde_yaml::to_string(config)?;
                std::fs::write(path, yaml)?;
            }
            _ => unreachable!(),
        }
        
        info!("Configuration saved to: {:?}", path);
        
        Ok(())
    }
    
    /// Get the sources that were used to load the configuration
    pub fn get_used_sources(&self) -> &[String] {
        &self.used_sources
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_config_loader_default() {
        let loader = ConfigLoader::default();
        assert_eq!(loader.env_prefix, "SOLANA_HFT");
        assert_eq!(loader.search_paths.len(), 4);
        assert!(loader.cli_config_path.is_none());
    }
    
    #[test]
    fn test_config_loader_with_cli_path() {
        let loader = ConfigLoader::default().with_cli_config_path(Some(PathBuf::from("test.json")));
        assert_eq!(loader.cli_config_path, Some(PathBuf::from("test.json")));
    }
    
    #[test]
    fn test_config_loader_with_env_prefix() {
        let loader = ConfigLoader::default().with_env_prefix("TEST");
        assert_eq!(loader.env_prefix, "TEST");
    }
    
    #[test]
    fn test_save_and_load_config() -> ConfigResult<()> {
        // Create a temporary directory
        let dir = tempdir()?;
        let config_path = dir.path().join("config.json");
        
        // Create a config
        let config = BotConfig::default();
        
        // Save the config
        let loader = ConfigLoader::default();
        loader.save_config(&config, &config_path)?;
        
        // Load the config
        let mut loader = ConfigLoader::default().with_cli_config_path(Some(&config_path));
        let loaded_config = loader.load()?;
        
        // Check that the config was loaded
        assert!(loader.get_used_sources().iter().any(|s| s.contains(&config_path.to_string_lossy())));
        
        // Clean up
        dir.close()?;
        
        Ok(())
    }
}