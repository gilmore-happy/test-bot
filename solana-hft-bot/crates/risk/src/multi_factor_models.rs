//! Multi-factor risk calculation models for the Solana HFT Bot
//!
//! This module provides advanced risk calculation models that incorporate
//! multiple factors and cross-correlation analysis between strategies.
//! It includes simulation-based risk projection and dynamic risk adjustment.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use ndarray::{Array1, Array2, s};
use ndarray_stats::CorrelationExt;
use parking_lot::RwLock;
use rand::distributions::{Distribution, Normal};
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, StudentsT};
use tracing::{debug, error, info, warn};

use crate::metrics::RiskMetricsSnapshot;
use crate::models::{RiskModel, ModelRiskMetrics};
use crate::Position;

/// Risk factor type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskFactorType {
    /// Market volatility
    MarketVolatility,
    /// Liquidity
    Liquidity,
    /// Correlation
    Correlation,
    /// Execution quality
    ExecutionQuality,
    /// Network congestion
    NetworkCongestion,
    /// Blockchain-specific risk
    BlockchainSpecific,
    /// Strategy-specific risk
    StrategySpecific,
    /// Counterparty risk
    CounterpartyRisk,
    /// Regulatory risk
    RegulatoryRisk,
    /// Custom risk factor
    Custom,
}

/// Risk factor data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    /// Risk factor type
    pub factor_type: RiskFactorType,
    /// Risk factor name
    pub name: String,
    /// Risk factor value
    pub value: f64,
    /// Risk factor weight in the model
    pub weight: f64,
    /// Risk factor threshold
    pub threshold: f64,
    /// Whether the factor is above threshold
    pub is_above_threshold: bool,
    /// Risk factor description
    pub description: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl RiskFactor {
    /// Create a new risk factor
    pub fn new(
        factor_type: RiskFactorType,
        name: String,
        value: f64,
        weight: f64,
        threshold: f64,
        description: String,
    ) -> Self {
        let is_above_threshold = value > threshold;
        
        Self {
            factor_type,
            name,
            value,
            weight,
            threshold,
            is_above_threshold,
            description,
            metadata: HashMap::new(),
        }
    }
    
    /// Get the weighted risk contribution
    pub fn weighted_contribution(&self) -> f64 {
        self.value * self.weight
    }
    
    /// Check if the factor is above threshold
    pub fn is_above_threshold(&self) -> bool {
        self.value > self.threshold
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Multi-factor risk model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiFactorModelConfig {
    /// Model name
    pub name: String,
    /// Risk factor weights
    pub factor_weights: HashMap<RiskFactorType, f64>,
    /// Risk factor thresholds
    pub factor_thresholds: HashMap<RiskFactorType, f64>,
    /// Overall risk threshold
    pub overall_risk_threshold: f64,
    /// Whether to use exponential weighting for historical data
    pub use_exponential_weighting: bool,
    /// Decay factor for exponential weighting
    pub decay_factor: f64,
    /// Lookback window for historical data
    pub lookback_window: Duration,
    /// Confidence level for risk metrics (0.0-1.0)
    pub confidence_level: f64,
    /// Number of simulation scenarios
    pub simulation_scenarios: usize,
    /// Simulation time horizon in days
    pub simulation_horizon_days: u32,
    /// Whether to include correlation in risk calculations
    pub include_correlation: bool,
    /// Whether to include tail risk in calculations
    pub include_tail_risk: bool,
}

impl Default for MultiFactorModelConfig {
    fn default() -> Self {
        let mut factor_weights = HashMap::new();
        factor_weights.insert(RiskFactorType::MarketVolatility, 0.25);
        factor_weights.insert(RiskFactorType::Liquidity, 0.20);
        factor_weights.insert(RiskFactorType::Correlation, 0.15);
        factor_weights.insert(RiskFactorType::ExecutionQuality, 0.15);
        factor_weights.insert(RiskFactorType::NetworkCongestion, 0.10);
        factor_weights.insert(RiskFactorType::BlockchainSpecific, 0.10);
        factor_weights.insert(RiskFactorType::CounterpartyRisk, 0.05);
        
        let mut factor_thresholds = HashMap::new();
        factor_thresholds.insert(RiskFactorType::MarketVolatility, 0.8);
        factor_thresholds.insert(RiskFactorType::Liquidity, 0.7);
        factor_thresholds.insert(RiskFactorType::Correlation, 0.8);
        factor_thresholds.insert(RiskFactorType::ExecutionQuality, 0.7);
        factor_thresholds.insert(RiskFactorType::NetworkCongestion, 0.8);
        factor_thresholds.insert(RiskFactorType::BlockchainSpecific, 0.7);
        factor_thresholds.insert(RiskFactorType::CounterpartyRisk, 0.6);
        
        Self {
            name: "Default Multi-Factor Model".to_string(),
            factor_weights,
            factor_thresholds,
            overall_risk_threshold: 0.7,
            use_exponential_weighting: true,
            decay_factor: 0.94,
            lookback_window: Duration::from_secs(86400 * 30), // 30 days
            confidence_level: 0.95,
            simulation_scenarios: 1000,
            simulation_horizon_days: 10,
            include_correlation: true,
            include_tail_risk: true,
        }
    }
}

/// Strategy risk profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRiskProfile {
    /// Strategy ID
    pub strategy_id: String,
    /// Strategy name
    pub name: String,
    /// Strategy description
    pub description: String,
    /// Risk factors specific to this strategy
    pub risk_factors: Vec<RiskFactor>,
    /// Historical returns
    pub historical_returns: VecDeque<f64>,
    /// Historical volatility
    pub volatility: f64,
    /// Sharpe ratio
    pub sharpe_ratio: f64,
    /// Maximum drawdown
    pub max_drawdown: f64,
    /// Value at Risk (VaR)
    pub var: f64,
    /// Expected Shortfall (ES)
    pub expected_shortfall: f64,
    /// Risk score (0-100)
    pub risk_score: u8,
    /// Risk category
    pub risk_category: RiskCategory,
    /// Last update time
    pub last_update: DateTime<Utc>,
}

impl StrategyRiskProfile {
    /// Create a new strategy risk profile
    pub fn new(strategy_id: &str, name: &str, description: &str) -> Self {
        Self {
            strategy_id: strategy_id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            risk_factors: Vec::new(),
            historical_returns: VecDeque::with_capacity(100),
            volatility: 0.0,
            sharpe_ratio: 0.0,
            max_drawdown: 0.0,
            var: 0.0,
            expected_shortfall: 0.0,
            risk_score: 50,
            risk_category: RiskCategory::Medium,
            last_update: Utc::now(),
        }
    }
    
    /// Add a risk factor
    pub fn add_risk_factor(&mut self, factor: RiskFactor) {
        self.risk_factors.push(factor);
    }
    
    /// Add a historical return
    pub fn add_return(&mut self, ret: f64) {
        self.historical_returns.push_back(ret);
        if self.historical_returns.len() > 100 {
            self.historical_returns.pop_front();
        }
    }
    
    /// Calculate risk metrics
    pub fn calculate_metrics(&mut self) {
        if self.historical_returns.is_empty() {
            return;
        }
        
        // Calculate volatility
        let returns: Vec<f64> = self.historical_returns.iter().cloned().collect();
        self.volatility = calculate_volatility(&returns);
        
        // Calculate Sharpe ratio (assuming risk-free rate of 0 for simplicity)
        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        if self.volatility > 0.0 {
            self.sharpe_ratio = mean_return / self.volatility;
        }
        
        // Calculate maximum drawdown
        self.max_drawdown = calculate_max_drawdown(&returns);
        
        // Calculate VaR and ES
        let sorted_returns: Vec<f64> = {
            let mut sorted = returns.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            sorted
        };
        
        let var_index = ((1.0 - 0.95) * sorted_returns.len() as f64).floor() as usize;
        self.var = sorted_returns[var_index];
        
        let es_sum = sorted_returns.iter()
            .take(var_index + 1)
            .sum::<f64>();
        self.expected_shortfall = es_sum / (var_index + 1) as f64;
        
        // Calculate risk score (0-100)
        let vol_score = (self.volatility * 100.0).min(100.0);
        let drawdown_score = (self.max_drawdown * 100.0).min(100.0);
        let var_score = (self.var.abs() * 100.0).min(100.0);
        let es_score = (self.expected_shortfall.abs() * 100.0).min(100.0);
        
        let risk_score = (vol_score * 0.3 + drawdown_score * 0.3 + var_score * 0.2 + es_score * 0.2) as u8;
        self.risk_score = risk_score.min(100);
        
        // Determine risk category
        self.risk_category = if risk_score < 30 {
            RiskCategory::Low
        } else if risk_score < 60 {
            RiskCategory::Medium
        } else if risk_score < 80 {
            RiskCategory::High
        } else {
            RiskCategory::VeryHigh
        };
        
        self.last_update = Utc::now();
    }
}

/// Risk category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskCategory {
    /// Low risk
    Low,
    /// Medium risk
    Medium,
    /// High risk
    High,
    /// Very high risk
    VeryHigh,
}

/// Multi-factor risk model
pub struct MultiFactorRiskModel {
    /// Model configuration
    pub config: RwLock<MultiFactorModelConfig>,
    /// Strategy risk profiles
    strategy_profiles: RwLock<HashMap<String, StrategyRiskProfile>>,
    /// Historical risk factors
    historical_factors: RwLock<VecDeque<(DateTime<Utc>, Vec<RiskFactor>)>>,
    /// Correlation matrix between strategies
    correlation_matrix: RwLock<Option<Array2<f64>>>,
    /// Strategy names in correlation matrix
    correlation_strategies: RwLock<Vec<String>>,
    /// Last update time
    last_update: RwLock<DateTime<Utc>>,
}

impl MultiFactorRiskModel {
    /// Create a new multi-factor risk model
    pub fn new(config: MultiFactorModelConfig) -> Self {
        Self {
            config: RwLock::new(config),
            strategy_profiles: RwLock::new(HashMap::new()),
            historical_factors: RwLock::new(VecDeque::with_capacity(100)),
            correlation_matrix: RwLock::new(None),
            correlation_strategies: RwLock::new(Vec::new()),
            last_update: RwLock::new(Utc::now()),
        }
    }
    
    /// Add a strategy risk profile
    pub fn add_strategy_profile(&self, profile: StrategyRiskProfile) {
        let mut profiles = self.strategy_profiles.write();
        profiles.insert(profile.strategy_id.clone(), profile);
        
        // Invalidate correlation matrix
        *self.correlation_matrix.write() = None;
    }
    
    /// Get a strategy risk profile
    pub fn get_strategy_profile(&self, strategy_id: &str) -> Option<StrategyRiskProfile> {
        let profiles = self.strategy_profiles.read();
        profiles.get(strategy_id).cloned()
    }
    
    /// Update a strategy with new return data
    pub fn update_strategy_return(&self, strategy_id: &str, ret: f64) -> Result<()> {
        let mut profiles = self.strategy_profiles.write();
        
        let profile = profiles.get_mut(strategy_id)
            .ok_or_else(|| anyhow!("Strategy not found: {}", strategy_id))?;
        
        profile.add_return(ret);
        profile.calculate_metrics();
        
        // Invalidate correlation matrix
        *self.correlation_matrix.write() = None;
        
        Ok(())
    }
    
    /// Add risk factors for the current time
    pub fn add_risk_factors(&self, factors: Vec<RiskFactor>) {
        let now = Utc::now();
        let mut historical = self.historical_factors.write();
        
        historical.push_back((now, factors));
        
        // Trim history if needed
        let config = self.config.read();
        while let Some((time, _)) = historical.front() {
            if now.duration_since(*time).unwrap_or_default() > config.lookback_window {
                historical.pop_front();
            } else {
                break;
            }
        }
        
        *self.last_update.write() = now;
    }
    
    /// Calculate overall risk score based on all factors
    pub fn calculate_overall_risk(&self) -> f64 {
        let historical = self.historical_factors.read();
        if historical.is_empty() {
            return 0.5; // Default mid-range risk if no data
        }
        
        let config = self.config.read();
        
        // Get the most recent factors
        let (_, latest_factors) = historical.back().unwrap();
        
        // Calculate weighted sum of factor values
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;
        
        for factor in latest_factors {
            let weight = config.factor_weights.get(&factor.factor_type)
                .cloned()
                .unwrap_or(0.1);
            
            weighted_sum += factor.value * weight;
            total_weight += weight;
        }
        
        // Normalize to 0-1 range
        if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.5 // Default mid-range risk if no weights
        }
    }
    
    /// Calculate correlation matrix between strategies
    pub fn calculate_correlation_matrix(&self) -> Result<Array2<f64>> {
        let profiles = self.strategy_profiles.read();
        
        // Get strategies with enough data
        let valid_strategies: Vec<(&String, &StrategyRiskProfile)> = profiles.iter()
            .filter(|(_, profile)| profile.historical_returns.len() >= 10)
            .collect();
        
        if valid_strategies.len() < 2 {
            return Err(anyhow!("Not enough strategies with sufficient data"));
        }
        
        // Create return matrix
        let n_strategies = valid_strategies.len();
        let min_history = valid_strategies.iter()
            .map(|(_, profile)| profile.historical_returns.len())
            .min()
            .unwrap_or(0);
        
        let mut return_matrix = Array2::zeros((min_history, n_strategies));
        let strategy_ids: Vec<String> = valid_strategies.iter()
            .map(|(id, _)| (*id).clone())
            .collect();
        
        for (col, (_, profile)) in valid_strategies.iter().enumerate() {
            let returns: Vec<f64> = profile.historical_returns.iter()
                .rev()
                .take(min_history)
                .cloned()
                .collect();
            
            for (row, ret) in returns.iter().enumerate() {
                return_matrix[[row, col]] = *ret;
            }
        }
        
        // Calculate correlation matrix
        let correlation = return_matrix.t().correlation()?;
        
        // Store for later use
        *self.correlation_matrix.write() = Some(correlation.clone());
        *self.correlation_strategies.write() = strategy_ids;
        
        Ok(correlation)
    }
    
    /// Get correlation between two strategies
    pub fn get_strategy_correlation(&self, strategy1: &str, strategy2: &str) -> Result<f64> {
        // Check if we have a cached correlation matrix
        let correlation_matrix = {
            let matrix = self.correlation_matrix.read();
            if let Some(m) = matrix.clone() {
                m
            } else {
                // Calculate if not available
                drop(matrix);
                self.calculate_correlation_matrix()?
            }
        };
        
        let strategies = self.correlation_strategies.read();
        
        // Find indices of the strategies
        let idx1 = strategies.iter().position(|s| s == strategy1)
            .ok_or_else(|| anyhow!("Strategy not found in correlation matrix: {}", strategy1))?;
        
        let idx2 = strategies.iter().position(|s| s == strategy2)
            .ok_or_else(|| anyhow!("Strategy not found in correlation matrix: {}", strategy2))?;
        
        Ok(correlation_matrix[[idx1, idx2]])
    }
    
    /// Run Monte Carlo simulation for risk projection
    pub fn run_risk_simulation(&self, positions: &[Position]) -> Result<SimulationResult> {
        let config = self.config.read();
        let profiles = self.strategy_profiles.read();
        
        // Group positions by strategy
        let mut strategy_positions: HashMap<String, Vec<&Position>> = HashMap::new();
        for position in positions {
            strategy_positions.entry(position.strategy_id.clone())
                .or_insert_with(Vec::new)
                .push(position);
        }
        
        // Calculate initial portfolio value
        let initial_value: f64 = positions.iter()
            .map(|p| p.value_usd)
            .sum();
        
        if initial_value == 0.0 {
            return Err(anyhow!("Portfolio has zero value"));
        }
        
        // Prepare simulation parameters
        let n_scenarios = config.simulation_scenarios;
        let horizon_days = config.simulation_horizon_days;
        let mut rng = StdRng::from_entropy();
        
        // Store simulation paths and metrics
        let mut simulation_paths = Vec::with_capacity(n_scenarios);
        let mut final_values = Vec::with_capacity(n_scenarios);
        let mut min_values = Vec::with_capacity(n_scenarios);
        let mut max_drawdowns = Vec::with_capacity(n_scenarios);
        
        // Run simulations
        for _ in 0..n_scenarios {
            let mut path = Vec::with_capacity(horizon_days as usize + 1);
            path.push(initial_value);
            
            let mut current_value = initial_value;
            let mut max_value = initial_value;
            let mut min_value = initial_value;
            let mut max_drawdown = 0.0;
            
            // Simulate each day
            for _ in 0..horizon_days {
                let mut daily_pnl = 0.0;
                
                // Simulate each strategy's contribution
                for (strategy_id, positions) in &strategy_positions {
                    if let Some(profile) = profiles.get(strategy_id) {
                        // Use strategy's volatility and historical performance
                        let strategy_value: f64 = positions.iter().map(|p| p.value_usd).sum();
                        let weight = strategy_value / initial_value;
                        
                        // Generate random return based on strategy's distribution
                        let mean_return = profile.historical_returns.iter().sum::<f64>() / 
                                         profile.historical_returns.len() as f64;
                        
                        let normal = Normal::new(mean_return, profile.volatility).unwrap();
                        let daily_return = normal.sample(&mut rng);
                        
                        daily_pnl += strategy_value * daily_return;
                    }
                }
                
                // Update portfolio value
                current_value += daily_pnl;
                path.push(current_value);
                
                // Track min value
                if current_value < min_value {
                    min_value = current_value;
                }
                
                // Track max value and drawdown
                if current_value > max_value {
                    max_value = current_value;
                } else {
                    let drawdown = (max_value - current_value) / max_value;
                    if drawdown > max_drawdown {
                        max_drawdown = drawdown;
                    }
                }
            }
            
            simulation_paths.push(path);
            final_values.push(current_value);
            min_values.push(min_value);
            max_drawdowns.push(max_drawdown);
        }
        
        // Calculate risk metrics from simulation
        let mut sorted_final_values = final_values.clone();
        sorted_final_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let var_index = ((1.0 - config.confidence_level) * n_scenarios as f64).floor() as usize;
        let var = (initial_value - sorted_final_values[var_index]) / initial_value;
        
        let es_sum: f64 = sorted_final_values.iter()
            .take(var_index + 1)
            .map(|v| (initial_value - v) / initial_value)
            .sum();
        let expected_shortfall = es_sum / (var_index + 1) as f64;
        
        // Calculate average metrics
        let avg_final_value = final_values.iter().sum::<f64>() / n_scenarios as f64;
        let avg_return = (avg_final_value - initial_value) / initial_value;
        let avg_min_value = min_values.iter().sum::<f64>() / n_scenarios as f64;
        let avg_max_drawdown = max_drawdowns.iter().sum::<f64>() / n_scenarios as f64;
        
        // Calculate standard deviation of final values
        let mean = avg_final_value;
        let variance: f64 = final_values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / n_scenarios as f64;
        let std_dev = variance.sqrt();
        let std_dev_pct = std_dev / initial_value;
        
        Ok(SimulationResult {
            initial_value,
            avg_final_value,
            avg_return,
            avg_min_value,
            avg_max_drawdown,
            value_at_risk: var,
            expected_shortfall,
            std_dev,
            std_dev_pct,
            confidence_level: config.confidence_level,
            horizon_days,
            n_scenarios,
            simulation_paths,
            final_values,
        })
    }
    
    /// Calculate cross-strategy risk
    pub fn calculate_cross_strategy_risk(&self) -> Result<CrossStrategyRisk> {
        // Calculate correlation matrix if needed
        let correlation = {
            let matrix = self.correlation_matrix.read();
            if let Some(m) = matrix.clone() {
                m
            } else {
                // Calculate if not available
                drop(matrix);
                self.calculate_correlation_matrix()?
            }
        };
        
        let strategies = self.correlation_strategies.read();
        let profiles = self.strategy_profiles.read();
        
        // Calculate average correlation
        let mut sum_correlation = 0.0;
        let mut count = 0;
        
        for i in 0..correlation.shape()[0] {
            for j in (i+1)..correlation.shape()[1] {
                sum_correlation += correlation[[i, j]].abs();
                count += 1;
            }
        }
        
        let avg_correlation = if count > 0 { sum_correlation / count as f64 } else { 0.0 };
        
        // Find highly correlated strategy pairs
        let mut high_correlation_pairs = Vec::new();
        let threshold = 0.7; // Correlation threshold
        
        for i in 0..correlation.shape()[0] {
            for j in (i+1)..correlation.shape()[1] {
                if correlation[[i, j]].abs() > threshold {
                    high_correlation_pairs.push((
                        strategies[i].clone(),
                        strategies[j].clone(),
                        correlation[[i, j]],
                    ));
                }
            }
        }
        
        // Calculate diversification score (lower is better)
        let diversification_score = avg_correlation;
        
        // Calculate concentration risk
        let mut strategy_values = HashMap::new();
        let mut total_value = 0.0;
        
        for (id, profile) in profiles.iter() {
            // In a real implementation, this would use actual position values
            // For now, use a placeholder based on risk score
            let value = 100.0 * (1.0 - (profile.risk_score as f64 / 100.0));
            strategy_values.insert(id.clone(), value);
            total_value += value;
        }
        
        let mut concentration_risk = 0.0;
        if total_value > 0.0 {
            for value in strategy_values.values() {
                let weight = value / total_value;
                concentration_risk += weight.powi(2);
            }
        }
        
        // Calculate overall cross-strategy risk score
        let cross_strategy_risk_score = 
            0.4 * diversification_score + 
            0.4 * concentration_risk + 
            0.2 * (high_correlation_pairs.len() as f64 / strategies.len().max(1) as f64);
        
        Ok(CrossStrategyRisk {
            avg_correlation,
            high_correlation_pairs,
            diversification_score,
            concentration_risk,
            cross_strategy_risk_score,
            timestamp: Utc::now(),
        })
    }
}

/// Simulation result from risk projection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Initial portfolio value
    pub initial_value: f64,
    /// Average final portfolio value
    pub avg_final_value: f64,
    /// Average return
    pub avg_return: f64,
    /// Average minimum value
    pub avg_min_value: f64,
    /// Average maximum drawdown
    pub avg_max_drawdown: f64,
    /// Value at Risk (VaR)
    pub value_at_risk: f64,
    /// Expected Shortfall (ES)
    pub expected_shortfall: f64,
    /// Standard deviation of final values
    pub std_dev: f64,
    /// Standard deviation as percentage of initial value
    pub std_dev_pct: f64,
    /// Confidence level used
    pub confidence_level: f64,
    /// Simulation horizon in days
    pub horizon_days: u32,
    /// Number of simulation scenarios
    pub n_scenarios: usize,
    /// Simulation paths
    pub simulation_paths: Vec<Vec<f64>>,
    /// Final values from all scenarios
    pub final_values: Vec<f64>,
}

/// Cross-strategy risk analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossStrategyRisk {
    /// Average correlation between strategies
    pub avg_correlation: f64,
    /// Highly correlated strategy pairs
    pub high_correlation_pairs: Vec<(String, String, f64)>,
    /// Diversification score (lower is better)
    pub diversification_score: f64,
    /// Concentration risk
    pub concentration_risk: f64,
    /// Overall cross-strategy risk score
    pub cross_strategy_risk_score: f64,
    /// Timestamp of the analysis
    pub timestamp: DateTime<Utc>,
}

/// Calculate volatility from a series of returns
fn calculate_volatility(returns: &[f64]) -> f64 {
    if returns.len() < 2 {
        return 0.0;
    }
    
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns.iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>() / (returns.len() - 1) as f64;
    
    variance.sqrt()
}

/// Calculate maximum drawdown from a series of returns
fn calculate_max_drawdown(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    
    // Convert returns to cumulative returns
    let mut cumulative = Vec::with_capacity(returns.len() + 1);
    cumulative.push(1.0); // Start with $1
    
    let mut cum_value = 1.0;
    for ret in returns {
        cum_value *= (1.0 + ret);
        cumulative.push(cum_value);
    }
    
    // Calculate maximum drawdown
    let mut max_drawdown = 0.0;
    let mut peak = cumulative[0];
    
    for &value in &cumulative {
        if value > peak {
            peak = value;
        } else {
            let drawdown = (peak - value) / peak;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
    }
    
    max_drawdown
}

/// Multi-factor risk calculator
pub struct MultiFactorRiskCalculator {
    /// Risk model
    model: Arc<MultiFactorRiskModel>,
    /// Last calculation time
    last_calculation: RwLock<DateTime<Utc>>,
    /// Last risk factors
    last_factors: RwLock<Vec<RiskFactor>>,
    /// Last overall risk score
    last_risk_score: RwLock<f64>,
    /// Last cross-strategy risk
    last_cross_strategy_risk: RwLock<Option<CrossStrategyRisk>>,
    /// Last simulation result
    last_simulation: RwLock<Option<SimulationResult>>,
}

impl MultiFactorRiskCalculator {
    /// Create a new multi-factor risk calculator
    pub fn new(model: Arc<MultiFactorRiskModel>) -> Self {
        Self {
            model,
            last_calculation: RwLock::new(Utc::now()),
            last_factors: RwLock::new(Vec::new()),
            last_risk_score: RwLock::new(0.5),
            last_cross_strategy_risk: RwLock::new(None),
            last_simulation: RwLock::new(None),
        }
    }
    
    /// Calculate risk for the current state
    pub async fn calculate_risk(
        &self,
        metrics: &RiskMetricsSnapshot,
        positions: &[Position],
    ) -> Result<RiskAssessment> {
        // Generate risk factors from metrics
        let factors = self.generate_risk_factors(metrics);
        
        // Update the model with new factors
        self.model.add_risk_factors(factors.clone());
        
        // Calculate overall risk score
        let risk_score = self.model.calculate_overall_risk();
        
        // Calculate cross-strategy risk
        let cross_strategy_risk = self.model.calculate_cross_strategy_risk().ok();
        
        // Run simulation
        let simulation = self.model.run_risk_simulation(positions).ok();
        
        // Store results
        *self.last_factors.write() = factors.clone();
        *self.last_risk_score.write() = risk_score;
        *self.last_cross_strategy_risk.write() = cross_strategy_risk.clone();
        *self.last_simulation.write() = simulation.clone();
        *self.last_calculation.write() = Utc::now();
        
        // Create risk assessment
        let assessment = RiskAssessment {
            risk_score,
            risk_factors: factors,
            cross_strategy_risk,
            simulation_result: simulation,
            timestamp: Utc::now(),
        };
        
        Ok(assessment)
    }
    
    /// Generate risk factors from metrics
    fn generate_risk_factors(&self, metrics: &RiskMetricsSnapshot) -> Vec<RiskFactor> {
        let config = self.model.config.read();
        let mut factors = Vec::new();
        
        // Market volatility factor
        let volatility_factor = RiskFactor::new(
            RiskFactorType::MarketVolatility,
            "Market Volatility".to_string(),
            metrics.volatility,
            config.factor_weights.get(&RiskFactorType::MarketVolatility).cloned().unwrap_or(0.25),
            config.factor_thresholds.get(&RiskFactorType::MarketVolatility).cloned().unwrap_or(0.8),
            "Measures market price volatility".to_string(),
        );
        factors.push(volatility_factor);
        
        // Liquidity factor (using spread as proxy)
        let liquidity_factor = RiskFactor::new(
            RiskFactorType::Liquidity,
            "Market Liquidity".to_string(),
            metrics.avg_spread / 100.0, // Normalize to 0-1 range
            config.factor_weights.get(&RiskFactorType::Liquidity).cloned().unwrap_or(0.2),
            config.factor_thresholds.get(&RiskFactorType::Liquidity).cloned().unwrap_or(0.7),
            "Measures market liquidity using spread".to_string(),
        );
        factors.push(liquidity_factor);
        
        // Execution quality factor
        let execution_factor = RiskFactor::new(
            RiskFactorType::ExecutionQuality,
            "Execution Quality".to_string(),
            1.0 - metrics.execution_quality, // Invert so higher is riskier
            config.factor_weights.get(&RiskFactorType::ExecutionQuality).cloned().unwrap_or(0.15),
            config.factor_thresholds.get(&RiskFactorType::ExecutionQuality).cloned().unwrap_or(0.7),
            "Measures quality of trade execution".to_string(),
        );
        factors.push(execution_factor);
        
        // Network congestion factor
        let network_factor = RiskFactor::new(
            RiskFactorType::NetworkCongestion,
            "Network Congestion".to_string(),
            metrics.network_congestion,
            config.factor_weights.get(&RiskFactorType::NetworkCongestion).cloned().unwrap_or(0.1),
            config.factor_thresholds.get(&RiskFactorType::NetworkCongestion).cloned().unwrap_or(0.8),
            "Measures Solana network congestion".to_string(),
        );
        factors.push(network_factor);
        
        // Drawdown factor
        let drawdown_factor = RiskFactor::new(
            RiskFactorType::BlockchainSpecific,
            "Account Drawdown".to_string(),
            metrics.drawdown,
            config.factor_weights.get(&RiskFactorType::BlockchainSpecific).cloned().unwrap_or(0.1),
            config.factor_thresholds.get(&RiskFactorType::BlockchainSpecific).cloned().unwrap_or(0.7),
            "Measures account drawdown".to_string(),
        );
        factors.push(drawdown_factor);
        
        // Exposure factor
        let exposure_factor = RiskFactor::new(
            RiskFactorType::StrategySpecific,
            "Portfolio Exposure".to_string(),
            metrics.total_exposure_pct,
            config.factor_weights.get(&RiskFactorType::StrategySpecific).cloned().unwrap_or(0.1),
            config.factor_thresholds.get(&RiskFactorType::StrategySpecific).cloned().unwrap_or(0.8),
            "Measures total portfolio exposure".to_string(),
        );
        factors.push(exposure_factor);
        
        // Loss streak factor
        let loss_streak_factor = RiskFactor::new(
            RiskFactorType::CounterpartyRisk,
            "Consecutive Losses".to_string(),
            metrics.consecutive_losses as f64 / 10.0, // Normalize to 0-1 range
            config.factor_weights.get(&RiskFactorType::CounterpartyRisk).cloned().unwrap_or(0.05),
            config.factor_thresholds.get(&RiskFactorType::CounterpartyRisk).cloned().unwrap_or(0.6),
            "Measures consecutive losing trades".to_string(),
        );
        factors.push(loss_streak_factor);
        
        factors
    }
    
    /// Get the last risk assessment
    pub fn get_last_assessment(&self) -> RiskAssessment {
        RiskAssessment {
            risk_score: *self.last_risk_score.read(),
            risk_factors: self.last_factors.read().clone(),
            cross_strategy_risk: self.last_cross_strategy_risk.read().clone(),
            simulation_result: self.last_simulation.read().clone(),
            timestamp: *self.last_calculation.read(),
        }
    }
}

/// Risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Overall risk score (0.0-1.0)
    pub risk_score: f64,
    /// Individual risk factors
    pub risk_factors: Vec<RiskFactor>,
    /// Cross-strategy risk analysis
    pub cross_strategy_risk: Option<CrossStrategyRisk>,
    /// Simulation result
    pub simulation_result: Option<SimulationResult>,
    /// Timestamp of the assessment
    pub timestamp: DateTime<Utc>,
}

impl RiskAssessment {
    /// Get risk factors above threshold
    pub fn get_factors_above_threshold(&self) -> Vec<&RiskFactor> {
        self.risk_factors.iter()
            .filter(|f| f.is_above_threshold())
            .collect()
    }
    
    /// Get the risk category
    pub fn get_risk_category(&self) -> RiskCategory {
        let score = self.risk_score * 100.0;
        if score < 30.0 {
            RiskCategory::Low
        } else if score < 60.0 {
            RiskCategory::Medium
        } else if score < 80.0 {
            RiskCategory::High
        } else {
            RiskCategory::VeryHigh
        }
    }
    
    /// Check if risk is acceptable
    pub fn is_acceptable(&self, threshold: f64) -> bool {
        self.risk_score <= threshold
    }
    
    /// Get a summary of the risk assessment
    pub fn get_summary(&self) -> String {
        let category = self.get_risk_category();
        let factors_above = self.get_factors_above_threshold();
        
        let mut summary = format!(
            "Risk Score: {:.2} ({:?})\n",
            self.risk_score * 100.0,
            category
        );
        
        if !factors_above.is_empty() {
            summary.push_str("Risk Factors Above Threshold:\n");
            for factor in factors_above {
                summary.push_str(&format!(
                    "- {}: {:.2} (threshold: {:.2})\n",
                    factor.name,
                    factor.value * 100.0,
                    factor.threshold * 100.0
                ));
            }
        }
        
        if let Some(cross) = &self.cross_strategy_risk {
            summary.push_str(&format!(
                "Cross-Strategy Risk: {:.2}\n",
                cross.cross_strategy_risk_score * 100.0
            ));
            
            if !cross.high_correlation_pairs.is_empty() {
                summary.push_str("Highly Correlated Strategies:\n");
                for (s1, s2, corr) in &cross.high_correlation_pairs {
                    summary.push_str(&format!(
                        "- {} & {}: {:.2}\n",
                        s1, s2, corr
                    ));
                }
            }
        }
        
        if let Some(sim) = &self.simulation_result {
            summary.push_str(&format!(
                "Simulation Results ({} days):\n",
                sim.horizon_days
            ));
            summary.push_str(&format!(
                "- Expected Return: {:.2}%\n",
                sim.avg_return * 100.0
            ));
            summary.push_str(&format!(
                "- Value at Risk ({:.0}%): {:.2}%\n",
                sim.confidence_level * 100.0,
                sim.value_at_risk * 100.0
            ));
            summary.push_str(&format!(
                "- Expected Shortfall: {:.2}%\n",
                sim.expected_shortfall * 100.0
            ));
            summary.push_str(&format!(
                "- Max Drawdown: {:.2}%\n",
                sim.avg_max_drawdown * 100.0
            ));
        }
        
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;
    
    #[test]
    fn test_risk_factor() {
        let factor = RiskFactor::new(
            RiskFactorType::MarketVolatility,
            "Test Factor".to_string(),
            0.75,
            0.2,
            0.7,
            "Test description".to_string(),
        );
        
        assert_eq!(factor.factor_type, RiskFactorType::MarketVolatility);
        assert_eq!(factor.name, "Test Factor");
        assert_eq!(factor.value, 0.75);
        assert_eq!(factor.weight, 0.2);
        assert_eq!(factor.threshold, 0.7);
        assert!(factor.is_above_threshold());
        assert_eq!(factor.weighted_contribution(), 0.75 * 0.2);
    }
    
    #[test]
    fn test_strategy_risk_profile() {
        let mut profile = StrategyRiskProfile::new(
            "test-strategy",
            "Test Strategy",
            "A test strategy",
        );
        
        // Add some returns
        profile.add_return(0.01);
        profile.add_return(0.02);
        profile.add_return(-0.01);
        profile.add_return(0.015);
        profile.add_return(-0.005);
        
        // Calculate metrics
        profile.calculate_metrics();
        
        assert!(profile.volatility > 0.0);
        assert!(profile.max_drawdown > 0.0);
        assert!(profile.risk_score > 0);
    }
    
    #[test]
    fn test_multi_factor_model() {
        let config = MultiFactorModelConfig::default();
        let model = MultiFactorRiskModel::new(config);
        
        // Add a strategy profile
        let mut profile = StrategyRiskProfile::new(
            "test-strategy",
            "Test Strategy",
            "A test strategy",
        );
        
        // Add some returns
        for _ in 0..20 {
            profile.add_return(0.01 * (rand::random::<f64>() - 0.5));
        }
        
        profile.calculate_metrics();
        model.add_strategy_profile(profile);
        
        // Add some risk factors
        let factors = vec![
            RiskFactor::new(
                RiskFactorType::MarketVolatility,
                "Market Volatility".to_string(),
                0.3,
                0.25,
                0.8,
                "Market volatility".to_string(),
            ),
            RiskFactor::new(
                RiskFactorType::Liquidity,
                "Liquidity".to_string(),
                0.2,
                0.2,
                0.7,
                "Market liquidity".to_string(),
            ),
        ];
        
        model.add_risk_factors(factors);
        
        // Calculate overall risk
        let risk_score = model.calculate_overall_risk();
        assert!(risk_score >= 0.0 && risk_score <= 1.0);
    }
    
    #[test]
    fn test_risk_calculator() {
        let config = MultiFactorModelConfig::default();
        let model = Arc::new(MultiFactorRiskModel::new(config));
        let calculator = MultiFactorRiskCalculator::new(model);
        
        // Create a metrics snapshot
        let metrics = RiskMetricsSnapshot {
            capital_usd: 20000.0,
            exposure_usd: 10000.0,
            exposure_percentage: 0.5,
            drawdown: 0.1,
            max_drawdown: 0.15,
            daily_pnl_usd: 500.0,
            weekly_pnl_usd: 2500.0,
            monthly_pnl_usd: 10000.0,
            profit_decline: 0.05,
            volatility: 0.2,
            sharpe_ratio: 1.5,
            sortino_ratio: 2.0,
            calmar_ratio: 2.0,
            consecutive_profits: 2,
            consecutive_losses: 0,
            win_rate: 0.6,
            avg_profit: 250.0,
            avg_loss: 150.0,
            profit_factor: 1.2,
            expected_value: 80.0,
            open_positions: 3,
            largest_position_pct: 0.25,
            largest_position_asset: "SOL".to_string(),
            timestamp: chrono::Utc::now(),
        };
        
        // Create some positions
        let positions = vec![
            Position {
                symbol: "SOL".to_string(),
                size: 10.0,
                avg_price: 100.0,
                market_price: 105.0,
                unrealized_pnl: 50.0,
                realized_pnl: 0.0,
                value_usd: 1050.0,
                strategy_id: "test-strategy".to_string(),
                entry_time: Some(chrono::Utc::now()),
                last_update: Some(chrono::Utc::now()),
            },
        ];
        
        // Calculate risk
        let rt = Runtime::new().unwrap();
        let result = rt.block_on(async {
            calculator.calculate_risk(&metrics, &positions)
        });
        
        assert!(result.is_ok());
        let assessment = result.unwrap();
        
        assert!(assessment.risk_score >= 0.0 && assessment.risk_score <= 1.0);
        assert!(!assessment.risk_factors.is_empty());
        
        // Get last assessment
        let last = calculator.get_last_assessment();
        assert_eq!(last.risk_score, assessment.risk_score);
    }
}