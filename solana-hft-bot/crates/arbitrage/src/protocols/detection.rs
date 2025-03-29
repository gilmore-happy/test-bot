//! Protocol detection for arbitrage opportunities
//!
//! This module provides functions to detect and categorize instructions
//! from various Solana DeFi protocols.

use std::collections::HashSet;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use solana_program::instruction::CompiledInstruction;
use solana_program::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use tracing::{debug, trace};

use super::identifiers::*;

/// Helper Functions for Protocol Detection

pub fn is_swap_instruction(program_id: &Pubkey, instruction: &CompiledInstruction) -> bool {
    let program_id_str = program_id.to_string();
    
    // First check if it's a known swap program
    let is_swap_program = matches!(program_id_str.as_str(), 
        RAYDIUM_SWAP_V2 | RAYDIUM_LIQUIDITY_V4 | RAYDIUM_CONCENTRATED_LIQUIDITY |
        ORCA_WHIRLPOOL | ORCA_SWAP_V1 | ORCA_SWAP_V2 |
        JUPITER_V6 | JUPITER_AGGREGATOR_V4 |
        OPENBOOK_V2 | OPENBOOK_DEX_V1 |
        METEORA_POOL | PHOENIX_DEX | DRIFT_PROTOCOL | MANGO_V4
    );
    
    if !is_swap_program {
        return false;
    }
    
    // Then check the instruction data for specific swap patterns
    match program_id_str.as_str() {
        RAYDIUM_SWAP_V2 => is_raydium_swap_instruction(instruction.data.as_slice()),
        ORCA_WHIRLPOOL => is_orca_whirlpool_swap_instruction(instruction.data.as_slice()),
        JUPITER_V6 | JUPITER_AGGREGATOR_V4 => is_jupiter_swap_instruction(instruction.data.as_slice()),
        PHOENIX_DEX => is_phoenix_swap_instruction(instruction.data.as_slice()),
        // For other protocols, implement specific detection logic
        _ => is_generic_swap_instruction(instruction.data.as_slice()), // Fallback to generic pattern matching
    }
}

pub fn is_liquidation_instruction(program_id: &Pubkey, instruction: &CompiledInstruction) -> bool {
    let program_id_str = program_id.to_string();
    
    let is_lending_program = matches!(program_id_str.as_str(),
        SOLEND_PROGRAM | SOLEND_V2_PROGRAM | PORT_FINANCE |
        LARIX_LENDING | JET_PROTOCOL | APRICOT_FINANCE | TULIP_PROTOCOL |
        MANGO_V4 | DRIFT_PROTOCOL
    );
    
    if !is_lending_program {
        return false;
    }
    
    // Check protocol-specific liquidation patterns
    match program_id_str.as_str() {
        SOLEND_PROGRAM | SOLEND_V2_PROGRAM => is_solend_liquidation_instruction(instruction.data.as_slice()),
        JET_PROTOCOL => is_jet_liquidation_instruction(instruction.data.as_slice()),
        MANGO_V4 => is_mango_liquidation_instruction(instruction.data.as_slice()),
        DRIFT_PROTOCOL => is_drift_liquidation_instruction(instruction.data.as_slice()),
        // For other protocols
        _ => is_generic_liquidation_instruction(instruction.data.as_slice()),
    }
}

pub fn is_token_transfer(program_id: &Pubkey, instruction: &CompiledInstruction) -> bool {
    let program_id_str = program_id.to_string();
    
    let is_token_program = matches!(program_id_str.as_str(), 
        TOKEN_PROGRAM | TOKEN_2022_PROGRAM
    );
    
    if !is_token_program {
        return false;
    }
    
    is_token_transfer_instruction_data(instruction.data.as_slice())
}

pub fn is_nft_instruction(program_id: &Pubkey, instruction: &CompiledInstruction) -> bool {
    let program_id_str = program_id.to_string();
    
    let is_nft_program = matches!(program_id_str.as_str(), 
        MAGIC_EDEN | TENSOR_MARKETPLACE | HYPERSPACE | METAPLEX_TOKEN_METADATA
    );
    
    if !is_nft_program {
        return false;
    }
    
    match program_id_str.as_str() {
        MAGIC_EDEN => is_magic_eden_nft_instruction(instruction.data.as_slice()),
        TENSOR_MARKETPLACE => is_tensor_nft_instruction(instruction.data.as_slice()),
        METAPLEX_TOKEN_METADATA => is_metaplex_nft_instruction(instruction.data.as_slice()),
        _ => is_generic_nft_instruction(instruction.data.as_slice()),
    }
}

pub fn is_jito_bundle_instruction(program_id: &Pubkey, instruction: &CompiledInstruction) -> bool {
    let program_id_str = program_id.to_string();
    
    matches!(program_id_str.as_str(), 
        JITO_BLOCK_ENGINE | JITO_SOLANA_ACCESS
    ) && is_jito_instruction_data(instruction.data.as_slice())
}

// Detailed protocol-specific instruction analyzers

fn is_token_transfer_instruction_data(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    
    // Check for Transfer (3) or TransferChecked (12)
    match data[0] {
        3 => true, // Standard Transfer
        12 => true, // TransferChecked (includes token decimals validation)
        _ => false,
    }
}

fn is_solend_liquidation_instruction(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    
    // Solend uses 9 as the liquidation instruction discriminator
    data[0] == 9 // LiquidateObligor instruction
}

fn is_jet_liquidation_instruction(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    
    // Jet Protocol uses 8-byte discriminators
    // This is a simplified version - real implementation would check specific discriminator bytes
    // Example byte pattern for liquidation in Jet (hypothetical)
    let liquidation_pattern = [0x36, 0xA8, 0x74, 0x1D, 0x9E, 0x2F, 0x30, 0x41]; 
    data[0..8] == liquidation_pattern
}

fn is_mango_liquidation_instruction(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    
    // Mango v4 uses instruction discriminator 8 for liquidation
    data[0] == 8 // Liquidate instruction
}

fn is_drift_liquidation_instruction(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    
    // Drift uses 8-byte discriminators
    // Example byte pattern for liquidation in Drift (hypothetical)
    let drift_liquidation_pattern = [0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x6F, 0x7A, 0x8B];
    data[0..8] == drift_liquidation_pattern
}

fn is_generic_liquidation_instruction(data: &[u8]) -> bool {
    // Fallback pattern matching for unknown lending protocols
    // This is a simplified approach - real implementation would be more sophisticated
    if data.is_empty() {
        return false;
    }
    
    // Many protocols use specific discriminator values for liquidations
    // This is a generic check that might catch some but miss others
    data[0] >= 8 && data[0] <= 15 // Common range for liquidation instructions in many protocols
}

fn is_raydium_swap_instruction(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    
    // Raydium swap instruction has discriminator 1
    data[0] == 1
}

fn is_orca_whirlpool_swap_instruction(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    
    // Orca Whirlpool uses instruction discriminator 9 for swaps
    data[0] == 9
}

fn is_jupiter_swap_instruction(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    
    // Jupiter uses 8-byte discriminators
    // Simplified check - real implementation would check specific discriminator pattern
    // These are example values - exact values would need to be confirmed
    let exchange_pattern = [0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99];
    let exact_out_pattern = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22];
    
    data[0..8] == exchange_pattern || data[0..8] == exact_out_pattern
}

fn is_phoenix_swap_instruction(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    
    // Phoenix DEX uses instruction discriminator 3 for direct swaps
    data[0] == 3
}

fn is_generic_swap_instruction(data: &[u8]) -> bool {
    // Fallback pattern matching for unknown DEX protocols
    if data.is_empty() {
        return false;
    }
    
    // Many DEX protocols use low discriminator values (0-5) for swaps
    data[0] >= 0 && data[0] <= 5
}

fn is_magic_eden_nft_instruction(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    
    // Simplified check for Magic Eden transactions
    // Real implementation would check specific instruction patterns
    let buy_pattern = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    let sell_pattern = [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11];
    
    data[0..8] == buy_pattern || data[0..8] == sell_pattern
}

fn is_tensor_nft_instruction(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    
    // Simplified check for Tensor transactions
    // Real implementation would check specific instruction patterns
    let tensor_pattern = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11];
    
    data[0..8] == tensor_pattern
}

fn is_metaplex_nft_instruction(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    
    // Metaplex token metadata program uses different instruction formats
    // This is a simplified check
    data[0] >= 0 && data[0] <= 33 // Range covers most Metaplex instructions
}

fn is_generic_nft_instruction(data: &[u8]) -> bool {
    // Fallback pattern matching for unknown NFT marketplaces
    if data.len() < 8 {
        return false;
    }
    
    // Many NFT transactions involve token transfers before/after
    // This is a very generic check
    true // In practice, you would implement more sophisticated detection
}

fn is_jito_instruction_data(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    
    // Simplified check for Jito bundle transactions
    // Real implementation would check specific bundle submission patterns
    let bundle_pattern = [0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80];
    
    data[0..8] == bundle_pattern
}

/// Detect if a transaction contains arbitrage patterns
pub fn is_arbitrage_transaction(transaction: &Transaction) -> bool {
    // Check for multiple swaps across different DEXs
    let mut dex_programs = HashSet::new();
    let mut swap_count = 0;
    
    for (idx, instruction) in transaction.message.instructions.iter().enumerate() {
        let program_idx = instruction.program_id_index as usize;
        if program_idx >= transaction.message.account_keys.len() {
            continue;
        }
        
        let program_id = &transaction.message.account_keys[program_idx];
        
        if is_swap_instruction(program_id, instruction) {
            swap_count += 1;
            dex_programs.insert(program_id.to_string());
        }
    }
    
    // If we have multiple swaps across different DEXs, it's likely an arbitrage
    swap_count >= 2 && dex_programs.len() >= 2
}

/// Complete analyzer that categorizes instructions
pub fn categorize_instruction(program_id: &Pubkey, instruction: &CompiledInstruction) -> InstructionTag {
    if is_swap_instruction(program_id, instruction) {
        InstructionTag::Swap
    } else if is_liquidation_instruction(program_id, instruction) {
        InstructionTag::Liquidation
    } else if is_token_transfer(program_id, instruction) {
        InstructionTag::TokenTransfer
    } else if is_nft_instruction(program_id, instruction) {
        InstructionTag::NftTransaction
    } else if is_jito_bundle_instruction(program_id, instruction) {
        InstructionTag::MevBundle
    } else {
        // Can extend with more categories as needed
        InstructionTag::Unknown
    }
}

/// Implementation of a transaction filter for MEV opportunities
pub fn is_mev_opportunity_transaction(program_ids: &[Pubkey], instructions: &[CompiledInstruction]) -> bool {
    // Quick check if any program is MEV-relevant
    let has_relevant_program = program_ids.iter().any(|id| is_mev_relevant_program(id));
    
    if !has_relevant_program {
        return false;
    }
    
    // Categorize all instructions
    let mut has_swap = false;
    let mut has_liquidation = false;
    let mut has_token_transfer = false;
    let mut has_nft = false;
    
    for (idx, instruction) in instructions.iter().enumerate() {
        if instruction.program_id_index as usize >= program_ids.len() {
            continue;
        }
        
        let program_id = &program_ids[instruction.program_id_index as usize];
        let tag = categorize_instruction(program_id, instruction);
        
        match tag {
            InstructionTag::Swap => has_swap = true,
            InstructionTag::Liquidation => has_liquidation = true,
            InstructionTag::TokenTransfer => has_token_transfer = true,
            InstructionTag::NftTransaction => has_nft = true,
            _ => {}
        }
    }
    
    // MEV opportunity can be:
    // 1. Any liquidation (high priority)
    // 2. Multiple swaps in one tx (potential sandwich)
    // 3. Swap combined with large token transfer
    // 4. NFT transactions (for NFT MEV)
    
    has_liquidation || has_swap || (has_token_transfer && has_swap) || has_nft
}

/// Analyze a transaction for arbitrage patterns
pub fn analyze_transaction_for_arbitrage(transaction: &Transaction) -> Option<ArbitragePattern> {
    // Check for circular arbitrage (same token start and end)
    if is_circular_arbitrage(transaction) {
        return Some(ArbitragePattern::Circular);
    }
    
    // Check for triangular arbitrage (three different tokens)
    if is_triangular_arbitrage(transaction) {
        return Some(ArbitragePattern::Triangular);
    }
    
    // Check for cross-exchange arbitrage (same token pair on different exchanges)
    if is_cross_exchange_arbitrage(transaction) {
        return Some(ArbitragePattern::CrossExchange);
    }
    
    None
}

/// Arbitrage pattern types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArbitragePattern {
    /// Circular arbitrage (A -> B -> C -> A)
    Circular,
    
    /// Triangular arbitrage (A -> B -> C)
    Triangular,
    
    /// Cross-exchange arbitrage (DEX1: A -> B, DEX2: B -> A)
    CrossExchange,
}

// Detect circular arbitrage pattern
fn is_circular_arbitrage(transaction: &Transaction) -> bool {
    // This is a simplified implementation
    // A real implementation would track token accounts and transfers
    
    // Check for at least 3 swap instructions
    let mut swap_count = 0;
    let mut dex_programs = HashSet::new();
    
    for (idx, instruction) in transaction.message.instructions.iter().enumerate() {
        let program_idx = instruction.program_id_index as usize;
        if program_idx >= transaction.message.account_keys.len() {
            continue;
        }
        
        let program_id = &transaction.message.account_keys[program_idx];
        
        if is_swap_instruction(program_id, instruction) {
            swap_count += 1;
            dex_programs.insert(program_id.to_string());
        }
    }
    
    // Circular arbitrage typically has 3+ swaps
    swap_count >= 3
}

// Detect triangular arbitrage pattern
fn is_triangular_arbitrage(transaction: &Transaction) -> bool {
    // This is a simplified implementation
    // A real implementation would track token accounts and transfers
    
    // Check for exactly 3 swap instructions
    let mut swap_count = 0;
    
    for (idx, instruction) in transaction.message.instructions.iter().enumerate() {
        let program_idx = instruction.program_id_index as usize;
        if program_idx >= transaction.message.account_keys.len() {
            continue;
        }
        
        let program_id = &transaction.message.account_keys[program_idx];
        
        if is_swap_instruction(program_id, instruction) {
            swap_count += 1;
        }
    }
    
    // Triangular arbitrage typically has exactly 3 swaps
    swap_count == 3
}

// Detect cross-exchange arbitrage pattern
fn is_cross_exchange_arbitrage(transaction: &Transaction) -> bool {
    // This is a simplified implementation
    // A real implementation would track token accounts and transfers
    
    // Check for 2 swap instructions on different DEXs
    let mut dex_programs = HashSet::new();
    
    for (idx, instruction) in transaction.message.instructions.iter().enumerate() {
        let program_idx = instruction.program_id_index as usize;
        if program_idx >= transaction.message.account_keys.len() {
            continue;
        }
        
        let program_id = &transaction.message.account_keys[program_idx];
        
        if is_swap_instruction(program_id, instruction) {
            dex_programs.insert(program_id.to_string());
        }
    }
    
    // Cross-exchange arbitrage uses different DEXs
    dex_programs.len() >= 2
}