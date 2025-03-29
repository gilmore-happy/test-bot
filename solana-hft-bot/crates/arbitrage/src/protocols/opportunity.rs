//! Arbitrage opportunity detection and analysis
//!
//! This module provides functionality for detecting and analyzing
//! arbitrage opportunities across Solana DeFi protocols.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use solana_program::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use tracing::{debug, info, trace, warn};

use crate::strategies::{ArbitrageOpportunity, ExecutionPriority, RiskAssessment};
use crate::paths::ArbitragePath;
use crate::pools::PoolRegistry;
use crate::pricing::PriceManager;
use crate::dexes::DEX;

use super::detection::{ArbitragePattern, analyze_transaction_for_arbitrage, is_arbitrage_transaction};
use super::identifiers::*;
use super::ProtocolRegistry;

/// Arbitrage opportunity detector
pub struct ArbitrageOpportunityDetector {
    /// Protocol registry
    protocol_registry: Arc<ProtocolRegistry>,
    
    /// Pool registry
    pool_registry: Arc<PoolRegistry>,
    
    /// Price manager
    price_manager: Arc<PriceManager>,
    
    /// Minimum profit threshold in USD
    min_profit_threshold_usd: f64,
    
    /// Minimum profit threshold in basis points
    min_profit_threshold_bps: u32,
    
    /// Maximum gas cost in lamports
    max_gas_cost_lamports: u64,
    
    /// Recently seen opportunities (to avoid duplicates)
    recent_opportunities: HashSet<String>,
}

impl ArbitrageOpportunityDetector {
    /// Create a new arbitrage opportunity detector
    pub fn new(
        protocol_registry: Arc<ProtocolRegistry>,
        pool_registry: Arc<PoolRegistry>,
        price_manager: Arc<PriceManager>,
        min_profit_threshold_usd: f64,
        min_profit_threshold_bps: u32,
        max_gas_cost_lamports: u64,
    ) -> Self {
        Self {
            protocol_registry,
            pool_registry,
            price_manager,
            min_profit_threshold_usd,
            min_profit_threshold_bps,
            max_gas_cost_lamports,
            recent_opportunities: HashSet::new(),
        }
    }
    
    /// Analyze a transaction for arbitrage opportunities
    pub fn analyze_transaction(&self, transaction: &Transaction) -> Option<ArbitrageOpportunity> {
        // Skip if not an arbitrage transaction
        if !is_arbitrage_transaction(transaction) {
            return None;
        }
        
        // Analyze the transaction for arbitrage patterns
        let pattern = analyze_transaction_for_arbitrage(transaction)?;
        
        // Extract key information from the transaction
        let (input_token, output_token, path, estimated_profit) = self.extract_arbitrage_info(transaction, pattern)?;
        
        // Calculate profit in USD
        let input_token_price = self.price_manager.get_token_price(&input_token).ok()?;
        let profit_usd = estimated_profit as f64 * input_token_price;
        
        // Calculate profit in basis points
        let input_amount = self.extract_input_amount(transaction)?;
        let profit_bps = ((estimated_profit as f64 / input_amount as f64) * 10000.0) as u32;
        
        // Check if profitable enough
        if profit_usd < self.min_profit_threshold_usd || profit_bps < self.min_profit_threshold_bps {
            return None;
        }
        
        // Create opportunity ID
        let opportunity_id = format!("arb-{}-{}", pattern_to_string(pattern), transaction.signatures[0]);
        
        // Skip if already seen
        if self.recent_opportunities.contains(&opportunity_id) {
            return None;
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
            id: opportunity_id.clone(),
            path,
            input_amount,
            expected_output: input_amount + estimated_profit,
            expected_profit_usd: profit_usd,
            expected_profit_bps: profit_bps,
            timestamp: chrono::Utc::now(),
            ttl_ms: 10_000, // 10 seconds TTL
            priority,
            strategy: pattern_to_string(pattern).to_string(),
            risk,
        };
        
        // Add to recent opportunities
        self.recent_opportunities.insert(opportunity_id);
        
        // Limit size of recent opportunities
        if self.recent_opportunities.len() > 1000 {
            // Remove oldest (not ideal but simple)
            if let Some(oldest) = self.recent_opportunities.iter().next().cloned() {
                self.recent_opportunities.remove(&oldest);
            }
        }
        
        Some(opportunity)
    }
    
    /// Extract arbitrage information from a transaction
    fn extract_arbitrage_info(
        &self,
        transaction: &Transaction,
        pattern: ArbitragePattern,
    ) -> Option<(Pubkey, Pubkey, ArbitragePath, u64)> {
        // This is a simplified implementation
        // A real implementation would parse the transaction in detail
        
        // For now, we'll create a dummy path based on the pattern
        match pattern {
            ArbitragePattern::Circular => {
                // For circular, we assume the same token at start and end
                let token = transaction.message.account_keys[5]; // Arbitrary token account
                let path = ArbitragePath {
                    tokens: vec![token, token, token, token], // Start and end with same token
                    pools: vec![
                        "Raydium".to_string(),
                        "Orca".to_string(),
                        "Jupiter".to_string(),
                    ],
                    dexes: vec![
                        DEX::Raydium,
                        DEX::Orca,
                        DEX::Jupiter,
                    ],
                };
                
                Some((token, token, path, 20000)) // Estimated profit
            },
            ArbitragePattern::Triangular => {
                // For triangular, we use three different tokens
                let token1 = transaction.message.account_keys[5]; // Arbitrary token accounts
                let token2 = transaction.message.account_keys[6];
                let token3 = transaction.message.account_keys[7];
                
                let path = ArbitragePath {
                    tokens: vec![token1, token2, token3, token1], // Three different tokens
                    pools: vec![
                        "Raydium".to_string(),
                        "Orca".to_string(),
                        "Jupiter".to_string(),
                    ],
                    dexes: vec![
                        DEX::Raydium,
                        DEX::Orca,
                        DEX::Jupiter,
                    ],
                };
                
                Some((token1, token1, path, 20000)) // Estimated profit
            },
            ArbitragePattern::CrossExchange => {
                // For cross-exchange, we use two tokens across different exchanges
                let token1 = transaction.message.account_keys[5]; // Arbitrary token accounts
                let token2 = transaction.message.account_keys[6];
                
                let path = ArbitragePath {
                    tokens: vec![token1, token2, token1], // Two tokens
                    pools: vec![
                        "Raydium".to_string(),
                        "Orca".to_string(),
                    ],
                    dexes: vec![
                        DEX::Raydium,
                        DEX::Orca,
                    ],
                };
                
                Some((token1, token1, path, 20000)) // Estimated profit
            },
        }
    }
    
    /// Extract input amount from a transaction
    fn extract_input_amount(&self, transaction: &Transaction) -> Option<u64> {
        // This is a simplified implementation
        // A real implementation would parse the transaction in detail
        
        // For now, return a dummy amount
        Some(1000000)
    }
    
    /// Clear recent opportunities
    pub fn clear_recent_opportunities(&mut self) {
        self.recent_opportunities.clear();
    }
}

/// Convert arbitrage pattern to string
fn pattern_to_string(pattern: ArbitragePattern) -> &'static str {
    match pattern {
        ArbitragePattern::Circular => "circular",
        ArbitragePattern::Triangular => "triangular",
        ArbitragePattern::CrossExchange => "cross-exchange",
    }
}

/// Arbitrage opportunity scanner for mempool and block data
pub struct ArbitrageScanner {
    /// Opportunity detector
    detector: Arc<ArbitrageOpportunityDetector>,
    
    /// Protocol registry
    protocol_registry: Arc<ProtocolRegistry>,
    
    /// Detected opportunities
    opportunities: Vec<ArbitrageOpportunity>,
    
    /// Maximum opportunities to track
    max_opportunities: usize,
}

impl ArbitrageScanner {
    /// Create a new arbitrage scanner
    pub fn new(
        detector: Arc<ArbitrageOpportunityDetector>,
        protocol_registry: Arc<ProtocolRegistry>,
        max_opportunities: usize,
    ) -> Self {
        Self {
            detector,
            protocol_registry,
            opportunities: Vec::new(),
            max_opportunities,
        }
    }
    
    /// Scan a batch of transactions for arbitrage opportunities
    pub fn scan_transactions(&mut self, transactions: &[Transaction]) -> Vec<ArbitrageOpportunity> {
        let mut new_opportunities = Vec::new();
        
        for tx in transactions {
            if let Some(opportunity) = self.detector.analyze_transaction(tx) {
                new_opportunities.push(opportunity.clone());
                self.opportunities.push(opportunity);
            }
        }
        
        // Limit size of opportunities list
        if self.opportunities.len() > self.max_opportunities {
            self.opportunities.drain(0..(self.opportunities.len() - self.max_opportunities));
        }
        
        new_opportunities
    }
    
    /// Get all detected opportunities
    pub fn get_opportunities(&self) -> &[ArbitrageOpportunity] {
        &self.opportunities
    }
    
    /// Clear all opportunities
    pub fn clear_opportunities(&mut self) {
        self.opportunities.clear();
    }
}

/// Arbitrage opportunity analyzer for historical data
pub struct ArbitrageAnalyzer {
    /// Protocol registry
    protocol_registry: Arc<ProtocolRegistry>,
    
    /// Pool registry
    pool_registry: Arc<PoolRegistry>,
    
    /// Price manager
    price_manager: Arc<PriceManager>,
    
    /// Protocol usage statistics
    protocol_stats: HashMap<String, ProtocolStats>,
}

/// Protocol usage statistics
#[derive(Debug, Clone, Default)]
pub struct ProtocolStats {
    /// Number of arbitrage opportunities involving this protocol
    pub opportunity_count: usize,
    
    /// Total profit in USD
    pub total_profit_usd: f64,
    
    /// Average profit in basis points
    pub avg_profit_bps: f64,
    
    /// Success rate (0-100)
    pub success_rate: f64,
}

impl ArbitrageAnalyzer {
    /// Create a new arbitrage analyzer
    pub fn new(
        protocol_registry: Arc<ProtocolRegistry>,
        pool_registry: Arc<PoolRegistry>,
        price_manager: Arc<PriceManager>,
    ) -> Self {
        Self {
            protocol_registry,
            pool_registry,
            price_manager,
            protocol_stats: HashMap::new(),
        }
    }
    
    /// Analyze an arbitrage opportunity
    pub fn analyze_opportunity(&mut self, opportunity: &ArbitrageOpportunity, success: bool) {
        // Update protocol stats for each pool in the path
        for pool_name in &opportunity.path.pools {
            let stats = self.protocol_stats.entry(pool_name.clone())
                .or_insert_with(ProtocolStats::default);
            
            stats.opportunity_count += 1;
            stats.total_profit_usd += opportunity.expected_profit_usd;
            stats.avg_profit_bps = (stats.avg_profit_bps * (stats.opportunity_count - 1) as f64 + opportunity.expected_profit_bps as f64) / stats.opportunity_count as f64;
            
            if success {
                stats.success_rate = (stats.success_rate * (stats.opportunity_count - 1) as f64 + 100.0) / stats.opportunity_count as f64;
            } else {
                stats.success_rate = (stats.success_rate * (stats.opportunity_count - 1) as f64) / stats.opportunity_count as f64;
            }
        }
    }
    
    /// Get protocol statistics
    pub fn get_protocol_stats(&self) -> &HashMap<String, ProtocolStats> {
        &self.protocol_stats
    }
    
    /// Get statistics for a specific protocol
    pub fn get_protocol_stat(&self, protocol_name: &str) -> Option<&ProtocolStats> {
        self.protocol_stats.get(protocol_name)
    }
    
    /// Clear all statistics
    pub fn clear_stats(&mut self) {
        self.protocol_stats.clear();
    }
}