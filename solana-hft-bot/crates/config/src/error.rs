//! Error types for the configuration system

use thiserror::Error;

/// Configuration error type
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Error loading configuration file
    #[error("Failed to load configuration file: {0}")]
    LoadError(String),

    /// Error parsing configuration
    #[error("Failed to parse configuration: {0}")]
    ParseError(String),

    /// Error validating configuration
    #[error("Configuration validation error: {0}")]
    ValidationError(String),

    /// Error detecting hardware capabilities
    #[error("Hardware detection error: {0}")]
    HardwareDetectionError(String),

    /// Error with network configuration
    #[error("Network configuration error: {0}")]
    NetworkError(String),

    /// Error with environment variables
    #[error("Environment variable error: {0}")]
    EnvVarError(String),

    /// Error with file I/O
    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Error with JSON serialization/deserialization
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Error with YAML serialization/deserialization
    #[error("YAML error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    /// Error with configuration library
    #[error("Config library error: {0}")]
    ConfigError(#[from] config::ConfigError),

    /// General error
    #[error("{0}")]
    General(String),
}

/// Result type for configuration operations
pub type ConfigResult<T> = Result<T, ConfigError>;