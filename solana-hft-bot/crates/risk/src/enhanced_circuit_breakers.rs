//! Enhanced circuit breakers with cascading trigger levels for the Solana HFT Bot
//!
//! This module extends the basic circuit breaker functionality with:
//! - Multi-level cascading triggers with progressive severity
//! - Correlation-aware circuit breakers that monitor related metrics
//! - Time-weighted circuit breakers that consider historical patterns
//! - Automatic recovery mechanisms with configurable cool-down periods

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::circuit_breakers::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
use crate::metrics::RiskMetricsSnapshot;

/// Severity level for circuit breakers
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SeverityLevel {
    /// Informational level - no action required
    Info = 0,
    /// Warning level - increased monitoring
    Warning = 1,
    /// Caution level - reduce position sizes
    Caution = 2,
    /// Danger level - stop opening new positions
    Danger = 3,
    /// Critical level - emergency shutdown
    Critical = 4,
}

impl From<u8> for SeverityLevel {
    fn from(value: u8) -> Self {
        match value {
            0 => SeverityLevel::Info,
            1 => SeverityLevel::Warning,
            2 => SeverityLevel::Caution,
            3 => SeverityLevel::Danger,
            _ => SeverityLevel::Critical,
        }
    }
}

/// Trigger level for cascading circuit breakers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerLevel {
    /// Severity level
    pub severity: SeverityLevel,
    /// Threshold value
    pub threshold: f64,
    /// Duration the threshold must be exceeded
    pub duration: Option<Duration>,
    /// Cooldown period before downgrading severity
    pub cooldown: Duration,
    /// Actions to take when triggered
    pub actions: Vec<TriggerAction>,
    /// Description of this trigger level
    pub description: String,
}

/// Action to take when a trigger level is reached
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerAction {
    /// Log a message
    Log(String),
    /// Reduce position sizes by a percentage
    ReducePositions(f64),
    /// Set maximum position size
    SetMaxPosition(f64),
    /// Stop opening new positions
    StopNewPositions,
    /// Close all positions
    CloseAllPositions,
    /// Notify administrators
    Notify(String),
    /// Trigger another circuit breaker
    TriggerBreaker(String),
    /// Custom action with parameters
    Custom(String, HashMap<String, String>),
}

/// Enhanced circuit breaker with cascading trigger levels
pub struct EnhancedCircuitBreaker {
    /// Base circuit breaker configuration
    pub base_config: CircuitBreakerConfig,
    /// Trigger levels in ascending order of severity
    pub trigger_levels: Vec<TriggerLevel>,
    /// Current severity level
    current_severity: RwLock<SeverityLevel>,
    /// Time when current severity was triggered
    severity_triggered_at: RwLock<HashMap<SeverityLevel, Instant>>,
    /// Historical values with timestamps
    historical_values: RwLock<VecDeque<(Instant, f64)>>,
    /// Maximum history size
    max_history_size: usize,
    /// Last update time
    last_update: RwLock<Instant>,
    /// Last value
    last_value: RwLock<f64>,
    /// Whether the breaker is enabled
    enabled: RwLock<bool>,
}

impl EnhancedCircuitBreaker {
    /// Create a new enhanced circuit breaker
    pub fn new(base_config: CircuitBreakerConfig, trigger_levels: Vec<TriggerLevel>) -> Self {
        // Ensure trigger levels are sorted by severity
        let mut sorted_levels = trigger_levels;
        sorted_levels.sort_by_key(|level| level.severity);
        
        let mut severity_map = HashMap::new();
        for level in &sorted_levels {
            severity_map.insert(level.severity, Instant::now() - Duration::from_secs(86400));
        }
        
        Self {
            base_config,
            trigger_levels: sorted_levels,
            current_severity: RwLock::new(SeverityLevel::Info),
            severity_triggered_at: RwLock::new(severity_map),
            historical_values: RwLock::new(VecDeque::with_capacity(100)),
            max_history_size: 100,
            last_update: RwLock::new(Instant::now()),
            last_value: RwLock::new(0.0),
            enabled: RwLock::new(true),
        }
    }
    
    /// Get the current severity level
    pub fn get_severity(&self) -> SeverityLevel {
        *self.current_severity.read()
    }
    
    /// Get the last value
    pub fn get_last_value(&self) -> f64 {
        *self.last_value.read()
    }
    
    /// Check if the circuit breaker is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }
    
    /// Enable the circuit breaker
    pub fn enable(&self) {
        *self.enabled.write() = true;
        info!("Enhanced circuit breaker {} enabled", self.base_config.name);
    }
    
    /// Disable the circuit breaker
    pub fn disable(&self) {
        *self.enabled.write() = false;
        info!("Enhanced circuit breaker {} disabled", self.base_config.name);
    }
    
    /// Update the circuit breaker with a new value
    pub fn update(&self, value: f64) -> Option<SeverityLevel> {
        if !*self.enabled.read() {
            return None;
        }
        
        let now = Instant::now();
        let mut last_value = self.last_value.write();
        *last_value = value;
        *self.last_update.write() = now;
        
        // Add to historical values
        let mut historical_values = self.historical_values.write();
        historical_values.push_back((now, value));
        
        // Trim history if needed
        while historical_values.len() > self.max_history_size {
            historical_values.pop_front();
        }
        
        // Check trigger levels
        let mut current_severity = self.current_severity.write();
        let mut severity_triggered_at = self.severity_triggered_at.write();
        let old_severity = *current_severity;
        
        // First check if we need to increase severity
        for level in &self.trigger_levels {
            if value >= level.threshold && level.severity > *current_severity {
                // Check duration requirement if specified
                if let Some(required_duration) = level.duration {
                    // Check if we have enough history
                    let duration_met = historical_values.iter()
                        .rev()
                        .take_while(|(t, v)| *v >= level.threshold && now.duration_since(*t) <= required_duration)
                        .count() > 0;
                    
                    if !duration_met {
                        continue;
                    }
                }
                
                // Update severity
                *current_severity = level.severity;
                severity_triggered_at.insert(level.severity, now);
                
                info!("Enhanced circuit breaker {} severity increased to {:?}: value={}, threshold={}",
                      self.base_config.name, level.severity, value, level.threshold);
                
                // Execute actions for this level
                self.execute_actions(level);
            }
        }
        
        // Then check if we can decrease severity
        if *current_severity > SeverityLevel::Info {
            // Find the current level
            if let Some(current_level) = self.trigger_levels.iter()
                .find(|level| level.severity == *current_severity) {
                
                // Check if cooldown has elapsed and value is below threshold
                if let Some(triggered_at) = severity_triggered_at.get(&current_level.severity) {
                    let elapsed = now.duration_since(*triggered_at);
                    
                    if elapsed >= current_level.cooldown && value < current_level.threshold {
                        // Find the next lower severity that's still triggered
                        let mut new_severity = SeverityLevel::Info;
                        
                        for level in &self.trigger_levels {
                            if level.severity < *current_severity && value >= level.threshold && level.severity > new_severity {
                                new_severity = level.severity;
                            }
                        }
                        
                        // Update severity
                        *current_severity = new_severity;
                        
                        info!("Enhanced circuit breaker {} severity decreased to {:?}: value={}, cooldown elapsed",
                              self.base_config.name, new_severity, value);
                    }
                }
            }
        }
        
        // Return the new severity if it changed
        if old_severity != *current_severity {
            Some(*current_severity)
        } else {
            None
        }
    }
    
    /// Execute actions for a trigger level
    fn execute_actions(&self, level: &TriggerLevel) {
        for action in &level.actions {
            match action {
                TriggerAction::Log(message) => {
                    info!("Circuit breaker action: {} - {}", self.base_config.name, message);
                },
                TriggerAction::ReducePositions(percentage) => {
                    info!("Circuit breaker action: {} - Reduce positions by {}%", 
                          self.base_config.name, percentage * 100.0);
                    // In a real implementation, this would call into position management
                },
                TriggerAction::SetMaxPosition(size) => {
                    info!("Circuit breaker action: {} - Set max position to {}", 
                          self.base_config.name, size);
                    // In a real implementation, this would update risk parameters
                },
                TriggerAction::StopNewPositions => {
                    info!("Circuit breaker action: {} - Stop opening new positions", 
                          self.base_config.name);
                    // In a real implementation, this would update trading flags
                },
                TriggerAction::CloseAllPositions => {
                    info!("Circuit breaker action: {} - Close all positions", 
                          self.base_config.name);
                    // In a real implementation, this would initiate position closing
                },
                TriggerAction::Notify(recipient) => {
                    info!("Circuit breaker action: {} - Notify {}", 
                          self.base_config.name, recipient);
                    // In a real implementation, this would send notifications
                },
                TriggerAction::TriggerBreaker(breaker_id) => {
                    info!("Circuit breaker action: {} - Trigger breaker {}", 
                          self.base_config.name, breaker_id);
                    // In a real implementation, this would trigger another breaker
                },
                TriggerAction::Custom(name, params) => {
                    info!("Circuit breaker action: {} - Custom action: {} with params: {:?}", 
                          self.base_config.name, name, params);
                    // In a real implementation, this would execute custom logic
                },
            }
        }
    }
    
    /// Reset the circuit breaker
    pub fn reset(&self) {
        *self.current_severity.write() = SeverityLevel::Info;
        info!("Enhanced circuit breaker {} reset to Info level", self.base_config.name);
    }
    
    /// Get time-weighted average value over a specified duration
    pub fn get_time_weighted_average(&self, duration: Duration) -> Option<f64> {
        let now = Instant::now();
        let historical_values = self.historical_values.read();
        
        let relevant_values: Vec<(Instant, f64)> = historical_values.iter()
            .filter(|(t, _)| now.duration_since(*t) <= duration)
            .cloned()
            .collect();
        
        if relevant_values.is_empty() {
            return None;
        }
        
        // Calculate time-weighted average
        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;
        
        for i in 1..relevant_values.len() {
            let (t1, v1) = relevant_values[i-1];
            let (t2, v2) = relevant_values[i];
            
            let time_diff = t2.duration_since(t1).as_secs_f64();
            let avg_value = (v1 + v2) / 2.0;
            
            weighted_sum += avg_value * time_diff;
            total_weight += time_diff;
        }
        
        if total_weight > 0.0 {
            Some(weighted_sum / total_weight)
        } else {
            // If only one value or all values at same time, return simple average
            let sum: f64 = relevant_values.iter().map(|(_, v)| *v).sum();
            Some(sum / relevant_values.len() as f64)
        }
    }
}

/// Enhanced circuit breaker manager with correlation awareness
pub struct EnhancedCircuitBreakerManager {
    /// Enhanced circuit breakers
    breakers: RwLock<HashMap<String, Arc<EnhancedCircuitBreaker>>>,
    /// Correlation groups
    correlation_groups: RwLock<HashMap<String, Vec<String>>>,
    /// Correlation thresholds
    correlation_thresholds: RwLock<HashMap<String, f64>>,
}

impl EnhancedCircuitBreakerManager {
    /// Create a new enhanced circuit breaker manager
    pub fn new() -> Self {
        Self {
            breakers: RwLock::new(HashMap::new()),
            correlation_groups: RwLock::new(HashMap::new()),
            correlation_thresholds: RwLock::new(HashMap::new()),
        }
    }
    
    /// Add an enhanced circuit breaker
    pub fn add_breaker(&self, breaker: Arc<EnhancedCircuitBreaker>) {
        let mut breakers = self.breakers.write();
        breakers.insert(breaker.base_config.id.clone(), breaker);
    }
    
    /// Get an enhanced circuit breaker by ID
    pub fn get_breaker(&self, id: &str) -> Option<Arc<EnhancedCircuitBreaker>> {
        let breakers = self.breakers.read();
        breakers.get(id).cloned()
    }
    
    /// Remove an enhanced circuit breaker
    pub fn remove_breaker(&self, id: &str) -> bool {
        let mut breakers = self.breakers.write();
        breakers.remove(id).is_some()
    }
    
    /// Define a correlation group
    pub fn define_correlation_group(&self, group_name: &str, breaker_ids: Vec<String>, threshold: f64) {
        let mut groups = self.correlation_groups.write();
        let mut thresholds = self.correlation_thresholds.write();
        
        groups.insert(group_name.to_string(), breaker_ids);
        thresholds.insert(group_name.to_string(), threshold);
        
        info!("Defined correlation group '{}' with threshold {}", group_name, threshold);
    }
    
    /// Update all enhanced circuit breakers with metrics
    pub fn update_all(&self, metrics: &RiskMetricsSnapshot) -> HashMap<String, SeverityLevel> {
        let breakers = self.breakers.read();
        let mut severity_changes = HashMap::new();
        
        // Update individual breakers
        for (id, breaker) in breakers.iter() {
            let severity_change = match id.as_str() {
                "drawdown" => breaker.update(metrics.drawdown),
                "volatility" => breaker.update(metrics.volatility),
                "exposure" => breaker.update(metrics.total_exposure_pct),
                "loss_streak" => breaker.update(metrics.consecutive_losses as f64),
                "daily_loss" => breaker.update(-metrics.daily_pnl_usd),
                "sharpe_ratio" => breaker.update(-metrics.sharpe_ratio),
                "win_rate" => breaker.update(-metrics.win_rate),
                _ => {
                    warn!("Unknown enhanced circuit breaker ID: {}", id);
                    None
                }
            };
            
            if let Some(new_severity) = severity_change {
                severity_changes.insert(id.clone(), new_severity);
            }
        }
        
        // Check correlation groups
        self.check_correlations();
        
        severity_changes
    }
    
    /// Check correlations between circuit breakers in defined groups
    fn check_correlations(&self) {
        let groups = self.correlation_groups.read();
        let thresholds = self.correlation_thresholds.read();
        let breakers = self.breakers.read();
        
        for (group_name, breaker_ids) in groups.iter() {
            let threshold = thresholds.get(group_name).cloned().unwrap_or(0.7);
            
            // Get breakers in this group
            let group_breakers: Vec<Arc<EnhancedCircuitBreaker>> = breaker_ids.iter()
                .filter_map(|id| breakers.get(id).cloned())
                .collect();
            
            if group_breakers.len() < 2 {
                continue; // Need at least 2 breakers to correlate
            }
            
            // Count how many breakers are in elevated severity
            let elevated_count = group_breakers.iter()
                .filter(|b| b.get_severity() >= SeverityLevel::Warning)
                .count();
            
            // Calculate correlation percentage
            let correlation = elevated_count as f64 / group_breakers.len() as f64;
            
            // If correlation exceeds threshold, elevate all breakers in group
            if correlation >= threshold {
                info!("Correlation threshold exceeded in group '{}': {:.2} >= {:.2}",
                      group_name, correlation, threshold);
                
                // Find the highest severity in the group
                let max_severity = group_breakers.iter()
                    .map(|b| b.get_severity())
                    .max()
                    .unwrap_or(SeverityLevel::Info);
                
                // If at least Warning level, elevate all breakers to at least Warning
                if max_severity >= SeverityLevel::Warning {
                    for breaker in &group_breakers {
                        if breaker.get_severity() < SeverityLevel::Warning {
                            // Simulate a value that would trigger Warning level
                            if let Some(warning_level) = breaker.trigger_levels.iter()
                                .find(|l| l.severity == SeverityLevel::Warning) {
                                breaker.update(warning_level.threshold);
                                
                                info!("Elevated breaker {} to Warning due to correlation in group '{}'",
                                      breaker.base_config.name, group_name);
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Get all enhanced circuit breakers
    pub fn get_all_breakers(&self) -> Vec<Arc<EnhancedCircuitBreaker>> {
        let breakers = self.breakers.read();
        breakers.values().cloned().collect()
    }
    
    /// Get all breakers at or above a specified severity level
    pub fn get_breakers_at_severity(&self, min_severity: SeverityLevel) -> Vec<Arc<EnhancedCircuitBreaker>> {
        let breakers = self.breakers.read();
        breakers.values()
            .filter(|b| b.get_severity() >= min_severity)
            .cloned()
            .collect()
    }
    
    /// Reset all circuit breakers
    pub fn reset_all(&self) -> usize {
        let breakers = self.breakers.read();
        let mut reset_count = 0;
        
        for breaker in breakers.values() {
            breaker.reset();
            reset_count += 1;
        }
        
        reset_count
    }
}

/// Initialize default enhanced circuit breakers
pub fn init_default_enhanced_circuit_breakers(manager: &EnhancedCircuitBreakerManager) {
    // Drawdown circuit breaker with multiple levels
    let drawdown_base_config = CircuitBreakerConfig {
        id: "drawdown".to_string(),
        name: "Drawdown Circuit Breaker".to_string(),
        description: "Monitors account drawdown with multiple severity levels".to_string(),
        enabled: true,
        threshold: 0.15, // Base threshold (not used in enhanced version)
        cooldown_seconds: 3600,
        auto_reset: false,
    };
    
    let drawdown_levels = vec![
        TriggerLevel {
            severity: SeverityLevel::Warning,
            threshold: 0.05, // 5% drawdown
            duration: Some(Duration::from_secs(300)), // Must persist for 5 minutes
            cooldown: Duration::from_secs(1800), // 30 minutes cooldown
            actions: vec![
                TriggerAction::Log("Drawdown exceeds 5% - increasing monitoring".to_string()),
                TriggerAction::Notify("risk_team".to_string()),
            ],
            description: "Warning level for drawdown".to_string(),
        },
        TriggerLevel {
            severity: SeverityLevel::Caution,
            threshold: 0.10, // 10% drawdown
            duration: Some(Duration::from_secs(60)), // Must persist for 1 minute
            cooldown: Duration::from_secs(3600), // 1 hour cooldown
            actions: vec![
                TriggerAction::Log("Drawdown exceeds 10% - reducing position sizes".to_string()),
                TriggerAction::ReducePositions(0.25), // Reduce positions by 25%
                TriggerAction::Notify("risk_team,trading_desk".to_string()),
            ],
            description: "Caution level for drawdown".to_string(),
        },
        TriggerLevel {
            severity: SeverityLevel::Danger,
            threshold: 0.15, // 15% drawdown
            duration: None, // Trigger immediately
            cooldown: Duration::from_secs(7200), // 2 hours cooldown
            actions: vec![
                TriggerAction::Log("Drawdown exceeds 15% - stopping new positions".to_string()),
                TriggerAction::StopNewPositions,
                TriggerAction::Notify("risk_team,trading_desk,management".to_string()),
            ],
            description: "Danger level for drawdown".to_string(),
        },
        TriggerLevel {
            severity: SeverityLevel::Critical,
            threshold: 0.20, // 20% drawdown
            duration: None, // Trigger immediately
            cooldown: Duration::from_secs(86400), // 24 hours cooldown
            actions: vec![
                TriggerAction::Log("Drawdown exceeds 20% - emergency shutdown".to_string()),
                TriggerAction::CloseAllPositions,
                TriggerAction::Notify("risk_team,trading_desk,management,emergency".to_string()),
            ],
            description: "Critical level for drawdown".to_string(),
        },
    ];
    
    let drawdown_breaker = Arc::new(EnhancedCircuitBreaker::new(
        drawdown_base_config,
        drawdown_levels,
    ));
    
    // Volatility circuit breaker with multiple levels
    let volatility_base_config = CircuitBreakerConfig {
        id: "volatility".to_string(),
        name: "Volatility Circuit Breaker".to_string(),
        description: "Monitors market volatility with multiple severity levels".to_string(),
        enabled: true,
        threshold: 0.05, // Base threshold (not used in enhanced version)
        cooldown_seconds: 1800,
        auto_reset: true,
    };
    
    let volatility_levels = vec![
        TriggerLevel {
            severity: SeverityLevel::Warning,
            threshold: 0.03, // 3% volatility
            duration: Some(Duration::from_secs(300)), // Must persist for 5 minutes
            cooldown: Duration::from_secs(900), // 15 minutes cooldown
            actions: vec![
                TriggerAction::Log("Volatility exceeds 3% - increasing monitoring".to_string()),
            ],
            description: "Warning level for volatility".to_string(),
        },
        TriggerLevel {
            severity: SeverityLevel::Caution,
            threshold: 0.05, // 5% volatility
            duration: Some(Duration::from_secs(60)), // Must persist for 1 minute
            cooldown: Duration::from_secs(1800), // 30 minutes cooldown
            actions: vec![
                TriggerAction::Log("Volatility exceeds 5% - reducing position sizes".to_string()),
                TriggerAction::ReducePositions(0.2), // Reduce positions by 20%
            ],
            description: "Caution level for volatility".to_string(),
        },
        TriggerLevel {
            severity: SeverityLevel::Danger,
            threshold: 0.08, // 8% volatility
            duration: None, // Trigger immediately
            cooldown: Duration::from_secs(3600), // 1 hour cooldown
            actions: vec![
                TriggerAction::Log("Volatility exceeds 8% - stopping new positions".to_string()),
                TriggerAction::StopNewPositions,
                TriggerAction::Notify("risk_team,trading_desk".to_string()),
            ],
            description: "Danger level for volatility".to_string(),
        },
    ];
    
    let volatility_breaker = Arc::new(EnhancedCircuitBreaker::new(
        volatility_base_config,
        volatility_levels,
    ));
    
    // Loss streak circuit breaker with multiple levels
    let loss_streak_base_config = CircuitBreakerConfig {
        id: "loss_streak".to_string(),
        name: "Loss Streak Circuit Breaker".to_string(),
        description: "Monitors consecutive losses with multiple severity levels".to_string(),
        enabled: true,
        threshold: 5.0, // Base threshold (not used in enhanced version)
        cooldown_seconds: 7200,
        auto_reset: false,
    };
    
    let loss_streak_levels = vec![
        TriggerLevel {
            severity: SeverityLevel::Warning,
            threshold: 3.0, // 3 consecutive losses
            duration: None, // Trigger immediately
            cooldown: Duration::from_secs(1800), // 30 minutes cooldown
            actions: vec![
                TriggerAction::Log("3 consecutive losses - increasing monitoring".to_string()),
                TriggerAction::Notify("risk_team".to_string()),
            ],
            description: "Warning level for loss streak".to_string(),
        },
        TriggerLevel {
            severity: SeverityLevel::Caution,
            threshold: 5.0, // 5 consecutive losses
            duration: None, // Trigger immediately
            cooldown: Duration::from_secs(3600), // 1 hour cooldown
            actions: vec![
                TriggerAction::Log("5 consecutive losses - reducing position sizes".to_string()),
                TriggerAction::ReducePositions(0.3), // Reduce positions by 30%
                TriggerAction::Notify("risk_team,trading_desk".to_string()),
            ],
            description: "Caution level for loss streak".to_string(),
        },
        TriggerLevel {
            severity: SeverityLevel::Danger,
            threshold: 8.0, // 8 consecutive losses
            duration: None, // Trigger immediately
            cooldown: Duration::from_secs(7200), // 2 hours cooldown
            actions: vec![
                TriggerAction::Log("8 consecutive losses - stopping new positions".to_string()),
                TriggerAction::StopNewPositions,
                TriggerAction::Notify("risk_team,trading_desk,management".to_string()),
            ],
            description: "Danger level for loss streak".to_string(),
        },
        TriggerLevel {
            severity: SeverityLevel::Critical,
            threshold: 10.0, // 10 consecutive losses
            duration: None, // Trigger immediately
            cooldown: Duration::from_secs(14400), // 4 hours cooldown
            actions: vec![
                TriggerAction::Log("10 consecutive losses - emergency shutdown".to_string()),
                TriggerAction::CloseAllPositions,
                TriggerAction::Notify("risk_team,trading_desk,management,emergency".to_string()),
            ],
            description: "Critical level for loss streak".to_string(),
        },
    ];
    
    let loss_streak_breaker = Arc::new(EnhancedCircuitBreaker::new(
        loss_streak_base_config,
        loss_streak_levels,
    ));
    
    // Add circuit breakers to manager
    manager.add_breaker(drawdown_breaker);
    manager.add_breaker(volatility_breaker);
    manager.add_breaker(loss_streak_breaker);
    
    // Define correlation groups
    manager.define_correlation_group(
        "market_stress",
        vec!["drawdown".to_string(), "volatility".to_string()],
        0.5, // If 50% of breakers in this group are triggered, correlate them
    );
    
    manager.define_correlation_group(
        "trading_performance",
        vec!["drawdown".to_string(), "loss_streak".to_string()],
        0.5, // If 50% of breakers in this group are triggered, correlate them
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_enhanced_circuit_breaker_levels() {
        let base_config = CircuitBreakerConfig {
            id: "test".to_string(),
            name: "Test Breaker".to_string(),
            description: "Test circuit breaker".to_string(),
            enabled: true,
            threshold: 10.0,
            cooldown_seconds: 60,
            auto_reset: false,
        };
        
        let levels = vec![
            TriggerLevel {
                severity: SeverityLevel::Warning,
                threshold: 5.0,
                duration: None,
                cooldown: Duration::from_secs(60),
                actions: vec![TriggerAction::Log("Warning".to_string())],
                description: "Warning level".to_string(),
            },
            TriggerLevel {
                severity: SeverityLevel::Danger,
                threshold: 10.0,
                duration: None,
                cooldown: Duration::from_secs(60),
                actions: vec![TriggerAction::Log("Danger".to_string())],
                description: "Danger level".to_string(),
            },
        ];
        
        let breaker = EnhancedCircuitBreaker::new(base_config, levels);
        
        // Initial state
        assert_eq!(breaker.get_severity(), SeverityLevel::Info);
        
        // Update with value below any threshold
        assert_eq!(breaker.update(3.0), None);
        assert_eq!(breaker.get_severity(), SeverityLevel::Info);
        
        // Update with value above warning threshold
        assert_eq!(breaker.update(6.0), Some(SeverityLevel::Warning));
        assert_eq!(breaker.get_severity(), SeverityLevel::Warning);
        
        // Update with value above danger threshold
        assert_eq!(breaker.update(12.0), Some(SeverityLevel::Danger));
        assert_eq!(breaker.get_severity(), SeverityLevel::Danger);
        
        // Update with value below thresholds (should not change until cooldown)
        assert_eq!(breaker.update(3.0), None);
        assert_eq!(breaker.get_severity(), SeverityLevel::Danger);
        
        // Reset
        breaker.reset();
        assert_eq!(breaker.get_severity(), SeverityLevel::Info);
    }
    
    #[test]
    fn test_enhanced_circuit_breaker_manager() {
        let manager = EnhancedCircuitBreakerManager::new();
        
        // Add some circuit breakers
        let base_config1 = CircuitBreakerConfig {
            id: "test1".to_string(),
            name: "Test Breaker 1".to_string(),
            description: "Test circuit breaker 1".to_string(),
            enabled: true,
            threshold: 10.0,
            cooldown_seconds: 60,
            auto_reset: false,
        };
        
        let levels1 = vec![
            TriggerLevel {
                severity: SeverityLevel::Warning,
                threshold: 5.0,
                duration: None,
                cooldown: Duration::from_secs(60),
                actions: vec![TriggerAction::Log("Warning".to_string())],
                description: "Warning level".to_string(),
            },
        ];
        
        let base_config2 = CircuitBreakerConfig {
            id: "test2".to_string(),
            name: "Test Breaker 2".to_string(),
            description: "Test circuit breaker 2".to_string(),
            enabled: true,
            threshold: 20.0,
            cooldown_seconds: 60,
            auto_reset: false,
        };
        
        let levels2 = vec![
            TriggerLevel {
                severity: SeverityLevel::Warning,
                threshold: 15.0,
                duration: None,
                cooldown: Duration::from_secs(60),
                actions: vec![TriggerAction::Log("Warning".to_string())],
                description: "Warning level".to_string(),
            },
        ];
        
        let breaker1 = Arc::new(EnhancedCircuitBreaker::new(base_config1, levels1));
        let breaker2 = Arc::new(EnhancedCircuitBreaker::new(base_config2, levels2));
        
        manager.add_breaker(breaker1.clone());
        manager.add_breaker(breaker2.clone());
        
        // Define a correlation group
        manager.define_correlation_group(
            "test_group",
            vec!["test1".to_string(), "test2".to_string()],
            0.5,
        );
        
        // Get breakers
        let retrieved1 = manager.get_breaker("test1");
        let retrieved2 = manager.get_breaker("test2");
        
        assert!(retrieved1.is_some());
        assert!(retrieved2.is_some());
        
        // Trigger a breaker
        retrieved1.unwrap().update(6.0);
        
        // Get breakers at warning level
        let warning_breakers = manager.get_breakers_at_severity(SeverityLevel::Warning);
        assert_eq!(warning_breakers.len(), 1);
        
        // Reset all
        let reset_count = manager.reset_all();
        assert_eq!(reset_count, 2);
    }
}