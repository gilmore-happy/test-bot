use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::Duration;

/// Market data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    /// Market ID
    pub market_id: Option<String>,
    
    /// Exchange name
    pub exchange: Option<String>,
    
    /// Base token mint
    pub base_mint: Option<Pubkey>,
    
    /// Quote token mint
    pub quote_mint: Option<Pubkey>,
    
    /// Base token symbol
    pub base_symbol: Option<String>,
    
    /// Quote token symbol
    pub quote_symbol: Option<String>,
    
    /// Current price
    pub price: f64,
    
    /// 24h volume in base token
    pub volume_24h: Option<f64>,
    
    /// 24h volume in USD
    pub volume_usd_24h: Option<f64>,
    
    /// Bid price
    pub bid: Option<f64>,
    
    /// Ask price
    pub ask: Option<f64>,
    
    /// Spread percentage
    pub spread_pct: Option<f64>,
    
    /// 24h price change percentage
    pub price_change_pct_24h: Option<f64>,
    
    /// Market liquidity in USD
    pub liquidity_usd: Option<f64>,
    
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
    
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

impl Default for MarketData {
    fn default() -> Self {
        Self {
            market_id: None,
            exchange: None,
            base_mint: None,
            quote_mint: None,
            base_symbol: None,
            quote_symbol: None,
            price: 0.0,
            volume_24h: None,
            volume_usd_24h: None,
            bid: None,
            ask: None,
            spread_pct: None,
            price_change_pct_24h: None,
            liquidity_usd: None,
            last_updated: chrono::Utc::now(),
            metadata: None,
        }
    }
}

/// Token data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    /// Token mint address
    pub mint: Pubkey,
    
    /// Token name
    pub name: Option<String>,
    
    /// Token symbol
    pub symbol: Option<String>,
    
    /// Token decimals
    pub decimals: Option<u8>,
    
    /// Token supply
    pub supply: Option<u64>,
    
    /// Token price in USD
    pub price_usd: Option<f64>,
    
    /// 24h price change percentage
    pub price_change_pct_24h: Option<f64>,
    
    /// 24h volume in USD
    pub volume_usd_24h: Option<f64>,
    
    /// Market cap in USD
    pub market_cap_usd: Option<f64>,
    
    /// Token liquidity in USD
    pub liquidity_usd: Option<f64>,
    
    /// Token age in seconds
    pub age_seconds: Option<u64>,
    
    /// Token creation timestamp
    pub creation_time: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Token holders count
    pub holders_count: Option<u64>,
    
    /// Token transactions count
    pub transactions_count: Option<u64>,
    
    /// Social score (0-100)
    pub social_score: Option<u8>,
    
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
    
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

impl Default for TokenData {
    fn default() -> Self {
        Self {
            mint: Pubkey::default(),
            name: None,
            symbol: None,
            decimals: None,
            supply: None,
            price_usd: None,
            price_change_pct_24h: None,
            volume_usd_24h: None,
            market_cap_usd: None,
            liquidity_usd: None,
            age_seconds: None,
            creation_time: None,
            holders_count: None,
            transactions_count: None,
            social_score: None,
            last_updated: chrono::Utc::now(),
            metadata: None,
        }
    }
}

/// Order data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderData {
    /// Order ID
    pub order_id: String,
    
    /// Token mint address
    pub token_mint: Pubkey,
    
    /// Market ID
    pub market_id: Option<String>,
    
    /// Order side
    pub side: OrderSide,
    
    /// Order type
    pub order_type: OrderType,
    
    /// Order price
    pub price: Option<f64>,
    
    /// Order size
    pub size: f64,
    
    /// Order value in USD
    pub value_usd: Option<f64>,
    
    /// Order status
    pub status: OrderStatus,
    
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    
    /// Filled amount
    pub filled_amount: f64,
    
    /// Average fill price
    pub avg_fill_price: Option<f64>,
    
    /// Fee paid
    pub fee_paid: Option<f64>,
    
    /// Transaction signature
    pub signature: Option<String>,
    
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    /// Buy order
    Buy,
    
    /// Sell order
    Sell,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    /// Market order
    Market,
    
    /// Limit order
    Limit,
    
    /// Stop order
    Stop,
    
    /// Stop limit order
    StopLimit,
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order is created
    Created,
    
    /// Order is pending
    Pending,
    
    /// Order is open
    Open,
    
    /// Order is partially filled
    PartiallyFilled,
    
    /// Order is filled
    Filled,
    
    /// Order is cancelled
    Cancelled,
    
    /// Order failed
    Failed,
    
    /// Order expired
    Expired,
}

/// Position data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionData {
    /// Position ID
    pub position_id: String,
    
    /// Token mint address
    pub token_mint: Pubkey,
    
    /// Token symbol
    pub token_symbol: Option<String>,
    
    /// Position size
    pub size: f64,
    
    /// Entry price
    pub entry_price: f64,
    
    /// Current price
    pub current_price: Option<f64>,
    
    /// Position value in USD
    pub value_usd: f64,
    
    /// Unrealized profit/loss in USD
    pub unrealized_pnl_usd: Option<f64>,
    
    /// Unrealized profit/loss percentage
    pub unrealized_pnl_pct: Option<f64>,
    
    /// Realized profit/loss in USD
    pub realized_pnl_usd: f64,
    
    /// Realized profit/loss percentage
    pub realized_pnl_pct: f64,
    
    /// Entry timestamp
    pub entry_time: chrono::DateTime<chrono::Utc>,
    
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
    
    /// Position status
    pub status: PositionStatus,
    
    /// Stop loss price
    pub stop_loss: Option<f64>,
    
    /// Take profit price
    pub take_profit: Option<f64>,
    
    /// Associated orders
    pub orders: Vec<String>,
    
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Position status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionStatus {
    /// Position is open
    Open,
    
    /// Position is closed
    Closed,
    
    /// Position is partially closed
    PartiallyClosed,
}

/// Portfolio data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioData {
    /// Total value in USD
    pub total_value_usd: f64,
    
    /// Available balance in USD
    pub available_balance_usd: f64,
    
    /// Positions value in USD
    pub positions_value_usd: f64,
    
    /// Unrealized profit/loss in USD
    pub unrealized_pnl_usd: f64,
    
    /// Unrealized profit/loss percentage
    pub unrealized_pnl_pct: f64,
    
    /// Realized profit/loss in USD
    pub realized_pnl_usd: f64,
    
    /// Realized profit/loss percentage
    pub realized_pnl_pct: f64,
    
    /// Daily profit/loss in USD
    pub daily_pnl_usd: f64,
    
    /// Daily profit/loss percentage
    pub daily_pnl_pct: f64,
    
    /// Weekly profit/loss in USD
    pub weekly_pnl_usd: f64,
    
    /// Weekly profit/loss percentage
    pub weekly_pnl_pct: f64,
    
    /// Monthly profit/loss in USD
    pub monthly_pnl_usd: f64,
    
    /// Monthly profit/loss percentage
    pub monthly_pnl_pct: f64,
    
    /// All-time profit/loss in USD
    pub all_time_pnl_usd: f64,
    
    /// All-time profit/loss percentage
    pub all_time_pnl_pct: f64,
    
    /// Positions
    pub positions: HashMap<String, PositionData>,
    
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    /// Transaction signature
    pub signature: String,
    
    /// Transaction status
    pub status: TransactionStatus,
    
    /// Transaction type
    pub transaction_type: TransactionType,
    
    /// Token mint address
    pub token_mint: Option<Pubkey>,
    
    /// Token symbol
    pub token_symbol: Option<String>,
    
    /// Transaction amount
    pub amount: Option<f64>,
    
    /// Transaction value in USD
    pub value_usd: Option<f64>,
    
    /// Transaction fee in SOL
    pub fee_sol: f64,
    
    /// Transaction fee in USD
    pub fee_usd: Option<f64>,
    
    /// Block time
    pub block_time: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Confirmation status
    pub confirmation_status: Option<String>,
    
    /// Recent blockhash
    pub recent_blockhash: Option<String>,
    
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction is pending
    Pending,
    
    /// Transaction is confirmed
    Confirmed,
    
    /// Transaction is finalized
    Finalized,
    
    /// Transaction failed
    Failed,
}

/// Transaction type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    /// Buy transaction
    Buy,
    
    /// Sell transaction
    Sell,
    
    /// Swap transaction
    Swap,
    
    /// Transfer transaction
    Transfer,
    
    /// Deposit transaction
    Deposit,
    
    /// Withdraw transaction
    Withdraw,
    
    /// Stake transaction
    Stake,
    
    /// Unstake transaction
    Unstake,
    
    /// Claim transaction
    Claim,
    
    /// Other transaction
    Other,
}

/// Bot status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BotStatus {
    /// Bot is initializing
    Initializing,
    
    /// Bot is running
    Running,
    
    /// Bot is paused
    Paused,
    
    /// Bot is stopped
    Stopped,
    
    /// Bot is in error state
    Error,
}

/// Module status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleStatus {
    /// Module is initializing
    Initializing,
    
    /// Module is running
    Running,
    
    /// Module is paused
    Paused,
    
    /// Module is stopped
    Stopped,
    
    /// Module is in error state
    Error,
}

/// Performance snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// CPU usage percentage
    pub cpu_usage_pct: f64,
    
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    
    /// Network usage in bytes per second
    pub network_usage_bps: u64,
    
    /// Disk usage in bytes
    pub disk_usage_bytes: u64,
    
    /// Transaction throughput (transactions per second)
    pub tx_throughput_tps: f64,
    
    /// Average transaction latency
    pub avg_tx_latency: Duration,
    
    /// 95th percentile transaction latency
    pub p95_tx_latency: Duration,
    
    /// 99th percentile transaction latency
    pub p99_tx_latency: Duration,
    
    /// Error rate (errors per second)
    pub error_rate_eps: f64,
    
    /// Success rate percentage
    pub success_rate_pct: f64,
    
    /// Additional metrics
    pub additional_metrics: HashMap<String, f64>,
}

/// Risk limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskLimits {
    /// Maximum position size in USD
    pub max_position_size_usd: f64,
    
    /// Maximum total exposure in USD
    pub max_total_exposure_usd: f64,
    
    /// Maximum drawdown percentage
    pub max_drawdown_pct: f64,
    
    /// Maximum daily loss in USD
    pub max_daily_loss_usd: f64,
    
    /// Maximum transaction size in USD
    pub max_transaction_size_usd: f64,
    
    /// Maximum number of concurrent positions
    pub max_concurrent_positions: usize,
    
    /// Maximum number of daily transactions
    pub max_daily_transactions: usize,
    
    /// Maximum price impact percentage
    pub max_price_impact_pct: f64,
    
    /// Minimum liquidity in USD
    pub min_liquidity_usd: f64,
    
    /// Maximum leverage
    pub max_leverage: f64,
    
    /// Additional limits
    pub additional_limits: HashMap<String, f64>,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_position_size_usd: 1000.0,
            max_total_exposure_usd: 10000.0,
            max_drawdown_pct: 10.0,
            max_daily_loss_usd: 1000.0,
            max_transaction_size_usd: 500.0,
            max_concurrent_positions: 10,
            max_daily_transactions: 100,
            max_price_impact_pct: 1.0,
            min_liquidity_usd: 10000.0,
            max_leverage: 1.0,
            additional_limits: HashMap::new(),
        }
    }
}

/// Bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    /// Bot name
    pub name: String,
    
    /// Bot version
    pub version: String,
    
    /// Environment (mainnet, devnet, testnet, localnet)
    pub environment: String,
    
    /// RPC URL
    pub rpc_url: String,
    
    /// WebSocket URL
    pub ws_url: Option<String>,
    
    /// Commitment level
    pub commitment: String,
    
    /// Whether to use Jito bundles
    pub use_jito_bundles: bool,
    
    /// Jito bundle URL
    pub jito_bundle_url: String,
    
    /// Log level
    pub log_level: String,
    
    /// Auto start
    pub auto_start: bool,
    
    /// Risk limits
    pub risk_limits: RiskLimits,
    
    /// Strategy configurations
    pub strategies: HashMap<String, serde_json::Value>,
    
    /// Module configurations
    pub modules: HashMap<String, serde_json::Value>,
    
    /// Additional settings
    pub additional_settings: HashMap<String, serde_json::Value>,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            name: "Solana HFT Bot".to_string(),
            version: "0.1.0".to_string(),
            environment: "mainnet".to_string(),
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            ws_url: Some("wss://api.mainnet-beta.solana.com".to_string()),
            commitment: "confirmed".to_string(),
            use_jito_bundles: false,
            jito_bundle_url: "https://mainnet.block-engine.jito.io".to_string(),
            log_level: "info".to_string(),
            auto_start: false,
            risk_limits: RiskLimits::default(),
            strategies: HashMap::new(),
            modules: HashMap::new(),
            additional_settings: HashMap::new(),
        }
    }
}