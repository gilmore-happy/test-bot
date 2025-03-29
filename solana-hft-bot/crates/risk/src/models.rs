//! Risk models for the Solana HFT Bot
//!
//! This module provides various risk models for assessing and managing risk.

use std::collections::VecDeque;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Risk metrics for models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    /// Value at Risk (VaR)
    pub value_at_risk: f64,
    
    /// Expected Shortfall (ES)
    pub expected_shortfall: f64,
    
    /// Sharpe ratio
    pub sharpe_ratio: f64,
    
    /// Sortino ratio
    pub sortino_ratio: f64,
    
    /// Maximum drawdown
    pub max_drawdown: f64,
    
    /// Win rate
    pub win_rate: f64,
    
    /// Profit factor
    pub profit_factor: f64,
    
    /// Kelly criterion
    pub kelly_criterion: f64,
}

/// Risk model trait
pub trait RiskModel: Send + Sync {
    /// Calculate risk metrics
    fn calculate_metrics(&self, returns: &[f64]) -> Result<RiskMetrics>;
    
    /// Calculate position size
    fn calculate_position_size(&self, max_position: f64, returns: &[f64]) -> Result<f64>;
    
    /// Calculate risk score (0-100)
    fn calculate_risk_score(&self, metrics: &RiskMetrics) -> u8;
    
    /// Get model name
    fn name(&self) -> &str;
}

/// Value at Risk (VaR) model
pub struct ValueAtRiskModel {
    /// Confidence level (e.g., 0.95 for 95% confidence)
    confidence_level: f64,
    
    /// Model name
    name: String,
}

impl ValueAtRiskModel {
    /// Create a new Value at Risk model
    pub fn new(confidence_level: f64) -> Self {
        Self {
            confidence_level,
            name: format!("VaR({})", confidence_level),
        }
    }
    
    /// Calculate Value at Risk
    fn calculate_var(&self, returns: &[f64]) -> Result<f64> {
        if returns.is_empty() {
            return Err(anyhow!("Empty returns data"));
        }
        
        // Sort returns in ascending order
        let mut sorted_returns = returns.to_vec();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        // Find the index corresponding to the confidence level
        let index = ((1.0 - self.confidence_level) * returns.len() as f64).floor() as usize;
        
        // Return the value at that index
        Ok(-sorted_returns[index.min(returns.len() - 1)])
    }
    
    /// Calculate Expected Shortfall
    fn calculate_es(&self, returns: &[f64]) -> Result<f64> {
        if returns.is_empty() {
            return Err(anyhow!("Empty returns data"));
        }
        
        // Sort returns in ascending order
        let mut sorted_returns = returns.to_vec();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        // Find the index corresponding to the confidence level
        let index = ((1.0 - self.confidence_level) * returns.len() as f64).floor() as usize;
        
        // Calculate the average of returns below the VaR
        let tail_sum: f64 = sorted_returns.iter().take(index + 1).sum();
        let es = -tail_sum / (index + 1) as f64;
        
        Ok(es)
    }
}

impl RiskModel for ValueAtRiskModel {
    fn calculate_metrics(&self, returns: &[f64]) -> Result<RiskMetrics> {
        if returns.is_empty() {
            return Err(anyhow!("Empty returns data"));
        }
        
        // Calculate VaR
        let var = self.calculate_var(returns)?;
        
        // Calculate ES
        let es = self.calculate_es(returns)?;
        
        // Calculate mean return
        let mean_return: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        
        // Calculate standard deviation
        let variance: f64 = returns.iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>() / returns.len() as f64;
        let std_dev = variance.sqrt();
        
        // Calculate Sharpe ratio (assuming risk-free rate of 0)
        let sharpe_ratio = if std_dev != 0.0 {
            mean_return / std_dev
        } else {
            0.0
        };
        
        // Calculate Sortino ratio (downside deviation)
        let downside_returns: Vec<f64> = returns.iter()
            .filter(|&&r| r < 0.0)
            .cloned()
            .collect();
        
        let downside_deviation = if !downside_returns.is_empty() {
            let sum_squared_downside = downside_returns.iter()
                .map(|r| r.powi(2))
                .sum::<f64>();
            (sum_squared_downside / downside_returns.len() as f64).sqrt()
        } else {
            0.0
        };
        
        let sortino_ratio = if downside_deviation != 0.0 {
            mean_return / downside_deviation
        } else {
            0.0
        };
        
        // Calculate maximum drawdown
        let mut max_drawdown = 0.0;
        let mut peak = 0.0;
        let mut cumulative = 0.0;
        
        for &ret in returns {
            cumulative += ret;
            if cumulative > peak {
                peak = cumulative;
            } else {
                let drawdown = peak - cumulative;
                if drawdown > max_drawdown {
                    max_drawdown = drawdown;
                }
            }
        }
        
        // Calculate win rate
        let wins = returns.iter().filter(|&&r| r > 0.0).count();
        let win_rate = wins as f64 / returns.len() as f64 * 100.0;
        
        // Calculate profit factor
        let gross_profit: f64 = returns.iter().filter(|&&r| r > 0.0).sum();
        let gross_loss: f64 = returns.iter().filter(|&&r| r < 0.0).sum();
        
        let profit_factor = if gross_loss != 0.0 {
            gross_profit / -gross_loss
        } else {
            if gross_profit > 0.0 { f64::MAX } else { 0.0 }
        };
        
        // Calculate Kelly criterion
        let win_probability = wins as f64 / returns.len() as f64;
        let avg_win: f64 = if wins > 0 {
            returns.iter().filter(|&&r| r > 0.0).sum::<f64>() / wins as f64
        } else {
            0.0
        };
        
        let avg_loss: f64 = if returns.len() - wins > 0 {
            returns.iter().filter(|&&r| r < 0.0).sum::<f64>() / (returns.len() - wins) as f64
        } else {
            0.0
        };
        
        let kelly_criterion = if avg_loss != 0.0 {
            win_probability - (1.0 - win_probability) / (avg_win / -avg_loss)
        } else {
            0.0
        };
        
        Ok(RiskMetrics {
            value_at_risk: var,
            expected_shortfall: es,
            sharpe_ratio,
            sortino_ratio,
            max_drawdown,
            win_rate,
            profit_factor,
            kelly_criterion,
        })
    }
    
    fn calculate_position_size(&self, max_position: f64, returns: &[f64]) -> Result<f64> {
        if returns.is_empty() {
            return Err(anyhow!("Empty returns data"));
        }
        
        // Calculate VaR
        let var = self.calculate_var(returns)?;
        
        // Calculate position size based on VaR
        // Limit risk to 2% of max position
        let risk_limit = max_position * 0.02;
        
        // Position size = risk limit / VaR
        let position_size = if var > 0.0 {
            risk_limit / var
        } else {
            max_position
        };
        
        // Cap at max position
        Ok(position_size.min(max_position))
    }
    
    fn calculate_risk_score(&self, metrics: &RiskMetrics) -> u8 {
        // Higher VaR, ES, and drawdown increase risk
        // Higher Sharpe, Sortino, win rate, and profit factor decrease risk
        
        let var_score = (metrics.value_at_risk * 100.0).min(100.0);
        let es_score = (metrics.expected_shortfall * 100.0).min(100.0);
        let drawdown_score = (metrics.max_drawdown * 100.0).min(100.0);
        
        let sharpe_score = 100.0 - (metrics.sharpe_ratio * 20.0).min(100.0).max(0.0);
        let sortino_score = 100.0 - (metrics.sortino_ratio * 20.0).min(100.0).max(0.0);
        let win_rate_score = 100.0 - metrics.win_rate;
        let profit_factor_score = 100.0 - (metrics.profit_factor * 20.0).min(100.0).max(0.0);
        
        // Weighted average
        let score = (
            var_score * 0.2 +
            es_score * 0.2 +
            drawdown_score * 0.2 +
            sharpe_score * 0.1 +
            sortino_score * 0.1 +
            win_rate_score * 0.1 +
            profit_factor_score * 0.1
        ).min(100.0).max(0.0);
        
        score as u8
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Kelly Criterion model
pub struct KellyCriterionModel {
    /// Fraction of Kelly to use (0-1)
    kelly_fraction: f64,
    
    /// Model name
    name: String,
}

impl KellyCriterionModel {
    /// Create a new Kelly Criterion model
    pub fn new(kelly_fraction: f64) -> Self {
        Self {
            kelly_fraction,
            name: format!("Kelly({})", kelly_fraction),
        }
    }
}

impl RiskModel for KellyCriterionModel {
    fn calculate_metrics(&self, returns: &[f64]) -> Result<RiskMetrics> {
        // Reuse VaR model for most metrics
        let var_model = ValueAtRiskModel::new(0.95);
        let metrics = var_model.calculate_metrics(returns)?;
        
        Ok(metrics)
    }
    
    fn calculate_position_size(&self, max_position: f64, returns: &[f64]) -> Result<f64> {
        if returns.is_empty() {
            return Err(anyhow!("Empty returns data"));
        }
        
        // Calculate win probability
        let wins = returns.iter().filter(|&&r| r > 0.0).count();
        let win_probability = wins as f64 / returns.len() as f64;
        
        // Calculate average win and loss
        let avg_win: f64 = if wins > 0 {
            returns.iter().filter(|&&r| r > 0.0).sum::<f64>() / wins as f64
        } else {
            0.0
        };
        
        let avg_loss: f64 = if returns.len() - wins > 0 {
            returns.iter().filter(|&&r| r < 0.0).sum::<f64>() / (returns.len() - wins) as f64
        } else {
            0.0
        };
        
        // Calculate Kelly criterion
        let kelly = if avg_loss != 0.0 {
            win_probability - (1.0 - win_probability) / (avg_win / -avg_loss)
        } else {
            0.0
        };
        
        // Apply Kelly fraction and cap at max position
        let position_size = max_position * kelly * self.kelly_fraction;
        
        // Ensure position size is positive and capped
        Ok(position_size.max(0.0).min(max_position))
    }
    
    fn calculate_risk_score(&self, metrics: &RiskMetrics) -> u8 {
        // Similar to VaR model but with more weight on Kelly criterion
        let var_model = ValueAtRiskModel::new(0.95);
        let base_score = var_model.calculate_risk_score(metrics);
        
        // Adjust based on Kelly criterion
        let kelly_score = 100 - (metrics.kelly_criterion * 100.0).min(100.0).max(0.0) as u8;
        
        // Weighted average
        ((base_score as f64 * 0.7) + (kelly_score as f64 * 0.3)) as u8
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Expected Shortfall model
pub struct ExpectedShortfallModel {
    /// Confidence level (e.g., 0.95 for 95% confidence)
    confidence_level: f64,
    
    /// Model name
    name: String,
}

impl ExpectedShortfallModel {
    /// Create a new Expected Shortfall model
    pub fn new(confidence_level: f64) -> Self {
        Self {
            confidence_level,
            name: format!("ES({})", confidence_level),
        }
    }
    
    /// Calculate Expected Shortfall
    fn calculate_es(&self, returns: &[f64]) -> Result<f64> {
        if returns.is_empty() {
            return Err(anyhow!("Empty returns data"));
        }
        
        // Sort returns in ascending order
        let mut sorted_returns = returns.to_vec();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        // Find the index corresponding to the confidence level
        let index = ((1.0 - self.confidence_level) * returns.len() as f64).floor() as usize;
        
        // Calculate the average of returns below the VaR
        let tail_sum: f64 = sorted_returns.iter().take(index + 1).sum();
        let es = -tail_sum / (index + 1) as f64;
        
        Ok(es)
    }
}

impl RiskModel for ExpectedShortfallModel {
    fn calculate_metrics(&self, returns: &[f64]) -> Result<RiskMetrics> {
        // Reuse VaR model for most metrics
        let var_model = ValueAtRiskModel::new(self.confidence_level);
        let metrics = var_model.calculate_metrics(returns)?;
        
        Ok(metrics)
    }
    
    fn calculate_position_size(&self, max_position: f64, returns: &[f64]) -> Result<f64> {
        if returns.is_empty() {
            return Err(anyhow!("Empty returns data"));
        }
        
        // Calculate ES
        let es = self.calculate_es(returns)?;
        
        // Calculate position size based on ES
        // Limit risk to 2% of max position
        let risk_limit = max_position * 0.02;
        
        // Position size = risk limit / ES
        let position_size = if es > 0.0 {
            risk_limit / es
        } else {
            max_position
        };
        
        // Cap at max position
        Ok(position_size.min(max_position))
    }
    
    fn calculate_risk_score(&self, metrics: &RiskMetrics) -> u8 {
        // Similar to VaR model but with more weight on Expected Shortfall
        let var_model = ValueAtRiskModel::new(self.confidence_level);
        let base_score = var_model.calculate_risk_score(metrics);
        
        // Adjust based on ES
        let es_score = (metrics.expected_shortfall * 100.0).min(100.0) as u8;
        
        // Weighted average
        ((base_score as f64 * 0.6) + (es_score as f64 * 0.4)) as u8
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_var_model() {
        let returns = vec![0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.01, -0.02, 0.02, -0.01];
        let model = ValueAtRiskModel::new(0.95);
        
        let metrics = model.calculate_metrics(&returns).unwrap();
        
        // VaR should be positive
        assert!(metrics.value_at_risk > 0.0);
        
        // ES should be greater than or equal to VaR
        assert!(metrics.expected_shortfall >= metrics.value_at_risk);
        
        // Position size should be less than or equal to max position
        let max_position = 1000.0;
        let position_size = model.calculate_position_size(max_position, &returns).unwrap();
        assert!(position_size <= max_position);
        
        // Risk score should be between 0 and 100
        let risk_score = model.calculate_risk_score(&metrics);
        assert!(risk_score <= 100);
    }
    
    #[test]
    fn test_kelly_model() {
        let returns = vec![0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.01, -0.02, 0.02, -0.01];
        let model = KellyCriterionModel::new(0.5);
        
        let metrics = model.calculate_metrics(&returns).unwrap();
        
        // Position size should be less than or equal to max position
        let max_position = 1000.0;
        let position_size = model.calculate_position_size(max_position, &returns).unwrap();
        assert!(position_size <= max_position);
        
        // Risk score should be between 0 and 100
        let risk_score = model.calculate_risk_score(&metrics);
        assert!(risk_score <= 100);
    }
    
    #[test]
    fn test_es_model() {
        let returns = vec![0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.01, -0.02, 0.02, -0.01];
        let model = ExpectedShortfallModel::new(0.95);
        
        let metrics = model.calculate_metrics(&returns).unwrap();
        
        // ES should be positive
        assert!(metrics.expected_shortfall > 0.0);
        
        // Position size should be less than or equal to max position
        let max_position = 1000.0;
        let position_size = model.calculate_position_size(max_position, &returns).unwrap();
        assert!(position_size <= max_position);
        
        // Risk score should be between 0 and 100
        let risk_score = model.calculate_risk_score(&metrics);
        assert!(risk_score <= 100);
    }
}