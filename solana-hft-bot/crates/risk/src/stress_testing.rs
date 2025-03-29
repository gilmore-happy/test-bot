//! Stress testing module for the Solana HFT Bot
//!
//! This module provides stress testing functionality to evaluate how the trading
//! system would perform under extreme market conditions.

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use tracing::{debug, info, warn};

use crate::models::{RiskModel, RiskMetrics};
use crate::Position;

/// Stress test scenario types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StressScenarioType {
    /// Market crash scenario
    MarketCrash,
    
    /// Liquidity crisis scenario
    LiquidityCrisis,
    
    /// Volatility spike scenario
    VolatilitySpike,
    
    /// Network congestion scenario
    NetworkCongestion,
    
    /// Validator outage scenario
    ValidatorOutage,
    
    /// Flash crash scenario
    FlashCrash,
    
    /// Correlation breakdown scenario
    CorrelationBreakdown,
    
    /// Custom scenario
    Custom,
}

/// Stress test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestConfig {
    /// Whether stress testing is enabled
    pub enabled: bool,
    
    /// Market crash scenario parameters
    pub market_crash: MarketCrashConfig,
    
    /// Liquidity crisis scenario parameters
    pub liquidity_crisis: LiquidityCrisisConfig,
    
    /// Volatility spike scenario parameters
    pub volatility_spike: VolatilitySpikeConfig,
    
    /// Network congestion scenario parameters
    pub network_congestion: NetworkCongestionConfig,
    
    /// Custom scenario parameters
    pub custom: CustomScenarioConfig,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            market_crash: MarketCrashConfig::default(),
            liquidity_crisis: LiquidityCrisisConfig::default(),
            volatility_spike: VolatilitySpikeConfig::default(),
            network_congestion: NetworkCongestionConfig::default(),
            custom: CustomScenarioConfig::default(),
        }
    }
}

/// Market crash scenario configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketCrashConfig {
    /// Price decline percentage
    pub price_decline_pct: f64,
    
    /// Duration in seconds
    pub duration_seconds: u64,
    
    /// Recovery percentage
    pub recovery_pct: f64,
}

impl Default for MarketCrashConfig {
    fn default() -> Self {
        Self {
            price_decline_pct: 30.0,
            duration_seconds: 3600, // 1 hour
            recovery_pct: 10.0,
        }
    }
}

/// Liquidity crisis scenario configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityCrisisConfig {
    /// Liquidity reduction percentage
    pub liquidity_reduction_pct: f64,
    
    /// Slippage increase factor
    pub slippage_increase_factor: f64,
    
    /// Duration in seconds
    pub duration_seconds: u64,
}

impl Default for LiquidityCrisisConfig {
    fn default() -> Self {
        Self {
            liquidity_reduction_pct: 80.0,
            slippage_increase_factor: 5.0,
            duration_seconds: 7200, // 2 hours
        }
    }
}

/// Volatility spike scenario configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolatilitySpikeConfig {
    /// Volatility increase factor
    pub volatility_increase_factor: f64,
    
    /// Price swing percentage
    pub price_swing_pct: f64,
    
    /// Duration in seconds
    pub duration_seconds: u64,
}

impl Default for VolatilitySpikeConfig {
    fn default() -> Self {
        Self {
            volatility_increase_factor: 3.0,
            price_swing_pct: 15.0,
            duration_seconds: 1800, // 30 minutes
        }
    }
}

/// Network congestion scenario configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCongestionConfig {
    /// Transaction confirmation delay factor
    pub confirmation_delay_factor: f64,
    
    /// Transaction failure rate percentage
    pub failure_rate_pct: f64,
    
    /// Prioritization fee increase factor
    pub prioritization_fee_increase_factor: f64,
    
    /// Duration in seconds
    pub duration_seconds: u64,
}

impl Default for NetworkCongestionConfig {
    fn default() -> Self {
        Self {
            confirmation_delay_factor: 5.0,
            failure_rate_pct: 20.0,
            prioritization_fee_increase_factor: 10.0,
            duration_seconds: 3600, // 1 hour
        }
    }
}

/// Custom scenario configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomScenarioConfig {
    /// Price change percentage
    pub price_change_pct: f64,
    
    /// Liquidity change percentage
    pub liquidity_change_pct: f64,
    
    /// Volatility change factor
    pub volatility_change_factor: f64,
    
    /// Network delay factor
    pub network_delay_factor: f64,
    
    /// Duration in seconds
    pub duration_seconds: u64,
}

impl Default for CustomScenarioConfig {
    fn default() -> Self {
        Self {
            price_change_pct: -20.0,
            liquidity_change_pct: -50.0,
            volatility_change_factor: 2.0,
            network_delay_factor: 3.0,
            duration_seconds: 3600, // 1 hour
        }
    }
}

/// Stress test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestResult {
    /// Scenario type
    pub scenario_type: StressScenarioType,
    
    /// Initial portfolio value
    pub initial_portfolio_value: f64,
    
    /// Final portfolio value
    pub final_portfolio_value: f64,
    
    /// Absolute loss
    pub absolute_loss: f64,
    
    /// Percentage loss
    pub percentage_loss: f64,
    
    /// Maximum drawdown
    pub max_drawdown: f64,
    
    /// Risk metrics before stress test
    pub initial_metrics: RiskMetrics,
    
    /// Risk metrics after stress test
    pub final_metrics: RiskMetrics,
    
    /// Position-specific results
    pub position_results: HashMap<String, PositionStressResult>,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Passed the stress test
    pub passed: bool,
    
    /// Failure reason if failed
    pub failure_reason: Option<String>,
    
    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Position-specific stress test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionStressResult {
    /// Symbol
    pub symbol: String,
    
    /// Initial position size
    pub initial_size: f64,
    
    /// Initial position value
    pub initial_value: f64,
    
    /// Final position size
    pub final_size: f64,
    
    /// Final position value
    pub final_value: f64,
    
    /// Absolute loss
    pub absolute_loss: f64,
    
    /// Percentage loss
    pub percentage_loss: f64,
    
    /// Liquidation risk (0-100)
    pub liquidation_risk: u8,
}

/// Stress tester
pub struct StressTester {
    /// RPC client
    rpc_client: Arc<RpcClient>,
    
    /// Configuration
    config: StressTestConfig,
    
    /// Risk models
    risk_models: HashMap<String, Box<dyn RiskModel + Send + Sync>>,
}

impl StressTester {
    /// Create a new stress tester
    pub fn new(
        rpc_client: Arc<RpcClient>,
        config: StressTestConfig,
        risk_models: HashMap<String, Box<dyn RiskModel + Send + Sync>>,
    ) -> Self {
        Self {
            rpc_client,
            config,
            risk_models,
        }
    }
    
    /// Run a stress test
    pub async fn run_stress_test(
        &self,
        positions: &[Position],
        scenario_type: StressScenarioType,
    ) -> Result<StressTestResult> {
        if !self.config.enabled {
            return Err(anyhow!("Stress testing is disabled"));
        }
        
        if positions.is_empty() {
            return Err(anyhow!("No positions to stress test"));
        }
        
        info!("Running stress test for scenario: {:?}", scenario_type);
        
        // Calculate initial portfolio value
        let initial_portfolio_value: f64 = positions.iter()
            .map(|p| p.size * p.market_price)
            .sum();
        
        // Generate simulated returns based on scenario
        let simulated_returns = self.generate_simulated_returns(scenario_type)?;
        
        // Calculate initial risk metrics
        let initial_metrics = if let Some(var_model) = self.risk_models.get("VaR") {
            var_model.calculate_metrics(&simulated_returns)?
        } else {
            return Err(anyhow!("VaR model not found"));
        };
        
        // Apply stress scenario to each position
        let mut position_results = HashMap::new();
        let mut final_portfolio_value = 0.0;
        
        for position in positions {
            let result = self.stress_test_position(position, scenario_type)?;
            final_portfolio_value += result.final_value;
            position_results.insert(position.symbol.clone(), result);
        }
        
        // Calculate final risk metrics
        let stressed_returns = self.generate_stressed_returns(&simulated_returns, scenario_type)?;
        let final_metrics = if let Some(var_model) = self.risk_models.get("VaR") {
            var_model.calculate_metrics(&stressed_returns)?
        } else {
            return Err(anyhow!("VaR model not found"));
        };
        
        // Calculate loss
        let absolute_loss = initial_portfolio_value - final_portfolio_value;
        let percentage_loss = if initial_portfolio_value > 0.0 {
            absolute_loss / initial_portfolio_value * 100.0
        } else {
            0.0
        };
        
        // Calculate maximum drawdown
        let max_drawdown = final_metrics.max_drawdown;
        
        // Determine if the stress test passed
        let (passed, failure_reason) = self.evaluate_stress_test_result(
            percentage_loss,
            max_drawdown,
            &position_results,
            scenario_type,
        );
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(
            &position_results,
            percentage_loss,
            max_drawdown,
            scenario_type,
        );
        
        Ok(StressTestResult {
            scenario_type,
            initial_portfolio_value,
            final_portfolio_value,
            absolute_loss,
            percentage_loss,
            max_drawdown,
            initial_metrics,
            final_metrics,
            position_results,
            timestamp: chrono::Utc::now(),
            passed,
            failure_reason,
            recommendations,
        })
    }
    
    /// Generate simulated returns based on historical data
    fn generate_simulated_returns(&self, scenario_type: StressScenarioType) -> Result<Vec<f64>> {
        // In a real implementation, this would use historical data or Monte Carlo simulation
        // For this example, we'll use a simplified approach with synthetic data
        
        let mut returns = Vec::new();
        
        // Generate base returns (normal market conditions)
        for _ in 0..100 {
            let r = rand::random::<f64>() * 0.02 - 0.01; // Random returns between -1% and 1%
            returns.push(r);
        }
        
        Ok(returns)
    }
    
    /// Generate stressed returns based on scenario
    fn generate_stressed_returns(
        &self,
        base_returns: &[f64],
        scenario_type: StressScenarioType,
    ) -> Result<Vec<f64>> {
        let mut stressed_returns = base_returns.to_vec();
        
        match scenario_type {
            StressScenarioType::MarketCrash => {
                // Add a severe market crash
                let crash_magnitude = -self.config.market_crash.price_decline_pct / 100.0;
                stressed_returns.push(crash_magnitude);
                
                // Add some recovery
                let recovery_magnitude = self.config.market_crash.recovery_pct / 100.0;
                stressed_returns.push(recovery_magnitude);
            },
            StressScenarioType::VolatilitySpike => {
                // Increase volatility
                for r in &mut stressed_returns {
                    *r *= self.config.volatility_spike.volatility_increase_factor;
                }
                
                // Add some extreme swings
                let swing_magnitude = self.config.volatility_spike.price_swing_pct / 100.0;
                stressed_returns.push(swing_magnitude);
                stressed_returns.push(-swing_magnitude);
            },
            StressScenarioType::LiquidityCrisis => {
                // Simulate wider spreads and higher slippage
                for r in &mut stressed_returns {
                    if *r < 0.0 {
                        *r *= self.config.liquidity_crisis.slippage_increase_factor;
                    }
                }
            },
            StressScenarioType::NetworkCongestion | StressScenarioType::ValidatorOutage => {
                // Simulate delayed executions (which can lead to worse prices)
                for r in &mut stressed_returns {
                    if *r < 0.0 {
                        *r *= 1.5; // Worse execution on negative returns
                    }
                }
            },
            StressScenarioType::FlashCrash => {
                // Simulate a flash crash and recovery
                stressed_returns.push(-0.3); // 30% drop
                stressed_returns.push(0.2);  // 20% recovery
            },
            StressScenarioType::CorrelationBreakdown => {
                // Simulate correlation breakdown (more extreme moves)
                for r in &mut stressed_returns {
                    *r *= 2.0;
                }
            },
            StressScenarioType::Custom => {
                // Apply custom scenario parameters
                let price_change = self.config.custom.price_change_pct / 100.0;
                stressed_returns.push(price_change);
                
                // Adjust volatility
                for r in &mut stressed_returns {
                    *r *= self.config.custom.volatility_change_factor;
                }
            },
        }
        
        Ok(stressed_returns)
    }
    
    /// Stress test a single position
    fn stress_test_position(
        &self,
        position: &Position,
        scenario_type: StressScenarioType,
    ) -> Result<PositionStressResult> {
        let initial_size = position.size;
        let initial_value = position.size * position.market_price;
        
        // Apply stress scenario to position
        let (final_size, final_price) = match scenario_type {
            StressScenarioType::MarketCrash => {
                let price_decline = self.config.market_crash.price_decline_pct / 100.0;
                let final_price = position.market_price * (1.0 - price_decline);
                (initial_size, final_price)
            },
            StressScenarioType::LiquidityCrisis => {
                // In a liquidity crisis, we might not be able to exit positions at desired prices
                let slippage_factor = self.config.liquidity_crisis.slippage_increase_factor;
                let final_price = if position.size > 0.0 {
                    // For long positions, price decreases
                    position.market_price * (1.0 - 0.1 * slippage_factor)
                } else {
                    // For short positions, price increases
                    position.market_price * (1.0 + 0.1 * slippage_factor)
                };
                (initial_size, final_price)
            },
            StressScenarioType::VolatilitySpike => {
                let price_swing = self.config.volatility_spike.price_swing_pct / 100.0;
                let final_price = if position.size > 0.0 {
                    // For long positions, assume worst case (price down)
                    position.market_price * (1.0 - price_swing)
                } else {
                    // For short positions, assume worst case (price up)
                    position.market_price * (1.0 + price_swing)
                };
                (initial_size, final_price)
            },
            StressScenarioType::NetworkCongestion | StressScenarioType::ValidatorOutage => {
                // In network issues, we might not be able to exit positions quickly
                let delay_factor = self.config.network_congestion.confirmation_delay_factor;
                let price_impact = 0.05 * delay_factor; // 5% per delay factor
                let final_price = if position.size > 0.0 {
                    position.market_price * (1.0 - price_impact)
                } else {
                    position.market_price * (1.0 + price_impact)
                };
                (initial_size, final_price)
            },
            StressScenarioType::FlashCrash => {
                // Flash crash: 30% drop followed by partial recovery
                let final_price = position.market_price * 0.7 * 1.2; // 30% drop, 20% recovery
                (initial_size, final_price)
            },
            StressScenarioType::CorrelationBreakdown => {
                // Correlation breakdown: assume 15% adverse move
                let final_price = if position.size > 0.0 {
                    position.market_price * 0.85
                } else {
                    position.market_price * 1.15
                };
                (initial_size, final_price)
            },
            StressScenarioType::Custom => {
                let price_change = self.config.custom.price_change_pct / 100.0;
                let final_price = position.market_price * (1.0 + price_change);
                (initial_size, final_price)
            },
        };
        
        let final_value = final_size * final_price;
        let absolute_loss = initial_value - final_value;
        let percentage_loss = if initial_value != 0.0 {
            absolute_loss / initial_value.abs() * 100.0
        } else {
            0.0
        };
        
        // Calculate liquidation risk
        let liquidation_risk = if percentage_loss > 50.0 {
            100
        } else if percentage_loss > 30.0 {
            80
        } else if percentage_loss > 20.0 {
            60
        } else if percentage_loss > 10.0 {
            40
        } else if percentage_loss > 5.0 {
            20
        } else {
            0
        };
        
        Ok(PositionStressResult {
            symbol: position.symbol.clone(),
            initial_size,
            initial_value,
            final_size,
            final_value,
            absolute_loss,
            percentage_loss,
            liquidation_risk,
        })
    }
    
    /// Evaluate if the stress test passed
    fn evaluate_stress_test_result(
        &self,
        percentage_loss: f64,
        max_drawdown: f64,
        position_results: &HashMap<String, PositionStressResult>,
        scenario_type: StressScenarioType,
    ) -> (bool, Option<String>) {
        // Define failure thresholds based on scenario
        let (max_loss_threshold, max_drawdown_threshold) = match scenario_type {
            StressScenarioType::MarketCrash => (50.0, 60.0),
            StressScenarioType::LiquidityCrisis => (40.0, 50.0),
            StressScenarioType::VolatilitySpike => (30.0, 40.0),
            StressScenarioType::NetworkCongestion => (20.0, 30.0),
            StressScenarioType::ValidatorOutage => (25.0, 35.0),
            StressScenarioType::FlashCrash => (35.0, 45.0),
            StressScenarioType::CorrelationBreakdown => (30.0, 40.0),
            StressScenarioType::Custom => (30.0, 40.0),
        };
        
        // Check if any position has high liquidation risk
        let high_liquidation_risk = position_results.values()
            .any(|result| result.liquidation_risk > 80);
        
        if percentage_loss > max_loss_threshold {
            (false, Some(format!("Portfolio loss ({:.2}%) exceeds threshold ({:.2}%)", 
                                percentage_loss, max_loss_threshold)))
        } else if max_drawdown > max_drawdown_threshold {
            (false, Some(format!("Maximum drawdown ({:.2}%) exceeds threshold ({:.2}%)", 
                                max_drawdown * 100.0, max_drawdown_threshold)))
        } else if high_liquidation_risk {
            (false, Some("One or more positions have high liquidation risk".to_string()))
        } else {
            (true, None)
        }
    }
    
    /// Generate recommendations based on stress test results
    fn generate_recommendations(
        &self,
        position_results: &HashMap<String, PositionStressResult>,
        percentage_loss: f64,
        max_drawdown: f64,
        scenario_type: StressScenarioType,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Overall portfolio recommendations
        if percentage_loss > 30.0 {
            recommendations.push("Consider reducing overall portfolio exposure".to_string());
        }
        
        if max_drawdown > 0.4 {
            recommendations.push("Implement stronger drawdown protection measures".to_string());
        }
        
        // Scenario-specific recommendations
        match scenario_type {
            StressScenarioType::MarketCrash => {
                recommendations.push("Maintain higher cash reserves for market crash scenarios".to_string());
                recommendations.push("Consider implementing crash protection strategies like put options".to_string());
            },
            StressScenarioType::LiquidityCrisis => {
                recommendations.push("Focus on more liquid assets to reduce slippage risk".to_string());
                recommendations.push("Implement gradual position sizing to avoid liquidity issues".to_string());
            },
            StressScenarioType::VolatilitySpike => {
                recommendations.push("Reduce position sizes during high volatility periods".to_string());
                recommendations.push("Implement volatility-based position sizing".to_string());
            },
            StressScenarioType::NetworkCongestion | StressScenarioType::ValidatorOutage => {
                recommendations.push("Implement network health monitoring and circuit breakers".to_string());
                recommendations.push("Use higher prioritization fees during network congestion".to_string());
            },
            StressScenarioType::FlashCrash => {
                recommendations.push("Implement flash crash detection and trading pauses".to_string());
                recommendations.push("Avoid using market orders during high volatility".to_string());
            },
            StressScenarioType::CorrelationBreakdown => {
                recommendations.push("Diversify across uncorrelated assets and strategies".to_string());
                recommendations.push("Implement correlation-based position sizing".to_string());
            },
            StressScenarioType::Custom => {
                recommendations.push("Review custom scenario parameters and adjust risk controls".to_string());
            },
        }
        
        // Position-specific recommendations
        for (symbol, result) in position_results {
            if result.percentage_loss > 40.0 {
                recommendations.push(format!("Consider reducing position size for {} (loss: {:.2}%)", 
                                           symbol, result.percentage_loss));
            }
            
            if result.liquidation_risk > 60 {
                recommendations.push(format!("High liquidation risk for {}: implement stronger protections", 
                                           symbol));
            }
        }
        
        recommendations
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: StressTestConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_client::rpc_client::RpcClient;
    use crate::models::ValueAtRiskModel;
    
    #[test]
    fn test_stress_test_position() {
        let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
        
        let mut risk_models = HashMap::new();
        risk_models.insert("VaR".to_string(), Box::new(ValueAtRiskModel::new(0.95)) as Box<dyn RiskModel + Send + Sync>);
        
        let config = StressTestConfig::default();
        let tester = StressTester::new(rpc_client, config, risk_models);
        
        let position = Position {
            symbol: "SOL".to_string(),
            size: 10.0,
            avg_price: 100.0,
            market_price: 100.0,
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            value_usd: 1000.0,
            strategy_id: "test".to_string(),
            entry_time: Some(chrono::Utc::now()),
            last_update: Some(chrono::Utc::now()),
        };
        
        // Test market crash scenario
        let result = tester.stress_test_position(&position, StressScenarioType::MarketCrash).unwrap();
        
        assert_eq!(result.symbol, "SOL");
        assert_eq!(result.initial_size, 10.0);
        assert_eq!(result.initial_value, 1000.0);
        assert!(result.final_value < result.initial_value); // Value should decrease in crash
        assert!(result.percentage_loss > 0.0); // Should have some loss
        
        // Test volatility spike scenario
        let result = tester.stress_test_position(&position, StressScenarioType::VolatilitySpike).unwrap();
        
        assert!(result.percentage_loss > 0.0); // Should have some loss
    }
    
    #[test]
    fn test_generate_simulated_returns() {
        let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
        
        let mut risk_models = HashMap::new();
        risk_models.insert("VaR".to_string(), Box::new(ValueAtRiskModel::new(0.95)) as Box<dyn RiskModel + Send + Sync>);
        
        let config = StressTestConfig::default();
        let tester = StressTester::new(rpc_client, config, risk_models);
        
        let returns = tester.generate_simulated_returns(StressScenarioType::MarketCrash).unwrap();
        
        assert!(!returns.is_empty());
        
        // Test stressed returns
        let stressed_returns = tester.generate_stressed_returns(&returns, StressScenarioType::MarketCrash).unwrap();
        
        assert!(stressed_returns.len() > returns.len()); // Should add some stress events
        
        // Check if there's a large negative return (crash)
        let has_crash = stressed_returns.iter().any(|&r| r < -0.1);
        assert!(has_crash);
    }
}