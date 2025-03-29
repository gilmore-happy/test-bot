//! Transaction builder
//! 
//! This module provides utilities for building optimized transactions.

use solana_program::instruction::Instruction;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::{Transaction, TransactionError},
};
use std::time::Duration;
use tracing::{debug, error, info, trace, warn};

use crate::{ExecutionError, PriorityLevel};

/// Transaction builder for creating optimized transactions
pub struct TransactionBuilder {
    /// Instructions to include in the transaction
    instructions: Vec<Instruction>,
    
    /// Signers for the transaction
    signers: Vec<Keypair>,
    
    /// Payer account
    payer: Keypair,
    
    /// Recent blockhash
    blockhash: Option<Hash>,
    
    /// Priority level
    priority: PriorityLevel,
    
    /// Compute units to request
    compute_units: Option<u32>,
    
    /// Priority fee in micro-lamports
    priority_fee: Option<u64>,
    
    /// Whether to use Jito bundles
    use_jito: bool,
    
    /// Durable nonce account
    durable_nonce: Option<(Pubkey, Pubkey)>,
}

impl TransactionBuilder {
    /// Create a new transaction builder
    pub fn new(payer: Keypair) -> Self {
        Self {
            instructions: Vec::new(),
            signers: Vec::new(),
            payer,
            blockhash: None,
            priority: PriorityLevel::Medium,
            compute_units: None,
            priority_fee: None,
            use_jito: false,
            durable_nonce: None,
        }
    }
    
    /// Add an instruction to the transaction
    pub fn add_instruction(mut self, instruction: Instruction) -> Self {
        self.instructions.push(instruction);
        self
    }
    
    /// Add multiple instructions to the transaction
    pub fn add_instructions(mut self, instructions: Vec<Instruction>) -> Self {
        self.instructions.extend(instructions);
        self
    }
    
    /// Add a signer to the transaction
    pub fn add_signer(mut self, signer: Keypair) -> Self {
        self.signers.push(signer);
        self
    }
    
    /// Set the recent blockhash
    pub fn set_blockhash(mut self, blockhash: Hash) -> Self {
        self.blockhash = Some(blockhash);
        self
    }
    
    /// Set the priority level
    pub fn set_priority(mut self, priority: PriorityLevel) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set the compute units
    pub fn set_compute_units(mut self, compute_units: u32) -> Self {
        self.compute_units = Some(compute_units);
        self
    }
    
    /// Set the priority fee
    pub fn set_priority_fee(mut self, priority_fee: u64) -> Self {
        self.priority_fee = Some(priority_fee);
        self
    }
    
    /// Set whether to use Jito bundles
    pub fn set_use_jito(mut self, use_jito: bool) -> Self {
        self.use_jito = use_jito;
        self
    }
    
    /// Set the durable nonce account
    pub fn set_durable_nonce(mut self, nonce_account: Pubkey, nonce_authority: Pubkey) -> Self {
        self.durable_nonce = Some((nonce_account, nonce_authority));
        self
    }
    
    /// Build the transaction
    pub fn build(self) -> Result<Transaction, ExecutionError> {
        // Check if we have a blockhash
        let blockhash = self.blockhash.ok_or_else(|| {
            ExecutionError::Transaction("Blockhash not set".to_string())
        })?;
        
        // Add compute budget instructions if needed
        let mut instructions = Vec::new();
        
        // Add compute budget instruction if needed
        if let Some(compute_units) = self.compute_units {
            instructions.push(
                ComputeBudgetInstruction::set_compute_unit_limit(compute_units)
            );
        }
        
        // Add priority fee instruction if needed
        if let Some(priority_fee) = self.priority_fee {
            instructions.push(
                ComputeBudgetInstruction::set_compute_unit_price(priority_fee)
            );
        }
        
        // Add the user instructions
        instructions.extend(self.instructions);
        
        // Create the message
        let message = Message::new(&instructions, Some(&self.payer.pubkey()));
        
        // Create the transaction
        let mut tx = Transaction::new_unsigned(message);
        
        // Sign the transaction
        let mut signers = vec![&self.payer];
        for signer in &self.signers {
            signers.push(signer);
        }
        
        tx.sign(&signers, blockhash);
        
        Ok(tx)
    }
    
    /// Build and optimize the transaction based on priority
    pub fn build_optimized(self) -> Result<Transaction, ExecutionError> {
        // Apply optimizations based on priority
        let mut builder = self;
        
        // Set compute units based on priority if not already set
        if builder.compute_units.is_none() {
            let default_compute_units = 200_000;
            let compute_units = (default_compute_units as f64 * builder.priority.compute_unit_multiplier()) as u32;
            builder = builder.set_compute_units(compute_units);
        }
        
        // Set priority fee based on priority if not already set
        if builder.priority_fee.is_none() {
            let default_priority_fee = 1_000_000; // 1 SOL per million CU
            let priority_fee = (default_priority_fee as f64 * builder.priority.fee_multiplier()) as u64;
            builder = builder.set_priority_fee(priority_fee);
        }
        
        // Set Jito usage based on priority if not already set
        if builder.use_jito == false {
            builder = builder.set_use_jito(builder.priority.use_jito());
        }
        
        // Build the transaction
        builder.build()
    }
}

/// Create a transfer transaction
pub fn create_transfer_transaction(
    from: &Keypair,
    to: &Pubkey,
    amount: u64,
    blockhash: Hash,
    priority: PriorityLevel,
) -> Result<Transaction, ExecutionError> {
    // Create the transfer instruction
    let instruction = solana_sdk::system_instruction::transfer(
        &from.pubkey(),
        to,
        amount,
    );
    
    // Build the transaction
    TransactionBuilder::new(from.insecure_clone())
        .add_instruction(instruction)
        .set_blockhash(blockhash)
        .set_priority(priority)
        .build_optimized()
}

/// Create a memo transaction
pub fn create_memo_transaction(
    from: &Keypair,
    memo: &str,
    blockhash: Hash,
    priority: PriorityLevel,
) -> Result<Transaction, ExecutionError> {
    // Create the memo instruction
    let instruction = solana_sdk::spl_memo::build_memo(memo, &[&from.pubkey()]);
    
    // Build the transaction
    TransactionBuilder::new(from.insecure_clone())
        .add_instruction(instruction)
        .set_blockhash(blockhash)
        .set_priority(priority)
        .build_optimized()
}
