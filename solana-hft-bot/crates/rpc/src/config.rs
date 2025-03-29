//! Configuration for RPC client
//!
//! This module provides configuration structures for the RPC client.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};

use crate::cache::CacheConfig;
use crate::endpoints::EndpointSelectionStrategy;

/// Endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// URL of the endpoint
    pub url: String,
    
    /// Weight for load balancing (higher = more traffic)
    pub weight: u32,
    
    /// Geographic region of the endpoint
    pub region: String,
    
    /// Endpoint features
    pub features: Vec<String>,
    
    /// Endpoint tier (free, paid, etc.)
    pub tier: String,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            url: "https://api.mainnet-beta.solana.com".to_string(),
            weight: 1,
            region: "global".to_string(),
            features: vec![],
            tier: "free".to_string(),
        }
    }
}

/// Batching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchingConfig {
    /// Whether to enable batching
    pub enable_batching: bool,
    
    /// Maximum batch size
    pub max_batch_size: usize,
    
    /// Maximum batch interval in milliseconds
    pub max_batch_interval_ms: u64,
    
    /// Minimum batch size to process immediately
    pub min_batch_size: usize,
}

impl Default for BatchingConfig {
    fn default() -> Self {
        Self {
            enable_batching: true,
            max_batch_size: 20,
            max_batch_interval_ms: 10,
            min_batch_size: 5,
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: usize,
    
    /// Base delay between retries in milliseconds
    pub base_delay_ms: u64,
    
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
    
    /// Whether to use exponential backoff
    pub use_exponential_backoff: bool,
    
    /// Whether to retry on timeout
    pub retry_on_timeout: bool,
    
    /// Whether to retry on rate limit
    pub retry_on_rate_limit: bool,
    
    /// Whether to retry on connection error
    pub retry_on_connection_error: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 100,
            max_delay_ms: 1000,
            use_exponential_backoff: true,
            retry_on_timeout: true,
            retry_on_rate_limit: true,
            retry_on_connection_error: true,
        }
    }
}

/// RPC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    /// List of endpoints
    pub endpoints: Vec<EndpointConfig>,
    
    /// List of websocket endpoints
    pub websocket_endpoints: Vec<String>,
    
    /// Commitment level
    #[serde(with = "CommitmentConfigDef")]
    pub commitment_config: CommitmentConfig,
    
    /// Number of connections per endpoint
    pub connection_pool_size_per_endpoint: usize,
    
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: usize,
    
    /// Whether to skip preflight checks when sending transactions
    pub skip_preflight: bool,
    
    /// Number of retries for send_transaction
    pub send_transaction_retry_count: usize,
    
    /// Rate limit in requests per second
    pub rate_limit_requests_per_second: f64,
    
    /// Rate limit burst size
    pub rate_limit_burst_size: f64,
    
    /// Endpoint health check interval in milliseconds
    pub endpoint_health_check_interval_ms: u64,
    
    /// Connection refresh interval in milliseconds
    pub connection_refresh_interval_ms: u64,
    
    /// Cache configuration
    pub cache_config: CacheConfig,
    
    /// Batching configuration
    pub batching_config: BatchingConfig,
    
    /// Retry configuration
    pub retry_config: RetryConfig,
    
    /// Default endpoint selection strategy
    pub endpoint_selection_strategy: EndpointSelectionStrategy,
    
    /// Whether to use websocket for subscriptions
    pub use_websocket: bool,
    
    /// Method-specific configurations
    pub method_configs: HashMap<String, MethodConfig>,
}

/// Method-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodConfig {
    /// Rate limit cost multiplier
    pub rate_limit_cost: f64,
    
    /// Cache TTL in milliseconds
    pub cache_ttl_ms: u64,
    
    /// Whether to allow batching
    pub allow_batching: bool,
    
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    
    /// Maximum retries
    pub max_retries: usize,
}

impl Default for MethodConfig {
    fn default() -> Self {
        Self {
            rate_limit_cost: 1.0,
            cache_ttl_ms: 0,
            allow_batching: true,
            timeout_ms: 15_000,
            max_retries: 3,
        }
    }
}

impl Default for RpcConfig {
    fn default() -> Self {
        let mut method_configs = HashMap::new();
        
        // Configure common methods
        method_configs.insert("getAccountInfo".to_string(), MethodConfig {
            rate_limit_cost: 1.0,
            cache_ttl_ms: 2_000,
            allow_batching: true,
            timeout_ms: 15_000,
            max_retries: 3,
        });
        
        method_configs.insert("getBalance".to_string(), MethodConfig {
            rate_limit_cost: 0.5,
            cache_ttl_ms: 2_000,
            allow_batching: true,
            timeout_ms: 10_000,
            max_retries: 3,
        });
        
        method_configs.insert("getLatestBlockhash".to_string(), MethodConfig {
            rate_limit_cost: 0.5,
            cache_ttl_ms: 1_000,
            allow_batching: false,
            timeout_ms: 10_000,
            max_retries: 3,
        });
        
        method_configs.insert("getMultipleAccounts".to_string(), MethodConfig {
            rate_limit_cost: 2.0,
            cache_ttl_ms: 2_000,
            allow_batching: true,
            timeout_ms: 20_000,
            max_retries: 3,
        });
        
        method_configs.insert("getProgramAccounts".to_string(), MethodConfig {
            rate_limit_cost: 5.0,
            cache_ttl_ms: 5_000,
            allow_batching: false,
            timeout_ms: 30_000,
            max_retries: 2,
        });
        
        method_configs.insert("sendTransaction".to_string(), MethodConfig {
            rate_limit_cost: 3.0,
            cache_ttl_ms: 0,
            allow_batching: false,
            timeout_ms: 15_000,
            max_retries: 3,
        });
        
        Self {
            endpoints: vec![
                EndpointConfig {
                    url: "https://api.mainnet-beta.solana.com".to_string(),
                    weight: 1,
                    region: "global".to_string(),
                    features: vec![],
                    tier: "free".to_string(),
                },
                EndpointConfig {
                    url: "https://solana-api.projectserum.com".to_string(),
                    weight: 1,
                    region: "global".to_string(),
                    features: vec![],
                    tier: "free".to_string(),
                },
            ],
            websocket_endpoints: vec![
                "wss://api.mainnet-beta.solana.com".to_string(),
                "wss://solana-api.projectserum.com".to_string(),
            ],
            commitment_config: CommitmentConfig::confirmed(),
            connection_pool_size_per_endpoint: 5,
            request_timeout_ms: 15_000,
            max_concurrent_requests: 100,
            skip_preflight: true,
            send_transaction_retry_count: 3,
            rate_limit_requests_per_second: 100.0,
            rate_limit_burst_size: 200.0,
            endpoint_health_check_interval_ms: 30_000,
            connection_refresh_interval_ms: 60_000,
            cache_config: CacheConfig::default(),
            batching_config: BatchingConfig::default(),
            retry_config: RetryConfig::default(),
            endpoint_selection_strategy: EndpointSelectionStrategy::WeightedRandom,
            use_websocket: true,
            method_configs,
        }
    }
}

/// Helper for serializing CommitmentConfig
#[derive(Serialize, Deserialize)]
#[serde(remote = "CommitmentConfig")]
pub struct CommitmentConfigDef {
    pub commitment: CommitmentLevel,
}

/// Load configuration from a file
pub fn load_config(path: &str) -> Result<RpcConfig, std::io::Error> {
    let config_str = std::fs::read_to_string(path)?;
    let config = serde_json::from_str(&config_str)?;
    Ok(config)
}

/// Save configuration to a file
pub fn save_config(config: &RpcConfig, path: &str) -> Result<(), std::io::Error> {
    let config_str = serde_json::to_string_pretty(config)?;
    std::fs::write(path, config_str)?;
    Ok(())
}
