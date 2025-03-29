use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Statistics collector
pub struct StatisticsCollector {
    /// Trade statistics
    trade_stats: Arc<RwLock<TradeStatistics>>,
    
    /// Performance statistics
    performance_stats: Arc<RwLock<PerformanceStatistics>>,
    
    /// Token statistics
    token_stats: Arc<RwLock<HashMap<Pubkey, TokenStatistics>>>,
    
    /// Strategy statistics
    strategy_stats: Arc<RwLock<HashMap<String, StrategyStatistics>>>,
    
    /// Maximum history size
    max_history_size: usize,
}

impl StatisticsCollector {
    /// Create a new statistics collector
    pub fn new(max_history_size: usize) -> Self {
        Self {
            trade_stats: Arc::new(RwLock::new(TradeStatistics::default())),
            performance_stats: Arc::new(RwLock::new(PerformanceStatistics::default())),
            token_stats: Arc::new(RwLock::new(HashMap::new())),
            strategy_stats: Arc::new(RwLock::new(HashMap::new())),
            max_history_size,
        }
    }
    
    /// Record a trade
    pub async fn record_trade(&self, trade: &Trade) -> anyhow::Result<()> {
        // Update trade statistics
        {
            let mut trade_stats = self.trade_stats.write().await;
            trade_stats.record_trade(trade);
        }
        
        // Update token statistics
        {
            let mut token_stats = self.token_stats.write().await;
            let stats = token_stats.entry(trade.token_mint).or_default();
            stats.record_trade(trade);
        }
        
        // Update strategy statistics
        if let Some(strategy) = &trade.strategy {
            let mut strategy_stats = self.strategy_stats.write().await;
            let stats = strategy_stats.entry(strategy.clone()).or_default();
            stats.record_trade(trade);
        }
        
        Ok(())
    }
    
    /// Record a performance snapshot
    pub async fn record_performance_snapshot(&self, snapshot: &PerformanceSnapshot) -> anyhow::Result<()> {
        let mut performance_stats = self.performance_stats.write().await;
        performance_stats.record_snapshot(snapshot, self.max_history_size);
        
        Ok(())
    }
    
    /// Get trade statistics
    pub async fn get_trade_statistics(&self) -> anyhow::Result<TradeStatistics> {
        let trade_stats = self.trade_stats.read().await;
        Ok(trade_stats.clone())
    }
    
    /// Get performance statistics
    pub async fn get_performance_statistics(&self) -> anyhow::Result<PerformanceStatistics> {
        let performance_stats = self.performance_stats.read().await;
        Ok(performance_stats.clone())
    }
    
    /// Get token statistics
    pub async fn get_token_statistics(&self, token_mint: &Pubkey) -> anyhow::Result<Option<TokenStatistics>> {
        let token_stats = self.token_stats.read().await;
        Ok(token_stats.get(token_mint).cloned())
    }
    
    /// Get all token statistics
    pub async fn get_all_token_statistics(&self) -> anyhow::Result<HashMap<Pubkey, TokenStatistics>> {
        let token_stats = self.token_stats.read().await;
        Ok(token_stats.clone())
    }
    
    /// Get strategy statistics
    pub async fn get_strategy_statistics(&self, strategy: &str) -> anyhow::Result<Option<StrategyStatistics>> {
        let strategy_stats = self.strategy_stats.read().await;
        Ok(strategy_stats.get(strategy).cloned())
    }
    
    /// Get all strategy statistics
    pub async fn get_all_strategy_statistics(&self) -> anyhow::Result<HashMap<String, StrategyStatistics>> {
        let strategy_stats = self.strategy_stats.read().await;
        Ok(strategy_stats.clone())
    }
    
    /// Reset all statistics
    pub async fn reset_all(&self) -> anyhow::Result<()> {
        {
            let mut trade_stats = self.trade_stats.write().await;
            *trade_stats = TradeStatistics::default();
        }
        
        {
            let mut performance_stats = self.performance_stats.write().await;
            *performance_stats = PerformanceStatistics::default();
        }
        
        {
            let mut token_stats = self.token_stats.write().await;
            token_stats.clear();
        }
        
        {
            let mut strategy_stats = self.strategy_stats.write().await;
            strategy_stats.clear();
        }
        
        Ok(())
    }
}

/// Trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    /// Trade ID
    pub id: String,
    
    /// Token mint address
    pub token_mint: Pubkey,
    
    /// Token symbol
    pub token_symbol: Option<String>,
    
    /// Trade side
    pub side: TradeSide,
    
    /// Trade size
    pub size: f64,
    
    /// Trade price
    pub price: f64,
    
    /// Trade value in USD
    pub value_usd: f64,
    
    /// Trade fee in USD
    pub fee_usd: f64,
    
    /// Trade profit/loss in USD
    pub pnl_usd: Option<f64>,
    
    /// Trade profit/loss percentage
    pub pnl_pct: Option<f64>,
    
    /// Trade timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Trade strategy
    pub strategy: Option<String>,
    
    /// Trade market
    pub market: Option<String>,
    
    /// Trade execution time
    pub execution_time: Duration,
    
    /// Trade transaction signature
    pub signature: Option<String>,
    
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Trade side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeSide {
    /// Buy trade
    Buy,
    
    /// Sell trade
    Sell,
}

/// Trade statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeStatistics {
    /// Total number of trades
    pub total_trades: usize,
    
    /// Number of buy trades
    pub buy_trades: usize,
    
    /// Number of sell trades
    pub sell_trades: usize,
    
    /// Total volume in USD
    pub total_volume_usd: f64,
    
    /// Buy volume in USD
    pub buy_volume_usd: f64,
    
    /// Sell volume in USD
    pub sell_volume_usd: f64,
    
    /// Total fees in USD
    pub total_fees_usd: f64,
    
    /// Total profit/loss in USD
    pub total_pnl_usd: f64,
    
    /// Number of profitable trades
    pub profitable_trades: usize,
    
    /// Number of unprofitable trades
    pub unprofitable_trades: usize,
    
    /// Win rate percentage
    pub win_rate_pct: f64,
    
    /// Average profit per trade in USD
    pub avg_profit_per_trade_usd: f64,
    
    /// Average loss per trade in USD
    pub avg_loss_per_trade_usd: f64,
    
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
    
    /// Average trade size in USD
    pub avg_trade_size_usd: f64,
    
    /// Average execution time
    pub avg_execution_time: Duration,
    
    /// Recent trades
    pub recent_trades: VecDeque<Trade>,
    
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for TradeStatistics {
    fn default() -> Self {
        Self {
            total_trades: 0,
            buy_trades: 0,
            sell_trades: 0,
            total_volume_usd: 0.0,
            buy_volume_usd: 0.0,
            sell_volume_usd: 0.0,
            total_fees_usd: 0.0,
            total_pnl_usd: 0.0,
            profitable_trades: 0,
            unprofitable_trades: 0,
            win_rate_pct: 0.0,
            avg_profit_per_trade_usd: 0.0,
            avg_loss_per_trade_usd: 0.0,
            profit_factor: 0.0,
            avg_trade_size_usd: 0.0,
            avg_execution_time: Duration::from_secs(0),
            recent_trades: VecDeque::new(),
            last_updated: chrono::Utc::now(),
        }
    }
}

impl TradeStatistics {
    /// Record a trade
    pub fn record_trade(&mut self, trade: &Trade) {
        self.total_trades += 1;
        
        match trade.side {
            TradeSide::Buy => {
                self.buy_trades += 1;
                self.buy_volume_usd += trade.value_usd;
            }
            TradeSide::Sell => {
                self.sell_trades += 1;
                self.sell_volume_usd += trade.value_usd;
            }
        }
        
        self.total_volume_usd += trade.value_usd;
        self.total_fees_usd += trade.fee_usd;
        
        if let Some(pnl_usd) = trade.pnl_usd {
            self.total_pnl_usd += pnl_usd;
            
            if pnl_usd > 0.0 {
                self.profitable_trades += 1;
            } else if pnl_usd < 0.0 {
                self.unprofitable_trades += 1;
            }
        }
        
        // Update derived statistics
        if self.total_trades > 0 {
            self.avg_trade_size_usd = self.total_volume_usd / self.total_trades as f64;
            
            let total_execution_time = self.avg_execution_time.as_nanos() as u64 * (self.total_trades - 1) as u64 + trade.execution_time.as_nanos() as u64;
            self.avg_execution_time = Duration::from_nanos(total_execution_time / self.total_trades as u64);
        }
        
        if self.profitable_trades + self.unprofitable_trades > 0 {
            self.win_rate_pct = self.profitable_trades as f64 / (self.profitable_trades + self.unprofitable_trades) as f64 * 100.0;
        }
        
        // Calculate average profit and loss
        let mut total_profit = 0.0;
        let mut total_loss = 0.0;
        
        for trade in &self.recent_trades {
            if let Some(pnl_usd) = trade.pnl_usd {
                if pnl_usd > 0.0 {
                    total_profit += pnl_usd;
                } else if pnl_usd < 0.0 {
                    total_loss += pnl_usd.abs();
                }
            }
        }
        
        if let Some(pnl_usd) = trade.pnl_usd {
            if pnl_usd > 0.0 {
                total_profit += pnl_usd;
            } else if pnl_usd < 0.0 {
                total_loss += pnl_usd.abs();
            }
        }
        
        if self.profitable_trades > 0 {
            self.avg_profit_per_trade_usd = total_profit / self.profitable_trades as f64;
        }
        
        if self.unprofitable_trades > 0 {
            self.avg_loss_per_trade_usd = total_loss / self.unprofitable_trades as f64;
        }
        
        if total_loss > 0.0 {
            self.profit_factor = total_profit / total_loss;
        } else if total_profit > 0.0 {
            self.profit_factor = f64::INFINITY;
        } else {
            self.profit_factor = 0.0;
        }
        
        // Add to recent trades
        self.recent_trades.push_back(trade.clone());
        
        // Limit recent trades size
        while self.recent_trades.len() > 100 {
            self.recent_trades.pop_front();
        }
        
        self.last_updated = chrono::Utc::now();
    }
}

/// Token statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStatistics {
    /// Token mint address
    pub token_mint: Pubkey,
    
    /// Token symbol
    pub token_symbol: Option<String>,
    
    /// Total number of trades
    pub total_trades: usize,
    
    /// Number of buy trades
    pub buy_trades: usize,
    
    /// Number of sell trades
    pub sell_trades: usize,
    
    /// Total volume in USD
    pub total_volume_usd: f64,
    
    /// Buy volume in USD
    pub buy_volume_usd: f64,
    
    /// Sell volume in USD
    pub sell_volume_usd: f64,
    
    /// Total fees in USD
    pub total_fees_usd: f64,
    
    /// Total profit/loss in USD
    pub total_pnl_usd: f64,
    
    /// Number of profitable trades
    pub profitable_trades: usize,
    
    /// Number of unprofitable trades
    pub unprofitable_trades: usize,
    
    /// Win rate percentage
    pub win_rate_pct: f64,
    
    /// Average profit per trade in USD
    pub avg_profit_per_trade_usd: f64,
    
    /// Average loss per trade in USD
    pub avg_loss_per_trade_usd: f64,
    
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
    
    /// Average trade size in USD
    pub avg_trade_size_usd: f64,
    
    /// Average execution time
    pub avg_execution_time: Duration,
    
    /// Recent trades
    pub recent_trades: VecDeque<Trade>,
    
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for TokenStatistics {
    fn default() -> Self {
        Self {
            token_mint: Pubkey::default(),
            token_symbol: None,
            total_trades: 0,
            buy_trades: 0,
            sell_trades: 0,
            total_volume_usd: 0.0,
            buy_volume_usd: 0.0,
            sell_volume_usd: 0.0,
            total_fees_usd: 0.0,
            total_pnl_usd: 0.0,
            profitable_trades: 0,
            unprofitable_trades: 0,
            win_rate_pct: 0.0,
            avg_profit_per_trade_usd: 0.0,
            avg_loss_per_trade_usd: 0.0,
            profit_factor: 0.0,
            avg_trade_size_usd: 0.0,
            avg_execution_time: Duration::from_secs(0),
            recent_trades: VecDeque::new(),
            last_updated: chrono::Utc::now(),
        }
    }
}

impl TokenStatistics {
    /// Record a trade
    pub fn record_trade(&mut self, trade: &Trade) {
        self.token_mint = trade.token_mint;
        self.token_symbol = trade.token_symbol.clone();
        
        self.total_trades += 1;
        
        match trade.side {
            TradeSide::Buy => {
                self.buy_trades += 1;
                self.buy_volume_usd += trade.value_usd;
            }
            TradeSide::Sell => {
                self.sell_trades += 1;
                self.sell_volume_usd += trade.value_usd;
            }
        }
        
        self.total_volume_usd += trade.value_usd;
        self.total_fees_usd += trade.fee_usd;
        
        if let Some(pnl_usd) = trade.pnl_usd {
            self.total_pnl_usd += pnl_usd;
            
            if pnl_usd > 0.0 {
                self.profitable_trades += 1;
            } else if pnl_usd < 0.0 {
                self.unprofitable_trades += 1;
            }
        }
        
        // Update derived statistics
        if self.total_trades > 0 {
            self.avg_trade_size_usd = self.total_volume_usd / self.total_trades as f64;
            
            let total_execution_time = self.avg_execution_time.as_nanos() as u64 * (self.total_trades - 1) as u64 + trade.execution_time.as_nanos() as u64;
            self.avg_execution_time = Duration::from_nanos(total_execution_time / self.total_trades as u64);
        }
        
        if self.profitable_trades + self.unprofitable_trades > 0 {
            self.win_rate_pct = self.profitable_trades as f64 / (self.profitable_trades + self.unprofitable_trades) as f64 * 100.0;
        }
        
        // Calculate average profit and loss
        let mut total_profit = 0.0;
        let mut total_loss = 0.0;
        
        for trade in &self.recent_trades {
            if let Some(pnl_usd) = trade.pnl_usd {
                if pnl_usd > 0.0 {
                    total_profit += pnl_usd;
                } else if pnl_usd < 0.0 {
                    total_loss += pnl_usd.abs();
                }
            }
        }
        
        if let Some(pnl_usd) = trade.pnl_usd {
            if pnl_usd > 0.0 {
                total_profit += pnl_usd;
            } else if pnl_usd < 0.0 {
                total_loss += pnl_usd.abs();
            }
        }
        
        if self.profitable_trades > 0 {
            self.avg_profit_per_trade_usd = total_profit / self.profitable_trades as f64;
        }
        
        if self.unprofitable_trades > 0 {
            self.avg_loss_per_trade_usd = total_loss / self.unprofitable_trades as f64;
        }
        
        if total_loss > 0.0 {
            self.profit_factor = total_profit / total_loss;
        } else if total_profit > 0.0 {
            self.profit_factor = f64::INFINITY;
        } else {
            self.profit_factor = 0.0;
        }
        
        // Add to recent trades
        self.recent_trades.push_back(trade.clone());
        
        // Limit recent trades size
        while self.recent_trades.len() > 50 {
            self.recent_trades.pop_front();
        }
        
        self.last_updated = chrono::Utc::now();
    }
}

/// Strategy statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyStatistics {
    /// Strategy name
    pub strategy_name: String,
    
    /// Total number of trades
    pub total_trades: usize,
    
    /// Number of buy trades
    pub buy_trades: usize,
    
    /// Number of sell trades
    pub sell_trades: usize,
    
    /// Total volume in USD
    pub total_volume_usd: f64,
    
    /// Buy volume in USD
    pub buy_volume_usd: f64,
    
    /// Sell volume in USD
    pub sell_volume_usd: f64,
    
    /// Total fees in USD
    pub total_fees_usd: f64,
    
    /// Total profit/loss in USD
    pub total_pnl_usd: f64,
    
    /// Number of profitable trades
    pub profitable_trades: usize,
    
    /// Number of unprofitable trades
    pub unprofitable_trades: usize,
    
    /// Win rate percentage
    pub win_rate_pct: f64,
    
    /// Average profit per trade in USD
    pub avg_profit_per_trade_usd: f64,
    
    /// Average loss per trade in USD
    pub avg_loss_per_trade_usd: f64,
    
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
    
    /// Average trade size in USD
    pub avg_trade_size_usd: f64,
    
    /// Average execution time
    pub avg_execution_time: Duration,
    
    /// Recent trades
    pub recent_trades: VecDeque<Trade>,
    
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for StrategyStatistics {
    fn default() -> Self {
        Self {
            strategy_name: String::new(),
            total_trades: 0,
            buy_trades: 0,
            sell_trades: 0,
            total_volume_usd: 0.0,
            buy_volume_usd: 0.0,
            sell_volume_usd: 0.0,
            total_fees_usd: 0.0,
            total_pnl_usd: 0.0,
            profitable_trades: 0,
            unprofitable_trades: 0,
            win_rate_pct: 0.0,
            avg_profit_per_trade_usd: 0.0,
            avg_loss_per_trade_usd: 0.0,
            profit_factor: 0.0,
            avg_trade_size_usd: 0.0,
            avg_execution_time: Duration::from_secs(0),
            recent_trades: VecDeque::new(),
            last_updated: chrono::Utc::now(),
        }
    }
}

impl StrategyStatistics {
    /// Record a trade
    pub fn record_trade(&mut self, trade: &Trade) {
        if let Some(strategy) = &trade.strategy {
            self.strategy_name = strategy.clone();
        }
        
        self.total_trades += 1;
        
        match trade.side {
            TradeSide::Buy => {
                self.buy_trades += 1;
                self.buy_volume_usd += trade.value_usd;
            }
            TradeSide::Sell => {
                self.sell_trades += 1;
                self.sell_volume_usd += trade.value_usd;
            }
        }
        
        self.total_volume_usd += trade.value_usd;
        self.total_fees_usd += trade.fee_usd;
        
        if let Some(pnl_usd) = trade.pnl_usd {
            self.total_pnl_usd += pnl_usd;
            
            if pnl_usd > 0.0 {
                self.profitable_trades += 1;
            } else if pnl_usd < 0.0 {
                self.unprofitable_trades += 1;
            }
        }
        
        // Update derived statistics
        if self.total_trades > 0 {
            self.avg_trade_size_usd = self.total_volume_usd / self.total_trades as f64;
            
            let total_execution_time = self.avg_execution_time.as_nanos() as u64 * (self.total_trades - 1) as u64 + trade.execution_time.as_nanos() as u64;
            self.avg_execution_time = Duration::from_nanos(total_execution_time / self.total_trades as u64);
        }
        
        if self.profitable_trades + self.unprofitable_trades > 0 {
            self.win_rate_pct = self.profitable_trades as f64 / (self.profitable_trades + self.unprofitable_trades) as f64 * 100.0;
        }
        
        // Calculate average profit and loss
        let mut total_profit = 0.0;
        let mut total_loss = 0.0;
        
        for trade in &self.recent_trades {
            if let Some(pnl_usd) = trade.pnl_usd {
                if pnl_usd > 0.0 {
                    total_profit += pnl_usd;
                } else if pnl_usd < 0.0 {
                    total_loss += pnl_usd.abs();
                }
            }
        }
        
        if let Some(pnl_usd) = trade.pnl_usd {
            if pnl_usd > 0.0 {
                total_profit += pnl_usd;
            } else if pnl_usd < 0.0 {
                total_loss += pnl_usd.abs();
            }
        }
        
        if self.profitable_trades > 0 {
            self.avg_profit_per_trade_usd = total_profit / self.profitable_trades as f64;
        }
        
        if self.unprofitable_trades > 0 {
            self.avg_loss_per_trade_usd = total_loss / self.unprofitable_trades as f64;
        }
        
        if total_loss > 0.0 {
            self.profit_factor = total_profit / total_loss;
        } else if total_profit > 0.0 {
            self.profit_factor = f64::INFINITY;
        } else {
            self.profit_factor = 0.0;
        }
        
        // Add to recent trades
        self.recent_trades.push_back(trade.clone());
        
        // Limit recent trades size
        while self.recent_trades.len() > 50 {
            self.recent_trades.pop_front();
        }
        
        self.last_updated = chrono::Utc::now();
    }
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

/// Performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStatistics {
    /// Performance snapshots
    pub snapshots: VecDeque<PerformanceSnapshot>,
    
    /// Average CPU usage percentage
    pub avg_cpu_usage_pct: f64,
    
    /// Average memory usage in bytes
    pub avg_memory_usage_bytes: u64,
    
    /// Average network usage in bytes per second
    pub avg_network_usage_bps: u64,
    
    /// Average disk usage in bytes
    pub avg_disk_usage_bytes: u64,
    
    /// Average transaction throughput (transactions per second)
    pub avg_tx_throughput_tps: f64,
    
    /// Average transaction latency
    pub avg_tx_latency: Duration,
    
    /// Average 95th percentile transaction latency
    pub avg_p95_tx_latency: Duration,
    
    /// Average 99th percentile transaction latency
    pub avg_p99_tx_latency: Duration,
    
    /// Average error rate (errors per second)
    pub avg_error_rate_eps: f64,
    
    /// Average success rate percentage
    pub avg_success_rate_pct: f64,
    
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for PerformanceStatistics {
    fn default() -> Self {
        Self {
            snapshots: VecDeque::new(),
            avg_cpu_usage_pct: 0.0,
            avg_memory_usage_bytes: 0,
            avg_network_usage_bps: 0,
            avg_disk_usage_bytes: 0,
            avg_tx_throughput_tps: 0.0,
            avg_tx_latency: Duration::from_secs(0),
            avg_p95_tx_latency: Duration::from_secs(0),
            avg_p99_tx_latency: Duration::from_secs(0),
            avg_error_rate_eps: 0.0,
            avg_success_rate_pct: 0.0,
            last_updated: chrono::Utc::now(),
        }
    }
}

impl PerformanceStatistics {
    /// Record a performance snapshot
    pub fn record_snapshot(&mut self, snapshot: &PerformanceSnapshot, max_history_size: usize) {
        // Add to snapshots
        self.snapshots.push_back(snapshot.clone());
        
        // Limit snapshots size
        while self.snapshots.len() > max_history_size {
            self.snapshots.pop_front();
        }
        
        // Update averages
        if !self.snapshots.is_empty() {
            let count = self.snapshots.len() as f64;
            
            let mut total_cpu_usage = 0.0;
            let mut total_memory_usage = 0;
            let mut total_network_usage = 0;
            let mut total_disk_usage = 0;
            let mut total_tx_throughput = 0.0;
            let mut total_tx_latency = 0;
            let mut total_p95_tx_latency = 0;
            let mut total_p99_tx_latency = 0;
            let mut total_error_rate = 0.0;
            let mut total_success_rate = 0.0;
            
            for snapshot in &self.snapshots {
                total_cpu_usage += snapshot.cpu_usage_pct;
                total_memory_usage += snapshot.memory_usage_bytes;
                total_network_usage += snapshot.network_usage_bps;
                total_disk_usage += snapshot.disk_usage_bytes;
                total_tx_throughput += snapshot.tx_throughput_tps;
                total_tx_latency += snapshot.avg_tx_latency.as_nanos() as u64;
                total_p95_tx_latency += snapshot.p95_tx_latency.as_nanos() as u64;
                total_p99_tx_latency += snapshot.p99_tx_latency.as_nanos() as u64;
                total_error_rate += snapshot.error_rate_eps;
                total_success_rate += snapshot.success_rate_pct;
            }
            
            self.avg_cpu_usage_pct = total_cpu_usage / count;
            self.avg_memory_usage_bytes = (total_memory_usage as f64 / count) as u64;
            self.avg_network_usage_bps = (total_network_usage as f64 / count) as u64;
            self.avg_disk_usage_bytes = (total_disk_usage as f64 / count) as u64;
            self.avg_tx_throughput_tps = total_tx_throughput / count;
            self.avg_tx_latency = Duration::from_nanos((total_tx_latency as f64 / count) as u64);
            self.avg_p95_tx_latency = Duration::from_nanos((total_p95_tx_latency as f64 / count) as u64);
            self.avg_p99_tx_latency = Duration::from_nanos((total_p99_tx_latency as f64 / count) as u64);
            self.avg_error_rate_eps = total_error_rate / count;
            self.avg_success_rate_pct = total_success_rate / count;
        }
        
        self.last_updated = chrono::Utc::now();
    }
}