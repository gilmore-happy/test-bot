//! Cross-exchange arbitrage strategy implementation
//!
//! This module provides an implementation of the cross-exchange arbitrage strategy,
//! which looks for arbitrage opportunities across different DEXes.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use solana_program::pubkey::Pubkey;
use solana_sdk::native_token;
use tracing::{debug, info, trace};

use super::{ArbitrageOpportunity, ArbitrageStrategy, ExecutionPriority, StrategyManager};
use crate::dexes::{DEX, DexRegistry};
use crate::paths::{ArbitragePath, PathFinder};
use crate::pools::{LiquidityPool, PoolRegistry};
use crate::pricing::PriceManager;

/// Cross-exchange arbitrage strategy
pub struct CrossExchangeArbitrageStrategy;

impl CrossExchangeArbitrageStrategy {
    /// Create a new cross-exchange arbitrage strategy
    pub fn new() -> Self {
        Self
    }
    
    /// Find price differences between DEXes for a token pair
    async fn find_dex_price_differences(
        &self,
        token_a: Pubkey,
        token_b: Pubkey,
        pool_registry: &PoolRegistry,
        dex_registry: &DexRegistry,
    ) -> Result<Vec<(LiquidityPool, LiquidityPool, f64)>> {
        let mut opportunities = Vec::new();
        
        // Get all pools for this token pair
        let pools = pool_registry.get_pools_for_pair(token_a, token_b);
        
        // Group pools by DEX
        let mut dex_pools: std::collections::HashMap<DEX, Vec<LiquidityPool>> = std::collections::HashMap::new();
        
        for pool in pools {
            dex_pools.entry(pool.dex).or_default().push(pool);
        }
        
        // Compare prices across DEXes
        for (dex1, pools1) in &dex_pools {
            for (dex2, pools2) in &dex_pools {
                // Skip comparing the same DEX
                if dex1 == dex2 {
                    continue;
                }
                
                // Compare each pool from dex1 with each pool from dex2
                for pool1 in pools1 {
                    for pool2 in pools2 {
                        // Get DEX clients
                        let dex1_client = match dex_registry.get_client(dex1) {
                            Some(client) => client,
                            None => continue,
                        };
                        
                        let dex2_client = match dex_registry.get_client(dex2) {
                            Some(client) => client,
                            None => continue,
                        };
                        
                        // Calculate rates for a standard amount (e.g., 1 SOL)
                        let amount = 1_000_000_000; // 1 SOL in lamports
                        
                        // Calculate output on dex1 (token_a -> token_b)
                        let output1 = match dex1_client.calculate_output_amount(pool1, token_a, amount) {
                            Ok(amount) => amount,
                            Err(_) => continue,
                        };
                        
                        // Calculate output on dex2 (token_b -> token_a)
                        let output2 = match dex2_client.calculate_output_amount(pool2, token_b, output1) {
                            Ok(amount) => amount,
                            Err(_) => continue,
                        };
                        
                        // Check if there's a profit opportunity
                        if output2 > amount {
                            // Calculate profit percentage
                            let profit_percentage = (output2 as f64 / amount as f64) - 1.0;
                            
                            opportunities.push((pool1.clone(), pool2.clone(), profit_percentage));
                        }
                    }
                }
            }
        }
        
        // Sort by profit percentage (descending)
        opportunities.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(opportunities)
    }
}

#[async_trait]
impl ArbitrageStrategy for CrossExchangeArbitrageStrategy {
    fn name(&self) -> &str {
        "CrossExchangeArbitrage"
    }
    
    async fn initialize(&self) -> Result<()> {
        info!("Initializing cross-exchange arbitrage strategy");
        Ok(())
    }
    
    async fn find_opportunities(
        &self,
        pool_registry: Arc<PoolRegistry>,
        price_manager: Arc<PriceManager>,
        path_finder: Arc<PathFinder>,
        dex_registry: Arc<DexRegistry>,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        debug!("Finding cross-exchange arbitrage opportunities");
        
        let mut opportunities = Vec::new();
        
        // Focus on major token pairs
        let token_pairs = vec![
            // SOL-USDC
            (
                native_token::id(),
                solana_program::pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
            ),
            // SOL-USDT
            (
                native_token::id(),
                solana_program::pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB")
            ),
            // USDC-USDT
            (
                solana_program::pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
                solana_program::pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB")
            ),
            // Add other major token pairs here
        ];
        
        // Check each token pair
        for (token_a, token_b) in token_pairs {
            trace!("Checking cross-exchange arbitrage for pair: {} - {}", token_a, token_b);
            
            // Find price differences between DEXes
            let dex_opportunities = self.find_dex_price_differences(
                token_a,
                token_b,
                &pool_registry,
                &dex_registry,
            ).await?;
            
            // Create arbitrage opportunities from price differences
            for (pool1, pool2, profit_percentage) in dex_opportunities {
                // Create path
                let path = ArbitragePath {
                    tokens: vec![token_a, token_b, token_a],
                    pools: vec![pool1.address.to_string(), pool2.address.to_string()],
                    dexes: vec![pool1.dex, pool2.dex],
                };
                
                // Try different amounts
                for &amount in &[1_000_000_000, 5_000_000_000, 10_000_000_000] {
                    // Simulate path execution
                    let sim_result = path_finder.simulate_path(
                        &path,
                        amount,
                        pool_registry.clone(),
                        dex_registry.clone(),
                    ).await?;
                    
                    // Check if profitable
                    if sim_result.profit_bps > 0 {
                        // Get token price for USD calculation
                        let price_feed = match price_manager.get_price(&token_a) {
                            Some(feed) => feed,
                            None => {
                                trace!("Price not found for token: {}", token_a);
                                continue;
                            }
                        };
                        
                        // Calculate profit in USD
                        let profit_amount = sim_result.output_amount.saturating_sub(amount);
                        let profit_usd = (profit_amount as f64 / 1_000_000_000.0) * price_feed.price_usd;
                        
                        // Create opportunity
                        let strategy_manager = StrategyManager::new(&Default::default());
                        let opportunity = strategy_manager.create_opportunity(
                            path,
                            amount,
                            sim_result.output_amount,
                            profit_usd,
                            sim_result.profit_bps,
                            self.name(),
                        );
                        
                        debug!("Found cross-exchange arbitrage opportunity: {} with profit {} bps (${:.2})",
                              opportunity.id, opportunity.expected_profit_bps, opportunity.expected_profit_usd);
                        
                        opportunities.push(opportunity);
                    }
                }
            }
        }
        
        info!("Found {} cross-exchange arbitrage opportunities", opportunities.len());
        Ok(opportunities)
    }
}