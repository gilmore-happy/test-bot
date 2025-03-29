//! Parallelized implementations for critical paths
//!
//! This module provides efficiently parallelized implementations of performance-critical
//! operations in the execution engine, including transaction validation,
//! signature verification, and batch processing.

use std::sync::Arc;
use std::time::Instant;
use rayon::prelude::*;
use solana_sdk::{
    hash::Hash,
    pubkey::Pubkey,
    signature::{Signature, Signer},
    transaction::Transaction,
};
use tracing::{debug, error, info, trace, warn};

use crate::ExecutionError;

// CPU feature detection and SIMD implementation types removed
// We now use a single efficient parallel implementation

/// Parallelized batch signature verification
pub struct BatchSignatureVerifier {
    /// Batch size for verification
    batch_size: usize,
}

impl BatchSignatureVerifier {
    /// Create a new batch signature verifier
    pub fn new() -> Self {
        // Use a reasonable batch size for parallel processing
        // This value can be tuned based on benchmarks
        let batch_size = 8;
        
        Self {
            batch_size,
        }
    }
    
    /// Verify a batch of signatures
    pub fn verify_batch(
        &self,
        transactions: &[Transaction],
    ) -> Vec<Result<(), ExecutionError>> {
        // Use the parallel implementation regardless of CPU features
        // This provides good performance across all architectures without
        // the complexity of maintaining SIMD-specific code
        self.verify_batch_parallel(transactions)
    }
    
    /// Verify a batch of signatures using the parallelized implementation
    fn verify_batch_parallel(
        &self,
        transactions: &[Transaction],
    ) -> Vec<Result<(), ExecutionError>> {
        // Process in batches of batch_size
        let mut results = Vec::with_capacity(transactions.len());
        
        for chunk in transactions.chunks(self.batch_size) {
            // Extract signatures and messages
            let signatures: Vec<_> = chunk.iter().map(|tx| tx.signatures[0]).collect();
            let messages: Vec<_> = chunk.iter().map(|tx| tx.message.hash()).collect();
            let pubkeys: Vec<_> = chunk.iter().map(|tx| tx.message.account_keys[0]).collect();
            
            // Verify signatures in parallel
            let chunk_results = self.verify_signatures(&signatures, &messages, &pubkeys);
            results.extend(chunk_results);
        }
        
        results
    }
    
    /// Verify signatures using parallelized implementation
    fn verify_signatures(
        &self,
        signatures: &[Signature],
        messages: &[Hash],
        pubkeys: &[Pubkey],
    ) -> Vec<Result<(), ExecutionError>> {
        // Use Rayon's parallel iterator for efficient parallelization
        use rayon::prelude::*;
        
        signatures.par_iter()
            .zip(messages.par_iter())
            .zip(pubkeys.par_iter())
            .map(|((signature, message), pubkey)| {
                if signature.verify(pubkey.as_ref(), message.as_ref()).is_ok() {
                    Ok(())
                } else {
                    Err(ExecutionError::Transaction("Signature verification failed".to_string()))
                }
            })
            .collect()
    }
    
    // AVX2 batch verification removed - using verify_batch_parallel instead
    
    // AVX2 implementation removed - using the common verify_signatures method instead
    
    // Scalar implementation removed - using verify_batch_parallel instead
}

impl Default for BatchSignatureVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Parallelized transaction validation
pub struct TransactionValidator {
    /// Signature verifier
    signature_verifier: BatchSignatureVerifier,
}

impl TransactionValidator {
    /// Create a new transaction validator
    pub fn new() -> Self {
        Self {
            signature_verifier: BatchSignatureVerifier::new(),
        }
    }
    
    /// Validate a batch of transactions
    pub fn validate_batch(
        &self,
        transactions: &[Transaction],
        recent_blockhash: Hash,
    ) -> Vec<Result<(), ExecutionError>> {
        // First check if transactions use the correct blockhash
        let blockhash_results = self.validate_blockhashes(transactions, recent_blockhash);
        
        // Then verify signatures for transactions with valid blockhashes
        let mut results = Vec::with_capacity(transactions.len());
        let mut valid_transactions = Vec::new();
        let mut valid_indices = Vec::new();
        
        for (i, result) in blockhash_results.into_iter().enumerate() {
            if result.is_ok() {
                valid_transactions.push(&transactions[i]);
                valid_indices.push(i);
            } else {
                results.push(result);
            }
        }
        
        // Verify signatures for valid transactions
        let signature_results = self.signature_verifier.verify_batch(&valid_transactions);
        
        // Merge results
        let mut signature_idx = 0;
        for i in 0..transactions.len() {
            if valid_indices.contains(&i) {
                results.push(signature_results[signature_idx].clone());
                signature_idx += 1;
            }
        }
        
        results
    }
    
    /// Validate blockhashes for a batch of transactions
    fn validate_blockhashes(
        &self,
        transactions: &[Transaction],
        recent_blockhash: Hash,
    ) -> Vec<Result<(), ExecutionError>> {
        // Use Rayon's parallel iterator for efficient parallelization
        use rayon::prelude::*;
        
        transactions.par_iter().map(|tx| {
            if tx.message.recent_blockhash == recent_blockhash {
                Ok(())
            } else {
                Err(ExecutionError::Transaction("Blockhash mismatch".to_string()))
            }
        }).collect()
    }
    
    // Scalar implementation removed - using the direct implementation in validate_blockhashes
}

impl Default for TransactionValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Fast path for common transaction patterns
pub struct TransactionFastPath {
    // No need for CPU features or SIMD implementation type
    // since we're using a common implementation for all architectures
}

impl TransactionFastPath {
    /// Create a new transaction fast path
    pub fn new() -> Self {
        Self {}
    }
    
    /// Check if a transaction can use a fast path
    pub fn can_use_fast_path(&self, transaction: &Transaction) -> bool {
        // Check if this is a simple transfer transaction
        self.is_simple_transfer(transaction)
    }
    
    /// Check if a transaction is a simple transfer
    fn is_simple_transfer(&self, transaction: &Transaction) -> bool {
        // A simple transfer has:
        // 1. One signature
        // 2. Two accounts (sender and receiver)
        // 3. One system program instruction (transfer)
        
        if transaction.signatures.len() != 1 {
            return false;
        }
        
        let message = &transaction.message;
        
        // Check if this is a system program transfer
        if message.instructions.len() != 1 {
            return false;
        }
        
        let instruction = &message.instructions[0];
        
        // Check if the program is the system program
        let program_id_index = instruction.program_id_index as usize;
        if program_id_index >= message.account_keys.len() {
            return false;
        }
        
        let program_id = message.account_keys[program_id_index];
        if program_id != solana_sdk::system_program::id() {
            return false;
        }
        
        // Check if the instruction is a transfer
        if instruction.data.len() < 4 {
            return false;
        }
        
        // The first 4 bytes of the data should be the transfer instruction (0)
        let instruction_index = u32::from_le_bytes([
            instruction.data[0],
            instruction.data[1],
            instruction.data[2],
            instruction.data[3],
        ]);
        
        instruction_index == 2 // 2 is the index of the Transfer instruction
    }
    
    /// Process a batch of transactions using fast paths where possible
    pub fn process_batch(
        &self,
        transactions: &[Transaction],
    ) -> Vec<bool> {
        transactions.par_iter().map(|tx| self.can_use_fast_path(tx)).collect()
    }
}

impl Default for TransactionFastPath {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch transaction processor
pub struct BatchProcessor {
    /// Signature verifier
    signature_verifier: BatchSignatureVerifier,
    
    /// Transaction validator
    validator: TransactionValidator,
    
    /// Fast path detector
    fast_path: TransactionFastPath,
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new() -> Self {
        Self {
            signature_verifier: BatchSignatureVerifier::new(),
            validator: TransactionValidator::new(),
            fast_path: TransactionFastPath::new(),
        }
    }
    
    /// Process a batch of transactions
    pub fn process_batch(
        &self,
        transactions: &[Transaction],
        recent_blockhash: Hash,
    ) -> Vec<Result<(), ExecutionError>> {
        // First, identify transactions that can use fast paths
        let fast_paths = self.fast_path.process_batch(transactions);
        
        // Split transactions into fast path and normal path
        let mut fast_path_txs = Vec::new();
        let mut normal_path_txs = Vec::new();
        let mut fast_path_indices = Vec::new();
        let mut normal_path_indices = Vec::new();
        
        for (i, (tx, can_use_fast_path)) in transactions.iter().zip(fast_paths.iter()).enumerate() {
            if *can_use_fast_path {
                fast_path_txs.push(tx);
                fast_path_indices.push(i);
            } else {
                normal_path_txs.push(tx);
                normal_path_indices.push(i);
            }
        }
        
        // Process fast path transactions
        let fast_path_results = self.process_transaction_batch(&fast_path_txs, recent_blockhash, true);
        
        // Process normal path transactions
        let normal_path_results = self.process_transaction_batch(&normal_path_txs, recent_blockhash, false);
        
        // Merge results
        let mut results = vec![Err(ExecutionError::Internal("Unprocessed transaction".to_string())); transactions.len()];
        
        for (i, result) in fast_path_indices.iter().zip(fast_path_results.iter()) {
            results[*i] = result.clone();
        }
        
        for (i, result) in normal_path_indices.iter().zip(normal_path_results.iter()) {
            results[*i] = result.clone();
        }
        
        results
    }
    
    /// Process a batch of transactions (for both fast and normal paths)
    fn process_transaction_batch(
        &self,
        transactions: &[&Transaction],
        recent_blockhash: Hash,
        use_fast_path: bool,  // Flag to indicate path type for future differentiation
    ) -> Vec<Result<(), ExecutionError>> {
        // Convert references to owned Transaction objects
        let transactions_vec: Vec<Transaction> = transactions.iter().map(|&tx| tx.clone()).collect();
        
        // Currently both paths use the same validation
        // If different validation is implemented in the future, this can be
        // expanded with a conditional based on use_fast_path
        self.validator.validate_batch(&transactions_vec, recent_blockhash)
    }
}

impl Default for BatchProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{
        signature::Keypair,
        system_instruction,
    };
    
    // CPU features detection test removed
    
    #[test]
    fn test_fast_path_detection() {
        let fast_path = TransactionFastPath::new();
        
        // Create a simple transfer transaction
        let sender = Keypair::new();
        let receiver = Pubkey::new_unique();
        
        let transfer_instruction = system_instruction::transfer(
            &sender.pubkey(),
            &receiver,
            100,
        );
        
        let message = solana_sdk::message::Message::new(
            &[transfer_instruction],
            Some(&sender.pubkey()),
        );
        
        let blockhash = Hash::new_unique();
        let mut transaction = Transaction::new_unsigned(message);
        transaction.sign(&[&sender], blockhash);
        
        // This should be detected as a fast path transaction
        assert!(fast_path.can_use_fast_path(&transaction));
        
        // Create a more complex transaction
        let complex_instruction1 = system_instruction::transfer(
            &sender.pubkey(),
            &receiver,
            100,
        );
        
        let complex_instruction2 = system_instruction::create_account(
            &sender.pubkey(),
            &receiver,
            100,
            100,
            &solana_sdk::system_program::id(),
        );
        
        let message = solana_sdk::message::Message::new(
            &[complex_instruction1, complex_instruction2],
            Some(&sender.pubkey()),
        );
        
        let mut transaction = Transaction::new_unsigned(message);
        transaction.sign(&[&sender], blockhash);
        
        // This should not be detected as a fast path transaction
        assert!(!fast_path.can_use_fast_path(&transaction));
    }
}