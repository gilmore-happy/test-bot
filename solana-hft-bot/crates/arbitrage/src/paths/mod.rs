//! Path finding implementations for arbitrage
//!
//! This module provides algorithms and data structures for finding
//! profitable arbitrage paths between tokens.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use tracing::{debug, info, trace};

use crate::config::ArbitrageConfig;
use crate::dexes::{DEX, DexRegistry};
use crate::pools::{LiquidityPool, PoolRegistry};
use crate::pricing::PriceManager;

/// Arbitrage path between tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitragePath {
    /// Token addresses in the path
    pub tokens: Vec<Pubkey>,
    
    /// Pool addresses in the path
    pub pools: Vec<String>,
    
    /// DEX types for each pool
    pub dexes: Vec<DEX>,
}

/// Path simulation result
#[derive(Debug, Clone)]
pub struct PathSimulationResult {
    /// Input amount
    pub input_amount: u64,
    
    /// Output amount
    pub output_amount: u64,
    
    /// Profit in basis points
    pub profit_bps: u32,
    
    /// Intermediate amounts at each step
    pub intermediate_amounts: Vec<u64>,
}

/// Path finder for finding arbitrage paths
pub struct PathFinder {
    /// Configuration
    config: ArbitrageConfig,
}

impl PathFinder {
    /// Create a new path finder
    pub fn new(config: &ArbitrageConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }
    
    /// Find arbitrage paths for a token
    pub async fn find_paths(
        &self,
        token: Pubkey,
        pool_registry: Arc<PoolRegistry>,
        max_depth: usize,
    ) -> Result<Vec<ArbitragePath>> {
        info!("Finding arbitrage paths for token: {} with max depth: {}", token, max_depth);
        
        let mut paths = Vec::new();
        let mut visited = HashSet::new();
        let mut current_path = ArbitragePath {
            tokens: vec![token],
            pools: Vec::new(),
            dexes: Vec::new(),
        };
        
        // Find all paths that start and end with the same token
        self.dfs_paths(
            token,
            token,
            max_depth,
            pool_registry,
            &mut visited,
            &mut current_path,
            &mut paths,
        );
        
        info!("Found {} potential arbitrage paths", paths.len());
        Ok(paths)
    }
    
    /// Simulate execution of a path to calculate profit
    pub async fn simulate_path(
        &self,
        path: &ArbitragePath,
        input_amount: u64,
        pool_registry: Arc<PoolRegistry>,
        dex_registry: Arc<DexRegistry>,
    ) -> Result<PathSimulationResult> {
        trace!("Simulating path with input amount: {}", input_amount);
        
        // Check if path is valid
        if path.tokens.len() < 2 || path.pools.len() != path.tokens.len() - 1 {
            return Err(anyhow!("Invalid path"));
        }
        
        let mut amounts = vec![input_amount];
        let mut current_amount = input_amount;
        
        // Simulate each step in the path
        for i in 0..path.pools.len() {
            let pool_id = &path.pools[i];
            let token_in = path.tokens[i];
            let token_out = path.tokens[i + 1];
            
            let pool = pool_registry.get_pool(pool_id)
                .ok_or_else(|| anyhow!("Pool not found: {}", pool_id))?;
            
            // Get the DEX client
            let dex_client = dex_registry.get_client(&pool.dex)
                .ok_or_else(|| anyhow!("DEX client not found: {:?}", pool.dex))?;
            
            // Calculate output amount
            let output_amount = dex_client.calculate_output_amount(
                &pool,
                token_in,
                current_amount,
            )?;
            
            current_amount = output_amount;
            amounts.push(current_amount);
        }
        
        // Calculate profit in basis points
        let profit_bps = if input_amount > 0 {
            ((current_amount as f64 / input_amount as f64) - 1.0) * 10000.0
        } else {
            0.0
        } as u32;
        
        trace!("Path simulation result: input={}, output={}, profit_bps={}", 
              input_amount, current_amount, profit_bps);
        
        Ok(PathSimulationResult {
            input_amount,
            output_amount: current_amount,
            profit_bps,
            intermediate_amounts: amounts,
        })
    }
    
    /// Find optimal input amount for a path
    pub async fn find_optimal_amount(
        &self,
        path: &ArbitragePath,
        pool_registry: Arc<PoolRegistry>,
        dex_registry: Arc<DexRegistry>,
        min_amount: u64,
        max_amount: u64,
    ) -> Result<(u64, PathSimulationResult)> {
        debug!("Finding optimal amount for path between {} and {}", min_amount, max_amount);
        
        // Binary search for optimal amount
        let mut low = min_amount;
        let mut high = max_amount;
        let mut best_amount = min_amount;
        let mut best_result = self.simulate_path(path, min_amount, pool_registry.clone(), dex_registry.clone()).await?;
        
        // Number of iterations for binary search
        const ITERATIONS: usize = 10;
        
        for _ in 0..ITERATIONS {
            let mid = low + (high - low) / 2;
            
            let result = self.simulate_path(path, mid, pool_registry.clone(), dex_registry.clone()).await?;
            
            if result.profit_bps > best_result.profit_bps {
                best_amount = mid;
                best_result = result;
            }
            
            // Try to increase amount if profit is increasing
            let higher_amount = mid + (high - mid) / 2;
            let higher_result = self.simulate_path(path, higher_amount, pool_registry.clone(), dex_registry.clone()).await?;
            
            if higher_result.profit_bps > result.profit_bps {
                low = mid;
            } else {
                high = mid;
            }
        }
        
        debug!("Optimal amount found: {} with profit_bps: {}", 
              best_amount, best_result.profit_bps);
        
        Ok((best_amount, best_result))
    }
    
    /// DFS to find all paths between start and end tokens
    fn dfs_paths(
        &self,
        start: Pubkey,
        end: Pubkey,
        max_depth: usize,
        pool_registry: Arc<PoolRegistry>,
        visited: &mut HashSet<(Pubkey, String)>, // (token, pool)
        current_path: &mut ArbitragePath,
        paths: &mut Vec<ArbitragePath>,
    ) {
        // If we've reached the end token and have gone through at least one pool
        if current_path.tokens.len() > 1 && current_path.tokens.last().unwrap() == &end {
            paths.push(current_path.clone());
            return;
        }
        
        // If we've reached max depth
        if current_path.tokens.len() > max_depth {
            return;
        }
        
        let current_token = *current_path.tokens.last().unwrap();
        
        // Get all pools that have this token
        let pools = pool_registry.get_all_pools();
        
        for pool in pools {
            // Skip if we've already visited this pool
            let pool_id = pool.address.to_string();
            let key = (current_token, pool_id.clone());
            if visited.contains(&key) {
                continue;
            }
            
            // Find the other token in the pool
            let next_token = if pool.token_a == current_token {
                pool.token_b
            } else if pool.token_b == current_token {
                pool.token_a
            } else {
                continue; // Pool doesn't contain current token
            };
            
            // Skip if next token is already in the path (except for the end token)
            if current_path.tokens.contains(&next_token) && next_token != end {
                continue;
            }
            
            // Add to path
            visited.insert(key);
            current_path.tokens.push(next_token);
            current_path.pools.push(pool_id.clone());
            current_path.dexes.push(pool.dex);
            
            // Recursively search from next token
            self.dfs_paths(
                start,
                end,
                max_depth,
                pool_registry.clone(),
                visited,
                current_path,
                paths,
            );
            
            // Backtrack
            current_path.tokens.pop();
            current_path.pools.pop();
            current_path.dexes.pop();
            visited.remove(&(current_token, pool_id));
        }
    }
    
    /// Find arbitrage opportunities using Bellman-Ford algorithm
    pub async fn find_bellman_ford_opportunities(
        &self,
        pool_registry: Arc<PoolRegistry>,
        dex_registry: Arc<DexRegistry>,
    ) -> Result<Vec<ArbitragePath>> {
        info!("Finding arbitrage opportunities using Bellman-Ford algorithm");
        
        // Build graph from pools
        let pools = pool_registry.get_all_pools();
        let mut token_to_index = HashMap::new();
        let mut index_to_token = HashMap::new();
        
        // Collect all unique tokens
        let mut tokens = HashSet::new();
        for pool in &pools {
            tokens.insert(pool.token_a);
            tokens.insert(pool.token_b);
        }
        
        // Assign indices to tokens
        for (i, token) in tokens.iter().enumerate() {
            token_to_index.insert(*token, i);
            index_to_token.insert(i, *token);
        }
        
        let n = tokens.len();
        
        // Create adjacency list
        let mut adj_list = vec![Vec::new(); n];
        
        // Add edges for each pool
        for pool in &pools {
            let a_idx = token_to_index[&pool.token_a];
            let b_idx = token_to_index[&pool.token_b];
            
            // Get DEX client for rate calculation
            if let Some(dex_client) = dex_registry.get_client(&pool.dex) {
                // Add edge A -> B
                let rate_a_to_b = match dex_client.calculate_output_amount(pool, pool.token_a, 1_000_000) {
                    Ok(amount) => -((amount as f64) / 1_000_000.0).ln(), // Negative log for Bellman-Ford
                    Err(_) => continue,
                };
                
                adj_list[a_idx].push((b_idx, rate_a_to_b, pool.clone()));
                
                // Add edge B -> A
                let rate_b_to_a = match dex_client.calculate_output_amount(pool, pool.token_b, 1_000_000) {
                    Ok(amount) => -((amount as f64) / 1_000_000.0).ln(), // Negative log for Bellman-Ford
                    Err(_) => continue,
                };
                
                adj_list[b_idx].push((a_idx, rate_b_to_a, pool.clone()));
            }
        }
        
        // Run Bellman-Ford from each token
        let mut arbitrage_paths = Vec::new();
        
        for start in 0..n {
            // Initialize distances
            let mut dist = vec![f64::INFINITY; n];
            let mut prev = vec![None; n];
            let mut prev_pool = vec![None; n];
            
            dist[start] = 0.0;
            
            // Relax edges |V| - 1 times
            for _ in 0..n-1 {
                for u in 0..n {
                    for &(v, weight, ref pool) in &adj_list[u] {
                        if dist[u] != f64::INFINITY && dist[u] + weight < dist[v] {
                            dist[v] = dist[u] + weight;
                            prev[v] = Some(u);
                            prev_pool[v] = Some(pool.clone());
                        }
                    }
                }
            }
            
            // Check for negative weight cycles
            for u in 0..n {
                for &(v, weight, ref pool) in &adj_list[u] {
                    if dist[u] != f64::INFINITY && dist[u] + weight < dist[v] {
                        // Negative cycle detected
                        
                        // Reconstruct the cycle
                        let mut cycle_tokens = Vec::new();
                        let mut cycle_pools = Vec::new();
                        let mut cycle_dexes = Vec::new();
                        
                        let mut visited = vec![false; n];
                        let mut curr = v;
                        
                        while !visited[curr] {
                            visited[curr] = true;
                            cycle_tokens.push(index_to_token[&curr]);
                            
                            if let Some(prev_idx) = prev[curr] {
                                if let Some(ref pool) = prev_pool[curr] {
                                    cycle_pools.push(pool.address.to_string());
                                    cycle_dexes.push(pool.dex);
                                }
                                curr = prev_idx;
                            } else {
                                break;
                            }
                        }
                        
                        // Complete the cycle
                        cycle_tokens.push(index_to_token[&curr]);
                        
                        // Create arbitrage path
                        let path = ArbitragePath {
                            tokens: cycle_tokens,
                            pools: cycle_pools,
                            dexes: cycle_dexes,
                        };
                        
                        arbitrage_paths.push(path);
                    }
                }
            }
        }
        
        info!("Found {} potential arbitrage paths using Bellman-Ford", arbitrage_paths.len());
        Ok(arbitrage_paths)
    }
}