//! Circular arbitrage strategy implementation
//!
//! This module provides an implementation of the circular arbitrage strategy,
//! which looks for arbitrage opportunities that start and end with the same token.

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

/// Circular arbitrage strategy (same token in and out)
pub struct CircularArbitrageStrategy;

impl CircularArbitrageStrategy {
    /// Create a new circular arbitrage strategy
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ArbitrageStrategy for CircularArbitrageStrategy {
    fn name(&self) -> &str {
        "CircularArbitrage"
    }
    
    async fn initialize(&self) -> Result<()> {
        info!("Initializing circular arbitrage strategy");
        Ok(())
    }
    
    async fn find_opportunities(
        &self,
        pool_registry: Arc<PoolRegistry>,
        price_manager: Arc<PriceManager>,
        path_finder: Arc<PathFinder>,
        dex_registry: Arc<DexRegistry>,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        debug!("Finding circular arbitrage opportunities");
        
        let mut opportunities = Vec::new();
        
        // Focus on major tokens
        let tokens = vec![
            native_token::id(), // SOL
            solana_program::pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // USDC
            solana_program::pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"), // USDT
            // Add other major tokens here
        ];
        
        // Check each token
        for token in tokens {
            trace!("Checking circular arbitrage for token: {}", token);
            
            // Find paths that start and end with this token
            let paths = path_finder.find_paths(token, pool_registry.clone(), 4).await?;
            
            for path in paths {
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
                        let price_feed = match price_manager.get_price(&token) {
                            Some(feed) => feed,
                            None => {
                                trace!("Price not found for token: {}", token);
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
                        
                        debug!("Found circular arbitrage opportunity: {} with profit {} bps (${:.2})",
                              opportunity.id, opportunity.expected_profit_bps, opportunity.expected_profit_usd);
                        
                        opportunities.push(opportunity);
                    }
                }
            }
        }
        
        info!("Found {} circular arbitrage opportunities", opportunities.len());
        Ok(opportunities)
    }
}