//! Risk metrics for the Solana HFT Bot
//!
//! This module provides risk metrics collection and analysis.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::Position;

/// Risk metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskMetricsSnapshot {
    /// Current capital in USD
    pub capital_usd: f64,
    
    /// Total exposure in USD
    pub exposure_usd: f64,
    
    /// Exposure as percentage of capital
    pub exposure_percentage: f64,
    
    /// Current drawdown
    pub drawdown: f64,
    
    /// Maximum drawdown
    pub max_drawdown: f64,
    
    /// Daily profit/loss in USD
    pub daily_pnl_usd: f64,
    
    /// Weekly profit/loss in USD
    pub weekly_pnl_usd: f64,
    
    /// Monthly profit/loss in USD
    pub monthly_pnl_usd: f64,
    
    /// Profit decline percentage
    pub profit_decline: f64,
    
    /// Market volatility
    pub volatility: f64,
    
    /// Sharpe ratio
    pub sharpe_ratio: f64,
    
    /// Sortino ratio
    pub sortino_ratio: f64,
    
    /// Calmar ratio
    pub calmar_ratio: f64,
    
    /// Number of consecutive profitable trades
    pub consecutive_profits: usize,
    
    /// Number of consecutive losing trades
    pub consecutive_losses: usize,
    
    /// Win rate (percentage)
    pub win_rate: f64,
    
    /// Average profit per winning trade
    pub avg_profit: f64,
    
    /// Average loss per losing trade
    pub avg_loss: f64,
    
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
    
    /// Expected value per trade
    pub expected_value: f64,
    
    /// Number of open positions
    pub open_positions: usize,
    
    /// Largest position as percentage of capital
    pub largest_position_pct: f64,
    
    /// Asset with largest position
    pub largest_position_asset: String,
    
    /// Timestamp of the snapshot
    pub timestamp: DateTime<Utc>,
}

/// Profit and loss record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnLRecord {
    /// Asset symbol
    pub symbol: String,
    
    /// Profit/loss amount in USD
    pub pnl_usd: f64,
    
    /// Trade size in USD
    pub size_usd: f64,
    
    /// Entry price
    pub entry_price: f64,
    
    /// Exit price
    pub exit_price: f64,
    
    /// Trade duration
    pub duration_ms: u64,
    
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Trade ID
    pub trade_id: String,
    
    /// Strategy ID
    pub strategy_id: String,
}

/// Return data for volatility calculation
#[derive(Debug, Clone)]
struct ReturnData {
    /// Asset symbol
    symbol: String,
    
    /// Return percentage
    return_pct: f64,
    
    /// Timestamp
    timestamp: DateTime<Utc>,
}

/// Risk metrics collector
pub struct RiskMetrics {
    /// Current capital in USD
    capital_usd: RwLock<f64>,
    
    /// Total exposure in USD
    exposure_usd: RwLock<f64>,
    
    /// Current drawdown
    drawdown: RwLock<f64>,
    
    /// Maximum drawdown
    max_drawdown: RwLock<f64>,
    
    /// Peak capital
    peak_capital: RwLock<f64>,
    
    /// Daily profit/loss in USD
    daily_pnl_usd: RwLock<f64>,
    
    /// Weekly profit/loss in USD
    weekly_pnl_usd: RwLock<f64>,
    
    /// Monthly profit/loss in USD
    monthly_pnl_usd: RwLock<f64>,
    
    /// Profit/loss history
    pnl_history: RwLock<VecDeque<PnLRecord>>,
    
    /// Return history for volatility calculation
    return_history: RwLock<VecDeque<ReturnData>>,
    
    /// Consecutive profitable trades
    consecutive_profits: AtomicU64,
    
    /// Consecutive losing trades
    consecutive_losses: AtomicU64,
    
    /// Total number of trades
    total_trades: AtomicU64,
    
    /// Number of winning trades
    winning_trades: AtomicU64,
    
    /// Gross profit
    gross_profit: RwLock<f64>,
    
    /// Gross loss
    gross_loss: RwLock<f64>,
    
    /// Maximum history size
    max_history_size: usize,
    
    /// Last snapshot time
    last_snapshot: RwLock<DateTime<Utc>>,
    
    /// Last daily reset time
    last_daily_reset: RwLock<DateTime<Utc>>,
    
    /// Last weekly reset time
    last_weekly_reset: RwLock<DateTime<Utc>>,
    
    /// Last monthly reset time
    last_monthly_reset: RwLock<DateTime<Utc>>,
}

impl RiskMetrics {
    /// Create a new risk metrics collector
    pub fn new(max_history_size: usize) -> Self {
        let now = Utc::now();
        
        Self {
            capital_usd: RwLock::new(0.0),
            exposure_usd: RwLock::new(0.0),
            drawdown: RwLock::new(0.0),
            max_drawdown: RwLock::new(0.0),
            peak_capital: RwLock::new(0.0),
            daily_pnl_usd: RwLock::new(0.0),
            weekly_pnl_usd: RwLock::new(0.0),
            monthly_pnl_usd: RwLock::new(0.0),
            pnl_history: RwLock::new(VecDeque::with_capacity(max_history_size)),
            return_history: RwLock::new(VecDeque::with_capacity(max_history_size)),
            consecutive_profits: AtomicU64::new(0),
            consecutive_losses: AtomicU64::new(0),
            total_trades: AtomicU64::new(0),
            winning_trades: AtomicU64::new(0),
            gross_profit: RwLock::new(0.0),
            gross_loss: RwLock::new(0.0),
            max_history_size,
            last_snapshot: RwLock::new(now),
            last_daily_reset: RwLock::new(now),
            last_weekly_reset: RwLock::new(now),
            last_monthly_reset: RwLock::new(now),
        }
    }
    
    /// Update capital
    pub fn update_capital(&self, capital_usd: f64) {
        let mut capital = self.capital_usd.write();
        let mut peak_capital = self.peak_capital.write();
        let mut drawdown = self.drawdown.write();
        let mut max_drawdown = self.max_drawdown.write();
        
        *capital = capital_usd;
        
        // Update peak capital
        if capital_usd > *peak_capital {
            *peak_capital = capital_usd;
        }
        
        // Calculate drawdown
        if *peak_capital > 0.0 {
            *drawdown = 1.0 - (capital_usd / *peak_capital);
            
            // Update max drawdown
            if *drawdown > *max_drawdown {
                *max_drawdown = *drawdown;
            }
        }
    }
    
    /// Update exposure
    pub fn update_exposure(&self, positions: &[Position]) {
        let mut exposure = self.exposure_usd.write();
        
        // Calculate total exposure
        let total_exposure = positions.iter()
            .map(|p| p.size.abs() * p.market_price)
            .sum();
        
        *exposure = total_exposure;
    }
    
    /// Record a completed trade
    pub fn record_trade(&self, record: PnLRecord) {
        let mut pnl_history = self.pnl_history.write();
        let mut daily_pnl = self.daily_pnl_usd.write();
        let mut weekly_pnl = self.weekly_pnl_usd.write();
        let mut monthly_pnl = self.monthly_pnl_usd.write();
        
        // Update PnL
        *daily_pnl += record.pnl_usd;
        *weekly_pnl += record.pnl_usd;
        *monthly_pnl += record.pnl_usd;
        
        // Update trade statistics
        self.total_trades.fetch_add(1, Ordering::SeqCst);
        
        if record.pnl_usd > 0.0 {
            self.winning_trades.fetch_add(1, Ordering::SeqCst);
            self.consecutive_profits.fetch_add(1, Ordering::SeqCst);
            self.consecutive_losses.store(0, Ordering::SeqCst);
            
            let mut gross_profit = self.gross_profit.write();
            *gross_profit += record.pnl_usd;
        } else if record.pnl_usd < 0.0 {
            self.consecutive_losses.fetch_add(1, Ordering::SeqCst);
            self.consecutive_profits.store(0, Ordering::SeqCst);
            
            let mut gross_loss = self.gross_loss.write();
            *gross_loss += record.pnl_usd.abs();
        }
        
        // Add to history
        pnl_history.push_back(record.clone());
        
        // Keep maximum size
        while pnl_history.len() > self.max_history_size {
            pnl_history.pop_front();
        }
        
        // Record return data for volatility calculation
        if record.entry_price > 0.0 && record.exit_price > 0.0 {
            let return_pct = (record.exit_price - record.entry_price) / record.entry_price;
            
            let return_data = ReturnData {
                symbol: record.symbol.clone(),
                return_pct,
                timestamp: record.timestamp,
            };
            
            let mut return_history = self.return_history.write();
            return_history.push_back(return_data);
            
            // Keep maximum size
            while return_history.len() > self.max_history_size {
                return_history.pop_front();
            }
        }
    }
    
    /// Check and reset periodic metrics if needed
    pub fn check_periodic_reset(&self) {
        let now = Utc::now();
        
        // Check daily reset
        {
            let mut last_daily_reset = self.last_daily_reset.write();
            if now.date_naive() != last_daily_reset.date_naive() {
                // Reset daily PnL
                *self.daily_pnl_usd.write() = 0.0;
                *last_daily_reset = now;
                
                debug!("Daily PnL reset");
            }
        }
        
        // Check weekly reset
        {
            let mut last_weekly_reset = self.last_weekly_reset.write();
            let days_since_reset = (now - *last_weekly_reset).num_days();
            
            if days_since_reset >= 7 {
                // Reset weekly PnL
                *self.weekly_pnl_usd.write() = 0.0;
                *last_weekly_reset = now;
                
                debug!("Weekly PnL reset");
            }
        }
        
        // Check monthly reset
        {
            let mut last_monthly_reset = self.last_monthly_reset.write();
            if now.month() != last_monthly_reset.month() || now.year() != last_monthly_reset.year() {
                // Reset monthly PnL
                *self.monthly_pnl_usd.write() = 0.0;
                *last_monthly_reset = now;
                
                debug!("Monthly PnL reset");
            }
        }
    }
    
    /// Calculate volatility from return history
    fn calculate_volatility(&self) -> f64 {
        let return_history = self.return_history.read();
        
        if return_history.is_empty() {
            return 0.0;
        }
        
        // Get returns
        let returns: Vec<f64> = return_history.iter()
            .map(|r| r.return_pct)
            .collect();
        
        // Calculate standard deviation
        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns.iter()
            .map(|&r| (r - mean).powi(2))
            .sum::<f64>() / returns.len() as f64;
        
        variance.sqrt()
    }
    
    /// Calculate profit decline
    fn calculate_profit_decline(&self) -> f64 {
        let pnl_history = self.pnl_history.read();
        
        if pnl_history.len() < 10 {
            return 0.0;
        }
        
        // Calculate average PnL for first and second half of recent trades
        let half_size = pnl_history.len() / 2;
        let recent_trades: Vec<&PnLRecord> = pnl_history.iter().rev().take(half_size * 2).collect();
        
        let first_half: Vec<&PnLRecord> = recent_trades.iter().take(half_size).cloned().collect();
        let second_half: Vec<&PnLRecord> = recent_trades.iter().skip(half_size).take(half_size).cloned().collect();
        
        let first_half_avg = first_half.iter().map(|r| r.pnl_usd).sum::<f64>() / first_half.len() as f64;
        let second_half_avg = second_half.iter().map(|r| r.pnl_usd).sum::<f64>() / second_half.len() as f64;
        
        if second_half_avg <= 0.0 {
            return 1.0; // Maximum decline if second half average is negative or zero
        }
        
        if first_half_avg < second_half_avg {
            return (second_half_avg - first_half_avg) / second_half_avg;
        }
        
        0.0 // No decline
    }
    
    /// Calculate Sharpe ratio
    fn calculate_sharpe_ratio(&self) -> f64 {
        let return_history = self.return_history.read();
        
        if return_history.is_empty() {
            return 0.0;
        }
        
        // Get returns
        let returns: Vec<f64> = return_history.iter()
            .map(|r| r.return_pct)
            .collect();
        
        // Calculate mean return
        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        
        // Calculate standard deviation
        let std_dev = {
            let variance = returns.iter()
                .map(|&r| (r - mean_return).powi(2))
                .sum::<f64>() / returns.len() as f64;
            
            variance.sqrt()
        };
        
        if std_dev == 0.0 {
            return 0.0;
        }
        
        // Assume risk-free rate of 0 for simplicity
        // Annualize (assuming daily returns)
        mean_return * (252.0_f64).sqrt() / std_dev
    }
    
    /// Calculate Sortino ratio
    fn calculate_sortino_ratio(&self) -> f64 {
        let return_history = self.return_history.read();
        
        if return_history.is_empty() {
            return 0.0;
        }
        
        // Get returns
        let returns: Vec<f64> = return_history.iter()
            .map(|r| r.return_pct)
            .collect();
        
        // Calculate mean return
        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        
        // Calculate downside deviation (only negative returns)
        let downside_returns: Vec<f64> = returns.iter()
            .filter(|&&r| r < 0.0)
            .cloned()
            .collect();
        
        if downside_returns.is_empty() {
            return if mean_return > 0.0 { 100.0 } else { 0.0 }; // Perfect Sortino if no negative returns
        }
        
        let downside_deviation = {
            let sum_squared = downside_returns.iter()
                .map(|&r| r.powi(2))
                .sum::<f64>();
            
            (sum_squared / downside_returns.len() as f64).sqrt()
        };
        
        if downside_deviation == 0.0 {
            return 0.0;
        }
        
        // Annualize (assuming daily returns)
        mean_return * (252.0_f64).sqrt() / downside_deviation
    }
    
    /// Calculate Calmar ratio
    fn calculate_calmar_ratio(&self) -> f64 {
        let pnl_history = self.pnl_history.read();
        let max_drawdown = *self.max_drawdown.read();
        
        if pnl_history.is_empty() || max_drawdown == 0.0 {
            return 0.0;
        }
        
        // Calculate annualized return
        // For simplicity, use the average daily return and annualize
        let returns: Vec<f64> = self.return_history.read().iter()
            .map(|r| r.return_pct)
            .collect();
        
        if returns.is_empty() {
            return 0.0;
        }
        
        let avg_daily_return = returns.iter().sum::<f64>() / returns.len() as f64;
        let annualized_return = (1.0 + avg_daily_return).powf(252.0) - 1.0;
        
        annualized_return / max_drawdown
    }
    
    /// Create a snapshot of current metrics
    pub fn create_snapshot(&self, positions: &[Position]) -> RiskMetricsSnapshot {
        let now = Utc::now();
        *self.last_snapshot.write() = now;
        
        // Check if we need to reset periodic metrics
        self.check_periodic_reset();
        
        // Calculate metrics
        let capital_usd = *self.capital_usd.read();
        let exposure_usd = *self.exposure_usd.read();
        let exposure_percentage = if capital_usd > 0.0 {
            exposure_usd / capital_usd
        } else {
            0.0
        };
        
        let volatility = self.calculate_volatility();
        let profit_decline = self.calculate_profit_decline();
        let sharpe_ratio = self.calculate_sharpe_ratio();
        let sortino_ratio = self.calculate_sortino_ratio();
        let calmar_ratio = self.calculate_calmar_ratio();
        
        // Calculate win rate
        let total_trades = self.total_trades.load(Ordering::SeqCst);
        let winning_trades = self.winning_trades.load(Ordering::SeqCst);
        let win_rate = if total_trades > 0 {
            winning_trades as f64 / total_trades as f64 * 100.0
        } else {
            0.0
        };
        
        // Calculate average profit/loss
        let pnl_history = self.pnl_history.read();
        let (winning_trades_vec, losing_trades_vec): (Vec<&PnLRecord>, Vec<&PnLRecord>) = 
            pnl_history.iter().partition(|r| r.pnl_usd > 0.0);
        
        let avg_profit = if !winning_trades_vec.is_empty() {
            winning_trades_vec.iter().map(|r| r.pnl_usd).sum::<f64>() / winning_trades_vec.len() as f64
        } else {
            0.0
        };
        
        let avg_loss = if !losing_trades_vec.is_empty() {
            losing_trades_vec.iter().map(|r| r.pnl_usd.abs()).sum::<f64>() / losing_trades_vec.len() as f64
        } else {
            0.0
        };
        
        // Calculate profit factor
        let gross_profit = *self.gross_profit.read();
        let gross_loss = *self.gross_loss.read();
        let profit_factor = if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else {
            if gross_profit > 0.0 { 100.0 } else { 1.0 }
        };
        
        // Calculate expected value
        let expected_value = (win_rate / 100.0) * avg_profit - (1.0 - win_rate / 100.0) * avg_loss;
        
        // Find largest position
        let (largest_position_pct, largest_position_asset) = if !positions.is_empty() && capital_usd > 0.0 {
            let largest = positions.iter()
                .max_by(|a, b| {
                    let a_value = a.size.abs() * a.market_price;
                    let b_value = b.size.abs() * b.market_price;
                    a_value.partial_cmp(&b_value).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();
            
            let largest_value = largest.size.abs() * largest.market_price;
            (largest_value / capital_usd, largest.symbol.clone())
        } else {
            (0.0, "".to_string())
        };
        
        RiskMetricsSnapshot {
            capital_usd,
            exposure_usd,
            exposure_percentage,
            drawdown: *self.drawdown.read(),
            max_drawdown: *self.max_drawdown.read(),
            daily_pnl_usd: *self.daily_pnl_usd.read(),
            weekly_pnl_usd: *self.weekly_pnl_usd.read(),
            monthly_pnl_usd: *self.monthly_pnl_usd.read(),
            profit_decline,
            volatility,
            sharpe_ratio,
            sortino_ratio,
            calmar_ratio,
            consecutive_profits: self.consecutive_profits.load(Ordering::SeqCst) as usize,
            consecutive_losses: self.consecutive_losses.load(Ordering::SeqCst) as usize,
            win_rate,
            avg_profit,
            avg_loss,
            profit_factor,
            expected_value,
            open_positions: positions.len(),
            largest_position_pct,
            largest_position_asset,
            timestamp: now,
        }
    }
    
    /// Get PnL history
    pub fn get_pnl_history(&self) -> Vec<PnLRecord> {
        self.pnl_history.read().iter().cloned().collect()
    }
    
    /// Get recent PnL history (last n records)
    pub fn get_recent_pnl_history(&self, n: usize) -> Vec<PnLRecord> {
        let pnl_history = self.pnl_history.read();
        pnl_history.iter().rev().take(n).cloned().collect()
    }
    
    /// Get PnL history for a specific symbol
    pub fn get_symbol_pnl_history(&self, symbol: &str) -> Vec<PnLRecord> {
        let pnl_history = self.pnl_history.read();
        pnl_history.iter()
            .filter(|r| r.symbol == symbol)
            .cloned()
            .collect()
    }
    
    /// Get PnL history for a specific strategy
    pub fn get_strategy_pnl_history(&self, strategy_id: &str) -> Vec<PnLRecord> {
        let pnl_history = self.pnl_history.read();
        pnl_history.iter()
            .filter(|r| r.strategy_id == strategy_id)
            .cloned()
            .collect()
    }
    
    /// Reset all metrics
    pub fn reset(&self) {
        *self.capital_usd.write() = 0.0;
        *self.exposure_usd.write() = 0.0;
        *self.drawdown.write() = 0.0;
        *self.max_drawdown.write() = 0.0;
        *self.peak_capital.write() = 0.0;
        *self.daily_pnl_usd.write() = 0.0;
        *self.weekly_pnl_usd.write() = 0.0;
        *self.monthly_pnl_usd.write() = 0.0;
        self.pnl_history.write().clear();
        self.return_history.write().clear();
        self.consecutive_profits.store(0, Ordering::SeqCst);
        self.consecutive_losses.store(0, Ordering::SeqCst);
        self.total_trades.store(0, Ordering::SeqCst);
        self.winning_trades.store(0, Ordering::SeqCst);
        *self.gross_profit.write() = 0.0;
        *self.gross_loss.write() = 0.0;
        
        let now = Utc::now();
        *self.last_snapshot.write() = now;
        *self.last_daily_reset.write() = now;
        *self.last_weekly_reset.write() = now;
        *self.last_monthly_reset.write() = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_risk_metrics_basic() {
        let metrics = RiskMetrics::new(100);
        
        // Test capital update
        metrics.update_capital(10000.0);
        
        // Test exposure update
        let positions = vec![
            Position {
                symbol: "SOL".to_string(),
                size: 10.0,
                avg_price: 100.0,
                market_price: 110.0,
                unrealized_pnl: 100.0,
                realized_pnl: 0.0,
                value_usd: 1100.0,
                strategy_id: "strategy1".to_string(),
                entry_time: Some(chrono::Utc::now()),
                last_update: Some(chrono::Utc::now()),
            },
            Position {
                symbol: "BTC".to_string(),
                size: 0.1,
                avg_price: 50000.0,
                market_price: 52000.0,
                unrealized_pnl: 200.0,
                realized_pnl: 0.0,
                value_usd: 5200.0,
                strategy_id: "strategy1".to_string(),
                entry_time: Some(chrono::Utc::now()),
                last_update: Some(chrono::Utc::now()),
            },
        ];
        
        metrics.update_exposure(&positions);
        
        // Test trade recording
        let record = PnLRecord {
            symbol: "SOL".to_string(),
            pnl_usd: 100.0,
            size_usd: 1000.0,
            entry_price: 100.0,
            exit_price: 110.0,
            duration_ms: 1000,
            timestamp: Utc::now(),
            trade_id: "trade1".to_string(),
            strategy_id: "strategy1".to_string(),
        };
        
        metrics.record_trade(record);
        
        // Test snapshot creation
        let snapshot = metrics.create_snapshot(&positions);
        
        assert_eq!(snapshot.capital_usd, 10000.0);
        assert_eq!(snapshot.open_positions, 2);
        assert!(snapshot.daily_pnl_usd > 0.0);
        assert_eq!(snapshot.consecutive_profits, 1);
        assert_eq!(snapshot.consecutive_losses, 0);
    }
    
    #[test]
    fn test_drawdown_calculation() {
        let metrics = RiskMetrics::new(100);
        
        // Initial capital
        metrics.update_capital(10000.0);
        assert_eq!(*metrics.drawdown.read(), 0.0);
        
        // Increase capital (new peak)
        metrics.update_capital(12000.0);
        assert_eq!(*metrics.drawdown.read(), 0.0);
        assert_eq!(*metrics.peak_capital.read(), 12000.0);
        
        // Decrease capital (drawdown)
        metrics.update_capital(10800.0);
        assert_eq!(*metrics.drawdown.read(), 0.1); // 10% drawdown
        
        // Further decrease
        metrics.update_capital(9600.0);
        assert_eq!(*metrics.drawdown.read(), 0.2); // 20% drawdown
        assert_eq!(*metrics.max_drawdown.read(), 0.2);
        
        // Partial recovery
        metrics.update_capital(10800.0);
        assert_eq!(*metrics.drawdown.read(), 0.1); // 10% drawdown
        assert_eq!(*metrics.max_drawdown.read(), 0.2); // Max still 20%
        
        // New peak
        metrics.update_capital(13200.0);
        assert_eq!(*metrics.drawdown.read(), 0.0);
        assert_eq!(*metrics.peak_capital.read(), 13200.0);
    }
    
    #[test]
    fn test_pnl_history() {
        let metrics = RiskMetrics::new(5); // Small size for testing
        
        // Record some trades
        for i in 0..10 {
            let record = PnLRecord {
                symbol: "SOL".to_string(),
                pnl_usd: if i % 2 == 0 { 100.0 } else { -50.0 },
                size_usd: 1000.0,
                entry_price: 100.0,
                exit_price: if i % 2 == 0 { 110.0 } else { 95.0 },
                duration_ms: 1000,
                timestamp: Utc::now(),
                trade_id: format!("trade{}", i),
                strategy_id: "strategy1".to_string(),
            };
            
            metrics.record_trade(record);
        }
        
        // Check history size is limited
        let history = metrics.get_pnl_history();
        assert_eq!(history.len(), 5);
        
        // Check most recent trades are kept
        let recent = metrics.get_recent_pnl_history(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].trade_id, "trade9");
        
        // Check win rate calculation
        let snapshot = metrics.create_snapshot(&[]);
        assert!(snapshot.win_rate > 0.0);
        assert!(snapshot.profit_factor > 0.0);
    }
}