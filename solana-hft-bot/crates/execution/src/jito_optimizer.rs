//! Jito bundle optimization for Solana HFT Bot
//!
//! This module provides optimization capabilities for Jito MEV bundles:
//! - Transaction opportunity analysis
//! - Bundle optimization for maximum extraction
//! - Transaction grouping for atomic execution
//! - Pre-flight simulation for outcome prediction
//! - Tip optimization based on historical data

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::Signature,
    transaction::Transaction,
};
use tokio::sync::{mpsc, oneshot, Semaphore};
use tokio::time::timeout;
use tracing::{debug, error, info, instrument, trace, warn};

use crate::jito::SearcherClient;
use crate::ExecutionError;

// Import the protocol identifiers if available
#[cfg(feature = "arbitrage")]
use solana_hft_arbitrage::protocols::identifiers::*;
#[cfg(feature = "arbitrage")]
use solana_hft_arbitrage::protocols::detection::*;

/// Transaction opportunity type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpportunityType {
    /// Arbitrage opportunity
    Arbitrage,
    
    /// Liquidation opportunity
    Liquidation,
    
    /// Token launch opportunity
    TokenLaunch,
    
    /// Sandwich trade opportunity
    Sandwich,
    
    /// Other opportunity type
    Other,
}

/// Transaction opportunity
#[derive(Debug, Clone)]
pub struct TransactionOpportunity {
    /// Opportunity type
    pub opportunity_type: OpportunityType,
    
    /// Transaction
    pub transaction: Transaction,
    
    /// Estimated profit in lamports
    pub estimated_profit: u64,
    
    /// Transaction dependencies
    pub dependencies: Vec<Signature>,
    
    /// Priority (higher = more important)
    pub priority: u32,
    
    /// Timestamp when the opportunity was identified
    pub timestamp: Instant,
    
    /// Maximum tip to pay
    pub max_tip: u64,
    
    /// Whether the opportunity is time-sensitive
    pub time_sensitive: bool,
}

/// Bundle optimization result
#[derive(Debug, Clone)]
pub struct BundleOptimizationResult {
    /// Optimized transactions
    pub transactions: Vec<Transaction>,
    
    /// Total estimated profit
    pub total_profit: u64,
    
    /// Total tip amount
    pub total_tip: u64,
    
    /// Opportunity types included
    pub opportunity_types: Vec<OpportunityType>,
    
    /// Optimization timestamp
    pub timestamp: Instant,
}

/// Simulation result
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Whether the simulation was successful
    pub success: bool,
    
    /// Error message (if any)
    pub error: Option<String>,
    
    /// Simulation logs
    pub logs: Vec<String>,
    
    /// Compute units consumed
    pub units_consumed: u64,
    
    /// Fee estimate
    pub fee_estimate: u64,
}

/// Cost estimate
#[derive(Debug, Clone)]
pub struct CostEstimate {
    /// Transaction fee
    pub fee: u64,
    
    /// Tip amount
    pub tip: u64,
    
    /// Compute units
    pub compute_units: u64,
    
    /// Priority fee per compute unit
    pub priority_fee: u64,
}

/// Jito bundle optimizer
pub struct JitoBundleOptimizer {
    /// Maximum bundle size
    max_bundle_size: usize,
    
    /// Minimum tip in lamports
    min_tip_lamports: u64,
    
    /// Maximum tip in lamports
    max_tip_lamports: u64,
    
    /// Pending opportunities
    opportunities: Arc<RwLock<VecDeque<TransactionOpportunity>>>,
    
    /// Transaction dependencies
    dependencies: Arc<RwLock<HashMap<Signature, HashSet<Signature>>>>,
}

impl JitoBundleOptimizer {
    /// Create a new bundle optimizer
    pub fn new(
        max_bundle_size: usize,
        min_tip_lamports: u64,
        max_tip_lamports: u64,
    ) -> Self {
        Self {
            max_bundle_size,
            min_tip_lamports,
            max_tip_lamports,
            opportunities: Arc::new(RwLock::new(VecDeque::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add a transaction opportunity
    pub fn add_opportunity(&self, opportunity: TransactionOpportunity) {
        let mut opportunities = self.opportunities.write();
        
        // Add to queue
        opportunities.push_back(opportunity.clone());
        
        // Update dependencies
        let mut dependencies = self.dependencies.write();
        
        for dep in &opportunity.dependencies {
            dependencies.entry(*dep)
                .or_insert_with(HashSet::new)
                .insert(opportunity.transaction.signatures[0]);
        }
        
        // Keep queue size reasonable
        while opportunities.len() > 1000 {
            opportunities.pop_front();
        }
    }
    
    /// Optimize a bundle
    pub fn optimize_bundle(&self) -> Option<BundleOptimizationResult> {
        let opportunities = self.opportunities.read();
        
        if opportunities.is_empty() {
            return None;
        }
        
        // Sort opportunities by priority and profit
        let mut sorted_opportunities: Vec<_> = opportunities.iter().collect();
        
        sorted_opportunities.sort_by(|a, b| {
            // First sort by priority (descending)
            let priority_cmp = b.priority.cmp(&a.priority);
            
            if priority_cmp != std::cmp::Ordering::Equal {
                return priority_cmp;
            }
            
            // Then sort by profit per tip (descending)
            let a_profit_per_tip = if a.max_tip > 0 { a.estimated_profit as f64 / a.max_tip as f64 } else { 0.0 };
            let b_profit_per_tip = if b.max_tip > 0 { b.estimated_profit as f64 / b.max_tip as f64 } else { 0.0 };
            
            b_profit_per_tip.partial_cmp(&a_profit_per_tip).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Select transactions for the bundle
        let mut selected = Vec::new();
        let mut selected_signatures = HashSet::new();
        let mut total_profit = 0;
        let mut total_tip = 0;
        let mut opportunity_types = HashSet::new();
        
        let dependencies = self.dependencies.read();
        
        for opportunity in sorted_opportunities {
            // Skip if bundle is full
            if selected.len() >= self.max_bundle_size {
                break;
            }
            
            // Skip if any dependencies are not satisfied
            let mut dependencies_satisfied = true;
            
            for dep in &opportunity.dependencies {
                if !selected_signatures.contains(dep) {
                    dependencies_satisfied = false;
                    break;
                }
            }
            
            if !dependencies_satisfied {
                continue;
            }
            
            // Add to bundle
            selected.push(opportunity.transaction.clone());
            selected_signatures.insert(opportunity.transaction.signatures[0]);
            total_profit += opportunity.estimated_profit;
            // Calculate tip (clamped to min/max)
            let tip = if opportunity.max_tip > self.max_tip_lamports {
                self.max_tip_lamports
            } else if opportunity.max_tip < self.min_tip_lamports {
                self.min_tip_lamports
            } else {
                opportunity.max_tip
            };
            total_tip += tip;
            total_tip += tip;
            
            opportunity_types.insert(opportunity.opportunity_type);
        }
        
        if selected.is_empty() {
            return None;
        }
        
        Some(BundleOptimizationResult {
            transactions: selected,
            total_profit,
            total_tip,
            opportunity_types: opportunity_types.into_iter().collect(),
            timestamp: Instant::now(),
        })
    }
    
    /// Group related transactions
    pub fn group_related_transactions(&self, transactions: Vec<Transaction>) -> Vec<Vec<Transaction>> {
        if transactions.len() <= 1 {
            return vec![transactions];
        }
        
        // Build dependency graph
        let mut graph = HashMap::new();
        let dependencies = self.dependencies.read();
        
        for tx in &transactions {
            let signature = tx.signatures[0];
            
            // Add node
            graph.entry(signature).or_insert_with(HashSet::new);
            
            // Add edges for dependencies
            if let Some(deps) = dependencies.get(&signature) {
                for dep in deps {
                    if transactions.iter().any(|t| t.signatures[0] == *dep) {
                        graph.entry(signature).or_insert_with(HashSet::new).insert(*dep);
                    }
                }
            }
        }
        
        // Find connected components (groups of related transactions)
        let mut visited = HashSet::new();
        let mut groups = Vec::new();
        
        for tx in &transactions {
            let signature = tx.signatures[0];
            
            if !visited.contains(&signature) {
                let mut group = Vec::new();
                let mut queue = VecDeque::new();
                
                queue.push_back(signature);
                visited.insert(signature);
                
                while let Some(sig) = queue.pop_front() {
                    group.push(sig);
                    
                    if let Some(neighbors) = graph.get(&sig) {
                        for neighbor in neighbors {
                            if !visited.contains(neighbor) {
                                queue.push_back(*neighbor);
                                visited.insert(*neighbor);
                            }
                        }
                    }
                }
                
                // Convert signatures back to transactions
                let group_txs: Vec<_> = group.into_iter()
                    .filter_map(|sig| transactions.iter().find(|tx| tx.signatures[0] == sig))
                    .cloned()
                    .collect();
                
                if !group_txs.is_empty() {
                    groups.push(group_txs);
                }
            }
        }
        
        // If no groups were found, return the original transactions as a single group
        if groups.is_empty() {
            return vec![transactions];
        }
        
        groups
    }
    
    /// Clear all opportunities
    pub fn clear_opportunities(&self) {
        let mut opportunities = self.opportunities.write();
        opportunities.clear();
        
        let mut dependencies = self.dependencies.write();
        dependencies.clear();
    }
    
    /// Get the number of pending opportunities
    pub fn pending_count(&self) -> usize {
        let opportunities = self.opportunities.read();
        opportunities.len()
    }
}

/// Bundle simulator
pub struct BundleSimulator {
    /// RPC client
    rpc_client: Arc<RpcClient>,
    
    /// Searcher client (optional)
    searcher_client: Option<Arc<SearcherClient>>,
}

impl BundleSimulator {
    /// Create a new bundle simulator
    pub fn new(
        rpc_client: Arc<RpcClient>,
        searcher_client: Option<Arc<SearcherClient>>,
    ) -> Self {
        Self {
            rpc_client,
            searcher_client,
        }
    }
    
    /// Simulate a bundle of transactions
    pub async fn simulate_bundle(&self, transactions: &[Transaction]) -> Result<SimulationResult, ExecutionError> {
        if transactions.is_empty() {
            return Ok(SimulationResult {
                success: true,
                error: None,
                logs: Vec::new(),
                units_consumed: 0,
                fee_estimate: 0,
            });
        }
        
        // Use searcher client if available
        if let Some(searcher_client) = &self.searcher_client {
            // Convert transactions to bundle
            use crate::jito::{Bundle, BundleTransaction, TipDistribution};
            
            let mut bundle_transactions = Vec::with_capacity(transactions.len());
            
            for tx in transactions {
                let tip_distribution = TipDistribution::new_single_account(
                    tx.message.account_keys[0], // Use first account as tip account
                    1000, // Minimal tip for simulation
                );
                
                let bundle_tx = BundleTransaction::new(tx.clone(), tip_distribution);
                bundle_transactions.push(bundle_tx);
            }
            
            let bundle = match Bundle::new(bundle_transactions) {
                Ok(bundle) => bundle,
                Err(e) => {
                    return Err(ExecutionError::Simulation(format!("Failed to create bundle: {}", e)));
                }
            };
            
            // Simulate bundle
            match searcher_client.simulate_bundle(&bundle).await {
                Ok(result) => {
                    if result.success {
                        Ok(SimulationResult {
                            success: true,
                            error: None,
                            logs: result.logs,
                            units_consumed: result.units_consumed,
                            fee_estimate: result.fee,
                        })
                    } else {
                        Err(ExecutionError::Simulation(result.error.unwrap_or_else(|| "Unknown simulation error".to_string())))
                    }
                }
                Err(e) => {
                    Err(ExecutionError::Simulation(format!("Failed to simulate bundle: {}", e)))
                }
            }
        } else {
            // Fallback to individual transaction simulation
            let mut total_units = 0;
            let mut total_fee = 0;
            let mut all_logs = Vec::new();
            
            for tx in transactions {
                match self.rpc_client.simulate_transaction(tx).await {
                    Ok(result) => {
                        if let Some(err) = result.value.err {
                            return Err(ExecutionError::Simulation(format!("Transaction simulation failed: {:?}", err)));
                        }
                        
                        if let Some(units) = result.value.units_consumed {
                            total_units += units;
                        }
                        
                        if let Some(logs) = result.value.logs {
                            all_logs.extend(logs);
                        }
                        
                        // Estimate fee
                        total_fee += tx.message.header.num_required_signatures as u64 * 5000;
                    }
                    Err(e) => {
                        return Err(ExecutionError::Simulation(format!("Failed to simulate transaction: {}", e)));
                    }
                }
            }
            
            Ok(SimulationResult {
                success: true,
                error: None,
                logs: all_logs,
                units_consumed: total_units,
                fee_estimate: total_fee,
            })
        }
    }
}

/// Tip optimizer
pub struct TipOptimizer {
    /// Minimum tip in lamports
    min_tip_lamports: u64,
    
    /// Maximum tip in lamports
    max_tip_lamports: u64,
    
    /// Historical tip data by opportunity type
    historical_tips: Arc<RwLock<HashMap<OpportunityType, VecDeque<(u64, bool)>>>>,
    
    /// Network congestion level
    congestion_level: Arc<RwLock<f64>>,
}

impl TipOptimizer {
    /// Create a new tip optimizer
    pub fn new(
        min_tip_lamports: u64,
        max_tip_lamports: u64,
    ) -> Self {
        let mut historical_tips = HashMap::new();
        
        // Initialize historical tips for each opportunity type
        for &opportunity_type in &[
            OpportunityType::Arbitrage,
            OpportunityType::Liquidation,
            OpportunityType::TokenLaunch,
            OpportunityType::Sandwich,
            OpportunityType::Other,
        ] {
            historical_tips.insert(opportunity_type, VecDeque::with_capacity(100));
        }
        
        Self {
            min_tip_lamports,
            max_tip_lamports,
            historical_tips: Arc::new(RwLock::new(historical_tips)),
            congestion_level: Arc::new(RwLock::new(1.0)),
        }
    }
    
    /// Record a tip result
    pub fn record_tip_result(&self, opportunity_type: OpportunityType, tip: u64, success: bool) {
        let mut historical_tips = self.historical_tips.write();
        
        if let Some(tips) = historical_tips.get_mut(&opportunity_type) {
            tips.push_back((tip, success));
            
            // Keep queue size reasonable
            while tips.len() > 100 {
                tips.pop_front();
            }
        }
    }
    
    /// Update congestion level
    pub fn update_congestion_level(&self, level: f64) {
        let mut congestion = self.congestion_level.write();
        *congestion = level;
    }
    
    /// Calculate optimal tip
    pub fn calculate_optimal_tip(
        &self,
        opportunity_type: OpportunityType,
        value: u64,
        time_sensitive: bool,
    ) -> u64 {
        let historical_tips = self.historical_tips.read();
        let congestion = *self.congestion_level.read();
        
        // Get historical tips for this opportunity type
        let tips = historical_tips.get(&opportunity_type).unwrap_or_else(|| {
            historical_tips.get(&OpportunityType::Other).unwrap()
        });
        
        // Calculate base tip based on historical data
        let base_tip = if tips.is_empty() {
            // No historical data, use default
            self.min_tip_lamports
        } else {
            // Calculate average successful tip
            let successful_tips: Vec<_> = tips.iter()
                .filter(|(_, success)| *success)
                .map(|(tip, _)| *tip)
                .collect();
            
            if successful_tips.is_empty() {
                // No successful tips, use default
                self.min_tip_lamports
            } else {
                // Use median successful tip
                let mut sorted_tips = successful_tips.clone();
                sorted_tips.sort();
                
                sorted_tips[sorted_tips.len() / 2]
            }
        };
        
        // Adjust for value (higher value = higher tip)
        let value_factor = if value > 0 {
            // Cap at 10% of value
            (value as f64 * 0.1).min(self.max_tip_lamports as f64) / self.max_tip_lamports as f64
        } else {
            0.0
        };
        
        // Adjust for time sensitivity
        let time_factor = if time_sensitive { 2.0 } else { 1.0 };
        
        // Adjust for congestion
        let congestion_factor = congestion;
        
        // Calculate final tip
        let tip = (base_tip as f64 * (1.0 + value_factor) * time_factor * congestion_factor) as u64;
        
        // Clamp to min/max
        tip.clamp(self.min_tip_lamports, self.max_tip_lamports)
    }
    
    /// Get the success rate for a given tip amount
    pub fn get_success_rate(&self, opportunity_type: OpportunityType, tip: u64) -> f64 {
        let historical_tips = self.historical_tips.read();
        
        // Get historical tips for this opportunity type
        let tips = historical_tips.get(&opportunity_type).unwrap_or_else(|| {
            historical_tips.get(&OpportunityType::Other).unwrap()
        });
        
        if tips.is_empty() {
            return 0.5; // No data, assume 50% success rate
        }
        
        // Count tips around the given amount (±20%)
        let min_tip = (tip as f64 * 0.8) as u64;
        let max_tip = (tip as f64 * 1.2) as u64;
        
        let similar_tips: Vec<_> = tips.iter()
            .filter(|(t, _)| *t >= min_tip && *t <= max_tip)
            .collect();
        
        if similar_tips.is_empty() {
            return 0.5; // No similar tips, assume 50% success rate
        }
        
        // Calculate success rate
        let success_count = similar_tips.iter().filter(|(_, success)| **success).count();
        
        success_count as f64 / similar_tips.len() as f64
    }
    
    /// Clear historical tip data
    pub fn clear_historical_data(&self) {
        let mut historical_tips = self.historical_tips.write();
        
        for tips in historical_tips.values_mut() {
            tips.clear();
        }
    }
}