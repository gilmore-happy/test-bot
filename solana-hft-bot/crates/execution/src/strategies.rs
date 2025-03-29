//! Execution strategies
//! 
//! This module defines different strategies for transaction execution.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::Signature,
    transaction::Transaction,
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    config::ExecutionConfig,
    jito::{BundleOptions, JitoClient},
    retry::{retry_transaction, RetryStrategy},
    simulation::{simulate_transaction_with_timeout, TransactionSimulator},
    ExecutionError,
};

/// Strategy type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyType {
    /// Default strategy
    Default,
    
    /// Aggressive strategy
    Aggressive,
    
    /// Conservative strategy
    Conservative,
    
    /// Jito MEV strategy
    JitoMev,
}

/// Execution strategy trait
#[async_trait]
pub trait ExecutionStrategy: Send + Sync {
    /// Get the strategy type
    fn strategy_type(&self) -> StrategyType;
    
    /// Execute a transaction
    async fn execute(
        &self,
        transaction: &Transaction,
        rpc_client: Arc<solana_hft_rpc::EnhancedRpcClient>,
        jito_client: Option<Arc<JitoClient>>,
    ) -> Result<Signature, ExecutionError>;
}

/// Default execution strategy
pub struct DefaultStrategy {
    /// Configuration
    config: ExecutionConfig,
}

impl DefaultStrategy {
    /// Create a new default strategy
    pub fn new(config: ExecutionConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ExecutionStrategy for DefaultStrategy {
    fn strategy_type(&self) -> StrategyType {
        StrategyType::Default
    }
    
    async fn execute(
        &self,
        transaction: &Transaction,
        rpc_client: Arc<solana_hft_rpc::EnhancedRpcClient>,
        jito_client: Option<Arc<JitoClient>>,
    ) -> Result<Signature, ExecutionError> {
        debug!("Executing transaction with default strategy");
        
        // Simulate the transaction if enabled
        if self.config.simulate_transactions {
            let simulator = TransactionSimulator::new(
                rpc_client.clone(),
                self.config.commitment_config,
                false,
            );
            
            let simulation_result = simulate_transaction_with_timeout(
                &simulator,
                transaction,
                Duration::from_millis(1000),
            ).await?;
            
            if !simulation_result.success {
                return Err(ExecutionError::Simulation(
                    simulation_result.error.unwrap_or_else(|| "Simulation failed".to_string())
                ));
            }
        }
        
        // Create retry strategy
        let retry_strategy = RetryStrategy::exponential_backoff(
            Duration::from_millis(self.config.retry_backoff_ms),
            Duration::from_millis(self.config.max_retry_backoff_ms),
            2.0,
            self.config.send_transaction_retry_count,
        );
        
        // Send the transaction with retries
        retry_transaction(|| async {
            let result = rpc_client.send_transaction(transaction).await
                .map_err(|e| ExecutionError::Rpc(e.to_string()));
            
            result
        }, retry_strategy).await
    }
}

/// Aggressive execution strategy
pub struct AggressiveStrategy {
    /// Configuration
    config: ExecutionConfig,
}

impl AggressiveStrategy {
    /// Create a new aggressive strategy
    pub fn new(config: ExecutionConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ExecutionStrategy for AggressiveStrategy {
    fn strategy_type(&self) -> StrategyType {
        StrategyType::Aggressive
    }
    
    async fn execute(
        &self,
        transaction: &Transaction,
        rpc_client: Arc<solana_hft_rpc::EnhancedRpcClient>,
        jito_client: Option<Arc<JitoClient>>,
    ) -> Result<Signature, ExecutionError> {
        debug!("Executing transaction with aggressive strategy");
        
        // Try Jito first if available
        if self.config.use_jito && jito_client.is_some() {
            let jito_client = jito_client.unwrap();
            
            let options = BundleOptions {
                tip_account: self.config.jito_tip_account.as_ref().map(|s| s.parse().unwrap()),
                tip_amount: self.config.jito_tip_amount,
                timeout: Duration::from_millis(self.config.default_transaction_timeout_ms),
                wait_for_confirmation: false,
                max_retries: 2,
                simulate_first: self.config.simulate_transactions,
            };
            
            match jito_client.submit_transaction(transaction, options).await {
                Ok(signature) => {
                    info!("Transaction executed via Jito: {}", signature);
                    return Ok(signature);
                },
                Err(e) => {
                    warn!("Jito execution failed, falling back to RPC: {}", e);
                    // Fall back to RPC
                }
            }
        }
        
        // Create retry strategy with more aggressive retries
        let retry_strategy = RetryStrategy::exponential_backoff(
            Duration::from_millis(50), // Faster initial retry
            Duration::from_millis(500), // Lower max backoff
            1.5, // Lower backoff factor
            self.config.send_transaction_retry_count + 2, // More retries
        );
        
        // Send the transaction with retries
        retry_transaction(|| async {
            let result = rpc_client.send_transaction(transaction).await
                .map_err(|e| ExecutionError::Rpc(e.to_string()));
            
            result
        }, retry_strategy).await
    }
}

/// Conservative execution strategy
pub struct ConservativeStrategy {
    /// Configuration
    config: ExecutionConfig,
}

impl ConservativeStrategy {
    /// Create a new conservative strategy
    pub fn new(config: ExecutionConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ExecutionStrategy for ConservativeStrategy {
    fn strategy_type(&self) -> StrategyType {
        StrategyType::Conservative
    }
    
    async fn execute(
        &self,
        transaction: &Transaction,
        rpc_client: Arc<solana_hft_rpc::EnhancedRpcClient>,
        jito_client: Option<Arc<JitoClient>>,
    ) -> Result<Signature, ExecutionError> {
        debug!("Executing transaction with conservative strategy");
        
        // Always simulate the transaction
        let simulator = TransactionSimulator::new(
            rpc_client.clone(),
            self.config.commitment_config,
            true, // Include accounts
        );
        
        let simulation_result = simulate_transaction_with_timeout(
            &simulator,
            transaction,
            Duration::from_millis(2000), // Longer timeout
        ).await?;
        
        if !simulation_result.success {
            return Err(ExecutionError::Simulation(
                simulation_result.error.unwrap_or_else(|| "Simulation failed".to_string())
            ));
        }
        
        // Create retry strategy with more conservative retries
        let retry_strategy = RetryStrategy::fibonacci_backoff(
            Duration::from_millis(200), // Slower initial retry
            Duration::from_millis(5000), // Higher max backoff
            self.config.send_transaction_retry_count,
        );
        
        // Send the transaction with retries
        retry_transaction(|| async {
            let result = rpc_client.send_transaction(transaction).await
                .map_err(|e| ExecutionError::Rpc(e.to_string()));
            
            result
        }, retry_strategy).await
    }
}

/// Jito MEV strategy
pub struct JitoMevStrategy {
    /// Configuration
    config: ExecutionConfig,
}

impl JitoMevStrategy {
    /// Create a new Jito MEV strategy
    pub fn new(config: ExecutionConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ExecutionStrategy for JitoMevStrategy {
    fn strategy_type(&self) -> StrategyType {
        StrategyType::JitoMev
    }
    
    async fn execute(
        &self,
        transaction: &Transaction,
        rpc_client: Arc<solana_hft_rpc::EnhancedRpcClient>,
        jito_client: Option<Arc<JitoClient>>,
    ) -> Result<Signature, ExecutionError> {
        debug!("Executing transaction with Jito MEV strategy");
        
        // Ensure Jito client is available
        let jito_client = jito_client.ok_or_else(|| {
            ExecutionError::Strategy("Jito client not available".to_string())
        })?;
        
        // Simulate the transaction if enabled
        if self.config.simulate_transactions {
            let simulator = TransactionSimulator::new(
                rpc_client.clone(),
                self.config.commitment_config,
                false,
            );
            
            let simulation_result = simulate_transaction_with_timeout(
                &simulator,
                transaction,
                Duration::from_millis(1000),
            ).await?;
            
            if !simulation_result.success {
                return Err(ExecutionError::Simulation(
                    simulation_result.error.unwrap_or_else(|| "Simulation failed".to_string())
                ));
            }
        }
        
        // Create bundle options
        let options = BundleOptions {
            tip_account: self.config.jito_tip_account.as_ref().map(|s| s.parse().unwrap()),
            tip_amount: self.config.jito_tip_amount,
            timeout: Duration::from_millis(self.config.default_transaction_timeout_ms),
            wait_for_confirmation: true, // Wait for confirmation in MEV strategy
            max_retries: 3, // More retries for MEV strategy
            simulate_first: self.config.simulate_transactions,
        };
        
        // Submit the transaction via Jito
        let result = jito_client.submit_transaction(transaction, options).await;
        
        // If Jito fails, fall back to RPC
        if let Err(e) = result {
            warn!("Jito execution failed, falling back to RPC: {}", e);
            
            // Create retry strategy
            let retry_strategy = RetryStrategy::exponential_backoff(
                Duration::from_millis(self.config.retry_backoff_ms),
                Duration::from_millis(self.config.max_retry_backoff_ms),
                2.0,
                self.config.send_transaction_retry_count,
            );
            
            // Send the transaction with retries
            retry_transaction(|| async {
                let result = rpc_client.send_transaction(transaction).await
                    .map_err(|e| ExecutionError::Rpc(e.to_string()));
                
                result
            }, retry_strategy).await
        } else {
            result
        }
    }
}
