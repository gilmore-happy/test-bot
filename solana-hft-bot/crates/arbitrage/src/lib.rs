//! Arbitrage module for the Solana HFT Bot
//!
//! This module contains functionality for detecting and executing
//! arbitrage opportunities across Solana DEXes.

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
#[cfg(feature = "core")]
use solana_hft_core::CoreError;
#[cfg(not(feature = "core"))]
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Stub error: {0}")]
    Stub(String),
}
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use tokio::sync::{mpsc, oneshot, RwLock as TokioRwLock, Semaphore};
use tokio::time::{interval, sleep, timeout};
use tracing::{debug, error, info, warn};

pub mod config;
pub mod dexes;
pub mod flash_loan;
pub mod graph;
pub mod metrics;
pub mod paths;
pub mod pools;
pub mod pricing;
pub mod protocols;
pub mod simulation;
pub mod strategies;

pub use config::ArbitrageConfig;
pub use dexes::{DEX, DEXClient, DexRegistry};
pub use flash_loan::{FlashLoanProvider, FlashLoanManager};
pub use paths::{ArbitragePath, PathFinder};
pub use pools::{LiquidityPool, PoolRegistry};
pub use pricing::{PriceFeed, PriceManager};
pub use simulation::{ArbitrageSimulator, SimulationResult};
pub use strategies::{
    ArbitrageOpportunity, ArbitrageStrategy, ExecutionPriority, RiskAssessment, StrategyManager,
    CircularArbitrageStrategy, TriangularArbitrageStrategy, CrossExchangeArbitrageStrategy,
};

/// Arbitrage error types
#[derive(Debug, thiserror::Error)]
pub enum ArbitrageError {
    #[error("Arbitrage engine already running")]
    AlreadyRunning,
    
    #[error("Arbitrage engine not running")]
    NotRunning,
    
    #[error("Arbitrage opportunity not profitable: {0}")]
    NotProfitable(String),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Core error: {0}")]
    Core(#[from] CoreError),
    
    #[error("No profitable arbitrage found")]
    NoProfitableArbitrage,
    
    #[error("Price feed error: {0}")]
    PriceFeed(String),
    
    #[error("Liquidity insufficient: {0}")]
    InsufficientLiquidity(String),
    
    #[error("Path finding error: {0}")]
    PathFinding(String),
    
    #[error("Flash loan error: {0}")]
    FlashLoan(String),
    
    #[error("Simulation error: {0}")]
    Simulation(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("RPC error: {0}")]
    Rpc(String),
}

/// Arbitrage execution receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageReceipt {
    /// Opportunity that was executed
    pub opportunity: ArbitrageOpportunity,
    
    /// Execution status
    pub status: ExecutionStatus,
    
    /// Transaction signature
    pub signature: Option<Signature>,
    
    /// Actual profit in USD
    pub actual_profit_usd: Option<f64>,
    
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    
    /// Error message (if failed)
    pub error: Option<String>,
    
    /// Timestamp when execution started
    pub executed_at: chrono::DateTime<chrono::Utc>,
    
    /// Timestamp when execution completed
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Execution status for arbitrage opportunities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Execution pending
    Pending,
    
    /// Execution in progress
    InProgress,
    
    /// Execution successful
    Success,
    
    /// Execution failed
    Failed,
    
    /// Execution timed out
    Timeout,
}

/// Priority wrapper for arbitrage opportunities
#[derive(Debug, Clone)]
struct PrioritizedOpportunity {
    /// Arbitrage opportunity
    opportunity: ArbitrageOpportunity,
    
    /// Timestamp when the opportunity was added to the queue
    timestamp: Instant,
}

impl PartialEq for PrioritizedOpportunity {
    fn eq(&self, other: &Self) -> bool {
        self.opportunity.id == other.opportunity.id
    }
}

impl Eq for PrioritizedOpportunity {}

impl PartialOrd for PrioritizedOpportunity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedOpportunity {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by priority (higher is better)
        let priority_cmp = self.opportunity.priority.as_value().cmp(&other.opportunity.priority.as_value());
        if priority_cmp != Ordering::Equal {
            return priority_cmp.reverse(); // Reverse because BinaryHeap is a max-heap
        }
        
        // Then compare by expected profit (higher is better)
        let profit_cmp = self.opportunity.expected_profit_usd.partial_cmp(&other.opportunity.expected_profit_usd).unwrap_or(Ordering::Equal);
        if profit_cmp != Ordering::Equal {
            return profit_cmp.reverse(); // Reverse because BinaryHeap is a max-heap
        }
        
        // Then compare by timestamp (newer is better)
        self.timestamp.cmp(&other.timestamp).reverse() // Reverse because BinaryHeap is a max-heap
    }
}

/// Result of an arbitrage execution
#[derive(Debug, Clone)]
struct ArbitrageResult {
    /// Transaction signature
    signature: Signature,
    
    /// Actual profit in USD
    profit_usd: f64,
}

/// Arbitrage engine for detecting and executing arbitrage opportunities
pub struct ArbitrageEngine {
    /// RPC client
    rpc_client: Arc<RpcClient>,
    
    /// DEX registry
    dex_registry: Arc<DexRegistry>,
    
    /// Pool registry
    pool_registry: Arc<PoolRegistry>,
    
    /// Price manager
    price_manager: Arc<PriceManager>,
    
    /// Path finder
    path_finder: Arc<PathFinder>,
    
    /// Flash loan manager
    flash_loan_manager: Arc<FlashLoanManager>,
    
    /// Strategy manager
    strategy_manager: Arc<StrategyManager>,
    
    /// Simulator
    simulator: Arc<ArbitrageSimulator>,
    
    /// Opportunity queue
    opportunity_queue: Arc<TokioRwLock<BinaryHeap<PrioritizedOpportunity>>>,
    
    /// Recently executed arbitrages
    recent_arbitrages: Arc<RwLock<VecDeque<ArbitrageReceipt>>>,
    
    /// Channel for new opportunities
    opportunity_tx: mpsc::Sender<ArbitrageOpportunity>,
    opportunity_rx: mpsc::Receiver<ArbitrageOpportunity>,
    
    /// Channel for execution receipts
    receipt_tx: mpsc::Sender<ArbitrageReceipt>,
    receipt_rx: mpsc::Receiver<ArbitrageReceipt>,
    
    /// Execution semaphore to limit concurrent executions
    execution_semaphore: Arc<Semaphore>,
    
    /// Engine configuration
    config: ArbitrageConfig,
    
    /// Metrics for operation monitoring
    metrics: Arc<metrics::ArbitrageMetrics>,
    
    /// Whether the engine is running
    running: bool,
}

impl ArbitrageEngine {
    /// Create a new arbitrage engine with the given configuration
    pub fn new(config: ArbitrageConfig) -> Result<Self> {
        info!("Creating ArbitrageEngine with config: {:?}", config);
        
        // Create RPC client
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            config.commitment_config,
        ));
        
        // Create DEX registry
        let dex_registry = Arc::new(DexRegistry::new(&config));
        
        // Create pool registry
        let pool_registry = Arc::new(PoolRegistry::new(&config));
        
        // Create price manager
        let price_manager = Arc::new(PriceManager::new(rpc_client.clone(), &config));
        
        // Create path finder
        let path_finder = Arc::new(PathFinder::new(&config));
        
        // Create flash loan manager
        let flash_loan_manager = Arc::new(FlashLoanManager::new(rpc_client.clone(), &config));
        
        // Create strategy manager
        let strategy_manager = Arc::new(StrategyManager::new(&config));
        
        // Create simulator
        let simulator = Arc::new(ArbitrageSimulator::new(
            rpc_client.clone(),
            &config,
            dex_registry.clone(),
            pool_registry.clone(),
            price_manager.clone(),
            path_finder.clone(),
        ));
        
        // Create channels
        let (opportunity_tx, opportunity_rx) = mpsc::channel(1000);
        let (receipt_tx, receipt_rx) = mpsc::channel(1000);
        
        let engine = Self {
            rpc_client,
            dex_registry,
            pool_registry,
            price_manager,
            path_finder,
            flash_loan_manager,
            strategy_manager,
            simulator,
            opportunity_queue: Arc::new(TokioRwLock::new(BinaryHeap::new())),
            recent_arbitrages: Arc::new(RwLock::new(VecDeque::new())),
            opportunity_tx,
            opportunity_rx,
            receipt_tx,
            receipt_rx,
            execution_semaphore: Arc::new(Semaphore::new(config.max_concurrent_executions)),
            config,
            metrics: Arc::new(metrics::ArbitrageMetrics::new()),
            running: false,
        };
        
        Ok(engine)
    }
    
    /// Start the arbitrage engine
    pub async fn start(&mut self) -> Result<(), ArbitrageError> {
        if self.running {
            return Err(ArbitrageError::AlreadyRunning);
        }
        
        info!("Starting arbitrage engine with config: {:?}", self.config);
        
        // Initialize DEX clients
        self.dex_registry.initialize().await
            .map_err(|e| ArbitrageError::ExecutionFailed(format!("Failed to initialize DEX registry: {}", e)))?;
        
        // Initialize pool registry
        self.pool_registry.initialize(self.rpc_client.clone()).await
            .map_err(|e| ArbitrageError::ExecutionFailed(format!("Failed to initialize pool registry: {}", e)))?;
        
        // Initialize price feeds
        self.price_manager.initialize().await
            .map_err(|e| ArbitrageError::ExecutionFailed(format!("Failed to initialize price manager: {}", e)))?;
        
        // Initialize flash loan manager
        self.flash_loan_manager.initialize().await
            .map_err(|e| ArbitrageError::ExecutionFailed(format!("Failed to initialize flash loan manager: {}", e)))?;
        
        // Initialize strategies
        self.strategy_manager.initialize().await
            .map_err(|e| ArbitrageError::ExecutionFailed(format!("Failed to initialize strategy manager: {}", e)))?;
        
        // Start background workers
        self.spawn_background_workers();
        
        self.running = true;
        info!("Arbitrage engine started");
        Ok(())
    }
    
    /// Stop the arbitrage engine
    pub fn stop(&mut self) -> Result<(), ArbitrageError> {
        if !self.running {
            return Err(ArbitrageError::NotRunning);
        }
        
        info!("Stopping arbitrage engine");
        
        // In a real implementation, we would gracefully shut down background workers
        // For now, we just mark the engine as stopped
        
        self.running = false;
        info!("Arbitrage engine stopped");
        Ok(())
    }
    
    /// Spawn background workers for various tasks
    fn spawn_background_workers(&self) {
        self.spawn_price_monitor();
        self.spawn_opportunity_detector();
        self.spawn_opportunity_processor();
        self.spawn_pool_updater();
    }
    
    /// Spawn a worker to monitor prices
    fn spawn_price_monitor(&self) {
        let price_manager = self.price_manager.clone();
        let metrics = self.metrics.clone();
        let interval_ms = self.config.price_update_interval_ms;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
            
            loop {
                interval.tick().await;
                
                let start = Instant::now();
                if let Err(e) = price_manager.update_prices().await {
                    error!("Failed to update prices: {}", e);
                }
                
                metrics.record_price_update(start.elapsed());
            }
        });
    }
    
    /// Spawn a worker to detect arbitrage opportunities
    fn spawn_opportunity_detector(&self) {
        let engine = self.clone();
        let interval_ms = self.config.opportunity_detection_interval_ms;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
            
            loop {
                interval.tick().await;
                
                let start = Instant::now();
                if let Err(e) = engine.detect_opportunities().await {
                    error!("Failed to detect opportunities: {}", e);
                }
                
                engine.metrics.record_opportunity_detection(start.elapsed());
            }
        });
    }
    
    /// Spawn a worker to process arbitrage opportunities
    fn spawn_opportunity_processor(&self) {
        let engine = self.clone();
        
        tokio::spawn(async move {
            while let Some(opportunity) = engine.opportunity_rx.recv().await {
                let engine_clone = engine.clone();
                let opportunity_clone = opportunity.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = engine_clone.process_opportunity(opportunity_clone).await {
                        error!("Failed to process opportunity: {}", e);
                    }
                });
            }
        });
    }
    
    /// Spawn a worker to update liquidity pools
    fn spawn_pool_updater(&self) {
        let pool_registry = self.pool_registry.clone();
        let rpc_client = self.rpc_client.clone();
        let metrics = self.metrics.clone();
        let interval_ms = self.config.pool_update_interval_ms;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
            
            loop {
                interval.tick().await;
                
                let start = Instant::now();
                if let Err(e) = pool_registry.update_pools(rpc_client.clone()).await {
                    error!("Failed to update pools: {}", e);
                }
                
                metrics.record_pool_update(start.elapsed());
            }
        });
    }
    
    /// Detect arbitrage opportunities
    async fn detect_opportunities(&self) -> Result<()> {
        debug!("Detecting arbitrage opportunities");
        
        // Get registered strategies
        let strategies = self.strategy_manager.get_strategies();
        
        let mut futures = tokio::task::JoinSet::new();
        
        // Run all strategies in parallel
        for strategy in strategies {
            let strategy_clone = strategy.clone();
            let pool_registry = self.pool_registry.clone();
            let price_manager = self.price_manager.clone();
            let path_finder = self.path_finder.clone();
            let dex_registry = self.dex_registry.clone();
            let metrics = self.metrics.clone();
            
            futures.spawn(async move {
                let start = Instant::now();
                let result = strategy_clone.find_opportunities(
                    pool_registry,
                    price_manager,
                    path_finder,
                    dex_registry,
                ).await;
                
                metrics.record_strategy_execution(strategy_clone.name(), start.elapsed());
                
                (strategy_clone.name().to_string(), result)
            });
        }
        
        // Collect opportunities from all strategies
        let mut opportunities = Vec::new();
        
        while let Some(result) = futures.join_next().await {
            match result {
                Ok((strategy_name, Ok(mut strategy_opportunities))) => {
                    debug!("Strategy {} found {} opportunities", strategy_name, strategy_opportunities.len());
                    opportunities.append(&mut strategy_opportunities);
                },
                Ok((strategy_name, Err(e))) => {
                    warn!("Strategy {} failed: {}", strategy_name, e);
                },
                Err(e) => {
                    error!("Strategy task failed: {}", e);
                },
            }
        }
        
        // Filter opportunities based on minimum profit
        let min_profit_bps = self.config.min_profit_threshold_bps;
        let filtered_opportunities = opportunities.into_iter()
            .filter(|opp| opp.expected_profit_bps >= min_profit_bps)
            .collect::<Vec<_>>();
        
        info!("Found {} profitable arbitrage opportunities", filtered_opportunities.len());
        
        // Submit opportunities to the queue
        for opportunity in filtered_opportunities {
            self.submit_opportunity(opportunity).await?;
        }
        
        Ok(())
    }
    
    /// Submit an arbitrage opportunity for processing
    async fn submit_opportunity(&self, opportunity: ArbitrageOpportunity) -> Result<()> {
        debug!("Submitting arbitrage opportunity: {}", opportunity.id);
        
        // Add to the queue
        {
            let mut queue = self.opportunity_queue.write().await;
            let prioritized = PrioritizedOpportunity {
                opportunity: opportunity.clone(),
                timestamp: Instant::now(),
            };
            queue.push(prioritized);
            
            // Keep queue size in check
            while queue.len() > self.config.max_queue_size {
                queue.pop();
            }
            
            // Update metrics
            self.metrics.record_queue_size(queue.len() as u64);
        }
        
        // Send to the processor
        self.opportunity_tx.send(opportunity).await
            .map_err(|e| anyhow!("Failed to send opportunity: {}", e))?;
        
        Ok(())
    }
    
    /// Process an arbitrage opportunity
    async fn process_opportunity(&self, opportunity: ArbitrageOpportunity) -> Result<()> {
        info!("Processing arbitrage opportunity: {}", opportunity.id);
        
        let start = Instant::now();
        
        // Record that we're processing this opportunity
        self.metrics.record_opportunity_processing(opportunity.id.clone());
        
        // Create receipt
        let mut receipt = ArbitrageReceipt {
            opportunity: opportunity.clone(),
            status: ExecutionStatus::Pending,
            signature: None,
            actual_profit_usd: None,
            execution_time_ms: 0,
            error: None,
            executed_at: Utc::now(),
            completed_at: None,
        };
        
        // Get execution semaphore
        let permit = match self.execution_semaphore.try_acquire() {
            Ok(permit) => permit,
            Err(_) => {
                // Too many concurrent executions
                receipt.status = ExecutionStatus::Failed;
                receipt.error = Some("Too many concurrent executions".to_string());
                self.metrics.record_opportunity_failed(opportunity.id.clone());
                self.report_execution_receipt(receipt).await?;
                return Err(anyhow!("Too many concurrent executions"));
            }
        };
        
        // Validate opportunity is still viable
        if let Err(e) = self.validate_opportunity(&opportunity).await {
            receipt.status = ExecutionStatus::Failed;
            receipt.error = Some(format!("Validation failed: {}", e));
            self.metrics.record_opportunity_failed(opportunity.id.clone());
            self.report_execution_receipt(receipt).await?;
            return Err(e);
        }
        
        // Set status to in progress
        receipt.status = ExecutionStatus::InProgress;
        
        // Execute the arbitrage
        match self.execute_arbitrage(&opportunity).await {
            Ok(result) => {
                receipt.status = ExecutionStatus::Success;
                receipt.signature = Some(result.signature);
                receipt.actual_profit_usd = Some(result.profit_usd);
                receipt.completed_at = Some(Utc::now());
                self.metrics.record_opportunity_succeeded(
                    opportunity.id.clone(),
                    opportunity.strategy.clone(),
                    result.profit_usd,
                );
            },
            Err(e) => {
                receipt.status = ExecutionStatus::Failed;
                receipt.error = Some(format!("Execution failed: {}", e));
                receipt.completed_at = Some(Utc::now());
                self.metrics.record_opportunity_failed(opportunity.id.clone());
            }
        }
        
        // Calculate execution time
        receipt.execution_time_ms = start.elapsed().as_millis() as u64;
        
        // Record execution time
        self.metrics.record_execution_time(receipt.execution_time_ms);
        
        // Report the receipt
        self.report_execution_receipt(receipt).await?;
        
        // Release the permit
        drop(permit);
        
        Ok(())
    }
    
    /// Validate that an opportunity is still viable
    async fn validate_opportunity(&self, opportunity: &ArbitrageOpportunity) -> Result<()> {
        debug!("Validating arbitrage opportunity: {}", opportunity.id);
        
        // Check if opportunity has expired
        let now = Utc::now();
        let age = now.signed_duration_since(opportunity.timestamp);
        if age.num_milliseconds() > opportunity.ttl_ms as i64 {
            return Err(anyhow!("Opportunity has expired"));
        }
        
        // Simulate the opportunity
        let sim_result = self.simulator.simulate_opportunity(opportunity).await?;
        
        // Check if still profitable
        if !sim_result.success || sim_result.expected_profit_bps < self.config.min_profit_threshold_bps {
            return Err(anyhow!("Opportunity is no longer profitable"));
        }
        
        // Check if risk is acceptable
        if sim_result.risk_score > 80 {
            return Err(anyhow!("Opportunity risk is too high: {}", sim_result.risk_score));
        }
        
        Ok(())
    }
    
    /// Execute an arbitrage opportunity
    async fn execute_arbitrage(&self, opportunity: &ArbitrageOpportunity) -> Result<ArbitrageResult> {
        info!("Executing arbitrage opportunity: {}", opportunity.id);
        
        let flash_loan_required = opportunity.path.tokens.first() == opportunity.path.tokens.last();
        
        // Create the execution transaction
        let tx = if flash_loan_required {
            // Flash loan execution
            self.create_flash_loan_transaction(opportunity).await?
        } else {
            // Direct execution
            self.create_direct_transaction(opportunity).await?
        };
        
        // Send the transaction
        match self.send_transaction(&tx).await {
            Ok(signature) => {
                // Wait for confirmation
                let confirmation_result = self.wait_for_confirmation(&signature).await;
                
                // Calculate actual profit
                let profit_usd = if confirmation_result.is_ok() {
                    // In a real implementation, we would extract the actual profit from the transaction result
                    // For now, use the expected profit
                    opportunity.expected_profit_usd
                } else {
                    return Err(anyhow!("Transaction failed: {:?}", confirmation_result.err()));
                };
                
                Ok(ArbitrageResult {
                    signature,
                    profit_usd,
                })
            },
            Err(e) => {
                Err(anyhow!("Failed to send transaction: {}", e))
            }
        }
    }
    
    /// Create a direct arbitrage transaction (no flash loan)
    async fn create_direct_transaction(&self, opportunity: &ArbitrageOpportunity) -> Result<Transaction> {
        let path = &opportunity.path;
        let amount = opportunity.input_amount;
        
        // Create instructions for each swap in the path
        let mut instructions = Vec::new();
        
        // Add compute budget instruction
        let compute_budget = ComputeBudgetInstruction::set_compute_unit_price(
            self.calculate_priority_fee(opportunity).await?
        );
        instructions.push(compute_budget);
        
        // Add swap instructions
        for (i, pool_id) in path.pools.iter().enumerate() {
            let pool = self.pool_registry.get_pool(pool_id)
                .ok_or_else(|| anyhow!("Pool not found: {}", pool_id))?;
            
            let token_in = path.tokens[i];
            let token_out = path.tokens[i + 1];
            
            // Get the DEX client
            let dex_client = self.dex_registry.get_client(&pool.dex)
                .ok_or_else(|| anyhow!("DEX client not found: {:?}", pool.dex))?;
            
            // Calculate expected amount out
            let amount_in = if i == 0 { amount } else { 0 }; // Only set for first swap, others will use output from previous
            
            // Create swap instruction
            let swap_instruction = dex_client.create_swap_instruction(
                pool.address,
                token_in,
                token_out,
                amount_in,
            )?;
            
            instructions.push(swap_instruction);
        }
        
        // Create the transaction
        let keypair = Keypair::new(); // In a real implementation, would be the bot's keypair
        
        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&keypair.pubkey()),
            &[&keypair],
            recent_blockhash,
        );
        
        Ok(transaction)
    }
    
    /// Create a flash loan arbitrage transaction
    async fn create_flash_loan_transaction(&self, opportunity: &ArbitrageOpportunity) -> Result<Transaction> {
        let path = &opportunity.path;
        let amount = opportunity.input_amount;
        
        // Get flash loan provider
        let provider = self.flash_loan_manager.get_best_provider(
            path.tokens[0],
            amount,
        ).await?;
        
        // Create instructions
        let mut instructions = Vec::new();
        
        // Add compute budget instruction
        let compute_budget = ComputeBudgetInstruction::set_compute_unit_price(
            self.calculate_priority_fee(opportunity).await?
        );
        instructions.push(compute_budget);
        
        // Add flash loan borrow instruction
        let borrow_instruction = provider.create_borrow_instruction(
            path.tokens[0],
            amount,
        )?;
        instructions.push(borrow_instruction);
        
        // Add swap instructions
        for (i, pool_id) in path.pools.iter().enumerate() {
            let pool = self.pool_registry.get_pool(pool_id)
                .ok_or_else(|| anyhow!("Pool not found: {}", pool_id))?;
            
            let token_in = path.tokens[i];
            let token_out = path.tokens[i + 1];
            
            // Get the DEX client
            let dex_client = self.dex_registry.get_client(&pool.dex)
                .ok_or_else(|| anyhow!("DEX client not found: {:?}", pool.dex))?;
            
            // Calculate expected amount out
            let amount_in = if i == 0 { amount } else { 0 }; // Only set for first swap, others will use output from previous
            
            // Create swap instruction
            let swap_instruction = dex_client.create_swap_instruction(
                pool.address,
                token_in,
                token_out,
                amount_in,
            )?;
            
            instructions.push(swap_instruction);
        }
        
        // Add flash loan repay instruction
        let repay_instruction = provider.create_repay_instruction(
            path.tokens[0],
            amount,
        )?;
        instructions.push(repay_instruction);
        
        // Create the transaction
        let keypair = Keypair::new(); // In a real implementation, would be the bot's keypair
        
        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&keypair.pubkey()),
            &[&keypair],
            recent_blockhash,
        );
        
        Ok(transaction)
    }
    
    /// Calculate priority fee for an arbitrage opportunity
    async fn calculate_priority_fee(&self, opportunity: &ArbitrageOpportunity) -> Result<u64> {
        // Calculate fee based on priority and profit
        let base_fee = match opportunity.priority {
            ExecutionPriority::Low => 1_000,     // 0.000001 SOL per CU
            ExecutionPriority::Medium => 5_000,  // 0.000005 SOL per CU
            ExecutionPriority::High => 10_000,   // 0.00001 SOL per CU
            ExecutionPriority::Urgent => 100_000, // 0.0001 SOL per CU
        };
        
        // Scale fee based on profit
        let profit_scale = (opportunity.expected_profit_bps as f64 / 100.0).min(10.0);
        let fee = (base_fee as f64 * (1.0 + profit_scale * 0.1)) as u64;
        
        Ok(fee)
    }
    
    /// Send a transaction
    async fn send_transaction(&self, transaction: &Transaction) -> Result<Signature> {
        let signature = self.rpc_client.send_transaction(transaction).await?;
        Ok(signature)
    }
    
    /// Wait for transaction confirmation
    async fn wait_for_confirmation(&self, signature: &Signature) -> Result<()> {
        let timeout_ms = self.config.confirmation_timeout_ms;
        
        // Use RPC to wait for confirmation
        match tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            self.rpc_client.confirm_transaction(signature),
        ).await {
            Ok(Ok(confirmed)) => {
                if confirmed {
                    Ok(())
                } else {
                    Err(anyhow!("Transaction not confirmed"))
                }
            },
            Ok(Err(e)) => {
                Err(anyhow!("Confirmation error: {}", e))
            },
            Err(_) => {
                Err(anyhow!("Confirmation timed out after {}ms", timeout_ms))
            }
        }
    }
    
    /// Report an execution receipt
    async fn report_execution_receipt(&self, receipt: ArbitrageReceipt) -> Result<()> {
        debug!("Reporting execution receipt for opportunity: {}", receipt.opportunity.id);
        
        // Add to recent arbitrages
        {
            let mut recent = self.recent_arbitrages.write();
            recent.push_back(receipt.clone());
            
            // Keep maximum size
            while recent.len() > 100 {
                recent.pop_front();
            }
        }
        
        // Send to receipt channel
        self.receipt_tx.send(receipt).await
            .map_err(|e| anyhow!("Failed to send receipt: {}", e))?;
        
        Ok(())
    }
    
    /// Get arbitrage opportunity receiver
    pub fn get_opportunity_receiver(&self) -> mpsc::Receiver<ArbitrageOpportunity> {
        self.opportunity_rx.clone()
    }
    
    /// Get execution receipt receiver
    pub fn get_receipt_receiver(&self) -> mpsc::Receiver<ArbitrageReceipt> {
        self.receipt_rx.clone()
    }
    
    /// Get recent arbitrage executions
    pub fn get_recent_arbitrages(&self) -> Vec<ArbitrageReceipt> {
        let recent = self.recent_arbitrages.read();
        recent.iter().cloned().collect()
    }
    
    /// Get metrics for the arbitrage engine
    pub fn get_metrics(&self) -> metrics::ArbitrageMetricsSnapshot {
        self.metrics.snapshot()
    }
    
    /// Check if the engine is running
    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl Clone for ArbitrageEngine {
    fn clone(&self) -> Self {
        Self {
            rpc_client: self.rpc_client.clone(),
            dex_registry: self.dex_registry.clone(),
            pool_registry: self.pool_registry.clone(),
            price_manager: self.price_manager.clone(),
            path_finder: self.path_finder.clone(),
            flash_loan_manager: self.flash_loan_manager.clone(),
            strategy_manager: self.strategy_manager.clone(),
            simulator: self.simulator.clone(),
            opportunity_queue: self.opportunity_queue.clone(),
            recent_arbitrages: self.recent_arbitrages.clone(),
            opportunity_tx: self.opportunity_tx.clone(),
            opportunity_rx: self.opportunity_rx.clone(),
            receipt_tx: self.receipt_tx.clone(),
            receipt_rx: self.receipt_rx.clone(),
            execution_semaphore: self.execution_semaphore.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            running: self.running,
        }
    }
}

/// Module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_arbitrage_config_default() {
        let config = ArbitrageConfig::default();
        assert_eq!(config.min_profit_threshold_bps, 50);
        assert_eq!(config.max_slippage_bps, 30);
        assert_eq!(config.confirmation_timeout_ms, 30_000);
    }
    
    #[tokio::test]
    async fn test_arbitrage_engine_start_stop() {
        let config = ArbitrageConfig::default();
        let mut engine = ArbitrageEngine::new(config).unwrap();
        
        assert!(!engine.is_running());
        
        // Skip actual start since it requires RPC connection
        // let result = engine.start().await;
        // assert!(result.is_ok());
        // assert!(engine.is_running());
        
        // Manually set running to true for testing stop
        engine.running = true;
        
        let result = engine.stop();
        assert!(result.is_ok());
        assert!(!engine.is_running());
        
        // Trying to stop again should fail
        let result = engine.stop();
        assert!(result.is_err());
    }
}