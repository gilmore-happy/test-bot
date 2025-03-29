use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use chrono::{DateTime, Utc};

/// DEX type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DEX {
    /// Raydium DEX
    Raydium,
    
    /// Orca DEX
    Orca,
    
    /// Serum DEX
    Serum,
    
    /// Lifinity DEX
    Lifinity,
    
    /// Meteora DEX
    Meteora,
    
    /// Jupiter Aggregator
    Jupiter,
    
    /// Other DEX
    Other,
}

impl DEX {
    /// Get the name of the DEX
    pub fn name(&self) -> &'static str {
        match self {
            DEX::Raydium => "Raydium",
            DEX::Orca => "Orca",
            DEX::Serum => "Serum",
            DEX::Lifinity => "Lifinity",
            DEX::Meteora => "Meteora",
            DEX::Jupiter => "Jupiter",
            DEX::Other => "Other",
        }
    }
    
    /// Get the program ID for the DEX
    pub fn program_id(&self) -> Option<Pubkey> {
        match self {
            DEX::Raydium => Some(crate::programs::RAYDIUM_AMM_PROGRAM_ID),
            DEX::Orca => Some(crate::programs::ORCA_SWAP_PROGRAM_ID),
            DEX::Serum => None, // Add Serum program ID when available
            DEX::Lifinity => None, // Add Lifinity program ID when available
            DEX::Meteora => None, // Add Meteora program ID when available
            DEX::Jupiter => Some(crate::programs::JUPITER_PROGRAM_ID),
            DEX::Other => None,
        }
    }
    
    /// Get the fee rate in basis points (e.g., 30 = 0.3%)
    pub fn default_fee_rate(&self) -> u16 {
        match self {
            DEX::Raydium => 30,
            DEX::Orca => 30,
            DEX::Serum => 20,
            DEX::Lifinity => 25,
            DEX::Meteora => 25,
            DEX::Jupiter => 0, // Jupiter is an aggregator, not a DEX
            DEX::Other => 30,
        }
    }
}

/// Liquidity pool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPool {
    /// Pool address
    pub address: Pubkey,
    
    /// DEX type
    pub dex: DEX,
    
    /// Token A mint
    pub token_a: Pubkey,
    
    /// Token B mint
    pub token_b: Pubkey,
    
    /// Token A amount
    pub token_a_amount: u64,
    
    /// Token B amount
    pub token_b_amount: u64,
    
    /// Fee rate in basis points (e.g., 30 = 0.3%)
    pub fee_rate: u16,
    
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

impl LiquidityPool {
    /// Create a new liquidity pool
    pub fn new(
        address: Pubkey,
        dex: DEX,
        token_a: Pubkey,
        token_b: Pubkey,
        token_a_amount: u64,
        token_b_amount: u64,
    ) -> Self {
        Self {
            address,
            dex,
            token_a,
            token_b,
            token_a_amount,
            token_b_amount,
            fee_rate: dex.default_fee_rate(),
            last_updated: Utc::now(),
        }
    }

    /// Calculate the liquidity in USD
    pub fn calculate_liquidity_usd(&self, token_prices: &HashMap<Pubkey, f64>) -> Option<f64> {
        let token_a_price = token_prices.get(&self.token_a)?;
        let token_b_price = token_prices.get(&self.token_b)?;
        
        let token_a_value = *token_a_price * (self.token_a_amount as f64);
        let token_b_value = *token_b_price * (self.token_b_amount as f64);
        
        Some(token_a_value + token_b_value)
    }
    
    /// Calculate the price of token A in terms of token B
    pub fn calculate_price_a_in_b(&self) -> Option<f64> {
        if self.token_b_amount == 0 {
            return None;
        }
        
        Some((self.token_a_amount as f64) / (self.token_b_amount as f64))
    }
    
    /// Calculate the price of token B in terms of token A
    pub fn calculate_price_b_in_a(&self) -> Option<f64> {
        if self.token_a_amount == 0 {
            return None;
        }
        
        Some((self.token_b_amount as f64) / (self.token_a_amount as f64))
    }
    
    /// Calculate the price impact for a swap of the given amount
    pub fn calculate_price_impact(&self, amount_in: u64, is_a_to_b: bool) -> Option<f64> {
        if self.token_a_amount == 0 || self.token_b_amount == 0 {
            return None;
        }
        
        let (reserve_in, reserve_out) = if is_a_to_b {
            (self.token_a_amount, self.token_b_amount)
        } else {
            (self.token_b_amount, self.token_a_amount)
        };
        
        // Constant product formula: x * y = k
        let k = (reserve_in as f64) * (reserve_out as f64);
        
        // New reserve_in after swap
        let new_reserve_in = reserve_in + amount_in;
        
        // New reserve_out after swap
        let new_reserve_out = k / (new_reserve_in as f64);
        
        // Amount out
        let amount_out = reserve_out as f64 - new_reserve_out;
        
        // Price before swap
        let price_before = (reserve_out as f64) / (reserve_in as f64);
        
        // Price after swap
        let price_after = amount_out / (amount_in as f64);
        
        // Price impact
        let price_impact = (price_before - price_after) / price_before;
        
        Some(price_impact)
    }
    
    /// Update pool reserves
    pub fn update_reserves(&mut self, token_a_amount: u64, token_b_amount: u64) {
        self.token_a_amount = token_a_amount;
        self.token_b_amount = token_b_amount;
        self.last_updated = Utc::now();
    }
    
    /// Check if the pool contains a specific token
    pub fn contains_token(&self, token_mint: &Pubkey) -> bool {
        self.token_a == *token_mint || self.token_b == *token_mint
    }
    
    /// Get the other token in the pair
    pub fn get_other_token(&self, token_mint: &Pubkey) -> Option<Pubkey> {
        if self.token_a == *token_mint {
            Some(self.token_b)
        } else if self.token_b == *token_mint {
            Some(self.token_a)
        } else {
            None
        }
    }
}

/// Pool manager for tracking liquidity pools
pub struct PoolManager {
    /// Pools by address
    pools: HashMap<Pubkey, LiquidityPool>,
    
    /// Pools by token mint
    pools_by_token: HashMap<Pubkey, Vec<Pubkey>>,
}

impl PoolManager {
    /// Create a new pool manager
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            pools_by_token: HashMap::new(),
        }
    }
    
    /// Add a pool to the manager
    pub fn add_pool(&mut self, pool: LiquidityPool) {
        // Add to pools by token
        self.add_to_token_index(&pool.token_a, pool.address);
        self.add_to_token_index(&pool.token_b, pool.address);
        
        // Add to pools by address
        self.pools.insert(pool.address, pool);
    }
    
    /// Add a pool address to the token index
    fn add_to_token_index(&mut self, token_mint: &Pubkey, pool_address: Pubkey) {
        self.pools_by_token
            .entry(*token_mint)
            .or_insert_with(Vec::new)
            .push(pool_address);
    }
    
    /// Get a pool by address
    pub fn get_pool(&self, address: &Pubkey) -> Option<&LiquidityPool> {
        self.pools.get(address)
    }
    
    /// Get a mutable reference to a pool by address
    pub fn get_pool_mut(&mut self, address: &Pubkey) -> Option<&mut LiquidityPool> {
        self.pools.get_mut(address)
    }
    
    /// Get all pools containing a specific token
    pub fn get_pools_for_token(&self, token_mint: &Pubkey) -> Vec<&LiquidityPool> {
        match self.pools_by_token.get(token_mint) {
            Some(pool_addresses) => {
                pool_addresses
                    .iter()
                    .filter_map(|addr| self.pools.get(addr))
                    .collect()
            }
            None => Vec::new(),
        }
    }
    
    /// Get all pools
    pub fn get_all_pools(&self) -> Vec<&LiquidityPool> {
        self.pools.values().collect()
    }
    
    /// Get the number of pools
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
    
    /// Remove a pool
    pub fn remove_pool(&mut self, address: &Pubkey) -> Option<LiquidityPool> {
        if let Some(pool) = self.pools.remove(address) {
            // Remove from token indices
            self.remove_from_token_index(&pool.token_a, *address);
            self.remove_from_token_index(&pool.token_b, *address);
            Some(pool)
        } else {
            None
        }
    }
    
    /// Remove a pool address from the token index
    fn remove_from_token_index(&mut self, token_mint: &Pubkey, pool_address: Pubkey) {
        if let Some(pools) = self.pools_by_token.get_mut(token_mint) {
            pools.retain(|addr| *addr != pool_address);
            if pools.is_empty() {
                self.pools_by_token.remove(token_mint);
            }
        }
    }
}