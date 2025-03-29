//! Configuration for the risk management module
//!
//! This module provides configuration structures for the risk management system.

use std::time::Duration;
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

/// Risk configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    /// Key account for capital tracking
    pub key_account: Pubkey,
    
    /// Maximum drawdown percentage
    pub max_drawdown_pct: f64,
    
    /// Maximum exposure percentage
    pub max_exposure_pct: f64,
    
    /// Maximum token concentration
    pub max_token_concentration: f64,
    
    /// Maximum strategy allocation
    pub max_strategy_allocation: f64,
    
    /// Maximum risk score
    pub max_risk_score: u8,
    
    /// Maximum consecutive losses
    pub max_consecutive_losses: usize,
    
    /// Capital update interval in milliseconds
    pub capital_update_interval_ms: u64,
    
    /// Circuit breaker check interval in milliseconds
    pub circuit_breaker_check_interval_ms: u64,
    
    /// Risk report interval in milliseconds
    pub risk_report_interval_ms: u64,
    
    /// Maximum PnL history size
    pub max_pnl_history_size: usize,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            key_account: Pubkey::new_unique(), // Example value
            max_drawdown_pct: 10.0, // 10%
            max_exposure_pct: 0.8, // 80%
            max_token_concentration: 0.2, // 20%
            max_strategy_allocation: 0.3, // 30%
            max_risk_score: 70,
            max_consecutive_losses: 5,
            capital_update_interval_ms: 60_000, // 1 minute
            circuit_breaker_check_interval_ms: 30_000, // 30 seconds
            risk_report_interval_ms: 300_000, // 5 minutes
            max_pnl_history_size: 1000,
        }
    }
}