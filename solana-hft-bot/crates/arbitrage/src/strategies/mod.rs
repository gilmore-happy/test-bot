//! Arbitrage strategies
//!
//! This module provides implementations of various arbitrage strategies.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use solana_sdk::native_token;
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

use crate::config::ArbitrageConfig;
use crate::dexes::DexRegistry;
use crate::paths::{ArbitragePath, PathFinder};
use crate::pools::PoolRegistry;
use crate::pricing::PriceManager;

mod circular;
mod triangular;
mod cross_exchange;
mod protocol_aware;

pub use circular::CircularArbitrageStrategy;
pub use triangular::TriangularArbitrageStrategy;
pub use cross_exchange::CrossExchangeArbitrageStrategy;
pub use protocol_aware::ProtocolAwareArbitrageStrategy;

/// Arbitrage opportunity details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    /// Unique identifier for the opportunity
    pub id: String,
    
    /// Path for the arbitrage
    pub path: ArbitragePath,
    
    /// Input amount in tokens
    pub input_amount: u64,
    
    /// Expected output amount in tokens
    pub expected_output: u64,
    
    /// Expected profit in USD
    pub expected_profit_usd: f64,
    
    /// Expected profit in basis points
    pub expected_profit_bps: u32,
    
    /// Timestamp when the opportunity was detected
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Time-to-live in milliseconds
    pub ttl_ms: u64,
    
    /// Execution priority
    pub priority: ExecutionPriority,
    
    /// Strategy that identified the opportunity
    pub strategy: String,
    
    /// Risk assessment
    pub risk: RiskAssessment,
}

/// Execution priority for arbitrage opportunities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionPriority {
    /// Low priority
    Low,
    
    /// Medium priority
    Medium,
    
    /// High priority
    High,
    
    /// Urgent priority
    Urgent,
}

impl ExecutionPriority {
    /// Convert to numeric priority value (higher is more important)
    pub fn as_value(&self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Urgent => 3,
        }
    }
}

/// Risk assessment for an arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Risk score (0-100, higher is riskier)
    pub risk_score: u8,
    
    /// Probability of execution success (0-100)
    pub success_probability: u8,
    
    /// Risk factors
    pub risk_factors: Vec<String>,
    
    /// Maximum potential loss in USD
    pub max_potential_loss_usd: f64,
}

/// Arbitrage strategy trait
#[async_trait]
pub trait ArbitrageStrategy: Send + Sync {
    /// Get strategy name
    fn name(&self) -> &str;
    
    /// Initialize the strategy
    async fn initialize(&self) -> Result<()>;
    
    /// Find arbitrage opportunities
    async fn find_opportunities(
        &self,
        pool_registry: Arc<PoolRegistry>,
        price_manager: Arc<PriceManager>,
        path_finder: Arc<PathFinder>,
        dex_registry: Arc<DexRegistry>,
    ) -> Result<Vec<ArbitrageOpportunity>>;
}

/// Strategy manager
pub struct StrategyManager {
    /// Registered strategies
    strategies: RwLock<Vec<Arc<dyn ArbitrageStrategy>>>,
    
    /// Configuration
    config: ArbitrageConfig,
}

impl StrategyManager {
    /// Create a new strategy manager
    pub fn new(config: &ArbitrageConfig) -> Self {
        Self {
            strategies: RwLock::new(Vec::new()),
            config: config.clone(),
        }
    }
    
    /// Initialize all strategies
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing strategy manager");
        
        // Register default strategies
        self.register_strategy(Arc::new(CircularArbitrageStrategy::new()));
        self.register_strategy(Arc::new(TriangularArbitrageStrategy::new()));
        
        // Register cross-exchange strategy if enabled
        if self.config.use_flash_loans {
            self.register_strategy(Arc::new(CrossExchangeArbitrageStrategy::new()));
        }
        
        // Register protocol-aware strategy
        self.register_strategy(Arc::new(ProtocolAwareArbitrageStrategy::new()
            .with_min_profit_threshold_usd(self.config.min_profit_threshold_usd)
            .with_min_profit_threshold_bps(self.config.min_profit_threshold_bps)
            .with_max_gas_cost_lamports(self.config.max_gas_cost_lamports)
            .with_flash_loans(self.config.use_flash_loans)
        ));
        
        // Initialize all strategies
        for strategy in self.strategies.read().iter() {
            debug!("Initializing strategy: {}", strategy.name());
            strategy.initialize().await?;
        }
        
        info!("Strategy manager initialized with {} strategies", self.strategies.read().len());
        Ok(())
    }
    
    /// Register a strategy
    pub fn register_strategy(&self, strategy: Arc<dyn ArbitrageStrategy>) {
        info!("Registering strategy: {}", strategy.name());
        self.strategies.write().push(strategy);
    }
    
    /// Get all registered strategies
    pub fn get_strategies(&self) -> Vec<Arc<dyn ArbitrageStrategy>> {
        self.strategies.read().clone()
    }
    
    /// Create a new arbitrage opportunity
    pub fn create_opportunity(
        &self,
        path: ArbitragePath,
        input_amount: u64,
        expected_output: u64,
        expected_profit_usd: f64,
        expected_profit_bps: u32,
        strategy_name: &str,
    ) -> ArbitrageOpportunity {
        // Determine priority based on profit
        let priority = if expected_profit_bps > 100 {
            ExecutionPriority::High
        } else if expected_profit_bps > 50 {
            ExecutionPriority::Medium
        } else {
            ExecutionPriority::Low
        };
        
        // Create risk assessment
        let risk = RiskAssessment {
            risk_score: 20, // Default moderate risk
            success_probability: 80, // Default high probability
            risk_factors: vec!["Price volatility".to_string()],
            max_potential_loss_usd: expected_profit_usd * 0.5, // Conservative estimate
        };
        
        // Create opportunity
        ArbitrageOpportunity {
            id: format!("{}-{}", strategy_name, Uuid::new_v4()),
            path,
            input_amount,
            expected_output,
            expected_profit_usd,
            expected_profit_bps,
            timestamp: Utc::now(),
            ttl_ms: 10_000, // 10 seconds TTL
            priority,
            strategy: strategy_name.to_string(),
            risk,
        }
    }
}