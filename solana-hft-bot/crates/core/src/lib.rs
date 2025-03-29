                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        //! Core module for the Solana HFT Bot
//!
//! This module serves as the central command center, coordinating all other modules:
//! - Strategy coordination
//! - Module lifecycle management
//! - Configuration and settings
//! - Statistics and performance reporting
//! - Signal processing and execution

#![allow(unused_imports)]
#![feature(async_fn_in_trait)]

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use futures::{future::Either, stream::{FuturesUnordered, StreamExt}, SinkExt};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signature},
    transaction::Transaction,
};
use tokio::sync::{mpsc, oneshot, RwLock as TokioRwLock, Semaphore};
use tokio::time::{interval, sleep, timeout};
use tracing::{debug, error, info, instrument, trace, warn};

mod bot;
mod config;
mod metrics;
mod performance;
mod signals;
mod stats;
mod status;
mod strategies;
mod types;

pub use bot::HftBot;
pub use config::{BotConfig, ModuleConfig, DefaultConfigs};
pub use metrics::CoreMetrics;
pub use performance::{PerformanceMonitor, PerformanceSnapshot};
pub use signals::{
    Signal, SignalType, SignalStrength, SignalSource, SignalProcessor, 
    SignalHandler, LoggingSignalHandler, TradingSignalHandler, SignalAggregator
};
pub use stats::{
    StatisticsCollector, Trade, TradeSide, TradeStatistics, 
    TokenStatistics, StrategyStatistics, PerformanceStatistics
};
pub use status::{BotStatus, ModuleStatus};
pub use strategies::{
    Strategy, StrategyManager, StrategyConfig, StrategyType, 
    StrategyPerformance, PositionSizingMethod, TokenSniperStrategy, 
    ArbitrageStrategy, MevStrategy, StrategyOptimizer
};
pub use types::{
    MarketData, TokenData, OrderData, OrderSide, OrderType, OrderStatus,
    PositionData, PositionStatus, PortfolioData, TransactionData,
    TransactionStatus, TransactionType, BotStatus as BotStatusType,
    ModuleStatus as ModuleStatusType, PerformanceSnapshot as PerfSnapshot,
    RiskLimits, BotConfig as BotConfiguration
};

/// Error types for the core module
#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Module error: {0}")]
    Module(String),
    
    #[error("Strategy error: {0}")]
    Strategy(String),
    
    #[error("Signal error: {0}")]
    Signal(String),
    
    #[error("Execution error: {0}")]
    Execution(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("RPC error: {0}")]
    Rpc(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Timeout error")]
    Timeout,
    
    #[error("Not initialized")]
    NotInitialized,
    
    #[error("Already initialized")]
    AlreadyInitialized,
    
    #[error("Not running")]
    NotRunning,
    
    #[error("Already running")]
    AlreadyRunning,
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for the core module
pub type CoreResult<T> = std::result::Result<T, CoreError>;

/// Initialize the core module
pub fn init() {
    info!("Initializing core module");
}

/// Initialize logging for the application
pub fn init_logging() {
    use tracing_subscriber::{fmt, EnvFilter};
    
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();
    
    info!("Logging initialized");
}

/// Module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_trading_signal() {
        let signal = Signal {
            token_mint: Pubkey::new_unique(),
            signal_type: SignalType::Buy,
            strength: SignalStrength::Strong,
            price: Some(100.0),
            timestamp: chrono::Utc::now(),
            source: SignalSource::Strategy("test-strategy".to_string()),
            expiration: Some(chrono::Utc::now() + chrono::Duration::minutes(5)),
            confidence: 0.85,
            metadata: None,
        };
        
        assert_eq!(signal.signal_type, SignalType::Buy);
        assert_eq!(signal.strength, SignalStrength::Strong);
        assert!(signal.confidence > 0.8);
    }
}