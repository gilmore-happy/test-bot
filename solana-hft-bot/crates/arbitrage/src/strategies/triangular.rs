//! Triangular arbitrage strategy implementation
//!
//! This module provides an implementation of the triangular arbitrage strategy,
//! which looks for arbitrage opportunities across three different tokens.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use solana_program::pubkey::Pubkey;
use solana_sdk::native_token;
use tracing::{debug, info, trace};

use super::{ArbitrageOpportunity, ArbitrageStrategy, StrategyManager};
use crate::dexes::DexRegistry;
use crate::paths::{ArbitragePath, PathFinder};
use crate::pools::PoolRegistry;
use crate::pricing::PriceManager;

/// Triangular arbitrage strategy (different tokens)
pub struct TriangularArbitrageStrategy;

impl TriangularArbitrageStrategy {
    /// Create a new triangular arbitrage strategy
    pub fn new() -> Self {
        Self
    }
    
    /// Find triangular arbitrage opportunities for a specific token
    async fn find_triangular_paths(
        &self,
        start_token: Pubkey,
        pool_registry: Arc<PoolRegistry>,
        price_manager: Arc<PriceManager>,
        path_finder: Arc<PathFinder>,
        dex_registry: Arc<DexRegistry>,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        // Get all pools that contain the start token
        let all_pools = pool_registry.get_all_pools();
        let mut intermediate_tokens = Vec::new();
        
        // Find all tokens that can be directly traded with the start token
        for pool in &all_pools {
            if pool.token_a == start_token {
                intermediate_tokens.push(pool.token_b);
            } else if pool.token_b == start_token {
                intermediate_tokens.push(pool.token_a);
            }
        }
        
        // For each intermediate token, find paths back to the start token
        for intermediate_token in intermediate_tokens {
            // Find pools that connect the intermediate token to other tokens
            let mut third_tokens = Vec::new();
            
            for pool in &all_pools {
                if pool.token_a == intermediate_token && pool.token_b != start_token {
                    third_tokens.push(pool.token_b);
                } else if pool.token_b == intermediate_token && pool.token_a != start_token {
                    third_tokens.push(pool.token_a);
                }
            }
            
            // For each third token, check if it can be traded back to the start token
            for third_token in third_tokens {
                // Check if there's a pool connecting the third token back to the start token
                let has_path_back = all_pools.iter().any(|pool| 
                    (pool.token_a == third_token && pool.token_b == start_token) ||
                    (pool.token_b == third_token && pool.token_a == start_token)
                );
                
                if has_path_back {
                    // We have a triangular path: start -> intermediate -> third -> start
                    // Construct the path
                    let mut tokens = vec![start_token, intermediate_token, third_token, start_token];
                    let mut pools = Vec::new();
                    let mut dexes = Vec::new();
                    
                    // Find the pools for each step
                    for i in 0..3 {
                        let token_a = tokens[i];
                        let token_b = tokens[i + 1];
                        
                        // Find a pool for this pair
                        if let Some(pool) = all_pools.iter().find(|p| 
                            (p.token_a == token_a && p.token_b == token_b) ||
                            (p.token_b == token_a && p.token_a == token_b)
                        ) {
                            pools.push(pool.address.to_string());
                            dexes.push(pool.dex);
                        } else {
                            // This shouldn't happen based on our checks above
                            trace!("Could not find pool for pair: {} -> {}", token_a, token_b);
                            continue;
                        }
                    }
                    
                    // Create the path
                    let path = ArbitragePath {
                        tokens,
                        pools,
                        dexes,
                    };
                    
                    // Try different amounts
                    for &amount in &[1_000_000, 10_000_000, 100_000_000, 1_000_000_000] {
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
                            let price_feed = match price_manager.get_price(&start_token) {
                                Some(feed) => feed,
                                None => {
                                    trace!("Price not found for token: {}", start_token);
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
                            
                            debug!("Found triangular arbitrage opportunity: {} with profit {} bps (${:.2})",
                                  opportunity.id, opportunity.expected_profit_bps, opportunity.expected_profit_usd);
                            
                            opportunities.push(opportunity);
                        }
                    }
                }
            }
        }
        
        Ok(opportunities)
    }
}

#[async_trait]
impl ArbitrageStrategy for TriangularArbitrageStrategy {
    fn name(&self) -> &str {
        "TriangularArbitrage"
    }
    
    async fn initialize(&self) -> Result<()> {
        info!("Initializing triangular arbitrage strategy");
        Ok(())
    }
    
    async fn find_opportunities(
        &self,
        pool_registry: Arc<PoolRegistry>,
        price_manager: Arc<PriceManager>,
        path_finder: Arc<PathFinder>,
        dex_registry: Arc<DexRegistry>,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        debug!("Finding triangular arbitrage opportunities");
        
        let mut opportunities = Vec::new();
        
        // Focus on major tokens as starting points
        let tokens = vec![
            native_token::id(), // SOL
            solana_program::pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // USDC
            solana_program::pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"), // USDT
            // Add other major tokens here
        ];
        
        // Check each token as a starting point
        for token in tokens {
            trace!("Checking triangular arbitrage for token: {}", token);
            
            let token_opportunities = self.find_triangular_paths(
                token,
                pool_registry.clone(),
                price_manager.clone(),
                path_finder.clone(),
                dex_registry.clone(),
            ).await?;
            
            opportunities.extend(token_opportunities);
        }
        
        info!("Found {} triangular arbitrage opportunities", opportunities.len());
        Ok(opportunities)
    }
}