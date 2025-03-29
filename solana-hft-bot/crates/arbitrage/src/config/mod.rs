//! Configuration for the arbitrage module
//!
//! This module provides configuration structures for the arbitrage engine.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_config::CommitmentConfig;

/// Arbitrage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageConfig {
    /// RPC URL
    pub rpc_url: String,
    
    /// Websocket URL
    pub websocket_url: String,
    
    /// Commitment level
    #[serde(with = "solana_client::rpc_config::commitment_config_serde")]
    pub commitment_config: CommitmentConfig,
    
    /// Minimum profit threshold in basis points
    pub min_profit_threshold_bps: u32,
    
    /// Maximum number of concurrent executions
    pub max_concurrent_executions: usize,
    
    /// Maximum queue size
    pub max_queue_size: usize,
    
    /// Confirmation timeout in milliseconds
    pub confirmation_timeout_ms: u64,
    
    /// Price update interval in milliseconds
    pub price_update_interval_ms: u64,
    
    /// Pool update interval in milliseconds
    pub pool_update_interval_ms: u64,
    
    /// Opportunity detection interval in milliseconds
    pub opportunity_detection_interval_ms: u64,
    
    /// Whether to use flash loans
    pub use_flash_loans: bool,
    
    /// Whether to use Jito bundles
    pub use_jito_bundles: bool,
    
    /// Whether to prioritize high-profit opportunities
    pub prioritize_high_profit: bool,
    /// Maximum slippage tolerance in basis points
    pub max_slippage_bps: u32,
    
    /// Minimum profit threshold in USD
    pub min_profit_threshold_usd: f64,
    
    /// Maximum gas cost in lamports
    pub max_gas_cost_lamports: u64,
    
    /// Whether to enable protocol-aware arbitrage
    pub enable_protocol_aware_arbitrage: bool,
    
    /// Protocol weights for scoring opportunities
    pub protocol_weights: HashMap<String, f64>,
    
    /// Whether to monitor mempool for arbitrage opportunities
    pub monitor_mempool: bool,
    
    /// Maximum number of transactions to analyze per mempool batch
    pub max_mempool_batch_size: usize,
    pub max_slippage_bps: u32,
}

impl Default for ArbitrageConfig {
    fn default() -> Self {
        // Default protocol weights
        let mut protocol_weights = HashMap::new();
        protocol_weights.insert("27haf8L6oxUeXrHrgEgsexjSY5hbVUWEmvv9Nyxg8vQv".to_string(), 1.0); // Raydium
        protocol_weights.insert("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string(), 1.0); // Orca
        protocol_weights.insert("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(), 1.2); // Jupiter
        protocol_weights.insert("opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb".to_string(), 0.9); // Openbook
        protocol_weights.insert("PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY".to_string(), 1.1); // Phoenix
        
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            websocket_url: "wss://api.mainnet-beta.solana.com".to_string(),
            commitment_config: CommitmentConfig::confirmed(),
            min_profit_threshold_bps: 50, // 0.5%
            max_concurrent_executions: 5,
            max_queue_size: 100,
            confirmation_timeout_ms: 30_000, // 30 seconds
            price_update_interval_ms: 1_000, // 1 second
            pool_update_interval_ms: 5_000, // 5 seconds
            opportunity_detection_interval_ms: 1_000, // 1 second
            use_flash_loans: true,
            use_jito_bundles: true,
            prioritize_high_profit: true,
            max_slippage_bps: 30, // 0.3%
            min_profit_threshold_usd: 0.5, // $0.50 minimum profit
            max_gas_cost_lamports: 100_000, // 0.0001 SOL max gas
            enable_protocol_aware_arbitrage: true,
            protocol_weights,
            monitor_mempool: true,
            max_mempool_batch_size: 100,
        }
    }
}