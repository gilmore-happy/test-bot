use serde::{Deserialize, Serialize};
use solana_sdk::commitment_config::CommitmentConfig;
use crate::scoring::ScoringConfig;

/// Screening configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningConfig {
    /// RPC URL for HTTP requests
    pub rpc_url: String,
    
    /// RPC URL for websocket connections
    pub rpc_ws_url: String,
    
    /// Commitment level
    #[serde(with = "solana_client::rpc_config::commitment_config_serde")]
    pub commitment_config: CommitmentConfig,
    
    /// Minimum liquidity in USD for a token to be considered
    pub min_liquidity_usd: f64,
    
    /// Minimum token age in seconds to be considered
    pub min_token_age_seconds: u64,
    
    /// Maximum rugpull risk score (0-100)
    pub max_rugpull_risk: u8,
    
    /// Price impact threshold for opportunity detection (%)
    pub price_impact_threshold: f64,
    
    /// Minimum profit in basis points for opportunity tracking
    pub min_profit_bps: u32,
    
    /// Maximum number of tokens to track
    pub max_tokens: usize,
    
    /// Maximum number of liquidity pools to track
    pub max_liquidity_pools: usize,
    
    /// Token update interval in milliseconds
    pub token_update_interval_ms: u64,
    
    /// Liquidity update interval in milliseconds
    pub liquidity_update_interval_ms: u64,
    
    /// Token scoring configuration
    pub scoring_config: ScoringConfig,
    
    /// Transaction screening configuration
    pub tx_screening: TransactionScreeningConfig,
    
    /// Rug protection configuration
    pub rug_protection: RugProtectionConfig,
    
    /// Whether to enable DEX integration
    pub enable_dex_integration: bool,
    
    /// Whether to enable websocket subscriptions
    pub enable_websocket_subscriptions: bool,
    
    /// Whether to enable token scoring
    pub enable_token_scoring: bool,
    
    /// Whether to enable opportunity detection
    pub enable_opportunity_detection: bool,
    
    /// Whether to enable metrics collection
    pub enable_metrics: bool,
}

/// Transaction screening configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionScreeningConfig {
    /// Whether to enable transaction screening
    pub enabled: bool,
    
    /// Maximum transaction size in bytes
    pub max_tx_size: usize,
    
    /// Maximum number of instructions per transaction
    pub max_instructions: usize,
    
    /// Whether to check for program upgrades
    pub check_program_upgrades: bool,
    
    /// Whether to check for admin-only instructions
    pub check_admin_instructions: bool,
}

/// Rug protection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugProtectionConfig {
    /// Minimum liquidity in USD to consider a token
    pub min_liquidity_usd: f64,
    
    /// Minimum token age in seconds to consider it established
    pub min_token_age_seconds: u64,
    
    /// Maximum acceptable rug pull risk score (0-100)
    pub max_rugpull_risk: u8,
    
    /// Minimum percentage of liquidity that should be locked
    pub min_liquidity_locked_pct: f64,
    
    /// Maximum acceptable ownership concentration (percentage held by top wallets)
    pub max_ownership_concentration: f64,
    
    /// Whether to check for honeypot characteristics
    pub check_honeypot: bool,
    
    /// Whether to verify contract code
    pub verify_contract: bool,
    
    /// Whether to check social media presence
    pub check_social_presence: bool,
    
    /// Minimum team credibility score (0-100)
    pub min_team_credibility: u8,
}

impl Default for RugProtectionConfig {
    fn default() -> Self {
        Self {
            min_liquidity_usd: 10000.0,
            min_token_age_seconds: 3600, // 1 hour
            max_rugpull_risk: 70,
            min_liquidity_locked_pct: 50.0,
            max_ownership_concentration: 30.0,
            check_honeypot: true,
            verify_contract: true,
            check_social_presence: true,
            min_team_credibility: 30,
        }
    }
}

impl Default for TransactionScreeningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_tx_size: 1232,  // Default Solana transaction size limit
            max_instructions: 10,
            check_program_upgrades: true,
            check_admin_instructions: true,
        }
    }
}

impl Default for ScreeningConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            rpc_ws_url: "wss://api.mainnet-beta.solana.com".to_string(),
            commitment_config: CommitmentConfig::confirmed(),
            min_liquidity_usd: 10_000.0, // $10K
            min_token_age_seconds: 3600, // 1 hour
            max_rugpull_risk: 70,
            price_impact_threshold: 0.05, // 5%
            min_profit_bps: 50, // 0.5%
            max_tokens: 10_000,
            max_liquidity_pools: 5_000,
            token_update_interval_ms: 30_000, // 30 seconds
            liquidity_update_interval_ms: 15_000, // 15 seconds
            scoring_config: ScoringConfig::default(),
            tx_screening: TransactionScreeningConfig::default(),
            rug_protection: RugProtectionConfig::default(),
            enable_dex_integration: true,
            enable_websocket_subscriptions: true,
            enable_token_scoring: true,
            enable_opportunity_detection: true,
            enable_metrics: true,
        }
    }
}

/// Builder for ScreeningConfig
pub struct ScreeningConfigBuilder {
    config: ScreeningConfig,
}

impl ScreeningConfigBuilder {
    /// Create a new config builder with default values
    pub fn new() -> Self {
        Self {
            config: ScreeningConfig::default(),
        }
    }
    
    /// Set RPC URL
    pub fn rpc_url(mut self, url: &str) -> Self {
        self.config.rpc_url = url.to_string();
        self
    }
    
    /// Set WebSocket URL
    pub fn rpc_ws_url(mut self, url: &str) -> Self {
        self.config.rpc_ws_url = url.to_string();
        self
    }
    
    /// Set commitment config
    pub fn commitment(mut self, commitment: CommitmentConfig) -> Self {
        self.config.commitment_config = commitment;
        self
    }
    
    /// Set minimum liquidity threshold
    pub fn min_liquidity(mut self, min_liquidity_usd: f64) -> Self {
        self.config.min_liquidity_usd = min_liquidity_usd;
        self.config.rug_protection.min_liquidity_usd = min_liquidity_usd;
        self
    }
    
    /// Set minimum token age
    pub fn min_token_age(mut self, min_token_age_seconds: u64) -> Self {
        self.config.min_token_age_seconds = min_token_age_seconds;
        self.config.rug_protection.min_token_age_seconds = min_token_age_seconds;
        self
    }
    
    /// Set maximum rugpull risk
    pub fn max_rugpull_risk(mut self, max_rugpull_risk: u8) -> Self {
        self.config.max_rugpull_risk = max_rugpull_risk;
        self.config.rug_protection.max_rugpull_risk = max_rugpull_risk;
        self
    }
    
    /// Set price impact threshold
    pub fn price_impact_threshold(mut self, price_impact_threshold: f64) -> Self {
        self.config.price_impact_threshold = price_impact_threshold;
        self
    }
    
    /// Set minimum profit in basis points
    pub fn min_profit_bps(mut self, min_profit_bps: u32) -> Self {
        self.config.min_profit_bps = min_profit_bps;
        self
    }
    
    /// Set maximum tokens to track
    pub fn max_tokens(mut self, max_tokens: usize) -> Self {
        self.config.max_tokens = max_tokens;
        self
    }
    
    /// Set maximum liquidity pools to track
    pub fn max_liquidity_pools(mut self, max_liquidity_pools: usize) -> Self {
        self.config.max_liquidity_pools = max_liquidity_pools;
        self
    }
    
    /// Set token update interval
    pub fn token_update_interval(mut self, token_update_interval_ms: u64) -> Self {
        self.config.token_update_interval_ms = token_update_interval_ms;
        self
    }
    
    /// Set liquidity update interval
    pub fn liquidity_update_interval(mut self, liquidity_update_interval_ms: u64) -> Self {
        self.config.liquidity_update_interval_ms = liquidity_update_interval_ms;
        self
    }
    
    /// Set scoring config
    pub fn scoring_config(mut self, scoring_config: ScoringConfig) -> Self {
        self.config.scoring_config = scoring_config;
        self
    }
    
    /// Set transaction screening config
    pub fn tx_screening(mut self, tx_screening: TransactionScreeningConfig) -> Self {
        self.config.tx_screening = tx_screening;
        self
    }
    
    /// Set rug protection config
    pub fn rug_protection(mut self, rug_protection: RugProtectionConfig) -> Self {
        self.config.rug_protection = rug_protection;
        self
    }
    
    /// Enable or disable DEX integration
    pub fn enable_dex_integration(mut self, enable: bool) -> Self {
        self.config.enable_dex_integration = enable;
        self
    }
    
    /// Enable or disable websocket subscriptions
    pub fn enable_websocket_subscriptions(mut self, enable: bool) -> Self {
        self.config.enable_websocket_subscriptions = enable;
        self
    }
    
    /// Enable or disable token scoring
    pub fn enable_token_scoring(mut self, enable: bool) -> Self {
        self.config.enable_token_scoring = enable;
        self
    }
    
    /// Enable or disable opportunity detection
    pub fn enable_opportunity_detection(mut self, enable: bool) -> Self {
        self.config.enable_opportunity_detection = enable;
        self
    }
    
    /// Enable or disable metrics collection
    pub fn enable_metrics(mut self, enable: bool) -> Self {
        self.config.enable_metrics = enable;
        self
    }
    
    /// Build the config
    pub fn build(self) -> ScreeningConfig {
        self.config
    }
}