//! Transaction Vault for Solana HFT Bot
//!
//! This module provides a high-performance transaction vault that pre-signs and caches
//! transactions for ultra-low-latency execution. The vault maintains a pool of pre-signed
//! transaction templates that can be quickly modified and submitted when needed.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, trace, warn};

use crate::ExecutionError;

/// Configuration for the transaction vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Whether to prewarm the vault on initialization
    pub prewarm_vault: bool,
    
    /// Number of pre-signed transactions to keep in the vault
    pub vault_size: usize,
    
    /// Refresh interval in milliseconds
    pub refresh_interval_ms: u64,
    
    /// Maximum transaction size in bytes
    pub max_tx_size: usize,
    
    /// Whether to use durable nonces for transactions
    pub use_durable_nonce: bool,
    
    /// Number of concurrent refreshes allowed
    pub max_concurrent_refreshes: usize,
    
    /// Whether to use zero-copy transaction construction
    pub use_zero_copy: bool,
    
    /// Whether to verify signatures after pre-signing
    pub verify_signatures: bool,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            prewarm_vault: true,
            vault_size: 100,
            refresh_interval_ms: 30_000, // 30 seconds
            max_tx_size: 1232,           // Default Solana transaction size limit
            use_durable_nonce: false,
            max_concurrent_refreshes: 4,
            use_zero_copy: true,
            verify_signatures: true,
        }
    }
}

/// Transaction template for pre-signed transactions
#[derive(Debug, Clone)]
pub struct TransactionTemplate {
    /// Transaction ID
    pub id: u64,
    
    /// Pre-signed transaction
    pub transaction: Option<Transaction>,
    
    /// Transaction instructions (for templates without a transaction)
    pub instructions: Vec<Instruction>,
    
    /// Recent blockhash
    pub blockhash: Option<Hash>,
    
    /// Creation timestamp
    pub created_at: Instant,
    
    /// Last used timestamp
    pub last_used: Option<Instant>,
    
    /// Number of times this template has been used
    pub use_count: u64,
}

impl Default for TransactionTemplate {
    fn default() -> Self {
        Self {
            id: 0,
            transaction: None,
            instructions: Vec::new(),
            blockhash: None,
            created_at: Instant::now(),
            last_used: None,
            use_count: 0,
        }
    }
}

/// Transaction vault for pre-signed transactions
pub struct TransactionVault {
    /// Keypair used for signing
    keypair: Arc<Keypair>,
    
    /// Vault configuration
    config: VaultConfig,
    
    /// Pre-signed transaction templates
    templates: RwLock<VecDeque<TransactionTemplate>>,
    
    /// Template ID counter
    next_id: Mutex<u64>,
    
    /// Last refresh time
    last_refresh: RwLock<Instant>,
    
    /// Refresh semaphore to limit concurrent refreshes
    refresh_semaphore: Arc<Semaphore>,
    
    /// Template usage statistics
    template_stats: DashMap<u64, u64>,
}

impl TransactionVault {
    /// Create a new transaction vault
    pub fn new(keypair: Arc<Keypair>, config: VaultConfig) -> Self {
        Self {
            keypair,
            config: config.clone(),
            templates: RwLock::new(VecDeque::with_capacity(config.vault_size)),
            next_id: Mutex::new(1),
            last_refresh: RwLock::new(Instant::now()),
            refresh_semaphore: Arc::new(Semaphore::new(config.max_concurrent_refreshes)),
            template_stats: DashMap::new(),
        }
    }
    
    /// Get the keypair
    pub fn keypair(&self) -> Arc<Keypair> {
        self.keypair.clone()
    }
    
    /// Get the next template ID
    fn next_id(&self) -> u64 {
        let mut id = self.next_id.lock();
        let current = *id;
        *id += 1;
        current
    }
    
    /// Prewarm the vault with transaction templates
    pub async fn prewarm(&self) -> Result<()> {
        info!("Prewarming transaction vault with {} templates", self.config.vault_size);
        
        // Acquire semaphore permit
        let _permit = self.refresh_semaphore.acquire().await?;
        
        let mut templates = self.templates.write();
        templates.clear();
        
        // Create empty templates
        for _ in 0..self.config.vault_size {
            templates.push_back(TransactionTemplate {
                id: self.next_id(),
                ..Default::default()
            });
        }
        
        *self.last_refresh.write() = Instant::now();
        
        info!("Transaction vault prewarmed with {} templates", templates.len());
        Ok(())
    }
    
    /// Refresh the transaction vault
    pub async fn refresh(&self) -> Result<()> {
        // Check if refresh is needed
        let last_refresh = *self.last_refresh.read();
        if last_refresh.elapsed().as_millis() < self.config.refresh_interval_ms as u128 {
            trace!("Skipping vault refresh, last refresh was {} ms ago", last_refresh.elapsed().as_millis());
            return Ok(());
        }
        
        debug!("Refreshing transaction vault");
        
        // Try to acquire semaphore permit
        match self.refresh_semaphore.try_acquire() {
            Ok(permit) => {
                // Refresh the vault
                let mut templates = self.templates.write();
                
                // Remove templates that haven't been used in a while
                let now = Instant::now();
                templates.retain(|t| {
                    if let Some(last_used) = t.last_used {
                        // Keep templates that have been used recently
                        now.duration_since(last_used).as_secs() < 300 // 5 minutes
                    } else {
                        // Keep templates that haven't been used yet
                        now.duration_since(t.created_at).as_secs() < 60 // 1 minute
                    }
                });
                
                // Add new templates to reach the desired size
                while templates.len() < self.config.vault_size {
                    templates.push_back(TransactionTemplate {
                        id: self.next_id(),
                        ..Default::default()
                    });
                }
                
                *self.last_refresh.write() = now;
                
                // Drop the permit
                drop(permit);
                
                debug!("Transaction vault refreshed, now has {} templates", templates.len());
                Ok(())
            }
            Err(_) => {
                // Another refresh is already in progress
                trace!("Skipping vault refresh, another refresh is in progress");
                Ok(())
            }
        }
    }
    
    /// Get a transaction template from the vault
    pub fn get_template(&self) -> Option<TransactionTemplate> {
        let mut templates = self.templates.write();
        
        // Try to find a template
        if let Some(template) = templates.pop_front() {
            // Record usage
            self.template_stats.entry(template.id).and_modify(|count| *count += 1).or_insert(1);
            
            // Schedule a refresh if the vault is getting low
            if templates.len() < self.config.vault_size / 4 {
                let vault = self.clone();
                tokio::spawn(async move {
                    if let Err(e) = vault.refresh().await {
                        error!("Failed to refresh transaction vault: {}", e);
                    }
                });
            }
            
            Some(template)
        } else {
            warn!("Transaction vault is empty");
            None
        }
    }
    
    /// Add a transaction template to the vault
    pub fn add_template(&self, mut template: TransactionTemplate) {
        // Update template ID if not set
        if template.id == 0 {
            template.id = self.next_id();
        }
        
        // Add template to vault
        let mut templates = self.templates.write();
        templates.push_back(template);
    }
    
    /// Create a pre-signed transaction
    pub fn create_presigned_transaction(
        &self,
        instructions: Vec<Instruction>,
        blockhash: Hash,
    ) -> Result<Transaction, ExecutionError> {
        // Create transaction
        let mut transaction = Transaction::new_with_payer(
            &instructions,
            Some(&self.keypair.pubkey()),
        );
        
        // Sign transaction
        transaction.sign(&[&*self.keypair], blockhash);
        
        // Verify signature if configured
        if self.config.verify_signatures {
            transaction.verify()?;
        }
        
        Ok(transaction)
    }
    
    /// Get the current vault size
    pub fn size(&self) -> usize {
        self.templates.read().len()
    }
    
    /// Get vault statistics
    pub fn stats(&self) -> VaultStats {
        let templates = self.templates.read();
        
        VaultStats {
            total_templates: templates.len(),
            ready_templates: templates.iter().filter(|t| t.transaction.is_some()).count(),
            empty_templates: templates.iter().filter(|t| t.transaction.is_none()).count(),
            last_refresh: *self.last_refresh.read(),
            most_used_template: self.template_stats.iter().max_by_key(|entry| *entry.value()).map(|entry| (*entry.key(), *entry.value())),
        }
    }
}

impl Clone for TransactionVault {
    fn clone(&self) -> Self {
        Self {
            keypair: self.keypair.clone(),
            config: self.config.clone(),
            templates: RwLock::new(self.templates.read().clone()),
            next_id: Mutex::new(*self.next_id.lock()),
            last_refresh: RwLock::new(*self.last_refresh.read()),
            refresh_semaphore: self.refresh_semaphore.clone(),
            template_stats: self.template_stats.clone(),
        }
    }
}

/// Vault statistics
#[derive(Debug)]
pub struct VaultStats {
    /// Total number of templates in the vault
    pub total_templates: usize,
    
    /// Number of ready templates (with pre-signed transactions)
    pub ready_templates: usize,
    
    /// Number of empty templates (without pre-signed transactions)
    pub empty_templates: usize,
    
    /// Last refresh time
    pub last_refresh: Instant,
    
    /// Most used template (ID, count)
    pub most_used_template: Option<(u64, u64)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::system_instruction;
    
    #[tokio::test]
    async fn test_vault_prewarm() {
        let keypair = Arc::new(Keypair::new());
        let config = VaultConfig {
            vault_size: 10,
            ..Default::default()
        };
        
        let vault = TransactionVault::new(keypair, config);
        vault.prewarm().await.unwrap();
        
        assert_eq!(vault.size(), 10);
    }
    
    #[tokio::test]
    async fn test_get_template() {
        let keypair = Arc::new(Keypair::new());
        let config = VaultConfig {
            vault_size: 5,
            ..Default::default()
        };
        
        let vault = TransactionVault::new(keypair, config);
        vault.prewarm().await.unwrap();
        
        // Get a template
        let template = vault.get_template().unwrap();
        assert_eq!(vault.size(), 4);
        
        // Add it back
        vault.add_template(template);
        assert_eq!(vault.size(), 5);
    }
    
    #[tokio::test]
    async fn test_create_presigned_transaction() {
        let keypair = Arc::new(Keypair::new());
        let config = VaultConfig::default();
        
        let vault = TransactionVault::new(keypair.clone(), config);
        
        // Create a simple transfer instruction
        let instruction = system_instruction::transfer(
            &keypair.pubkey(),
            &Pubkey::new_unique(),
            1000,
        );
        
        let blockhash = Hash::new_unique();
        
        // Create pre-signed transaction
        let transaction = vault.create_presigned_transaction(vec![instruction], blockhash).unwrap();
        
        // Verify the transaction
        assert_eq!(transaction.signatures.len(), 1);
        assert!(transaction.verify_with_results().iter().all(|&result| result));
    }
}