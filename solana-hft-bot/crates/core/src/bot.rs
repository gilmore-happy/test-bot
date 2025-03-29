use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use futures::{future::Either, stream::{FuturesUnordered, StreamExt}};
use parking_lot::{Mutex, RwLock};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signature},
    transaction::Transaction,
};
use tokio::sync::{mpsc, RwLock as TokioRwLock, Semaphore};
use tokio::time::{interval, sleep, timeout};
use tracing::{debug, error, info, instrument, trace, warn};

use crate::config::{BotConfig, ModuleConfig};
use crate::metrics::CoreMetrics;
use crate::performance::{PerformanceMonitor, PerformanceSnapshot};
use crate::signals::TradingSignal;
use crate::status::{BotStatus, ModuleStatus};
use crate::strategies::{Strategy, StrategyManager};
use crate::types::{CoreError, ExecutionMode, SignalType, is_temporary_error};

/// Core bot for managing all HFT components
pub struct HftBot {
    /// Bot configuration
    config: BotConfig,
    
    /// RPC client
    rpc_client: Arc<RpcClient>,
    
    /// Bot keypair
    keypair: Arc<Keypair>,
    
    /// Strategy manager
    strategy_manager: Arc<StrategyManager>,
    
    /// Network engine
    network_engine: Option<Arc<solana_hft_network::NetworkEngine>>,
    
    /// RPC engine
    rpc_engine: Option<Arc<solana_hft_rpc::EnhancedRpcClient>>,
    
    /// Screening engine
    screening_engine: Option<Arc<solana_hft_screening::ScreeningEngine>>,
    
    /// Execution engine
    execution_engine: Option<Arc<solana_hft_execution::ExecutionEngine>>,
    
    /// Arbitrage engine
    arbitrage_engine: Option<Arc<solana_hft_arbitrage::ArbitrageEngine>>,
    
    /// Risk engine
    risk_engine: Option<Arc<solana_hft_risk::RiskEngine>>,
    
    /// Bot status
    status: RwLock<BotStatus>,
    
    /// Module statuses
    module_statuses: RwLock<HashMap<String, ModuleStatus>>,
    
    /// Trading signal queue
    signal_queue: Arc<TokioRwLock<VecDeque<TradingSignal>>>,
    
    /// Channel for trading signals
    signal_tx: mpsc::Sender<TradingSignal>,
    signal_rx: mpsc::Receiver<TradingSignal>,
    
    /// Execution mode
    execution_mode: RwLock<ExecutionMode>,
    
    /// Start time
    start_time: Instant,
    
    /// Performance metrics
    metrics: Arc<CoreMetrics>,
    
    /// Performance monitor
    performance_monitor: Arc<Mutex<PerformanceMonitor>>,
}

impl HftBot {
    /// Create a new HFT bot
    pub async fn new(config_path: PathBuf) -> Result<Self> {
        info!("Initializing HftBot from config: {:?}", config_path);
        
        // Load configuration
        let config = BotConfig::from_file(config_path)?;
        
        // Create RPC client
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            config.commitment_config,
        ));
        
        // Load or create keypair
        let keypair = if let Some(keypair_path) = &config.keypair_path {
            let keypair_bytes = std::fs::read(keypair_path)?;
            Arc::new(Keypair::from_bytes(&keypair_bytes)?)
        } else {
            Arc::new(Keypair::new())
        };
        
        // Create strategy manager
        let strategy_manager = StrategyManager::new(&config.strategy_config);
        
        // Create signal channels
        let (signal_tx, signal_rx) = mpsc::channel(1000);
        
        // Create performance monitor
        let performance_monitor = PerformanceMonitor::new(100);
        
        Ok(Self {
            config,
            rpc_client,
            keypair,
            strategy_manager: Arc::new(strategy_manager),
            network_engine: None,
            rpc_engine: None,
            screening_engine: None,
            execution_engine: None,
            arbitrage_engine: None,
            risk_engine: None,
            status: RwLock::new(BotStatus::Initializing),
            module_statuses: RwLock::new(HashMap::new()),
            signal_queue: Arc::new(TokioRwLock::new(VecDeque::new())),
            signal_tx,
            signal_rx,
            execution_mode: RwLock::new(ExecutionMode::Normal),
            start_time: Instant::now(),
            metrics: Arc::new(CoreMetrics::new()),
            performance_monitor: Arc::new(Mutex::new(performance_monitor)),
        })
    }
    
    /// Start the HFT bot
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting HftBot");
        
        // Set status to starting
        *self.status.write() = BotStatus::Starting;
        
        // Initialize modules
        self.initialize_modules().await?;
        
        // Start modules
        self.start_modules().await?;
        
        // Initialize strategies
        self.strategy_manager.initialize().await?;
        
        // Start background workers
        self.spawn_background_workers();
        
        // Set status to running
        *self.status.write() = BotStatus::Running;
        self.start_time = Instant::now();
        
        info!("HftBot started successfully");
        
        Ok(())
    }
    
    /// Stop the HFT bot
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping HftBot");
        
        // Set status to stopping
        *self.status.write() = BotStatus::Stopping;
        
        // Stop modules
        self.stop_modules().await?;
        
        // Set status to stopped
        *self.status.write() = BotStatus::Stopped;
        
        info!("HftBot stopped successfully");
        
        Ok(())
    }
    
    /// Initialize all modules
    async fn initialize_modules(&mut self) -> Result<()> {
        info!("Initializing modules");
        
        // Update module statuses
        let mut module_statuses = self.module_statuses.write();
        
        // Initialize network module if enabled
        if self.config.module_configs.network.enabled {
            info!("Initializing network module");
            module_statuses.insert("network".to_string(), ModuleStatus::Initializing);
            
            let network_config = self.config.module_configs.network.config.clone()
                .ok_or_else(|| anyhow!("Network module config not found"))?;
            
            let network_config: solana_hft_network::NetworkConfig = 
                serde_json::from_value(network_config)?;
            
            let network_engine = solana_hft_network::NetworkEngine::new(network_config)?;
            self.network_engine = Some(Arc::new(network_engine));
            
            module_statuses.insert("network".to_string(), ModuleStatus::Initialized);
        }
        
        // Initialize RPC module if enabled
        if self.config.module_configs.rpc.enabled {
            info!("Initializing RPC module");
            module_statuses.insert("rpc".to_string(), ModuleStatus::Initializing);
            
            let rpc_config = self.config.module_configs.rpc.config.clone()
                .ok_or_else(|| anyhow!("RPC module config not found"))?;
            
            let rpc_config: solana_hft_rpc::RpcConfig = 
                serde_json::from_value(rpc_config)?;
            
            let rpc_engine = solana_hft_rpc::EnhancedRpcClient::new(rpc_config).await?;
            self.rpc_engine = Some(Arc::new(rpc_engine));
            
            module_statuses.insert("rpc".to_string(), ModuleStatus::Initialized);
        }
        
        // Initialize screening module if enabled
        if self.config.module_configs.screening.enabled {
            info!("Initializing screening module");
            module_statuses.insert("screening".to_string(), ModuleStatus::Initializing);
            
            let screening_config = self.config.module_configs.screening.config.clone()
                .ok_or_else(|| anyhow!("Screening module config not found"))?;
            
            let screening_config: solana_hft_screening::ScreeningConfig = 
                serde_json::from_value(screening_config)?;
            
            let screening_engine = solana_hft_screening::ScreeningEngine::new(
                screening_config,
                self.rpc_client.clone(),
            ).await?;
            
            self.screening_engine = Some(Arc::new(screening_engine));
            
            module_statuses.insert("screening".to_string(), ModuleStatus::Initialized);
        }
        
        // Initialize execution module if enabled
        if self.config.module_configs.execution.enabled {
            info!("Initializing execution module");
            module_statuses.insert("execution".to_string(), ModuleStatus::Initializing);
            
            let execution_config = self.config.module_configs.execution.config.clone()
                .ok_or_else(|| anyhow!("Execution module config not found"))?;
            
            let execution_config: solana_hft_execution::ExecutionConfig = 
                serde_json::from_value(execution_config)?;
            
            let execution_engine = solana_hft_execution::ExecutionEngine::new(
                execution_config,
                self.rpc_client.clone(),
                self.keypair.clone(),
            ).await?;
            
            self.execution_engine = Some(Arc::new(execution_engine));
            
            module_statuses.insert("execution".to_string(), ModuleStatus::Initialized);
        }
        
        // Initialize arbitrage module if enabled
        if self.config.module_configs.arbitrage.enabled {
            info!("Initializing arbitrage module");
            module_statuses.insert("arbitrage".to_string(), ModuleStatus::Initializing);
            
            let arbitrage_config = self.config.module_configs.arbitrage.config.clone()
                .ok_or_else(|| anyhow!("Arbitrage module config not found"))?;
            
            let arbitrage_config: solana_hft_arbitrage::ArbitrageConfig = 
                serde_json::from_value(arbitrage_config)?;
            
            let arbitrage_engine = solana_hft_arbitrage::ArbitrageEngine::new(
                arbitrage_config,
                self.rpc_client.clone(),
            ).await?;
            
            self.arbitrage_engine = Some(Arc::new(arbitrage_engine));
            
            module_statuses.insert("arbitrage".to_string(), ModuleStatus::Initialized);
        }
        
        // Initialize risk module if enabled
        if self.config.module_configs.risk.enabled {
            info!("Initializing risk module");
            module_statuses.insert("risk".to_string(), ModuleStatus::Initializing);
            
            let risk_config = self.config.module_configs.risk.config.clone()
                .ok_or_else(|| anyhow!("Risk module config not found"))?;
            
            let risk_config: solana_hft_risk::RiskConfig = 
                serde_json::from_value(risk_config)?;
            
            let risk_engine = solana_hft_risk::RiskEngine::new(
                risk_config,
                self.rpc_client.clone(),
            ).await?;
            
            self.risk_engine = Some(Arc::new(risk_engine));
            
            module_statuses.insert("risk".to_string(), ModuleStatus::Initialized);
        }
        
        info!("All modules initialized successfully");
        
        Ok(())
    }
    
    /// Start all modules
    async fn start_modules(&self) -> Result<()> {
        info!("Starting modules");
        
        // Update module statuses
        let mut module_statuses = self.module_statuses.write();
        
        // Start network module if initialized
        if let Some(network_engine) = &self.network_engine {
            info!("Starting network module");
            module_statuses.insert("network".to_string(), ModuleStatus::Starting);
            
            network_engine.start().await?;
            
            module_statuses.insert("network".to_string(), ModuleStatus::Running);
        }
        
        // Start RPC module if initialized
        if let Some(rpc_engine) = &self.rpc_engine {
            info!("Starting RPC module");
            module_statuses.insert("rpc".to_string(), ModuleStatus::Starting);
            
            // No explicit start method for RPC
            
            module_statuses.insert("rpc".to_string(), ModuleStatus::Running);
        }
        
        // Start screening module if initialized
        if let Some(screening_engine) = &self.screening_engine {
            info!("Starting screening module");
            module_statuses.insert("screening".to_string(), ModuleStatus::Starting);
            
            screening_engine.start().await?;
            
            module_statuses.insert("screening".to_string(), ModuleStatus::Running);
        }
        
        // Start execution module if initialized
        if let Some(execution_engine) = &self.execution_engine {
            info!("Starting execution module");
            module_statuses.insert("execution".to_string(), ModuleStatus::Starting);
            
            execution_engine.start().await?;
            
            module_statuses.insert("execution".to_string(), ModuleStatus::Running);
        }
        
        // Start arbitrage module if initialized
        if let Some(arbitrage_engine) = &self.arbitrage_engine {
            info!("Starting arbitrage module");
            module_statuses.insert("arbitrage".to_string(), ModuleStatus::Starting);
            
            arbitrage_engine.start().await?;
            
            module_statuses.insert("arbitrage".to_string(), ModuleStatus::Running);
        }
        
        // Start risk module if initialized
        if let Some(risk_engine) = &self.risk_engine {
            info!("Starting risk module");
            module_statuses.insert("risk".to_string(), ModuleStatus::Starting);
            
            risk_engine.start().await?;
            
            module_statuses.insert("risk".to_string(), ModuleStatus::Running);
        }
        
        info!("All modules started successfully");
        
        Ok(())
    }
    
    /// Stop all modules
    async fn stop_modules(&self) -> Result<()> {
        info!("Stopping modules");
        
        // Update module statuses
        let mut module_statuses = self.module_statuses.write();
        
        // Stop network module if running
        if let Some(network_engine) = &self.network_engine {
            info!("Stopping network module");
            module_statuses.insert("network".to_string(), ModuleStatus::Stopping);
            
            network_engine.stop().await?;
            
            module_statuses.insert("network".to_string(), ModuleStatus::Stopped);
        }
        
        // Stop screening module if running
        if let Some(screening_engine) = &self.screening_engine {
            info!("Stopping screening module");
            module_statuses.insert("screening".to_string(), ModuleStatus::Stopping);
            
            // No explicit stop method
            
            module_statuses.insert("screening".to_string(), ModuleStatus::Stopped);
        }
        
        // No need to explicitly stop other modules
        
        info!("All modules stopped successfully");
        
        Ok(())
    }
    
    /// Spawn background workers for various tasks
    fn spawn_background_workers(&self) {
        self.spawn_signal_processor();
        self.spawn_strategy_runner();
        self.spawn_performance_monitor();
        self.spawn_status_monitor();
    }
    
    /// Spawn a worker to process trading signals
    fn spawn_signal_processor(&self) {
        let this = self.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(10));
            
            while *this.status.read() == BotStatus::Running {
                interval.tick().await;
                
                // Process signals from the receiver
                while let Ok(signal) = this.signal_rx.try_recv() {
                    if let Err(e) = this.process_signal(signal).await {
                        error!("Failed to process signal: {}", e);
                    }
                }
                
                // Process signals from the queue
                this.process_signal_queue().await;
            }
        });
    }
    
    /// Process the signal queue
    async fn process_signal_queue(&self) {
        let mut queue = self.signal_queue.write().await;
        
        // Process signals in the queue
        while let Some(signal) = queue.pop_front() {
            // Check if the signal has expired
            if signal.expiration < chrono::Utc::now() {
                debug!("Signal {} expired, skipping", signal.id);
                continue;
            }
            
            if let Err(e) = self.process_signal(signal.clone()).await {
                error!("Failed to process signal from queue: {}", e);
                
                // Re-queue the signal if it was a temporary failure
                if is_temporary_error(&e) {
                    queue.push_back(signal);
                }
            }
        }
    }
    
    /// Process a trading signal
    async fn process_signal(&self, signal: TradingSignal) -> Result<()> {
        info!("Processing signal: {} ({:?})", signal.id, signal.signal_type);
        
        // Skip processing if in simulation or analysis mode
        let execution_mode = *self.execution_mode.read();
        if execution_mode == ExecutionMode::Simulation || execution_mode == ExecutionMode::Analysis {
            info!("Skipping execution in {:?} mode", execution_mode);
            self.metrics.record_signal_processed(signal.id.clone(), false);
            return Ok(());
        }
        
        // Check risk limits if risk engine is available
        if let Some(risk_engine) = &self.risk_engine {
            let token = signal.tokens.first().cloned().unwrap_or_else(Pubkey::new_unique);
            
            let risk_status = risk_engine.assess_risk(
                &signal.strategy_id,
                token,
                signal.amount,
                signal.expected_profit_usd,
            ).await?;
            
            if !risk_status.approved {
                warn!("Signal {} rejected by risk engine: {}", 
                    signal.id, 
                    risk_status.rejection_reason.unwrap_or_else(|| "Unknown reason".to_string())
                );
                
                self.metrics.record_signal_rejected(signal.id);
                return Ok(());
            }
        }
        
        // Execute the signal based on type
        match signal.signal_type {
            SignalType::Buy => {
                self.execute_buy(signal).await?;
            },
            SignalType::Sell => {
                self.execute_sell(signal).await?;
            },
            SignalType::Arbitrage => {
                self.execute_arbitrage(signal).await?;
            },
            SignalType::Mev => {
                self.execute_mev(signal).await?;
            },
            SignalType::TokenLaunch => {
                self.execute_token_launch(signal).await?;
            },
            SignalType::Other => {
                warn!("Ignoring signal with type Other");
            },
        }
        
        self.metrics.record_signal_processed(signal.id, true);
        
        Ok(())
    }
    
    /// Execute a buy signal
    async fn execute_buy(&self, signal: TradingSignal) -> Result<()> {
        info!("Executing buy signal: {}", signal.id);
        
        // Get execution engine
        let execution_engine = self.execution_engine.as_ref()
            .ok_or_else(|| anyhow!("Execution engine not initialized"))?;
        
        // Example implementation:
        // 1. Create a transaction template
        // 2. Execute the template
        
        let token = signal.tokens.first().cloned()
            .ok_or_else(|| anyhow!("No token specified in buy signal"))?;
        
        let transaction_template = solana_hft_execution::TransactionTemplate {
            instructions: vec![],  // In a real implementation, would create swap instruction
            signers: vec![self.keypair.pubkey()],
            blockhash: None,
            metadata: HashMap::new(),
        };
        
        let priority = solana_hft_execution::TransactionPriority::High;
        
        let receipt = execution_engine.execute_template(
            &transaction_template,
            priority,
            true,  // Wait for confirmation
        ).await?;
        
        info!("Buy signal executed: {}, tx: {:?}", signal.id, receipt.signature);
        
        Ok(())
    }
    
    /// Execute a sell signal
    async fn execute_sell(&self, signal: TradingSignal) -> Result<()> {
        info!("Executing sell signal: {}", signal.id);
        
        // Similar to execute_buy but for selling
        
        Ok(())
    }
    
    /// Execute an arbitrage signal
    async fn execute_arbitrage(&self, signal: TradingSignal) -> Result<()> {
        info!("Executing arbitrage signal: {}", signal.id);
        
        // Get arbitrage engine
        let arbitrage_engine = self.arbitrage_engine.as_ref()
            .ok_or_else(|| anyhow!("Arbitrage engine not initialized"))?;
        
        // In a real implementation, this would construct an ArbitrageOpportunity
        // and call arbitrage_engine.execute_opportunity
        
        Ok(())
    }
    
    /// Execute an MEV signal
    async fn execute_mev(&self, signal: TradingSignal) -> Result<()> {
        info!("Executing MEV signal: {}", signal.id);
        
        // Get execution engine
        let execution_engine = self.execution_engine.as_ref()
            .ok_or_else(|| anyhow!("Execution engine not initialized"))?;
        
        // In a real implementation, this would prepare a bundle transaction
        // and submit it via Jito MEV
        
        Ok(())
    }
    
    /// Execute a token launch signal
    async fn execute_token_launch(&self, signal: TradingSignal) -> Result<()> {
        info!("Executing token launch signal: {}", signal.id);
        
        // Get execution engine
        let execution_engine = self.execution_engine.as_ref()
            .ok_or_else(|| anyhow!("Execution engine not initialized"))?;
        
        // In a real implementation, this would prepare a transaction to buy
        // a newly launched token
        
        Ok(())
    }
    
    /// Spawn a worker to run strategies
    fn spawn_strategy_runner(&self) {
        let this = self.clone();
        let interval_ms = self.config.strategy_run_interval_ms;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
            
            while *this.status.read() == BotStatus::Running {
                interval.tick().await;
                
                // Run strategies
                if let Err(e) = this.run_strategies().await {
                    error!("Failed to run strategies: {}", e);
                }
            }
        });
    }
    
    /// Run all enabled strategies
    async fn run_strategies(&self) -> Result<()> {
        // Get enabled strategies
        let strategies = self.strategy_manager.get_enabled_strategies();
        
        if strategies.is_empty() {
            return Ok(());
        }
        
        let screening_engine = self.screening_engine.clone();
        let arbitrage_engine = self.arbitrage_engine.clone();
        let rpc_client = self.rpc_client.clone();
        let signal_tx = self.signal_tx.clone();
        
        // Run strategies in parallel
        let futures = FuturesUnordered::new();
        
        for strategy in strategies {
            let strategy_clone = strategy.clone();
            let screening_engine_clone = screening_engine.clone();
            let arbitrage_engine_clone = arbitrage_engine.clone();
            let rpc_client_clone = rpc_client.clone();
            let signal_tx_clone = signal_tx.clone();
            
            // Spawn a task for each strategy
            let future = tokio::spawn(async move {
                let start = Instant::now();
                let result = strategy_clone.run(
                    screening_engine_clone,
                    arbitrage_engine_clone,
                    rpc_client_clone,
                ).await;
                
                match result {
                    Ok(signals) => {
                        for signal in signals {
                            if let Err(e) = signal_tx_clone.send(signal).await {
                                error!("Failed to send signal: {}", e);
                            }
                        }
                        
                        Ok((strategy_clone.id().to_string(), start.elapsed(), signals.len()))
                    },
                    Err(e) => {
                        Err((strategy_clone.id().to_string(), e))
                    }
                }
            });
            
            futures.push(future);
        }
        
        // Process results
        let mut successes = 0;
        let mut failures = 0;
        
        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok((id, duration, signal_count))) => {
                    debug!("Strategy {} ran successfully in {:?}, generated {} signals", 
                        id, duration, signal_count);
                    self.metrics.record_strategy_run(&id, duration, signal_count);
                    successes += 1;
                },
                Ok(Err((id, error))) => {
                    error!("Strategy {} failed: {}", id, error);
                    self.metrics.record_strategy_error(&id);
                    failures += 1;
                },
                Err(e) => {
                    error!("Strategy task failed: {}", e);
                    failures += 1;
                }
            }
        }
        
        info!("Ran {} strategies: {} succeeded, {} failed", 
            successes + failures, successes, failures);
        
        Ok(())
    }
    
    /// Spawn a worker to monitor performance
    fn spawn_performance_monitor(&self) {
        let this = self.clone();
        let interval_ms = 60_000; // Every minute
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
            
            while *this.status.read() == BotStatus::Running {
                interval.tick().await;
                
                // Collect performance metrics
                if let Err(e) = this.collect_performance_metrics().await {
                    error!("Failed to collect performance metrics: {}", e);
                }
            }
        });
    }
    
    /// Collect performance metrics
    async fn collect_performance_metrics(&self) -> Result<()> {
        // Collect metrics from all modules
        let mut module_metrics = HashMap::new();
        
        // Network metrics
        if let Some(network_engine) = &self.network_engine {
            module_metrics.insert("network".to_string(), serde_json::to_value(network_engine.get_metrics())?);
        }
        
        // RPC metrics
        if let Some(rpc_engine) = &self.rpc_engine {
            module_metrics.insert("rpc".to_string(), serde_json::to_value(rpc_engine.get_metrics())?);
        }
        
        // Screening metrics
        if let Some(screening_engine) = &self.screening_engine {
            module_metrics.insert("screening".to_string(), serde_json::to_value(screening_engine.get_metrics())?);
        }
        
        // Execution metrics
        if let Some(execution_engine) = &self.execution_engine {
            module_metrics.insert("execution".to_string(), serde_json::to_value(execution_engine.get_metrics())?);
        }
        
        // Arbitrage metrics
        if let Some(arbitrage_engine) = &self.arbitrage_engine {
            module_metrics.insert("arbitrage".to_string(), serde_json::to_value(arbitrage_engine.get_metrics())?);
        }
        
        // Risk metrics
        if let Some(risk_engine) = &self.risk_engine {
            module_metrics.insert("risk".to_string(), serde_json::to_value(risk_engine.get_metrics())?);
        }
        
        // Get core metrics
        let core_metrics = self.metrics.snapshot();
        
        // Create performance snapshot
        let performance = PerformanceSnapshot {
            timestamp: chrono::Utc::now(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            core_metrics,
            module_metrics,
        };
        
        // Add to performance monitor
        let mut monitor = self.performance_monitor.lock();
        monitor.add_snapshot(performance.clone());
        
        // Log performance
        info!("Performance snapshot: {} modules, {} strategies, {} signals processed", 
            module_metrics.len(), 
            core_metrics.strategies_run,
            core_metrics.signals_processed);
        
        Ok(())
    }
    
    /// Spawn a worker to monitor status
    fn spawn_status_monitor(&self) {
        let this = self.clone();
        let interval_ms = 10_000; // Every 10 seconds
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
            
            while *this.status.read() != BotStatus::Stopped {
                interval.tick().await;
                
                // Update status
                if let Err(e) = this.update_status().await {
                    error!("Failed to update status: {}", e);
                }
            }
        });
    }
    
    /// Update bot status
    async fn update_status(&self) -> Result<()> {
        // Check all module statuses
        for (module, status) in self.module_statuses.read().iter() {
            if *status == ModuleStatus::Error {
                warn!("Module {} is in error state", module);
            }
        }
        
        // Check risk engine status
        if let Some(risk_engine) = &self.risk_engine {
            let risk_state = risk_engine.get_state();
            
            if risk_state == solana_hft_risk::RiskEngineState::Shutdown {
                warn!("Risk engine is in shutdown state");
                
                // Update bot status if in running state
                let mut status = self.status.write();
                if *status == BotStatus::Running {
                    *status = BotStatus::Paused;
                }
            }
        }
        
        // Log current status
        debug!("Bot status: {:?}", *self.status.read());
        
        Ok(())
    }
    
    /// Get bot status
    pub fn get_status(&self) -> BotStatus {
        *self.status.read()
    }
    
    /// Set execution mode
    pub fn set_execution_mode(&self, mode: ExecutionMode) {
        info!("Setting execution mode to {:?}", mode);
        *self.execution_mode.write() = mode;
    }
    
    /// Get execution mode
    pub fn get_execution_mode(&self) -> ExecutionMode {
        *self.execution_mode.read()
    }
    
    /// Get metrics
    pub fn get_metrics(&self) -> crate::metrics::CoreMetricsSnapshot {
        self.metrics.snapshot()
    }
    
    /// Submit a trading signal
    pub async fn submit_signal(&self, signal: TradingSignal) -> Result<()> {
        // Send to channel
        self.signal_tx.send(signal).await
            .map_err(|e| anyhow!("Failed to send signal: {}", e))?;
        
        Ok(())
    }
}

impl Clone for HftBot {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            rpc_client: self.rpc_client.clone(),
            keypair: self.keypair.clone(),
            strategy_manager: self.strategy_manager.clone(),
            network_engine: self.network_engine.clone(),
            rpc_engine: self.rpc_engine.clone(),
            screening_engine: self.screening_engine.clone(),
            execution_engine: self.execution_engine.clone(),
            arbitrage_engine: self.arbitrage_engine.clone(),
            risk_engine: self.risk_engine.clone(),
            status: RwLock::new(*self.status.read()),
            module_statuses: RwLock::new(self.module_statuses.read().clone()),
            signal_queue: self.signal_queue.clone(),
            signal_tx: self.signal_tx.clone(),
            signal_rx: self.signal_rx.clone(),
            execution_mode: RwLock::new(*self.execution_mode.read()),
            start_time: self.start_time,
            metrics: self.metrics.clone(),
            performance_monitor: self.performance_monitor.clone(),
        }
    }
}