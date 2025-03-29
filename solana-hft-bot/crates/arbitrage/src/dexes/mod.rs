//! DEX interfaces for Solana DEXes
//!
//! This module provides interfaces and implementations for interacting with
//! various Solana DEXes for arbitrage purposes.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use tracing::{debug, error, info, warn};

use crate::config::ArbitrageConfig;
use crate::pools::LiquidityPool;

mod raydium;
mod orca;
mod jupiter;
mod openbook;

pub use raydium::RaydiumClient;
pub use orca::OrcaClient;
pub use jupiter::JupiterClient;
pub use openbook::OpenbookClient;

/// DEX type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DEX {
    /// Raydium DEX
    Raydium,
    
    /// Orca DEX
    Orca,
    
    /// OpenBook DEX (formerly Serum)
    Openbook,
    
    /// Jupiter Aggregator
    Jupiter,
    
    /// Other DEX
    Other(u8),
}

/// DEX client trait for interacting with DEX protocols
#[async_trait]
pub trait DEXClient: Send + Sync {
    /// Get the DEX type
    fn dex_type(&self) -> DEX;
    
    /// Initialize the client
    async fn initialize(&self) -> Result<()>;
    
    /// Create a swap instruction
    fn create_swap_instruction(
        &self,
        pool_address: Pubkey,
        token_in: Pubkey,
        token_out: Pubkey,
        amount_in: u64,
    ) -> Result<Instruction>;
    
    /// Get all pools from the DEX
    async fn get_all_pools(&self, rpc_client: Arc<RpcClient>) -> Result<Vec<LiquidityPool>>;
    
    /// Calculate output amount for a swap
    fn calculate_output_amount(
        &self,
        pool: &LiquidityPool,
        token_in: Pubkey,
        amount_in: u64,
    ) -> Result<u64>;
}

/// Registry of DEX clients
pub struct DexRegistry {
    /// Registered DEX clients
    clients: RwLock<HashMap<DEX, Arc<dyn DEXClient>>>,
    
    /// Configuration
    config: ArbitrageConfig,
}

impl DexRegistry {
    /// Create a new DEX registry
    pub fn new(config: &ArbitrageConfig) -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            config: config.clone(),
        }
    }
    
    /// Initialize all registered DEX clients
    pub async fn initialize(&self) -> Result<()> {
        // Register Raydium client
        self.register_client(Arc::new(RaydiumClient::new()));
        
        // Register Orca client
        self.register_client(Arc::new(OrcaClient::new()));
        
        // Register OpenBook client
        self.register_client(Arc::new(OpenbookClient::new()));
        
        // Register Jupiter client
        self.register_client(Arc::new(JupiterClient::new()));
        
        // Initialize all clients
        for client in self.clients.read().values() {
            client.initialize().await?;
        }
        
        Ok(())
    }
    
    /// Register a DEX client
    pub fn register_client(&self, client: Arc<dyn DEXClient>) {
        let dex_type = client.dex_type();
        self.clients.write().insert(dex_type, client);
    }
    
    /// Get a DEX client by type
    pub fn get_client(&self, dex_type: &DEX) -> Option<Arc<dyn DEXClient>> {
        self.clients.read().get(dex_type).cloned()
    }
    
    /// Get all registered DEX clients
    pub fn get_all_clients(&self) -> Vec<Arc<dyn DEXClient>> {
        self.clients.read().values().cloned().collect()
    }
}