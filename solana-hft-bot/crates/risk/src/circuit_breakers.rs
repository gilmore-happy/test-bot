//! Circuit breakers for the Solana HFT Bot
//!
//! This module provides circuit breaker functionality to automatically
//! halt trading when certain risk thresholds are exceeded.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::metrics::RiskMetricsSnapshot;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    /// Circuit breaker is armed and monitoring
    Armed,
    
    /// Circuit breaker has been triggered
    Triggered,
    
    /// Circuit breaker has been manually disabled
    Disabled,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Unique identifier for the circuit breaker
    pub id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Description of what this circuit breaker monitors
    pub description: String,
    
    /// Whether the circuit breaker is enabled
    pub enabled: bool,
    
    /// Threshold value that triggers the circuit breaker
    pub threshold: f64,
    
    /// Cooldown period in seconds before the circuit breaker can be reset
    pub cooldown_seconds: u64,
    
    /// Whether to automatically reset after cooldown
    pub auto_reset: bool,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Default Circuit Breaker".to_string(),
            description: "Default circuit breaker configuration".to_string(),
            enabled: true,
            threshold: 0.0,
            cooldown_seconds: 300, // 5 minutes
            auto_reset: false,
        }
    }
}

/// Circuit breaker for risk management
pub struct CircuitBreaker {
    /// Configuration
    pub config: CircuitBreakerConfig,
    
    /// Current state
    state: RwLock<CircuitBreakerState>,
    
    /// Last triggered time
    last_triggered: RwLock<Option<Instant>>,
    
    /// Last value
    last_value: RwLock<f64>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: RwLock::new(CircuitBreakerState::Armed),
            last_triggered: RwLock::new(None),
            last_value: RwLock::new(0.0),
        }
    }
    
    /// Check if the circuit breaker is triggered
    pub fn is_triggered(&self) -> bool {
        *self.state.read() == CircuitBreakerState::Triggered
    }
    
    /// Get the current state
    pub fn get_state(&self) -> CircuitBreakerState {
        *self.state.read()
    }
    
    /// Get the last value
    pub fn get_last_value(&self) -> f64 {
        *self.last_value.read()
    }
    
    /// Update the circuit breaker with a new value
    pub fn update(&self, value: f64) -> bool {
        if !self.config.enabled {
            return false;
        }
        
        let mut state = self.state.write();
        let mut last_value = self.last_value.write();
        *last_value = value;
        
        match *state {
            CircuitBreakerState::Armed => {
                if value >= self.config.threshold {
                    *state = CircuitBreakerState::Triggered;
                    *self.last_triggered.write() = Some(Instant::now());
                    info!("Circuit breaker {} triggered: value={}, threshold={}", 
                          self.config.name, value, self.config.threshold);
                    true
                } else {
                    false
                }
            },
            CircuitBreakerState::Triggered => {
                // Check if cooldown period has passed
                if self.config.auto_reset {
                    if let Some(triggered_time) = *self.last_triggered.read() {
                        let elapsed = Instant::now().duration_since(triggered_time);
                        if elapsed >= Duration::from_secs(self.config.cooldown_seconds) {
                            *state = CircuitBreakerState::Armed;
                            info!("Circuit breaker {} auto-reset after cooldown", self.config.name);
                            return false;
                        }
                    }
                }
                false
            },
            CircuitBreakerState::Disabled => false,
        }
    }
    
    /// Reset the circuit breaker
    pub fn reset(&self) -> bool {
        let mut state = self.state.write();
        
        if *state == CircuitBreakerState::Triggered {
            *state = CircuitBreakerState::Armed;
            info!("Circuit breaker {} manually reset", self.config.name);
            true
        } else {
            false
        }
    }
    
    /// Disable the circuit breaker
    pub fn disable(&self) {
        let mut state = self.state.write();
        *state = CircuitBreakerState::Disabled;
        info!("Circuit breaker {} disabled", self.config.name);
    }
    
    /// Enable the circuit breaker
    pub fn enable(&self) {
        let mut state = self.state.write();
        *state = CircuitBreakerState::Armed;
        info!("Circuit breaker {} enabled", self.config.name);
    }
    
    /// Get time since last triggered
    pub fn time_since_triggered(&self) -> Option<Duration> {
        self.last_triggered.read().map(|time| Instant::now().duration_since(time))
    }
    
    /// Check if cooldown period has passed
    pub fn cooldown_elapsed(&self) -> bool {
        if let Some(triggered_time) = *self.last_triggered.read() {
            let elapsed = Instant::now().duration_since(triggered_time);
            elapsed >= Duration::from_secs(self.config.cooldown_seconds)
        } else {
            true
        }
    }
}

/// Circuit breaker manager
pub struct CircuitBreakerManager {
    /// Circuit breakers
    breakers: RwLock<HashMap<String, Arc<CircuitBreaker>>>,
}

impl CircuitBreakerManager {
    /// Create a new circuit breaker manager
    pub fn new() -> Self {
        Self {
            breakers: RwLock::new(HashMap::new()),
        }
    }
    
    /// Add a circuit breaker
    pub fn add_breaker(&self, breaker: Arc<CircuitBreaker>) {
        let mut breakers = self.breakers.write();
        breakers.insert(breaker.config.id.clone(), breaker);
    }
    
    /// Get a circuit breaker by ID
    pub fn get_breaker(&self, id: &str) -> Option<Arc<CircuitBreaker>> {
        let breakers = self.breakers.read();
        breakers.get(id).cloned()
    }
    
    /// Remove a circuit breaker
    pub fn remove_breaker(&self, id: &str) -> bool {
        let mut breakers = self.breakers.write();
        breakers.remove(id).is_some()
    }
    
    /// Update all circuit breakers with metrics
    pub fn update_all(&self, metrics: &RiskMetricsSnapshot) -> HashMap<String, CircuitBreakerState> {
        let breakers = self.breakers.read();
        let mut state_changes = HashMap::new();
        
        for (id, breaker) in breakers.iter() {
            let old_state = breaker.get_state();
            let triggered = match id.as_str() {
                "drawdown" => breaker.update(metrics.drawdown),
                "volatility" => breaker.update(metrics.volatility),
                "exposure" => breaker.update(metrics.total_exposure_pct),
                "loss_streak" => breaker.update(metrics.consecutive_losses as f64),
                "daily_loss" => breaker.update(-metrics.daily_pnl_usd),
                "sharpe_ratio" => breaker.update(-metrics.sharpe_ratio),
                "win_rate" => breaker.update(-metrics.win_rate),
                _ => {
                    warn!("Unknown circuit breaker ID: {}", id);
                    false
                }
            };
            
            let new_state = breaker.get_state();
            if old_state != new_state {
                state_changes.insert(id.clone(), new_state);
            }
        }
        
        state_changes
    }
    
    /// Check if any circuit breakers are triggered
    pub fn any_triggered(&self) -> bool {
        let breakers = self.breakers.read();
        breakers.values().any(|breaker| breaker.is_triggered())
    }
    
    /// Get all triggered circuit breakers
    pub fn get_triggered_breakers(&self) -> Vec<Arc<CircuitBreaker>> {
        let breakers = self.breakers.read();
        breakers.values()
            .filter(|breaker| breaker.is_triggered())
            .cloned()
            .collect()
    }
    
    /// Reset a circuit breaker
    pub fn reset_breaker(&self, id: &str) -> bool {
        if let Some(breaker) = self.get_breaker(id) {
            breaker.reset()
        } else {
            false
        }
    }
    
    /// Reset all circuit breakers
    pub fn reset_all(&self) -> usize {
        let breakers = self.breakers.read();
        let mut reset_count = 0;
        
        for breaker in breakers.values() {
            if breaker.reset() {
                reset_count += 1;
            }
        }
        
        reset_count
    }
    
    /// Get all circuit breakers
    pub fn get_all_breakers(&self) -> Vec<Arc<CircuitBreaker>> {
        let breakers = self.breakers.read();
        breakers.values().cloned().collect()
    }
}

/// Initialize default circuit breakers
pub fn init_default_circuit_breakers(manager: &CircuitBreakerManager) {
    // Drawdown circuit breaker
    let drawdown_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
        id: "drawdown".to_string(),
        name: "Drawdown Circuit Breaker".to_string(),
        description: "Triggers when drawdown exceeds threshold".to_string(),
        enabled: true,
        threshold: 0.15, // 15% drawdown
        cooldown_seconds: 3600, // 1 hour
        auto_reset: false,
    }));
    
    // Volatility circuit breaker
    let volatility_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
        id: "volatility".to_string(),
        name: "Volatility Circuit Breaker".to_string(),
        description: "Triggers when market volatility exceeds threshold".to_string(),
        enabled: true,
        threshold: 0.05, // 5% volatility
        cooldown_seconds: 1800, // 30 minutes
        auto_reset: true,
    }));
    
    // Exposure circuit breaker
    let exposure_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
        id: "exposure".to_string(),
        name: "Exposure Circuit Breaker".to_string(),
        description: "Triggers when total exposure exceeds threshold".to_string(),
        enabled: true,
        threshold: 0.9, // 90% exposure
        cooldown_seconds: 600, // 10 minutes
        auto_reset: true,
    }));
    
    // Loss streak circuit breaker
    let loss_streak_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
        id: "loss_streak".to_string(),
        name: "Loss Streak Circuit Breaker".to_string(),
        description: "Triggers when consecutive losses exceed threshold".to_string(),
        enabled: true,
        threshold: 5.0, // 5 consecutive losses
        cooldown_seconds: 7200, // 2 hours
        auto_reset: false,
    }));
    
    // Daily loss circuit breaker
    let daily_loss_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
        id: "daily_loss".to_string(),
        name: "Daily Loss Circuit Breaker".to_string(),
        description: "Triggers when daily loss exceeds threshold".to_string(),
        enabled: true,
        threshold: 1000.0, // $1000 loss
        cooldown_seconds: 86400, // 24 hours
        auto_reset: true,
    }));
    
    // Add circuit breakers to manager
    manager.add_breaker(drawdown_breaker);
    manager.add_breaker(volatility_breaker);
    manager.add_breaker(exposure_breaker);
    manager.add_breaker(loss_streak_breaker);
    manager.add_breaker(daily_loss_breaker);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_circuit_breaker_trigger() {
        let config = CircuitBreakerConfig {
            id: "test".to_string(),
            name: "Test Breaker".to_string(),
            description: "Test circuit breaker".to_string(),
            enabled: true,
            threshold: 10.0,
            cooldown_seconds: 60,
            auto_reset: false,
        };
        
        let breaker = CircuitBreaker::new(config);
        
        // Should not trigger below threshold
        assert!(!breaker.update(5.0));
        assert_eq!(breaker.get_state(), CircuitBreakerState::Armed);
        
        // Should trigger at threshold
        assert!(breaker.update(10.0));
        assert_eq!(breaker.get_state(), CircuitBreakerState::Triggered);
        
        // Should not trigger again when already triggered
        assert!(!breaker.update(15.0));
        assert_eq!(breaker.get_state(), CircuitBreakerState::Triggered);
        
        // Should reset when manually reset
        assert!(breaker.reset());
        assert_eq!(breaker.get_state(), CircuitBreakerState::Armed);
        
        // Should not trigger when disabled
        breaker.disable();
        assert!(!breaker.update(20.0));
        assert_eq!(breaker.get_state(), CircuitBreakerState::Disabled);
    }
    
    #[test]
    fn test_circuit_breaker_auto_reset() {
        let config = CircuitBreakerConfig {
            id: "test_auto".to_string(),
            name: "Test Auto Reset".to_string(),
            description: "Test auto reset circuit breaker".to_string(),
            enabled: true,
            threshold: 10.0,
            cooldown_seconds: 1, // Short cooldown for testing
            auto_reset: true,
        };
        
        let breaker = CircuitBreaker::new(config);
        
        // Trigger the breaker
        assert!(breaker.update(15.0));
        assert_eq!(breaker.get_state(), CircuitBreakerState::Triggered);
        
        // Wait for cooldown
        std::thread::sleep(Duration::from_secs(2));
        
        // Should auto-reset on next update
        assert!(!breaker.update(5.0));
        assert_eq!(breaker.get_state(), CircuitBreakerState::Armed);
    }
    
    #[test]
    fn test_circuit_breaker_manager() {
        let manager = CircuitBreakerManager::new();
        
        // Add some circuit breakers
        let breaker1 = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
            id: "test1".to_string(),
            name: "Test Breaker 1".to_string(),
            description: "Test circuit breaker 1".to_string(),
            enabled: true,
            threshold: 10.0,
            cooldown_seconds: 60,
            auto_reset: false,
        }));
        
        let breaker2 = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
            id: "test2".to_string(),
            name: "Test Breaker 2".to_string(),
            description: "Test circuit breaker 2".to_string(),
            enabled: true,
            threshold: 20.0,
            cooldown_seconds: 60,
            auto_reset: false,
        }));
        
        manager.add_breaker(breaker1.clone());
        manager.add_breaker(breaker2.clone());
        
        // Get breakers
        let retrieved1 = manager.get_breaker("test1");
        let retrieved2 = manager.get_breaker("test2");
        
        assert!(retrieved1.is_some());
        assert!(retrieved2.is_some());
        
        // Trigger a breaker
        retrieved1.unwrap().update(15.0);
        
        // Check if any are triggered
        assert!(manager.any_triggered());
        
        // Get triggered breakers
        let triggered = manager.get_triggered_breakers();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].config.id, "test1");
        
        // Reset a breaker
        assert!(manager.reset_breaker("test1"));
        
        // Check if any are triggered after reset
        assert!(!manager.any_triggered());
        
        // Remove a breaker
        assert!(manager.remove_breaker("test1"));
        assert!(manager.get_breaker("test1").is_none());
    }
}