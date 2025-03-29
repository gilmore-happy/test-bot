//! Risk management module for the Solana HFT Bot
//!
//! This module provides comprehensive risk management functionality for the Solana HFT Bot,
//! including position limits, exposure tracking, risk controls, circuit breakers,
//! stress testing, and dynamic risk adjustment.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::Signature,
    transaction::Transaction,
};
use tokio::sync::{mpsc, oneshot, RwLock as TokioRwLock, Semaphore};
use tracing::{debug, error, info, warn};

pub mod circuit_breakers;
pub mod config;
pub mod dynamic_adjustment;
pub mod enhanced_circuit_breakers;
pub mod metrics;
pub mod models;
pub mod multi_factor_models;
pub mod solana_specific;
pub mod stress_testing;

pub use circuit_breakers::{CircuitBreaker, CircuitBreakerManager, CircuitBreakerState};
pub use config::RiskConfig;
pub use dynamic_adjustment::{DynamicAdjustmentConfig, DynamicRiskAdjuster, RiskParameters};
pub use enhanced_circuit_breakers::{
    EnhancedCircuitBreaker, EnhancedCircuitBreakerManager, SeverityLevel,
    TriggerLevel, TriggerAction,
};
pub use metrics::{PnLRecord, RiskMetrics, RiskMetricsSnapshot};
pub use models::{RiskModel, RiskMetrics as ModelRiskMetrics, ValueAtRiskModel, KellyCriterionModel, ExpectedShortfallModel};
pub use multi_factor_models::{
    MultiFactorRiskModel, MultiFactorModelConfig, MultiFactorRiskCalculator,
    RiskFactor, RiskFactorType, StrategyRiskProfile, RiskCategory,
    RiskAssessment, SimulationResult, CrossStrategyRisk,
};
pub use solana_specific::{SolanaRiskAssessor, SolanaRiskFactor, SolanaRiskFactorType};
pub use stress_testing::{StressTester, StressTestConfig, StressTestResult, StressScenarioType};

/// Configuration for the risk management system
#[derive(Debug, Clone)]
pub struct RiskManagerConfig {
    /// Whether to enable risk management
    pub enabled: bool,
    
    /// Maximum position size in SOL
    pub max_position_sol: f64,
    
    /// Maximum position size in USD
    pub max_position_usd: f64,
    
    /// Maximum daily loss in USD
    pub max_daily_loss_usd: f64,
    
    /// Maximum transaction value in USD
    pub max_transaction_value_usd: f64,
    
    /// Whether to enable circuit breakers
    pub enable_circuit_breakers: bool,
    
    /// Whether to enable dynamic risk adjustment
    pub enable_dynamic_adjustment: bool,
    
    /// Whether to enable Solana-specific risk assessment
    pub enable_solana_risk_assessment: bool,
    
    /// Whether to enable stress testing
    pub enable_stress_testing: bool,
    
    /// Maximum exposure percentage of capital
    pub max_exposure_pct: f64,
    
    /// Maximum token concentration percentage
    pub max_token_concentration: f64,
    
    /// Maximum drawdown percentage
    pub max_drawdown_pct: f64,
    
    /// Maximum risk score (0-100)
    pub max_risk_score: u8,
    
    /// Circuit breaker configuration
    pub circuit_breaker_config: circuit_breakers::CircuitBreakerConfig,
    
    /// Dynamic adjustment configuration
    pub dynamic_adjustment_config: dynamic_adjustment::DynamicAdjustmentConfig,
    
    /// Stress test configuration
    pub stress_test_config: stress_testing::StressTestConfig,
}

impl Default for RiskManagerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_position_sol: 100.0,
            max_position_usd: 10000.0,
            max_daily_loss_usd: 1000.0,
            max_transaction_value_usd: 5000.0,
            enable_circuit_breakers: true,
            enable_dynamic_adjustment: true,
            enable_solana_risk_assessment: true,
            enable_stress_testing: true,
            max_exposure_pct: 0.8,
            max_token_concentration: 0.2,
            max_drawdown_pct: 10.0,
            max_risk_score: 70,
            circuit_breaker_config: circuit_breakers::CircuitBreakerConfig::default(),
            dynamic_adjustment_config: dynamic_adjustment::DynamicAdjustmentConfig::default(),
            stress_test_config: stress_testing::StressTestConfig::default(),
        }
    }
}

/// Position information
#[derive(Debug, Clone, Default)]
pub struct Position {
    /// Asset symbol
    pub symbol: String,
    
    /// Current position size
    pub size: f64,
    
    /// Average entry price
    pub avg_price: f64,
    
    /// Current market price
    pub market_price: f64,
    
    /// Unrealized profit/loss
    pub unrealized_pnl: f64,
    
    /// Realized profit/loss
    pub realized_pnl: f64,
    
    /// Position value in USD
    pub value_usd: f64,
    
    /// Strategy ID
    pub strategy_id: String,
    
    /// Entry timestamp
    pub entry_time: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Last update timestamp
    pub last_update: Option<chrono::DateTime<chrono::Utc>>,
}

/// Risk status for execution approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskStatus {
    /// Whether the risk is approved
    pub approved: bool,
    
    /// Risk score (0-100, higher is riskier)
    pub risk_score: u8,
    
    /// Rejection reason (if not approved)
    pub rejection_reason: Option<String>,
    
    /// Risk factors
    pub risk_factors: Vec<String>,
    
    /// Recommended position size
    pub recommended_size: Option<f64>,
    
    /// Maximum allowable position size
    pub max_allowable_size: f64,
    
    /// Solana-specific risk factors
    pub solana_risk_factors: Vec<SolanaRiskFactor>,
}

/// Risk engine state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskEngineState {
    /// Normal operation
    Normal,
    
    /// Cautious operation (increased scrutiny)
    Cautious,
    
    /// Reduced operation (partial shutdown)
    Reduced,
    
    /// Emergency shutdown
    Shutdown,
}

/// Risk error types
#[derive(Debug, thiserror::Error)]
pub enum RiskError {
    #[error("Risk management system already running")]
    AlreadyRunning,
    
    #[error("Risk management system not running")]
    NotRunning,
    
    #[error("Position limit exceeded: {0} (max: {1})")]
    PositionLimitExceeded(f64, f64),
    
    #[error("Position value exceeded: ${0} (max: ${1})")]
    PositionValueExceeded(f64, f64),
    
    #[error("Transaction value exceeded: ${0} (max: ${1})")]
    TransactionValueExceeded(f64, f64),
    
    #[error("Daily loss exceeded: ${0} (max: ${1})")]
    DailyLossExceeded(f64, f64),
    
    #[error("Circuit breaker triggered: {0}")]
    CircuitBreakerTriggered(String),
    
    #[error("Risk limit exceeded: {0}")]
    RiskLimitExceeded(String),
    
    #[error("Solana-specific risk: {0}")]
    SolanaRisk(String),
    
    #[error("Stress test failure: {0}")]
    StressTestFailure(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Core error: {0}")]
    Core(#[from] solana_hft_core::CoreError),
    
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

/// Comprehensive risk management system
pub struct RiskManager {
    /// Configuration
    pub config: RwLock<RiskManagerConfig>,
    
    /// Whether the system is running
    pub running: RwLock<bool>,
    
    /// Current positions
    positions: DashMap<String, Position>,
    
    /// Daily profit/loss
    daily_pnl: RwLock<f64>,
    
    /// RPC client
    rpc_client: Arc<RpcClient>,
    
    /// Risk metrics collector
    metrics: Arc<RiskMetrics>,
    
    /// Circuit breaker manager
    circuit_breaker_manager: Option<Arc<CircuitBreakerManager>>,
    
    /// Dynamic risk adjuster
    dynamic_adjuster: Option<Arc<DynamicRiskAdjuster>>,
    
    /// Solana risk assessor
    solana_assessor: Option<Arc<SolanaRiskAssessor>>,
    
    /// Stress tester
    stress_tester: Option<Arc<StressTester>>,
    
    /// Risk models
    risk_models: RwLock<HashMap<String, Box<dyn RiskModel + Send + Sync>>>,
    
    /// Current risk engine state
    state: RwLock<RiskEngineState>,
    
    /// Last metrics snapshot
    last_metrics: RwLock<Option<RiskMetricsSnapshot>>,
    
    /// Last adjustment time
    last_adjustment_time: RwLock<Instant>,
}

impl RiskManager {
    /// Create a new risk manager with the given configuration
    pub fn new(config: RiskManagerConfig, rpc_client: Arc<RpcClient>) -> Self {
        let metrics = Arc::new(RiskMetrics::new(1000));
        
        // Initialize risk models
        let mut risk_models = HashMap::new();
        risk_models.insert("VaR".to_string(), Box::new(ValueAtRiskModel::new(0.95)) as Box<dyn RiskModel + Send + Sync>);
        risk_models.insert("Kelly".to_string(), Box::new(KellyCriterionModel::new(0.5)) as Box<dyn RiskModel + Send + Sync>);
        risk_models.insert("ES".to_string(), Box::new(ExpectedShortfallModel::new(0.95)) as Box<dyn RiskModel + Send + Sync>);
        
        // Create optional components based on configuration
        let circuit_breaker_manager = if config.enable_circuit_breakers {
            Some(Arc::new(CircuitBreakerManager::new()))
        } else {
            None
        };
        
        let dynamic_adjuster = if config.enable_dynamic_adjustment {
            Some(Arc::new(DynamicRiskAdjuster::new(
                config.dynamic_adjustment_config.clone(),
                rpc_client.clone(),
            )))
        } else {
            None
        };
        
        let solana_assessor = if config.enable_solana_risk_assessment {
            Some(Arc::new(SolanaRiskAssessor::new(
                rpc_client.clone(),
                Duration::from_secs(60),
            )))
        } else {
            None
        };
        
        let stress_tester = if config.enable_stress_testing {
            Some(Arc::new(StressTester::new(
                rpc_client.clone(),
                config.stress_test_config.clone(),
                risk_models.clone(),
            )))
        } else {
            None
        };
        
        Self {
            config: RwLock::new(config),
            running: RwLock::new(false),
            positions: DashMap::new(),
            daily_pnl: RwLock::new(0.0),
            rpc_client,
            metrics,
            circuit_breaker_manager,
            dynamic_adjuster,
            solana_assessor,
            stress_tester,
            risk_models: RwLock::new(risk_models),
            state: RwLock::new(RiskEngineState::Normal),
            last_metrics: RwLock::new(None),
            last_adjustment_time: RwLock::new(Instant::now()),
        }
    }
    
    /// Start the risk management system
    pub async fn start(&self) -> Result<(), RiskError> {
        let mut running = self.running.write();
        if *running {
            return Err(RiskError::AlreadyRunning);
        }
        
        let config = self.config.read();
        info!("Starting risk management system with config: {:?}", config);
        
        // Initialize circuit breakers if enabled
        if let Some(cb_manager) = &self.circuit_breaker_manager {
            circuit_breakers::init_default_circuit_breakers(cb_manager);
        }
        
        // Start background tasks
        self.spawn_background_tasks();
        
        *running = true;
        info!("Risk management system started");
        Ok(())
    }
    
    /// Stop the risk management system
    pub fn stop(&self) -> Result<(), RiskError> {
        let mut running = self.running.write();
        if !*running {
            return Err(RiskError::NotRunning);
        }
        
        info!("Stopping risk management system");
        
        // Cleanup tasks would go here
        
        *running = false;
        info!("Risk management system stopped");
        Ok(())
    }
    
    /// Spawn background tasks for monitoring and adjustment
    fn spawn_background_tasks(&self) {
        self.spawn_metrics_updater();
        self.spawn_circuit_breaker_monitor();
        self.spawn_dynamic_adjustment_task();
    }
    
    /// Spawn a task to update metrics periodically
    fn spawn_metrics_updater(&self) {
        let metrics = self.metrics.clone();
        let positions_ref = self.positions.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                
                // Get current positions
                let positions: Vec<Position> = positions_ref.iter()
                    .map(|entry| entry.value().clone())
                    .collect();
                
                // Update metrics
                metrics.update_exposure(&positions);
                
                // Check for periodic resets
                metrics.check_periodic_reset();
            }
        });
    }
    
    /// Spawn a task to monitor circuit breakers
    fn spawn_circuit_breaker_monitor(&self) {
        if let Some(cb_manager) = self.circuit_breaker_manager.clone() {
            let metrics = self.metrics.clone();
            let state = self.state.clone();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                
                loop {
                    interval.tick().await;
                    
                    // Create metrics snapshot
                    let positions: Vec<Position> = Vec::new(); // Would get from positions store
                    let snapshot = metrics.create_snapshot(&positions);
                    
                    // Update circuit breakers
                    let state_changes = cb_manager.update_all(&snapshot);
                    
                    // Handle state changes
                    if !state_changes.is_empty() {
                        info!("Circuit breaker state changes: {:?}", state_changes);
                        
                        // Update risk engine state based on circuit breaker states
                        let mut new_state = RiskEngineState::Normal;
                        
                        for (id, breaker_state) in &state_changes {
                            if *breaker_state == CircuitBreakerState::Triggered {
                                // Set more restrictive state
                                new_state = match id.as_str() {
                                    "drawdown" => RiskEngineState::Reduced,
                                    "volatility" => RiskEngineState::Cautious,
                                    "exposure" => RiskEngineState::Cautious,
                                    "loss_streak" => RiskEngineState::Shutdown,
                                    _ => new_state,
                                };
                            }
                        }
                        
                        // Update state if more restrictive
                        let mut current_state = state.write();
                        *current_state = match (*current_state, new_state) {
                            (RiskEngineState::Normal, _) => new_state,
                            (RiskEngineState::Cautious, RiskEngineState::Reduced) => RiskEngineState::Reduced,
                            (RiskEngineState::Cautious, RiskEngineState::Shutdown) => RiskEngineState::Shutdown,
                            (RiskEngineState::Reduced, RiskEngineState::Shutdown) => RiskEngineState::Shutdown,
                            _ => *current_state,
                        };
                        
                        info!("Risk engine state updated to {:?}", *current_state);
                    }
                }
            });
        }
    }
    
    /// Spawn a task for dynamic risk adjustment
    fn spawn_dynamic_adjustment_task(&self) {
        if let Some(adjuster) = self.dynamic_adjuster.clone() {
            let metrics = self.metrics.clone();
            let solana_assessor = self.solana_assessor.clone();
            let last_metrics = self.last_metrics.clone();
            let last_adjustment_time = self.last_adjustment_time.clone();
            let config = self.config.clone();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
                
                loop {
                    interval.tick().await;
                    
                    if !config.read().enable_dynamic_adjustment {
                        continue;
                    }
                    
                    // Create metrics snapshot
                    let positions: Vec<Position> = Vec::new(); // Would get from positions store
                    let snapshot = metrics.create_snapshot(&positions);
                    
                    // Get network status if available
                    let network_status = if let Some(assessor) = &solana_assessor {
                        match assessor.get_network_status().await {
                            Ok(status) => Some(status),
                            Err(e) => {
                                warn!("Failed to get network status: {}", e);
                                None
                            }
                        }
                    } else {
                        None
                    };
                    
                    // Assess market conditions
                    let market_assessment = match adjuster.assess_market_conditions(&snapshot, network_status.as_ref()).await {
                        Ok(assessment) => assessment,
                        Err(e) => {
                            warn!("Failed to assess market conditions: {}", e);
                            continue;
                        }
                    };
                    
                    // Assess performance
                    let previous_snapshot_option = last_metrics.read().clone();
                    let performance_assessment = match adjuster.assess_performance(&snapshot, Some(&previous_snapshot_option)).await {
                        Ok(assessment) => assessment,
                        Err(e) => {
                            warn!("Failed to assess performance: {}", e);
                            continue;
                        }
                    };
                    
                    // Calculate adjustments
                    match adjuster.calculate_adjustments(&market_assessment, &performance_assessment).await {
                        Ok(adjustments) => {
                            info!("Dynamic risk adjustments calculated: position_size_factor={}, exposure_limit_factor={}, reason={}",
                                  adjustments.position_size_factor, adjustments.exposure_limit_factor, adjustments.adjustment_reason);
                            
                            // Update last adjustment time
                            *last_adjustment_time.write() = Instant::now();
                            
                            // In a real implementation, would apply these adjustments to risk parameters
                        },
                        Err(e) => {
                            warn!("Failed to calculate risk adjustments: {}", e);
                        }
                    }
                    
                    // Update last metrics
                    *last_metrics.write() = Some(snapshot);
                }
            });
        }
    }
    
    /// Check if a transaction is within risk limits
    pub async fn check_transaction(&self, symbol: &str, size: f64, price: f64) -> Result<RiskStatus, RiskError> {
        let config = self.config.read();
        
        if !config.enabled {
            return Ok(RiskStatus {
                approved: true,
                risk_score: 0,
                rejection_reason: None,
                risk_factors: Vec::new(),
                recommended_size: None,
                max_allowable_size: f64::MAX,
                solana_risk_factors: Vec::new(),
            });
        }
        
        if !*self.running.read() {
            return Err(RiskError::NotRunning);
        }
        
        // Check engine state
        let state = *self.state.read();
        if state == RiskEngineState::Shutdown {
            return Ok(RiskStatus {
                approved: false,
                risk_score: 100,
                rejection_reason: Some("Risk engine is in shutdown state".to_string()),
                risk_factors: vec!["Emergency shutdown activated".to_string()],
                recommended_size: None,
                max_allowable_size: 0.0,
                solana_risk_factors: Vec::new(),
            });
        }
        
        // Check transaction value
        let transaction_value = size * price;
        if transaction_value > config.max_transaction_value_usd {
            return Err(RiskError::TransactionValueExceeded(
                transaction_value,
                config.max_transaction_value_usd
            ));
        }
        
        // Check position limits
        let current_position = self.positions.get(symbol)
            .map(|p| p.size)
            .unwrap_or(0.0);
        
        let new_position = current_position + size;
        
        if symbol == "SOL" && new_position.abs() > config.max_position_sol {
            return Err(RiskError::PositionLimitExceeded(
                new_position,
                config.max_position_sol
            ));
        }
        
        let position_value = new_position * price;
        if position_value.abs() > config.max_position_usd {
            return Err(RiskError::PositionValueExceeded(
                position_value,
                config.max_position_usd
            ));
        }
        
        // Check daily loss limit
        let daily_pnl = *self.daily_pnl.read();
        if daily_pnl < -config.max_daily_loss_usd {
            return Err(RiskError::DailyLossExceeded(
                -daily_pnl,
                config.max_daily_loss_usd
            ));
        }
        
        // Check circuit breakers
        if let Some(cb_manager) = &self.circuit_breaker_manager {
            if cb_manager.any_triggered() {
                let triggered = cb_manager.get_triggered_breakers();
                if !triggered.is_empty() {
                    let breaker_names: Vec<String> = triggered.iter()
                        .map(|b| b.config.name.clone())
                        .collect();
                    
                    return Err(RiskError::CircuitBreakerTriggered(
                        format!("Circuit breakers triggered: {}", breaker_names.join(", "))
                    ));
                }
            }
        }
        
        // Assess Solana-specific risks
        let mut solana_risk_factors = Vec::new();
        if let Some(assessor) = &self.solana_assessor {
            // Example: Check prioritization fee risk
            if let Ok(risk_factor) = assessor.assess_prioritization_fee_risk(None).await {
                if risk_factor.severity > 70 {
                    return Err(RiskError::SolanaRisk(
                        format!("{}: {}", risk_factor.name, risk_factor.description)
                    ));
                }
                
                solana_risk_factors.push(risk_factor);
            }
            
            // Example: Check network congestion
            if let Ok(risk_factor) = assessor.assess_network_congestion_risk().await {
                if risk_factor.severity > 80 {
                    return Err(RiskError::SolanaRisk(
                        format!("{}: {}", risk_factor.name, risk_factor.description)
                    ));
                }
                
                solana_risk_factors.push(risk_factor);
            }
        }
        
        // Calculate risk score
        let risk_score = self.calculate_risk_score(symbol, size, price, &solana_risk_factors);
        
        // Check if risk score exceeds maximum
        if risk_score > config.max_risk_score {
            return Ok(RiskStatus {
                approved: false,
                risk_score,
                rejection_reason: Some("Risk score exceeds maximum threshold".to_string()),
                risk_factors: vec!["High risk score".to_string()],
                recommended_size: Some(size * 0.5), // Recommend half size
                max_allowable_size: 0.0,
                solana_risk_factors,
            });
        }
        
        // Calculate recommended size
        let recommended_size = if let Some(model) = self.risk_models.read().get("Kelly") {
            // Use Kelly Criterion for position sizing
            match model.calculate_position_size(config.max_position_usd, &[0.01, 0.02, -0.01]) {
                Ok(size) => Some(size),
                Err(_) => None,
            }
        } else {
            None
        };
        
        // Adjust for engine state
        let (approved, max_allowable_size) = match state {
            RiskEngineState::Normal => (true, config.max_position_usd),
            RiskEngineState::Cautious => (risk_score < 50, config.max_position_usd * 0.7),
            RiskEngineState::Reduced => (risk_score < 30, config.max_position_usd * 0.3),
            RiskEngineState::Shutdown => (false, 0.0),
        };
        
        Ok(RiskStatus {
            approved,
            risk_score,
            rejection_reason: if approved { None } else { Some("Risk engine state restricts trading".to_string()) },
            risk_factors: Vec::new(), // Would populate with actual risk factors
            recommended_size,
            max_allowable_size,
            solana_risk_factors,
        })
    }
    
    /// Calculate risk score for a transaction
    fn calculate_risk_score(&self, symbol: &str, size: f64, price: f64, solana_risks: &[SolanaRiskFactor]) -> u8 {
        let config = self.config.read();
        
        // Base risk score
        let mut score = 30.0;
        
        // Adjust for position size relative to maximum
        let position_size_factor = if symbol == "SOL" {
            size.abs() / config.max_position_sol
        } else {
            (size * price) / config.max_position_usd
        };
        
        score += position_size_factor * 30.0;
        
        // Adjust for daily PnL
        let daily_pnl = *self.daily_pnl.read();
        if daily_pnl < 0.0 {
            let pnl_factor = (-daily_pnl / config.max_daily_loss_usd).min(1.0);
            score += pnl_factor * 20.0;
        }
        
        // Adjust for Solana-specific risks
        for risk in solana_risks {
            score += risk.severity as f64 * 0.2;
        }
        
        // Adjust for engine state
        let state = *self.state.read();
        score += match state {
            RiskEngineState::Normal => 0.0,
            RiskEngineState::Cautious => 10.0,
            RiskEngineState::Reduced => 20.0,
            RiskEngineState::Shutdown => 50.0,
        };
        
        // Clamp to valid range
        score.clamp(0.0, 100.0) as u8
    }
    
    /// Update a position
    pub fn update_position(&self, symbol: &str, size: f64, price: f64) -> Result<Position, RiskError> {
        if !*self.running.read() {
            return Err(RiskError::NotRunning);
        }
        
        let now = chrono::Utc::now();
        
        // Get or create position
        let mut position = self.positions.entry(symbol.to_string())
            .or_insert_with(|| Position {
                symbol: symbol.to_string(),
                entry_time: Some(now),
                ..Default::default()
            });
        
        let old_position = position.size;
        let old_value = old_position * position.avg_price;
        
        // Update position
        position.size += size;
        
        // Update average price for buys
        if size > 0.0 {
            let new_value = old_value + (size * price);
            position.avg_price = if position.size != 0.0 {
                new_value / position.size
            } else {
                0.0
            };
        }
        
        // Update market price
        position.market_price = price;
        
        // Update value
        position.value_usd = position.size * price;
        
        // Update unrealized P&L
        position.unrealized_pnl = position.size * (position.market_price - position.avg_price);
        
        // Calculate realized P&L for sells
        if size < 0.0 {
            let realized_pnl = -size * (price - position.avg_price);
            position.realized_pnl += realized_pnl;
            
            // Update daily PnL
            let mut daily_pnl = self.daily_pnl.write();
            *daily_pnl += realized_pnl;
            
            // Record PnL in metrics
            self.metrics.record_trade(PnLRecord {
                symbol: symbol.to_string(),
                pnl_usd: realized_pnl,
                size_usd: size.abs() * price,
                entry_price: position.avg_price,
                exit_price: price,
                duration_ms: if let Some(entry) = position.entry_time {
                    (now - entry).num_milliseconds() as u64
                } else {
                    0
                },
                timestamp: now,
                trade_id: format!("trade-{}", uuid::Uuid::new_v4()),
                strategy_id: "default".to_string(),
            });
        }
        
        // Update last update time
        position.last_update = Some(now);
        
        Ok(position.clone())
    }
    
    /// Get the current position for a symbol
    pub fn get_position(&self, symbol: &str) -> Option<Position> {
        self.positions.get(symbol).map(|p| p.clone())
    }
    
    /// Get all current positions
    pub fn get_all_positions(&self) -> Vec<Position> {
        self.positions.iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
    
    /// Get the daily profit/loss
    pub fn get_daily_pnl(&self) -> f64 {
        *self.daily_pnl.read()
    }
    
    /// Reset the daily profit/loss
    pub fn reset_daily_pnl(&self) {
        *self.daily_pnl.write() = 0.0;
    }
    
    /// Get current risk metrics
    pub fn get_risk_metrics(&self) -> RiskMetricsSnapshot {
        let positions = self.get_all_positions();
        self.metrics.create_snapshot(&positions)
    }
    
    /// Get current risk engine state
    pub fn get_state(&self) -> RiskEngineState {
        *self.state.read()
    }
    
    /// Set risk engine state
    pub fn set_state(&self, state: RiskEngineState) {
        *self.state.write() = state;
        info!("Risk engine state manually set to {:?}", state);
    }
    
    /// Run a stress test
    pub async fn run_stress_test(&self, scenario_type: StressScenarioType) -> Result<StressTestResult, RiskError> {
        if !self.config.read().enable_stress_testing {
            return Err(RiskError::Internal("Stress testing is disabled".to_string()));
        }
        
        if let Some(tester) = &self.stress_tester {
            let positions = self.get_all_positions();
            
            match tester.run_stress_test(&positions, scenario_type).await {
                Ok(result) => Ok(result),
                Err(e) => Err(RiskError::StressTestFailure(e.to_string())),
            }
        } else {
            Err(RiskError::Internal("Stress tester not initialized".to_string()))
        }
    }
    
    /// Reset circuit breakers
    pub fn reset_circuit_breakers(&self) -> Result<(), RiskError> {
        if let Some(cb_manager) = &self.circuit_breaker_manager {
            for breaker in cb_manager.get_all_breakers() {
                cb_manager.reset_breaker(&breaker.config.id);
            }
            
            // Reset engine state
            *self.state.write() = RiskEngineState::Normal;
            
            Ok(())
        } else {
            Err(RiskError::Internal("Circuit breaker manager not initialized".to_string()))
        }
    }
    
    /// Reset dynamic adjustments
    pub fn reset_dynamic_adjustments(&self) -> Result<(), RiskError> {
        if let Some(adjuster) = &self.dynamic_adjuster {
            adjuster.reset_adjustments();
            Ok(())
        } else {
            Err(RiskError::Internal("Dynamic adjuster not initialized".to_string()))
        }
    }
}

/// Module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;
    use solana_client::rpc_client::RpcClient;
    
    #[test]
    fn test_risk_manager_config_default() {
        let config = RiskManagerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_position_sol, 100.0);
        assert_eq!(config.max_position_usd, 10000.0);
        assert_eq!(config.max_daily_loss_usd, 1000.0);
        assert_eq!(config.max_transaction_value_usd, 5000.0);
        assert!(config.enable_circuit_breakers);
    }
    
    #[tokio::test]
    async fn test_risk_manager_start_stop() {
        let config = RiskManagerConfig::default();
        let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
        let manager = RiskManager::new(config, rpc_client);
        
        assert!(!*manager.running.read());
        
        let result = manager.start().await;
        assert!(result.is_ok());
        assert!(*manager.running.read());
        
        // Trying to start again should fail
        let result = manager.start().await;
        assert!(result.is_err());
        
        let result = manager.stop();
        assert!(result.is_ok());
        assert!(!*manager.running.read());
        
        // Trying to stop again should fail
        let result = manager.stop();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_position_updates() {
        let config = RiskManagerConfig::default();
        let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
        let manager = RiskManager::new(config, rpc_client);
        
        // Set running state manually for testing
        *manager.running.write() = true;
        
        // Buy 10 SOL at $100
        let result = manager.update_position("SOL", 10.0, 100.0);
        assert!(result.is_ok());
        
        let position = manager.get_position("SOL").unwrap();
        assert_eq!(position.symbol, "SOL");
        assert_eq!(position.size, 10.0);
        assert_eq!(position.avg_price, 100.0);
        
        // Buy 5 more SOL at $110
        let result = manager.update_position("SOL", 5.0, 110.0);
        assert!(result.is_ok());
        
        let position = manager.get_position("SOL").unwrap();
        assert_eq!(position.size, 15.0);
        assert_eq!(position.avg_price, 103.33333333333333); // (10*100 + 5*110) / 15
        
        // Sell 8 SOL at $120
        let result = manager.update_position("SOL", -8.0, 120.0);
        assert!(result.is_ok());
        
        let position = manager.get_position("SOL").unwrap();
        assert_eq!(position.size, 7.0);
        assert_eq!(position.realized_pnl, 8.0 * (120.0 - 103.33333333333333));
        
        // Check daily PnL
        assert_eq!(manager.get_daily_pnl(), 8.0 * (120.0 - 103.33333333333333));
    }
    
    #[tokio::test]
    async fn test_risk_status() {
        let config = RiskManagerConfig::default();
        let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
        let manager = RiskManager::new(config, rpc_client);
        
        // Set running state manually for testing
        *manager.running.write() = true;
        
        // Check a small transaction
        let result = manager.check_transaction("SOL", 1.0, 100.0).await;
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.approved);
        
        // Check a large transaction
        let result = manager.check_transaction("SOL", 200.0, 100.0).await;
        assert!(result.is_err());
        
        // Set engine state to shutdown
        manager.set_state(RiskEngineState::Shutdown);
        
        // Check any transaction
        let result = manager.check_transaction("SOL", 1.0, 100.0).await;
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(!status.approved);
    }
}