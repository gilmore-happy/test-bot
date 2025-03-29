use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Trading signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Token mint address
    pub token_mint: Pubkey,
    
    /// Signal type
    pub signal_type: SignalType,
    
    /// Signal strength
    pub strength: SignalStrength,
    
    /// Current price in USD
    pub price: Option<f64>,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Signal source
    pub source: SignalSource,
    
    /// Signal expiration time
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Confidence level (0.0-1.0)
    pub confidence: f64,
    
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Signal type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalType {
    /// Buy signal
    Buy,
    
    /// Sell signal
    Sell,
    
    /// Arbitrage opportunity
    Arbitrage,
    
    /// MEV opportunity
    Mev,
    
    /// Liquidation opportunity
    Liquidation,
}

/// Signal strength
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalStrength {
    /// Weak signal
    Weak,
    
    /// Medium signal
    Medium,
    
    /// Strong signal
    Strong,
}

/// Signal source
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalSource {
    /// Strategy-generated signal
    Strategy(String),
    
    /// External API signal
    ExternalApi(String),
    
    /// Manual signal
    Manual(String),
    
    /// On-chain event signal
    OnChainEvent(String),
}

/// Signal processor
pub struct SignalProcessor {
    /// Active signals
    active_signals: Arc<RwLock<Vec<Signal>>>,
    
    /// Signal history
    signal_history: Arc<RwLock<Vec<Signal>>>,
    
    /// Maximum history size
    max_history_size: usize,
    
    /// Signal handlers
    handlers: Vec<Box<dyn SignalHandler + Send + Sync>>,
    
    /// Whether the processor is running
    running: bool,
}

/// Signal handler trait
#[async_trait::async_trait]
pub trait SignalHandler: Send + Sync {
    /// Handle a signal
    async fn handle_signal(&self, signal: &Signal) -> anyhow::Result<()>;
    
    /// Get handler name
    fn name(&self) -> &str;
}

impl SignalProcessor {
    /// Create a new signal processor
    pub fn new(max_history_size: usize) -> Self {
        Self {
            active_signals: Arc::new(RwLock::new(Vec::new())),
            signal_history: Arc::new(RwLock::new(Vec::new())),
            max_history_size,
            handlers: Vec::new(),
            running: false,
        }
    }
    
    /// Start the signal processor
    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.running {
            return Err(anyhow::anyhow!("Signal processor already running"));
        }
        
        self.running = true;
        info!("Signal processor started");
        
        Ok(())
    }
    
    /// Stop the signal processor
    pub fn stop(&mut self) -> anyhow::Result<()> {
        if !self.running {
            return Err(anyhow::anyhow!("Signal processor not running"));
        }
        
        self.running = false;
        info!("Signal processor stopped");
        
        Ok(())
    }
    
    /// Register a signal handler
    pub fn register_handler<H>(&mut self, handler: H) -> anyhow::Result<()>
    where
        H: SignalHandler + 'static,
    {
        let name = handler.name().to_string();
        
        // Check if handler with this name already exists
        if self.handlers.iter().any(|h| h.name() == name) {
            return Err(anyhow::anyhow!("Handler with name '{}' already registered", name));
        }
        
        self.handlers.push(Box::new(handler));
        info!("Registered signal handler: {}", name);
        
        Ok(())
    }
    
    /// Process a signal
    pub async fn process_signal(&self, signal: Signal) -> anyhow::Result<()> {
        if !self.running {
            return Err(anyhow::anyhow!("Signal processor not running"));
        }
        
        info!("Processing signal: {:?} for token {}", signal.signal_type, signal.token_mint);
        
        // Add to active signals
        {
            let mut active_signals = self.active_signals.write().await;
            active_signals.push(signal.clone());
        }
        
        // Add to history
        {
            let mut history = self.signal_history.write().await;
            history.push(signal.clone());
            
            // Trim history if needed
            if history.len() > self.max_history_size {
                history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                history.truncate(self.max_history_size);
            }
        }
        
        // Dispatch to handlers
        for handler in &self.handlers {
            if let Err(e) = handler.handle_signal(&signal).await {
                error!("Error in signal handler {}: {}", handler.name(), e);
            }
        }
        
        Ok(())
    }
    
    /// Process multiple signals
    pub async fn process_signals(&self, signals: Vec<Signal>) -> anyhow::Result<()> {
        for signal in signals {
            self.process_signal(signal).await?;
        }
        
        Ok(())
    }
    
    /// Clean up expired signals
    pub async fn cleanup_expired_signals(&self) -> anyhow::Result<()> {
        if !self.running {
            return Err(anyhow::anyhow!("Signal processor not running"));
        }
        
        let now = chrono::Utc::now();
        
        let mut active_signals = self.active_signals.write().await;
        
        // Remove expired signals
        let initial_count = active_signals.len();
        active_signals.retain(|signal| {
            if let Some(expiration) = signal.expiration {
                expiration > now
            } else {
                true
            }
        });
        
        let removed_count = initial_count - active_signals.len();
        if removed_count > 0 {
            debug!("Removed {} expired signals", removed_count);
        }
        
        Ok(())
    }
    
    /// Get active signals
    pub async fn get_active_signals(&self) -> anyhow::Result<Vec<Signal>> {
        if !self.running {
            return Err(anyhow::anyhow!("Signal processor not running"));
        }
        
        let active_signals = self.active_signals.read().await;
        Ok(active_signals.clone())
    }
    
    /// Get signal history
    pub async fn get_signal_history(&self) -> anyhow::Result<Vec<Signal>> {
        if !self.running {
            return Err(anyhow::anyhow!("Signal processor not running"));
        }
        
        let history = self.signal_history.read().await;
        Ok(history.clone())
    }
    
    /// Get signals by type
    pub async fn get_signals_by_type(&self, signal_type: SignalType) -> anyhow::Result<Vec<Signal>> {
        if !self.running {
            return Err(anyhow::anyhow!("Signal processor not running"));
        }
        
        let active_signals = self.active_signals.read().await;
        let filtered_signals = active_signals
            .iter()
            .filter(|signal| signal.signal_type == signal_type)
            .cloned()
            .collect();
            
        Ok(filtered_signals)
    }
    
    /// Get signals by token
    pub async fn get_signals_by_token(&self, token_mint: Pubkey) -> anyhow::Result<Vec<Signal>> {
        if !self.running {
            return Err(anyhow::anyhow!("Signal processor not running"));
        }
        
        let active_signals = self.active_signals.read().await;
        let filtered_signals = active_signals
            .iter()
            .filter(|signal| signal.token_mint == token_mint)
            .cloned()
            .collect();
            
        Ok(filtered_signals)
    }
}

/// Default signal handler that logs signals
pub struct LoggingSignalHandler;

#[async_trait::async_trait]
impl SignalHandler for LoggingSignalHandler {
    async fn handle_signal(&self, signal: &Signal) -> anyhow::Result<()> {
        info!(
            "Signal: {:?} {:?} for token {} with confidence {:.2}",
            signal.signal_type,
            signal.strength,
            signal.token_mint,
            signal.confidence
        );
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "LoggingSignalHandler"
    }
}

/// Signal handler that executes trades based on signals
pub struct TradingSignalHandler {
    /// Execution engine
    execution_engine: Arc<solana_hft_execution::ExecutionEngine>,
    
    /// Minimum confidence to execute a trade
    min_confidence: f64,
    
    /// Whether to execute trades automatically
    auto_execute: bool,
}

impl TradingSignalHandler {
    /// Create a new trading signal handler
    pub fn new(
        execution_engine: Arc<solana_hft_execution::ExecutionEngine>,
        min_confidence: f64,
        auto_execute: bool,
    ) -> Self {
        Self {
            execution_engine,
            min_confidence,
            auto_execute,
        }
    }
}

#[async_trait::async_trait]
impl SignalHandler for TradingSignalHandler {
    async fn handle_signal(&self, signal: &Signal) -> anyhow::Result<()> {
        // Skip signals with low confidence
        if signal.confidence < self.min_confidence {
            debug!(
                "Skipping signal with low confidence: {:.2} < {:.2}",
                signal.confidence,
                self.min_confidence
            );
            return Ok(());
        }
        
        // Skip if auto-execute is disabled
        if !self.auto_execute {
            info!(
                "Auto-execute disabled, not executing signal: {:?} for token {}",
                signal.signal_type,
                signal.token_mint
            );
            return Ok(());
        }
        
        // Execute trade based on signal type
        match signal.signal_type {
            SignalType::Buy => {
                info!("Executing buy for token {}", signal.token_mint);
                
                // In a real implementation, this would create and submit an order
                // For now, just log the action
                
                /*
                let order = solana_hft_execution::Order {
                    token_mint: signal.token_mint,
                    side: solana_hft_execution::OrderSide::Buy,
                    // ... other order parameters
                };
                
                self.execution_engine.submit_order(order).await?;
                */
            }
            SignalType::Sell => {
                info!("Executing sell for token {}", signal.token_mint);
                
                // Similar to buy, but with sell side
            }
            SignalType::Arbitrage => {
                info!("Executing arbitrage for token {}", signal.token_mint);
                
                // Extract arbitrage details from metadata
                if let Some(metadata) = &signal.metadata {
                    if let (Some(buy_market), Some(sell_market)) = (
                        metadata.get("buy_market").and_then(|v| v.as_str()),
                        metadata.get("sell_market").and_then(|v| v.as_str()),
                    ) {
                        info!("Arbitrage between {} and {}", buy_market, sell_market);
                        
                        // In a real implementation, this would execute the arbitrage
                    }
                }
            }
            SignalType::Mev => {
                info!("Executing MEV strategy for token {}", signal.token_mint);
                
                // Extract MEV details from metadata
                if let Some(metadata) = &signal.metadata {
                    if let Some(opportunity_type) = metadata.get("opportunity_type").and_then(|v| v.as_str()) {
                        info!("MEV opportunity type: {}", opportunity_type);
                        
                        // In a real implementation, this would execute the MEV strategy
                    }
                }
            }
            SignalType::Liquidation => {
                info!("Executing liquidation for token {}", signal.token_mint);
                
                // In a real implementation, this would execute the liquidation
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "TradingSignalHandler"
    }
}

/// Signal aggregator that combines signals from multiple sources
pub struct SignalAggregator {
    /// Signal processor
    signal_processor: Arc<SignalProcessor>,
    
    /// Aggregation rules
    aggregation_rules: HashMap<SignalType, AggregationRule>,
}

/// Aggregation rule
#[derive(Debug, Clone)]
pub struct AggregationRule {
    /// Minimum number of signals required
    pub min_signals: usize,
    
    /// Minimum average confidence
    pub min_avg_confidence: f64,
    
    /// Maximum age of signals to consider (seconds)
    pub max_age_seconds: u64,
    
    /// Whether to require signals from different sources
    pub require_different_sources: bool,
}

impl SignalAggregator {
    /// Create a new signal aggregator
    pub fn new(signal_processor: Arc<SignalProcessor>) -> Self {
        let mut aggregation_rules = HashMap::new();
        
        // Default rules
        aggregation_rules.insert(SignalType::Buy, AggregationRule {
            min_signals: 2,
            min_avg_confidence: 0.7,
            max_age_seconds: 60,
            require_different_sources: true,
        });
        
        aggregation_rules.insert(SignalType::Sell, AggregationRule {
            min_signals: 2,
            min_avg_confidence: 0.7,
            max_age_seconds: 60,
            require_different_sources: true,
        });
        
        aggregation_rules.insert(SignalType::Arbitrage, AggregationRule {
            min_signals: 1,
            min_avg_confidence: 0.8,
            max_age_seconds: 10,
            require_different_sources: false,
        });
        
        aggregation_rules.insert(SignalType::Mev, AggregationRule {
            min_signals: 1,
            min_avg_confidence: 0.9,
            max_age_seconds: 5,
            require_different_sources: false,
        });
        
        Self {
            signal_processor,
            aggregation_rules,
        }
    }
    
    /// Set aggregation rule for a signal type
    pub fn set_aggregation_rule(&mut self, signal_type: SignalType, rule: AggregationRule) {
        self.aggregation_rules.insert(signal_type, rule);
    }
    
    /// Aggregate signals for a token
    pub async fn aggregate_signals_for_token(&self, token_mint: Pubkey) -> anyhow::Result<Vec<Signal>> {
        let signals = self.signal_processor.get_signals_by_token(token_mint).await?;
        
        let mut aggregated_signals = Vec::new();
        
        // Group signals by type
        let mut signals_by_type: HashMap<SignalType, Vec<&Signal>> = HashMap::new();
        
        for signal in &signals {
            signals_by_type.entry(signal.signal_type).or_default().push(signal);
        }
        
        // Apply aggregation rules to each type
        for (signal_type, signals) in signals_by_type {
            if let Some(rule) = self.aggregation_rules.get(&signal_type) {
                if let Some(aggregated) = self.apply_aggregation_rule(signal_type, &signals, rule) {
                    aggregated_signals.push(aggregated);
                }
            }
        }
        
        Ok(aggregated_signals)
    }
    
    /// Apply aggregation rule to signals
    fn apply_aggregation_rule(
        &self,
        signal_type: SignalType,
        signals: &[&Signal],
        rule: &AggregationRule,
    ) -> Option<Signal> {
        // Filter by age
        let now = chrono::Utc::now();
        let max_age = chrono::Duration::seconds(rule.max_age_seconds as i64);
        
        let recent_signals: Vec<&Signal> = signals
            .iter()
            .filter(|s| now - s.timestamp < max_age)
            .copied()
            .collect();
        
        // Check if we have enough signals
        if recent_signals.len() < rule.min_signals {
            return None;
        }
        
        // Check if we need signals from different sources
        if rule.require_different_sources {
            let unique_sources: std::collections::HashSet<_> = recent_signals
                .iter()
                .map(|s| &s.source)
                .collect();
                
            if unique_sources.len() < rule.min_signals {
                return None;
            }
        }
        
        // Calculate average confidence
        let total_confidence: f64 = recent_signals.iter().map(|s| s.confidence).sum();
        let avg_confidence = total_confidence / recent_signals.len() as f64;
        
        if avg_confidence < rule.min_avg_confidence {
            return None;
        }
        
        // Create aggregated signal
        let strongest_signal = recent_signals
            .iter()
            .max_by_key(|s| match s.strength {
                SignalStrength::Strong => 3,
                SignalStrength::Medium => 2,
                SignalStrength::Weak => 1,
            })
            .unwrap();
            
        let token_mint = strongest_signal.token_mint;
        
        Some(Signal {
            token_mint,
            signal_type,
            strength: strongest_signal.strength,
            price: strongest_signal.price,
            timestamp: now,
            source: SignalSource::Strategy("Aggregator".to_string()),
            expiration: Some(now + chrono::Duration::seconds(60)),
            confidence: avg_confidence,
            metadata: Some(serde_json::json!({
                "aggregated": true,
                "signal_count": recent_signals.len(),
                "avg_confidence": avg_confidence,
                "sources": recent_signals.iter().map(|s| format!("{:?}", s.source)).collect::<Vec<_>>(),
            })),
        })
    }
}