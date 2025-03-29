//! Flash loan implementations
//!
//! This module provides interfaces and implementations for flash loan providers
//! on Solana, which are essential for capital-efficient arbitrage.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use parking_lot::RwLock;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use tracing::{debug, info, warn};

use crate::config::ArbitrageConfig;

mod solend;
mod mango;

pub use solend::SolendProvider;
pub use mango::MangoProvider;

/// Flash loan provider interface
#[async_trait]
pub trait FlashLoanProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;
    
    /// Get supported tokens
    fn supported_tokens(&self) -> Vec<Pubkey>;
    
    /// Get maximum loan amount for a token
    async fn max_loan_amount(&self, token: Pubkey) -> Result<u64>;
    
    /// Get loan fee in basis points
    fn loan_fee_bps(&self) -> u16;
    
    /// Create an instruction to borrow tokens
    fn create_borrow_instruction(&self, token: Pubkey, amount: u64) -> Result<Instruction>;
    
    /// Create an instruction to repay tokens
    fn create_repay_instruction(&self, token: Pubkey, amount: u64) -> Result<Instruction>;
}

/// Flash loan manager for handling flash loans
pub struct FlashLoanManager {
    /// RPC client
    rpc_client: Arc<RpcClient>,
    
    /// Registered providers
    providers: RwLock<Vec<Arc<dyn FlashLoanProvider>>>,
    
    /// Configuration
    config: ArbitrageConfig,
}

impl FlashLoanManager {
    /// Create a new flash loan manager
    pub fn new(rpc_client: Arc<RpcClient>, config: &ArbitrageConfig) -> Self {
        Self {
            rpc_client,
            providers: RwLock::new(Vec::new()),
            config: config.clone(),
        }
    }
    
    /// Initialize the flash loan manager
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing flash loan manager");
        
        // Register Solend provider
        self.register_provider(Arc::new(SolendProvider::new(self.rpc_client.clone())));
        
        // Register Mango provider
        self.register_provider(Arc::new(MangoProvider::new(self.rpc_client.clone())));
        
        info!("Flash loan manager initialized with {} providers", self.providers.read().len());
        Ok(())
    }
    
    /// Register a flash loan provider
    pub fn register_provider(&self, provider: Arc<dyn FlashLoanProvider>) {
        info!("Registering flash loan provider: {}", provider.name());
        self.providers.write().push(provider);
    }
    
    /// Get the best provider for a token and amount
    pub async fn get_best_provider(
        &self,
        token: Pubkey,
        amount: u64,
    ) -> Result<Arc<dyn FlashLoanProvider>> {
        debug!("Finding best flash loan provider for {} of token {}", amount, token);
        
        let providers = self.providers.read();
        
        // Find providers that support this token and amount
        let mut candidates = Vec::new();
        
        for provider in providers.iter() {
            if provider.supported_tokens().contains(&token) {
                match provider.max_loan_amount(token).await {
                    Ok(max_amount) => {
                        if max_amount >= amount {
                            candidates.push((provider.clone(), provider.loan_fee_bps()));
                            debug!("Provider {} can provide {} with fee {}", 
                                  provider.name(), amount, provider.loan_fee_bps());
                        } else {
                            debug!("Provider {} max amount {} is less than requested {}", 
                                  provider.name(), max_amount, amount);
                        }
                    },
                    Err(e) => {
                        warn!("Failed to get max loan amount from {}: {}", provider.name(), e);
                    }
                }
            }
        }
        
        if candidates.is_empty() {
            return Err(anyhow!("No flash loan provider available for token {}", token));
        }
        
        // Sort by fee (lowest first)
        candidates.sort_by_key(|(_, fee)| *fee);
        
        info!("Selected provider {} with fee {}", 
             candidates[0].0.name(), candidates[0].1);
        
        Ok(candidates[0].0.clone())
    }
    
    /// Create flash loan instructions for a token
    pub async fn create_flash_loan_instructions(
        &self,
        token: Pubkey,
        amount: u64,
    ) -> Result<(Instruction, Instruction)> {
        let provider = self.get_best_provider(token, amount).await?;
        
        let borrow_instruction = provider.create_borrow_instruction(token, amount)?;
        let repay_instruction = provider.create_repay_instruction(token, amount)?;
        
        Ok((borrow_instruction, repay_instruction))
    }
}