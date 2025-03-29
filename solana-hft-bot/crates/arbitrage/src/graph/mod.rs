//! Graph algorithm implementations for arbitrage
//!
//! This module provides graph-based algorithms for finding arbitrage opportunities.

use std::collections::{HashMap, HashSet};
use std::cmp::Ordering;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use tracing::{debug, info, trace};

use crate::dexes::{DEX, DEXClient};
use crate::paths::ArbitragePath;
use crate::pools::LiquidityPool;

/// Node in the arbitrage graph
#[derive(Debug, Clone)]
pub struct Node {
    /// Token address
    pub token: Pubkey,
    
    /// Edges to other nodes (token, pool, dex)
    pub edges: Vec<Edge>,
}

/// Edge in the arbitrage graph
#[derive(Debug, Clone)]
pub struct Edge {
    /// Destination token
    pub to_token: Pubkey,
    
    /// Pool address
    pub pool_id: String,
    
    /// DEX type
    pub dex: DEX,
    
    /// Exchange rate (output/input)
    pub rate: f64,
}

/// Arbitrage graph
pub struct ArbitrageGraph {
    /// Nodes in the graph
    nodes: HashMap<Pubkey, Node>,
}

impl ArbitrageGraph {
    /// Create a new arbitrage graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }
    
    /// Build the graph from liquidity pools
    pub fn build_from_pools(&mut self, pools: &[LiquidityPool]) {
        debug!("Building arbitrage graph from {} pools", pools.len());
        self.nodes.clear();
        
        // Add nodes for all tokens
        for pool in pools {
            self.add_token(pool.token_a);
            self.add_token(pool.token_b);
        }
        
        // Add edges for all pools
        for pool in pools {
            // Calculate exchange rates
            let rate_a_to_b = pool.token_b_amount as f64 / pool.token_a_amount as f64;
            let rate_b_to_a = pool.token_a_amount as f64 / pool.token_b_amount as f64;
            
            // Apply fee
            let fee_multiplier = 1.0 - (pool.fee_rate as f64 / 10000.0);
            let rate_a_to_b = rate_a_to_b * fee_multiplier;
            let rate_b_to_a = rate_b_to_a * fee_multiplier;
            
            // Add edges
            self.add_edge(pool.token_a, pool.token_b, pool.address.to_string(), pool.dex, rate_a_to_b);
            self.add_edge(pool.token_b, pool.token_a, pool.address.to_string(), pool.dex, rate_b_to_a);
        }
        
        info!("Built arbitrage graph with {} nodes", self.nodes.len());
    }
    
    /// Add a token to the graph
    fn add_token(&mut self, token: Pubkey) {
        if !self.nodes.contains_key(&token) {
            self.nodes.insert(token, Node {
                token,
                edges: Vec::new(),
            });
        }
    }
    
    /// Add an edge to the graph
    fn add_edge(&mut self, from: Pubkey, to: Pubkey, pool_id: String, dex: DEX, rate: f64) {
        if let Some(node) = self.nodes.get_mut(&from) {
            node.edges.push(Edge {
                to_token: to,
                pool_id,
                dex,
                rate,
            });
        }
    }
    
    /// Find arbitrage cycles using Bellman-Ford algorithm
    pub fn find_arbitrage_cycles(&self) -> Vec<ArbitragePath> {
        debug!("Finding arbitrage cycles using Bellman-Ford algorithm");
        let mut paths = Vec::new();
        
        // Check each token as a potential starting point
        for &start_token in self.nodes.keys() {
            trace!("Checking for negative cycles starting from token: {}", start_token);
            if let Some(path) = self.find_negative_cycle(start_token) {
                paths.push(path);
            }
        }
        
        info!("Found {} arbitrage cycles", paths.len());
        paths
    }
    
    /// Find a negative cycle using modified Bellman-Ford algorithm
    fn find_negative_cycle(&self, start_token: Pubkey) -> Option<ArbitragePath> {
        // Initialize distances and predecessors
        let mut distances: HashMap<Pubkey, f64> = HashMap::new();
        let mut predecessors: HashMap<Pubkey, (Pubkey, String, DEX)> = HashMap::new();
        
        // Set initial distance for start token
        for token in self.nodes.keys() {
            distances.insert(*token, if *token == start_token { 0.0 } else { f64::INFINITY });
        }
        
        // Relax edges |V| - 1 times
        let node_count = self.nodes.len();
        for _ in 0..node_count - 1 {
            self.relax_edges(&mut distances, &mut predecessors);
        }
        
        // Check for negative cycles
        // Since we're looking for arbitrage, we're actually looking for negative cycles
        // (we use negative log of the exchange rate)
        let mut improved = false;
        self.relax_edges_with_check(&mut distances, &mut predecessors, &mut improved);
        
        if improved {
            // Find cycle
            let mut cycle_tokens = Vec::new();
            let mut cycle_pools = Vec::new();
            let mut cycle_dexes = Vec::new();
            
            // Find a node that improved in the last relaxation
            let mut current = start_token;
            
            // Follow predecessors until we find a cycle
            let mut visited = HashSet::new();
            while !visited.contains(&current) {
                visited.insert(current);
                
                if let Some(&(pred, ref pool, dex)) = predecessors.get(&current) {
                    cycle_tokens.push(current);
                    cycle_pools.push(pool.clone());
                    cycle_dexes.push(dex);
                    current = pred;
                } else {
                    break;
                }
            }
            
            // If we found a cycle
            if !cycle_tokens.is_empty() {
                // Complete the cycle by adding the start token again
                cycle_tokens.push(current);
                
                // Build the arbitrage path
                return Some(ArbitragePath {
                    tokens: cycle_tokens,
                    pools: cycle_pools,
                    dexes: cycle_dexes,
                });
            }
        }
        
        None
    }
    
    /// Relax edges in the graph
    fn relax_edges(
        &self,
        distances: &mut HashMap<Pubkey, f64>,
        predecessors: &mut HashMap<Pubkey, (Pubkey, String, DEX)>,
    ) {
        for (from_token, node) in &self.nodes {
            if distances[from_token] != f64::INFINITY {
                for edge in &node.edges {
                    // Use negative log of rate for Bellman-Ford
                    // (we want to maximize product of rates, which is equivalent to minimizing sum of negative logs)
                    let weight = -edge.rate.ln();
                    let new_distance = distances[from_token] + weight;
                    
                    if new_distance < distances[&edge.to_token] {
                        distances.insert(edge.to_token, new_distance);
                        predecessors.insert(edge.to_token, (*from_token, edge.pool_id.clone(), edge.dex));
                    }
                }
            }
        }
    }
    
    /// Relax edges and check for improvement
    fn relax_edges_with_check(
        &self,
        distances: &mut HashMap<Pubkey, f64>,
        predecessors: &mut HashMap<Pubkey, (Pubkey, String, DEX)>,
        improved: &mut bool,
    ) {
        for (from_token, node) in &self.nodes {
            if distances[from_token] != f64::INFINITY {
                for edge in &node.edges {
                    // Use negative log of rate for Bellman-Ford
                    let weight = -edge.rate.ln();
                    let new_distance = distances[from_token] + weight;
                    
                    if new_distance < distances[&edge.to_token] {
                        *improved = true;
                        distances.insert(edge.to_token, new_distance);
                        predecessors.insert(edge.to_token, (*from_token, edge.pool_id.clone(), edge.dex));
                    }
                }
            }
        }
    }
    
    /// Get all tokens in the graph
    pub fn get_tokens(&self) -> Vec<Pubkey> {
        self.nodes.keys().cloned().collect()
    }
    
    /// Get node for a token
    pub fn get_node(&self, token: &Pubkey) -> Option<&Node> {
        self.nodes.get(token)
    }
    
    /// Get all edges from a token
    pub fn get_edges(&self, token: &Pubkey) -> Vec<&Edge> {
        if let Some(node) = self.nodes.get(token) {
            node.edges.iter().collect()
        } else {
            Vec::new()
        }
    }
}