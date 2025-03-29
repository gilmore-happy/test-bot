//! Protocol-aware arbitrage strategy
//!
//! This strategy leverages protocol-specific knowledge to identify
//! and execute arbitrage opportunities across multiple Solana DeFi protocols.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use solana_program::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use tracing::{debug, info, trace, warn};

use crate::dexes::DexRegistry;
use crate::paths::{ArbitragePath, PathFinder};
use crate::pools::PoolRegistry;
use crate::pricing::PriceManager;
use crate::protocols::{
    ArbitrageOpportunityDetector, ArbitrageScanner, ProtocolRegistry,
    detection::{ArbitragePattern, analyze_transaction_for_arbitrage},
};
use crate::strategies::{ArbitrageOpportunity, ArbitrageStrategy, ExecutionPriority, RiskAssessment};

/// Protocol-aware arbitrage strategy
pub struct ProtocolAwareArbitrageStrategy {
    /// Protocol registry
    protocol_registry: Arc<ProtocolRegistry>,
    
    /// Opportunity detector
    opportunity_detector: Arc<ArbitrageOpportunityDetector>,
    
    /// Arbitrage scanner
    scanner: Option<Arc<tokio::sync::Mutex<ArbitrageScanner>>>,
    
    /// Minimum profit threshold in USD
    min_profit_threshold_usd: f64,
    
    /// Minimum profit threshold in basis points
    min_profit_threshold_bps: u32,
    
    /// Maximum gas cost in lamports
    max_gas_cost_lamports: u64,
    
    /// Whether to use flash loans
    use_flash_loans: bool,
    
    /// Maximum concurrent opportunities
    max_concurrent_opportunities: usize,
    
    /// Protocol weights for opportunity scoring
    protocol_weights: HashMap<String, f64>,
}

impl ProtocolAwareArbitrageStrategy {
    /// Create a new protocol-aware arbitrage strategy
    pub fn new() -> Self {
        // Create protocol registry
        let protocol_registry = Arc::new(ProtocolRegistry::new());
        
        // Default protocol weights (higher = more preferred)
        let mut protocol_weights = HashMap::new();
        protocol_weights.insert(RAYDIUM_SWAP_V2.to_string(), 1.0);
        protocol_weights.insert(ORCA_WHIRLPOOL.to_string(), 1.0);
        protocol_weights.insert(JUPITER_V6.to_string(), 1.2); // Slightly prefer Jupiter
        protocol_weights.insert(OPENBOOK_V2.to_string(), 0.9);
        protocol_weights.insert(PHOENIX_DEX.to_string(), 1.1);
        
        Self {
            protocol_registry,
            opportunity_detector: Arc::new(ArbitrageOpportunityDetector::new(
                protocol_registry.clone(),
                Arc::new(PoolRegistry::new(&crate::config::ArbitrageConfig::default())),
                Arc::new(PriceManager::new(
                    Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(
                        "https://api.mainnet-beta.solana.com".to_string()
                    )),
                    &crate::config::ArbitrageConfig::default()
                )),
                0.5, // $0.50 min profit
                10,  // 10 bps min profit
                100000, // 0.0001 SOL max gas
            )),
            scanner: None,
            min_profit_threshold_usd: 0.5,
            min_profit_threshold_bps: 10,
            max_gas_cost_lamports: 100000,
            use_flash_loans: true,
            max_concurrent_opportunities: 10,
            protocol_weights,
        }
    }
    
    /// Set minimum profit threshold in USD
    pub fn with_min_profit_threshold_usd(mut self, threshold: f64) -> Self {
        self.min_profit_threshold_usd = threshold;
        self
    }
    
    /// Set minimum profit threshold in basis points
    pub fn with_min_profit_threshold_bps(mut self, threshold: u32) -> Self {
        self.min_profit_threshold_bps = threshold;
        self
    }
    
    /// Set maximum gas cost in lamports
    pub fn with_max_gas_cost_lamports(mut self, max_gas: u64) -> Self {
        self.max_gas_cost_lamports = max_gas;
        self
    }
    
    /// Set whether to use flash loans
    pub fn with_flash_loans(mut self, use_flash_loans: bool) -> Self {
        self.use_flash_loans = use_flash_loans;
        self
    }
    
    /// Set protocol weight
    pub fn with_protocol_weight(mut self, protocol_id: &str, weight: f64) -> Self {
        self.protocol_weights.insert(protocol_id.to_string(), weight);
        self
    }
    
    /// Calculate opportunity score based on protocol weights
    fn calculate_opportunity_score(&self, opportunity: &ArbitrageOpportunity) -> f64 {
        let mut score = opportunity.expected_profit_usd;
        
        // Apply protocol weights
        for pool in &opportunity.path.pools {
            if let Some(weight) = self.protocol_weights.get(pool) {
                score *= weight;
            }
        }
        
        score
    }
    
    /// Sort opportunities by score
    fn sort_opportunities_by_score(&self, opportunities: &mut Vec<ArbitrageOpportunity>) {
        opportunities.sort_by(|a, b| {
            let a_score = self.calculate_opportunity_score(a);
            let b_score = self.calculate_opportunity_score(b);
            b_score.partial_cmp(&a_score).unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    
    /// Scan mempool transactions for arbitrage opportunities
    pub async fn scan_mempool_transactions(&self, transactions: &[Transaction]) -> Vec<ArbitrageOpportunity> {
        if let Some(scanner) = &self.scanner {
            let mut scanner_guard = scanner.lock().await;
            scanner_guard.scan_transactions(transactions)
        } else {
            Vec::new()
        }
    }
}

#[async_trait]
impl ArbitrageStrategy for ProtocolAwareArbitrageStrategy {
    fn name(&self) -> &str {
        "protocol-aware"
    }
    
    async fn initialize(&self) -> Result<()> {
        info!("Initializing protocol-aware arbitrage strategy");
        
        // Initialize scanner if not already initialized
        if self.scanner.is_none() {
            let scanner = ArbitrageScanner::new(
                self.opportunity_detector.clone(),
                self.protocol_registry.clone(),
                self.max_concurrent_opportunities,
            );
            
            // Store scanner
            let _ = Arc::new(tokio::sync::Mutex::new(scanner));
        }
        
        Ok(())
    }
    
    async fn find_opportunities(
        &self,
        pool_registry: Arc<PoolRegistry>,
        price_manager: Arc<PriceManager>,
        path_finder: Arc<PathFinder>,
        dex_registry: Arc<DexRegistry>,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        debug!("Finding protocol-aware arbitrage opportunities");
        
        // Create a new opportunity detector with the latest pool and price data
        let detector = Arc::new(ArbitrageOpportunityDetector::new(
            self.protocol_registry.clone(),
            pool_registry.clone(),
            price_manager.clone(),
            self.min_profit_threshold_usd,
            self.min_profit_threshold_bps,
            self.max_gas_cost_lamports,
        ));
        
        // Get all pools
        let pools = pool_registry.get_all_pools();
        
        // Find potential arbitrage paths
        let mut opportunities = Vec::new();
        
        // For each DEX, find paths that start and end with the same token
        for dex in dex_registry.get_all_dexes() {
            let dex_name = dex.name();
            
            // Find circular paths (A -> B -> C -> A)
            let circular_paths = path_finder.find_circular_paths(
                pool_registry.clone(),
                price_manager.clone(),
                3, // 3-hop paths
                10, // Top 10 paths
            ).await?;
            
            for path in circular_paths {
                // Calculate expected profit
                let (input_amount, expected_output) = path_finder.calculate_path_output(
                    &path,
                    pool_registry.clone(),
                    price_manager.clone(),
                ).await?;
                
                if expected_output <= input_amount {
                    continue; // Not profitable
                }
                
                let profit = expected_output - input_amount;
                
                // Calculate profit in USD
                let input_token = path.tokens[0];
                let input_token_price = price_manager.get_token_price(&input_token)?;
                let profit_usd = profit as f64 * input_token_price;
                
                // Calculate profit in basis points
                let profit_bps = ((profit as f64 / input_amount as f64) * 10000.0) as u32;
                
                // Check if profitable enough
                if profit_usd < self.min_profit_threshold_usd || profit_bps < self.min_profit_threshold_bps {
                    continue;
                }
                
                // Determine priority based on profit
                let priority = if profit_bps > 100 {
                    ExecutionPriority::High
                } else if profit_bps > 50 {
                    ExecutionPriority::Medium
                } else {
                    ExecutionPriority::Low
                };
                
                // Create risk assessment
                let risk = RiskAssessment {
                    risk_score: 20, // Default moderate risk
                    success_probability: 80, // Default high probability
                    risk_factors: vec!["Price volatility".to_string()],
                    max_potential_loss_usd: profit_usd * 0.5, // Conservative estimate
                };
                
                // Create opportunity
                let opportunity = ArbitrageOpportunity {
                    id: format!("protocol-aware-circular-{}", uuid::Uuid::new_v4()),
                    path: path.clone(),
                    input_amount,
                    expected_output,
                    expected_profit_usd: profit_usd,
                    expected_profit_bps: profit_bps,
                    timestamp: chrono::Utc::now(),
                    ttl_ms: 10_000, // 10 seconds TTL
                    priority,
                    strategy: "protocol-aware".to_string(),
                    risk,
                };
                
                opportunities.push(opportunity);
            }
            
            // Find triangular paths (A -> B -> C)
            let triangular_paths = path_finder.find_triangular_paths(
                pool_registry.clone(),
                price_manager.clone(),
                10, // Top 10 paths
            ).await?;
            
            for path in triangular_paths {
                // Calculate expected profit
                let (input_amount, expected_output) = path_finder.calculate_path_output(
                    &path,
                    pool_registry.clone(),
                    price_manager.clone(),
                ).await?;
                
                if expected_output <= input_amount {
                    continue; // Not profitable
                }
                
                let profit = expected_output - input_amount;
                
                // Calculate profit in USD
                let input_token = path.tokens[0];
                let input_token_price = price_manager.get_token_price(&input_token)?;
                let profit_usd = profit as f64 * input_token_price;
                
                // Calculate profit in basis points
                let profit_bps = ((profit as f64 / input_amount as f64) * 10000.0) as u32;
                
                // Check if profitable enough
                if profit_usd < self.min_profit_threshold_usd || profit_bps < self.min_profit_threshold_bps {
                    continue;
                }
                
                // Determine priority based on profit
                let priority = if profit_bps > 100 {
                    ExecutionPriority::High
                } else if profit_bps > 50 {
                    ExecutionPriority::Medium
                } else {
                    ExecutionPriority::Low
                };
                
                // Create risk assessment
                let risk = RiskAssessment {
                    risk_score: 20, // Default moderate risk
                    success_probability: 80, // Default high probability
                    risk_factors: vec!["Price volatility".to_string()],
                    max_potential_loss_usd: profit_usd * 0.5, // Conservative estimate
                };
                
                // Create opportunity
                let opportunity = ArbitrageOpportunity {
                    id: format!("protocol-aware-triangular-{}", uuid::Uuid::new_v4()),
                    path: path.clone(),
                    input_amount,
                    expected_output,
                    expected_profit_usd: profit_usd,
                    expected_profit_bps: profit_bps,
                    timestamp: chrono::Utc::now(),
                    ttl_ms: 10_000, // 10 seconds TTL
                    priority,
                    strategy: "protocol-aware".to_string(),
                    risk,
                };
                
                opportunities.push(opportunity);
            }
        }
        
        // Sort opportunities by score
        self.sort_opportunities_by_score(&mut opportunities);
        
        // Limit to max concurrent opportunities
        if opportunities.len() > self.max_concurrent_opportunities {
            opportunities.truncate(self.max_concurrent_opportunities);
        }
        
        Ok(opportunities)
    }
}