//! Dynamic risk adjustment module for the Solana HFT Bot
//!
//! This module provides dynamic adjustment of risk parameters based on market conditions,
//! trading performance, and network status.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use tokio::sync::RwLock as TokioRwLock;
use tracing::{debug, info, warn};

use crate::metrics::RiskMetricsSnapshot;
use crate::solana_specific::SolanaNetworkStatus;

/// Dynamic adjustment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicAdjustmentConfig {
    /// Whether dynamic adjustment is enabled
    pub enabled: bool,
    
    /// Adjustment interval in seconds
    pub adjustment_interval_secs: u64,
    
    /// Maximum position size adjustment factor
    pub max_position_size_adjustment: f64,
    
    /// Maximum exposure adjustment factor
    pub max_exposure_adjustment: f64,
    
    /// Maximum risk threshold adjustment factor
    pub max_risk_threshold_adjustment: f64,
    
    /// Volatility sensitivity (0-1)
    pub volatility_sensitivity: f64,
    
    /// Network congestion sensitivity (0-1)
    pub network_sensitivity: f64,
    
    /// Performance sensitivity (0-1)
    pub performance_sensitivity: f64,
    
    /// Minimum adjustment interval in seconds
    pub min_adjustment_interval_secs: u64,
}

impl Default for DynamicAdjustmentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            adjustment_interval_secs: 300, // 5 minutes
            max_position_size_adjustment: 0.5, // 50% adjustment
            max_exposure_adjustment: 0.3, // 30% adjustment
            max_risk_threshold_adjustment: 0.2, // 20% adjustment
            volatility_sensitivity: 0.7,
            network_sensitivity: 0.8,
            performance_sensitivity: 0.6,
            min_adjustment_interval_secs: 60, // 1 minute
        }
    }
}

/// Market condition assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConditionAssessment {
    /// Volatility level (0-1, higher is more volatile)
    pub volatility_level: f64,
    
    /// Network congestion level (0-1, higher is more congested)
    pub network_congestion_level: f64,
    
    /// Liquidity level (0-1, higher is more liquid)
    pub liquidity_level: f64,
    
    /// Market trend (-1 to 1, negative is downtrend, positive is uptrend)
    pub market_trend: f64,
    
    /// Correlation level (0-1, higher is more correlated)
    pub correlation_level: f64,
    
    /// Overall market risk level (0-1, higher is riskier)
    pub overall_risk_level: f64,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Performance assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAssessment {
    /// Recent profit/loss trend (-1 to 1, negative is losing, positive is winning)
    pub pnl_trend: f64,
    
    /// Win rate trend (-1 to 1, negative is decreasing, positive is increasing)
    pub win_rate_trend: f64,
    
    /// Sharpe ratio trend (-1 to 1, negative is decreasing, positive is increasing)
    pub sharpe_trend: f64,
    
    /// Drawdown trend (-1 to 1, negative is increasing drawdown, positive is decreasing)
    pub drawdown_trend: f64,
    
    /// Overall performance trend (-1 to 1, negative is deteriorating, positive is improving)
    pub overall_trend: f64,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Risk parameter adjustments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskParameterAdjustments {
    /// Position size adjustment factor
    pub position_size_factor: f64,
    
    /// Exposure limit adjustment factor
    pub exposure_limit_factor: f64,
    
    /// Risk threshold adjustment factor
    pub risk_threshold_factor: f64,
    
    /// Prioritization fee adjustment factor
    pub prioritization_fee_factor: f64,
    
    /// Slippage tolerance adjustment factor
    pub slippage_tolerance_factor: f64,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Reason for adjustment
    pub adjustment_reason: String,
}

impl Default for RiskParameterAdjustments {
    fn default() -> Self {
        Self {
            position_size_factor: 1.0,
            exposure_limit_factor: 1.0,
            risk_threshold_factor: 1.0,
            prioritization_fee_factor: 1.0,
            slippage_tolerance_factor: 1.0,
            timestamp: chrono::Utc::now(),
            adjustment_reason: "Default initialization".to_string(),
        }
    }
}

/// Dynamic risk adjuster
pub struct DynamicRiskAdjuster {
    /// Configuration
    config: RwLock<DynamicAdjustmentConfig>,
    
    /// RPC client
    rpc_client: Arc<RpcClient>,
    
    /// Current adjustments
    current_adjustments: RwLock<RiskParameterAdjustments>,
    
    /// Market condition history
    market_condition_history: TokioRwLock<Vec<MarketConditionAssessment>>,
    
    /// Performance history
    performance_history: TokioRwLock<Vec<PerformanceAssessment>>,
    
    /// Last adjustment time
    last_adjustment_time: RwLock<Instant>,
    
    /// Maximum history size
    max_history_size: usize,
}

impl DynamicRiskAdjuster {
    /// Create a new dynamic risk adjuster
    pub fn new(config: DynamicAdjustmentConfig, rpc_client: Arc<RpcClient>) -> Self {
        Self {
            config: RwLock::new(config),
            rpc_client,
            current_adjustments: RwLock::new(RiskParameterAdjustments::default()),
            market_condition_history: TokioRwLock::new(Vec::new()),
            performance_history: TokioRwLock::new(Vec::new()),
            last_adjustment_time: RwLock::new(Instant::now()),
            max_history_size: 100,
        }
    }
    
    /// Update configuration
    pub fn update_config(&self, config: DynamicAdjustmentConfig) {
        let mut current_config = self.config.write();
        *current_config = config;
    }
    
    /// Get current adjustments
    pub fn get_current_adjustments(&self) -> RiskParameterAdjustments {
        self.current_adjustments.read().clone()
    }
    
    /// Assess market conditions
    pub async fn assess_market_conditions(
        &self,
        metrics: &RiskMetricsSnapshot,
        network_status: Option<&SolanaNetworkStatus>,
    ) -> Result<MarketConditionAssessment> {
        // Assess volatility
        let volatility_level = metrics.volatility.min(0.1) / 0.1; // Normalize to 0-1
        
        // Assess network congestion
        let network_congestion_level = if let Some(status) = network_status {
            status.congestion_level as f64 / 100.0
        } else {
            0.5 // Default to medium congestion if no data
        };
        
        // Assess liquidity (simplified)
        let liquidity_level = 1.0 - (metrics.largest_position_pct.min(0.5) / 0.5); // Normalize to 0-1
        
        // Assess market trend (simplified)
        let market_trend = if metrics.daily_pnl_usd > 0.0 { 0.5 } else { -0.5 };
        
        // Assess correlation (placeholder)
        let correlation_level = 0.5; // Default to medium correlation
        
        // Calculate overall risk level
        let overall_risk_level = (
            volatility_level * 0.3 +
            network_congestion_level * 0.2 +
            (1.0 - liquidity_level) * 0.3 +
            (if market_trend < 0.0 { -market_trend } else { 0.0 }) * 0.1 +
            correlation_level * 0.1
        ).min(1.0);
        
        let assessment = MarketConditionAssessment {
            volatility_level,
            network_congestion_level,
            liquidity_level,
            market_trend,
            correlation_level,
            overall_risk_level,
            timestamp: chrono::Utc::now(),
        };
        
        // Add to history
        let mut history = self.market_condition_history.write().await;
        history.push(assessment.clone());
        
        // Keep maximum size
        while history.len() > self.max_history_size {
            history.remove(0);
        }
        
        Ok(assessment)
    }
    
    /// Assess performance
    pub async fn assess_performance(
        &self,
        current_metrics: &RiskMetricsSnapshot,
        previous_metrics_option: Option<&Option<RiskMetricsSnapshot>>,
    ) -> Result<PerformanceAssessment> {
        // Unwrap the Option<&Option<RiskMetricsSnapshot>> to Option<&RiskMetricsSnapshot>
        let previous_metrics = previous_metrics_option.and_then(|opt| opt.as_ref());
        // Default trends if no previous metrics
        let mut pnl_trend = 0.0;
        let mut win_rate_trend = 0.0;
        let mut sharpe_trend = 0.0;
        let mut drawdown_trend = 0.0;
        
        // Calculate trends if previous metrics available
        if let Some(prev) = previous_metrics {
            // PnL trend
            let pnl_diff = current_metrics.daily_pnl_usd - prev.daily_pnl_usd;
            pnl_trend = if prev.daily_pnl_usd != 0.0 {
                (pnl_diff / prev.daily_pnl_usd.abs()).min(1.0).max(-1.0)
            } else if pnl_diff > 0.0 {
                0.5
            } else if pnl_diff < 0.0 {
                -0.5
            } else {
                0.0
            };
            
            // Win rate trend
            let win_rate_diff = current_metrics.win_rate - prev.win_rate;
            win_rate_trend = (win_rate_diff / 10.0).min(1.0).max(-1.0); // 10% change = full trend
            
            // Sharpe trend
            let sharpe_diff = current_metrics.sharpe_ratio - prev.sharpe_ratio;
            sharpe_trend = (sharpe_diff / 0.5).min(1.0).max(-1.0); // 0.5 change = full trend
            
            // Drawdown trend
            let drawdown_diff = prev.drawdown - current_metrics.drawdown;
            drawdown_trend = (drawdown_diff / 0.05).min(1.0).max(-1.0); // 5% change = full trend
        } else {
            // Set trends based on current metrics only
            pnl_trend = if current_metrics.daily_pnl_usd > 0.0 { 0.5 } else { -0.5 };
            win_rate_trend = if current_metrics.win_rate > 50.0 { 0.5 } else { -0.5 };
            sharpe_trend = if current_metrics.sharpe_ratio > 1.0 { 0.5 } else { -0.5 };
            drawdown_trend = if current_metrics.drawdown < 0.1 { 0.5 } else { -0.5 };
        }
        
        // Calculate overall performance trend
        let overall_trend = (
            pnl_trend * 0.3 +
            win_rate_trend * 0.2 +
            sharpe_trend * 0.3 +
            drawdown_trend * 0.2
        ).min(1.0).max(-1.0);
        
        let assessment = PerformanceAssessment {
            pnl_trend,
            win_rate_trend,
            sharpe_trend,
            drawdown_trend,
            overall_trend,
            timestamp: chrono::Utc::now(),
        };
        
        // Add to history
        let mut history = self.performance_history.write().await;
        history.push(assessment.clone());
        
        // Keep maximum size
        while history.len() > self.max_history_size {
            history.remove(0);
        }
        
        Ok(assessment)
    }
    
    /// Calculate risk parameter adjustments
    pub async fn calculate_adjustments(
        &self,
        market_assessment: &MarketConditionAssessment,
        performance_assessment: &PerformanceAssessment,
    ) -> Result<RiskParameterAdjustments> {
        let config = self.config.read();
        
        if !config.enabled {
            return Ok(RiskParameterAdjustments::default());
        }
        
        // Check if enough time has passed since last adjustment
        let now = Instant::now();
        let mut last_adjustment_time = self.last_adjustment_time.write();
        let elapsed = now.duration_since(*last_adjustment_time);
        
        if elapsed < Duration::from_secs(config.min_adjustment_interval_secs) {
            return Ok(self.current_adjustments.read().clone());
        }
        
        // Calculate position size adjustment
        let position_size_factor = self.calculate_position_size_factor(
            market_assessment,
            performance_assessment,
            &config,
        );
        
        // Calculate exposure limit adjustment
        let exposure_limit_factor = self.calculate_exposure_limit_factor(
            market_assessment,
            performance_assessment,
            &config,
        );
        
        // Calculate risk threshold adjustment
        let risk_threshold_factor = self.calculate_risk_threshold_factor(
            market_assessment,
            performance_assessment,
            &config,
        );
        
        // Calculate prioritization fee adjustment
        let prioritization_fee_factor = self.calculate_prioritization_fee_factor(
            market_assessment,
            &config,
        );
        
        // Calculate slippage tolerance adjustment
        let slippage_tolerance_factor = self.calculate_slippage_tolerance_factor(
            market_assessment,
            &config,
        );
        
        // Determine adjustment reason
        let adjustment_reason = if market_assessment.overall_risk_level > 0.7 {
            "High market risk conditions".to_string()
        } else if market_assessment.network_congestion_level > 0.8 {
            "High network congestion".to_string()
        } else if performance_assessment.overall_trend < -0.5 {
            "Deteriorating performance".to_string()
        } else if performance_assessment.overall_trend > 0.5 {
            "Improving performance".to_string()
        } else {
            "Regular adjustment based on market conditions".to_string()
        };
        
        let adjustments = RiskParameterAdjustments {
            position_size_factor,
            exposure_limit_factor,
            risk_threshold_factor,
            prioritization_fee_factor,
            slippage_tolerance_factor,
            timestamp: chrono::Utc::now(),
            adjustment_reason,
        };
        
        // Update current adjustments
        *self.current_adjustments.write() = adjustments.clone();
        
        // Update last adjustment time
        *last_adjustment_time = now;
        
        Ok(adjustments)
    }
    
    /// Calculate position size adjustment factor
    fn calculate_position_size_factor(
        &self,
        market: &MarketConditionAssessment,
        performance: &PerformanceAssessment,
        config: &DynamicAdjustmentConfig,
    ) -> f64 {
        // Start with neutral factor
        let mut factor = 1.0;
        
        // Adjust based on market volatility
        let volatility_adjustment = (0.5 - market.volatility_level) * config.volatility_sensitivity;
        factor += volatility_adjustment;
        
        // Adjust based on network congestion
        let network_adjustment = (0.5 - market.network_congestion_level) * config.network_sensitivity;
        factor += network_adjustment;
        
        // Adjust based on performance trend
        let performance_adjustment = performance.overall_trend * config.performance_sensitivity * 0.5;
        factor += performance_adjustment;
        
        // Limit the adjustment range
        let max_adjustment = config.max_position_size_adjustment;
        factor = factor.max(1.0 - max_adjustment).min(1.0 + max_adjustment);
        
        factor
    }
    
    /// Calculate exposure limit adjustment factor
    fn calculate_exposure_limit_factor(
        &self,
        market: &MarketConditionAssessment,
        performance: &PerformanceAssessment,
        config: &DynamicAdjustmentConfig,
    ) -> f64 {
        // Start with neutral factor
        let mut factor = 1.0;
        
        // Adjust based on overall risk level
        let risk_adjustment = (0.5 - market.overall_risk_level) * config.volatility_sensitivity;
        factor += risk_adjustment;
        
        // Adjust based on performance trend
        let performance_adjustment = performance.overall_trend * config.performance_sensitivity * 0.3;
        factor += performance_adjustment;
        
        // Limit the adjustment range
        let max_adjustment = config.max_exposure_adjustment;
        factor = factor.max(1.0 - max_adjustment).min(1.0 + max_adjustment);
        
        factor
    }
    
    /// Calculate risk threshold adjustment factor
    fn calculate_risk_threshold_factor(
        &self,
        market: &MarketConditionAssessment,
        performance: &PerformanceAssessment,
        config: &DynamicAdjustmentConfig,
    ) -> f64 {
        // Start with neutral factor
        let mut factor = 1.0;
        
        // Adjust based on overall risk level
        let risk_adjustment = (0.5 - market.overall_risk_level) * config.volatility_sensitivity * 0.5;
        factor += risk_adjustment;
        
        // Adjust based on performance trend
        let performance_adjustment = performance.overall_trend * config.performance_sensitivity * 0.2;
        factor += performance_adjustment;
        
        // Limit the adjustment range
        let max_adjustment = config.max_risk_threshold_adjustment;
        factor = factor.max(1.0 - max_adjustment).min(1.0 + max_adjustment);
        
        factor
    }
    
    /// Calculate prioritization fee adjustment factor
    fn calculate_prioritization_fee_factor(
        &self,
        market: &MarketConditionAssessment,
        config: &DynamicAdjustmentConfig,
    ) -> f64 {
        // Start with neutral factor
        let mut factor = 1.0;
        
        // Adjust based on network congestion
        let network_adjustment = market.network_congestion_level * config.network_sensitivity;
        factor += network_adjustment;
        
        // Limit the adjustment range
        factor = factor.max(1.0).min(3.0); // Allow up to 3x increase but no decrease
        
        factor
    }
    
    /// Calculate slippage tolerance adjustment factor
    fn calculate_slippage_tolerance_factor(
        &self,
        market: &MarketConditionAssessment,
        config: &DynamicAdjustmentConfig,
    ) -> f64 {
        // Start with neutral factor
        let mut factor = 1.0;
        
        // Adjust based on volatility
        let volatility_adjustment = market.volatility_level * config.volatility_sensitivity;
        factor += volatility_adjustment;
        
        // Adjust based on liquidity
        let liquidity_adjustment = (1.0 - market.liquidity_level) * 0.5;
        factor += liquidity_adjustment;
        
        // Limit the adjustment range
        factor = factor.max(1.0).min(2.5); // Allow up to 2.5x increase but no decrease
        
        factor
    }
    
    /// Apply adjustments to risk parameters
    pub fn apply_adjustments<T: RiskParameters>(
        &self,
        params: &mut T,
        adjustments: &RiskParameterAdjustments,
    ) {
        params.apply_adjustments(adjustments);
    }
    
    /// Get market condition history
    pub async fn get_market_condition_history(&self) -> Vec<MarketConditionAssessment> {
        self.market_condition_history.read().await.clone()
    }
    
    /// Get performance history
    pub async fn get_performance_history(&self) -> Vec<PerformanceAssessment> {
        self.performance_history.read().await.clone()
    }
    
    /// Reset to default adjustments
    pub fn reset_adjustments(&self) {
        *self.current_adjustments.write() = RiskParameterAdjustments::default();
    }
}

/// Trait for risk parameters that can be dynamically adjusted
pub trait RiskParameters {
    /// Apply adjustments to risk parameters
    fn apply_adjustments(&mut self, adjustments: &RiskParameterAdjustments);
}

/// Basic risk parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicRiskParameters {
    /// Maximum position size in USD
    pub max_position_size_usd: f64,
    
    /// Maximum exposure percentage
    pub max_exposure_pct: f64,
    
    /// Maximum risk score
    pub max_risk_score: u8,
    
    /// Base prioritization fee (micro-lamports)
    pub base_prioritization_fee: u64,
    
    /// Slippage tolerance percentage
    pub slippage_tolerance_pct: f64,
}

impl Default for BasicRiskParameters {
    fn default() -> Self {
        Self {
            max_position_size_usd: 10000.0,
            max_exposure_pct: 0.8,
            max_risk_score: 70,
            base_prioritization_fee: 1000,
            slippage_tolerance_pct: 0.5,
        }
    }
}

impl RiskParameters for BasicRiskParameters {
    fn apply_adjustments(&mut self, adjustments: &RiskParameterAdjustments) {
        // Apply position size adjustment
        self.max_position_size_usd *= adjustments.position_size_factor;
        
        // Apply exposure limit adjustment
        self.max_exposure_pct *= adjustments.exposure_limit_factor;
        self.max_exposure_pct = self.max_exposure_pct.min(0.95); // Cap at 95%
        
        // Apply risk threshold adjustment
        let new_risk_score = self.max_risk_score as f64 * adjustments.risk_threshold_factor;
        self.max_risk_score = new_risk_score.min(90.0).max(30.0) as u8;
        
        // Apply prioritization fee adjustment
        self.base_prioritization_fee = (self.base_prioritization_fee as f64 * 
            adjustments.prioritization_fee_factor) as u64;
        
        // Apply slippage tolerance adjustment
        self.slippage_tolerance_pct *= adjustments.slippage_tolerance_factor;
        self.slippage_tolerance_pct = self.slippage_tolerance_pct.min(5.0); // Cap at 5%
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_client::rpc_client::RpcClient;
    
    #[test]
    fn test_risk_parameter_adjustments() {
        let mut params = BasicRiskParameters::default();
        
        // Test increasing adjustments
        let increasing_adjustments = RiskParameterAdjustments {
            position_size_factor: 1.2,
            exposure_limit_factor: 1.1,
            risk_threshold_factor: 1.15,
            prioritization_fee_factor: 1.5,
            slippage_tolerance_factor: 1.3,
            timestamp: chrono::Utc::now(),
            adjustment_reason: "Test increasing".to_string(),
        };
        
        params.apply_adjustments(&increasing_adjustments);
        
        assert!(params.max_position_size_usd > 10000.0);
        assert!(params.max_exposure_pct > 0.8);
        assert!(params.max_risk_score > 70);
        assert!(params.base_prioritization_fee > 1000);
        assert!(params.slippage_tolerance_pct > 0.5);
        
        // Reset
        params = BasicRiskParameters::default();
        
        // Test decreasing adjustments
        let decreasing_adjustments = RiskParameterAdjustments {
            position_size_factor: 0.8,
            exposure_limit_factor: 0.9,
            risk_threshold_factor: 0.85,
            prioritization_fee_factor: 1.0, // Can't decrease below 1.0
            slippage_tolerance_factor: 1.0, // Can't decrease below 1.0
            timestamp: chrono::Utc::now(),
            adjustment_reason: "Test decreasing".to_string(),
        };
        
        params.apply_adjustments(&decreasing_adjustments);
        
        assert!(params.max_position_size_usd < 10000.0);
        assert!(params.max_exposure_pct < 0.8);
        assert!(params.max_risk_score < 70);
        assert_eq!(params.base_prioritization_fee, 1000); // Unchanged
        assert_eq!(params.slippage_tolerance_pct, 0.5); // Unchanged
    }
    
    #[test]
    fn test_calculate_adjustment_factors() {
        let config = DynamicAdjustmentConfig::default();
        let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
        
        let adjuster = DynamicRiskAdjuster::new(config, rpc_client);
        
        // Test with high risk market conditions
        let high_risk_market = MarketConditionAssessment {
            volatility_level: 0.8,
            network_congestion_level: 0.9,
            liquidity_level: 0.3,
            market_trend: -0.7,
            correlation_level: 0.8,
            overall_risk_level: 0.85,
            timestamp: chrono::Utc::now(),
        };
        
        let poor_performance = PerformanceAssessment {
            pnl_trend: -0.6,
            win_rate_trend: -0.5,
            sharpe_trend: -0.7,
            drawdown_trend: -0.8,
            overall_trend: -0.65,
            timestamp: chrono::Utc::now(),
        };
        
        let position_size_factor = adjuster.calculate_position_size_factor(
            &high_risk_market,
            &poor_performance,
            &config,
        );
        
        // Should reduce position size
        assert!(position_size_factor < 1.0);
        
        // Test with low risk market conditions
        let low_risk_market = MarketConditionAssessment {
            volatility_level: 0.2,
            network_congestion_level: 0.3,
            liquidity_level: 0.8,
            market_trend: 0.6,
            correlation_level: 0.3,
            overall_risk_level: 0.25,
            timestamp: chrono::Utc::now(),
        };
        
        let good_performance = PerformanceAssessment {
            pnl_trend: 0.7,
            win_rate_trend: 0.6,
            sharpe_trend: 0.8,
            drawdown_trend: 0.5,
            overall_trend: 0.7,
            timestamp: chrono::Utc::now(),
        };
        
        let position_size_factor = adjuster.calculate_position_size_factor(
            &low_risk_market,
            &good_performance,
            &config,
        );
        
        // Should increase position size
        assert!(position_size_factor > 1.0);
    }
}