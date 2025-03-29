use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use solana_client::rpc_config::CommitmentConfig;

/// Bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    /// Bot name
    pub name: String,
    
    /// RPC URL
    pub rpc_url: String,
    
    /// Websocket URL
    pub websocket_url: String,
    
    /// Commitment level
    #[serde(with = "solana_client::rpc_config::commitment_config_serde")]
    pub commitment_config: CommitmentConfig,
    
    /// Path to keypair file
    pub keypair_path: Option<PathBuf>,
    
    /// Module configurations
    pub module_configs: ModuleConfigs,
    
    /// Strategy configuration
    pub strategy_config: serde_json::Value,
    
    /// Strategy run interval in milliseconds
    pub strategy_run_interval_ms: u64,
}

/// Module configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfigs {
    /// Network module configuration
    pub network: ModuleConfig,
    
    /// RPC module configuration
    pub rpc: ModuleConfig,
    
    /// Screening module configuration
    pub screening: ModuleConfig,
    
    /// Execution module configuration
    pub execution: ModuleConfig,
    
    /// Arbitrage module configuration
    pub arbitrage: ModuleConfig,
    
    /// Risk module configuration
    pub risk: ModuleConfig,
}

/// Module configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// Whether the module is enabled
    pub enabled: bool,
    
    /// Module-specific configuration
    pub config: Option<serde_json::Value>,
}

impl BotConfig {
    /// Load configuration from file
    pub fn from_file(path: PathBuf) -> Result<Self> {
        // Read file
        let config_str = std::fs::read_to_string(&path)?;
        
        // Parse JSON
        let config: BotConfig = serde_json::from_str(&config_str)?;
        
        Ok(config)
    }
    
    /// Create default configuration
    pub fn default() -> Self {
        Self {
            name: "Solana HFT Bot".to_string(),
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            websocket_url: "wss://api.mainnet-beta.solana.com".to_string(),
            commitment_config: CommitmentConfig::confirmed(),
            keypair_path: None,
            module_configs: ModuleConfigs {
                network: ModuleConfig {
                    enabled: true,
                    config: Some(DefaultConfigs::network()),
                },
                rpc: ModuleConfig {
                    enabled: true,
                    config: Some(DefaultConfigs::rpc()),
                },
                screening: ModuleConfig {
                    enabled: true,
                    config: Some(DefaultConfigs::screening()),
                },
                execution: ModuleConfig {
                    enabled: true,
                    config: Some(DefaultConfigs::execution()),
                },
                arbitrage: ModuleConfig {
                    enabled: true,
                    config: Some(DefaultConfigs::arbitrage()),
                },
                risk: ModuleConfig {
                    enabled: true,
                    config: Some(DefaultConfigs::risk()),
                },
            },
            strategy_config: serde_json::json!({
                "enabled": true,
                "strategies": [
                    {
                        "id": "token_sniper",
                        "name": "Token Sniper",
                        "enabled": true,
                        "type": "TokenSniper"
                    },
                    {
                        "id": "arbitrage",
                        "name": "Cross-DEX Arbitrage",
                        "enabled": true,
                        "type": "Arbitrage"
                    },
                    {
                        "id": "mev",
                        "name": "MEV Searcher",
                        "enabled": true,
                        "type": "Mev"
                    }
                ]
            }),
            strategy_run_interval_ms: 1000,
        }
    }
}

/// Default configurations for modules
pub struct DefaultConfigs;

impl DefaultConfigs {
    /// Default network configuration
    pub fn network() -> serde_json::Value {
        serde_json::json!({
            "worker_threads": null,
            "use_dpdk": false,
            "use_io_uring": true,
            "send_buffer_size": 1048576,
            "recv_buffer_size": 1048576,
            "connection_timeout": {
                "secs": 30,
                "nanos": 0
            },
            "keepalive_interval": {
                "secs": 15,
                "nanos": 0
            },
            "max_connections": 1000,
            "bind_addresses": [],
            "socket_options": {
                "tcp_nodelay": true,
                "tcp_quickack": true,
                "so_reuseaddr": true,
                "so_reuseport": true,
                "so_sndbuf": 1048576,
                "so_rcvbuf": 1048576,
                "tcp_fastopen": true,
                "priority": null,
                "tos": 16
            }
        })
    }
    
    /// Default RPC configuration
    pub fn rpc() -> serde_json::Value {
        serde_json::json!({
            "endpoints": [
                {
                    "url": "https://api.mainnet-beta.solana.com",
                    "weight": 1,
                    "region": "global"
                },
                {
                    "url": "https://solana-api.projectserum.com",
                    "weight": 1,
                    "region": "global"
                }
            ],
            "commitment": "confirmed",
            "connection_pool_size_per_endpoint": 5,
            "request_timeout_ms": 15000,
            "batch_size": 10,
            "max_concurrent_requests": 100,
            "skip_preflight": true,
            "send_transaction_retry_count": 3,
            "rate_limit_requests_per_second": 100.0,
            "rate_limit_burst_size": 200,
            "endpoint_health_check_interval_ms": 30000,
            "connection_refresh_interval_ms": 60000,
            "cache_config": {
                "enable_cache": true,
                "max_cache_size": 10000,
                "account_ttl_ms": 2000,
                "blockhash_ttl_ms": 1000,
                "slot_ttl_ms": 500
            }
        })
    }
    
    /// Default screening configuration
    pub fn screening() -> serde_json::Value {
        serde_json::json!({
            "rpc_url": "https://api.mainnet-beta.solana.com",
            "rpc_ws_url": "wss://api.mainnet-beta.solana.com",
            "commitment_config": "confirmed",
            "min_liquidity_usd": 10000.0,
            "min_token_age_seconds": 3600,
            "max_rugpull_risk": 70,
            "price_impact_threshold": 0.05,
            "min_profit_bps": 50,
            "max_tokens": 10000,
            "max_liquidity_pools": 5000,
            "token_update_interval_ms": 30000,
            "liquidity_update_interval_ms": 15000,
            "scoring_config": {
                "liquidity_weight": 0.4,
                "volatility_weight": 0.2,
                "social_weight": 0.2,
                "code_quality_weight": 0.2,
                "high_liquidity_threshold": 1000000.0,
                "medium_liquidity_threshold": 100000.0,
                "high_volatility_threshold": 0.2,
                "medium_volatility_threshold": 0.1
            }
        })
    }
    
    /// Default execution configuration
    pub fn execution() -> serde_json::Value {
        serde_json::json!({
            "rpc_url": "https://api.mainnet-beta.solana.com",
            "commitment_config": "confirmed",
            "skip_preflight": true,
            "max_retries": 3,
            "confirmation_timeout_ms": 60000,
            "blockhash_update_interval_ms": 2000,
            "tx_status_check_interval_ms": 2000,
            "fee_model_config": {
                "use_ml_model": false,
                "base_fee": 5000,
                "max_fee": 1000000,
                "min_fee": 1000,
                "adjust_for_congestion": true,
                "congestion_multiplier": 2.0
            },
            "vault_config": {
                "prewarm_vault": true,
                "vault_size": 100,
                "refresh_interval_ms": 30000,
                "max_tx_size": 1232,
                "use_durable_nonce": false
            },
            "use_jito_bundles": false,
            "jito_bundle_url": "https://mainnet.block-engine.jito.io"
        })
    }
    
    /// Default arbitrage configuration
    pub fn arbitrage() -> serde_json::Value {
        serde_json::json!({
            "rpc_url": "https://api.mainnet-beta.solana.com",
            "websocket_url": "wss://api.mainnet-beta.solana.com",
            "commitment_config": "confirmed",
            "min_profit_threshold_bps": 10,
            "max_concurrent_executions": 5,
            "max_queue_size": 100,
            "confirmation_timeout_ms": 30000,
            "price_update_interval_ms": 1000,
            "pool_update_interval_ms": 5000,
            "opportunity_detection_interval_ms": 1000,
            "use_flash_loans": true,
            "use_jito_bundles": true,
            "prioritize_high_profit": true
        })
    }
    
    /// Default risk configuration
    pub fn risk() -> serde_json::Value {
        serde_json::json!({
            "key_account": "",
            "max_drawdown_pct": 10.0,
            "max_exposure_pct": 0.8,
            "max_token_concentration": 0.2,
            "max_strategy_allocation": 0.3,
            "max_risk_score": 70,
            "max_consecutive_losses": 5,
            "capital_update_interval_ms": 60000,
            "circuit_breaker_check_interval_ms": 30000,
            "risk_report_interval_ms": 300000,
            "max_pnl_history_size": 1000
        })
    }
}