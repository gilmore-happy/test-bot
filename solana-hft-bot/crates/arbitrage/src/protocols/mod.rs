//! Protocol identifiers and detection for arbitrage
//!
//! This module provides protocol identification, instruction parsing,
//! and opportunity detection across Solana DeFi protocols.

use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use solana_program::instruction::CompiledInstruction;
use solana_program::pubkey::Pubkey;
use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;
use solana_sdk::transaction::Transaction;
use tracing::{debug, trace};

mod identifiers;
mod detection;
mod opportunity;

pub use identifiers::*;
pub use detection::*;
pub use opportunity::*;

/// Protocol registry for tracking supported protocols
#[derive(Debug, Clone)]
pub struct ProtocolRegistry {
    /// Map of protocol IDs to their names
    protocol_map: HashMap<Pubkey, ProtocolInfo>,
}

/// Protocol information
#[derive(Debug, Clone)]
pub struct ProtocolInfo {
    /// Protocol name
    pub name: String,
    
    /// Protocol type
    pub protocol_type: ProtocolType,
    
    /// Protocol ID (Pubkey)
    pub id: Pubkey,
    
    /// Whether the protocol is supported for arbitrage
    pub supported_for_arbitrage: bool,
}

/// Protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolType {
    /// DEX or AMM
    Dex,
    
    /// Lending platform
    Lending,
    
    /// Yield farming
    Yield,
    
    /// NFT marketplace
    NftMarketplace,
    
    /// Governance
    Governance,
    
    /// MEV-specific
    Mev,
    
    /// Other protocol type
    Other,
}

impl ProtocolRegistry {
    /// Create a new protocol registry
    pub fn new() -> Self {
        let mut registry = Self {
            protocol_map: HashMap::new(),
        };
        
        // Initialize with known protocols
        registry.initialize();
        
        registry
    }
    
    /// Initialize the registry with known protocols
    fn initialize(&mut self) {
        // Add DEX protocols
        self.add_protocol(RAYDIUM_SWAP_V2, "Raydium Swap V2", ProtocolType::Dex, true);
        self.add_protocol(RAYDIUM_LIQUIDITY_V4, "Raydium Liquidity V4", ProtocolType::Dex, true);
        self.add_protocol(RAYDIUM_CONCENTRATED_LIQUIDITY, "Raydium Concentrated Liquidity", ProtocolType::Dex, true);
        self.add_protocol(ORCA_WHIRLPOOL, "Orca Whirlpool", ProtocolType::Dex, true);
        self.add_protocol(ORCA_SWAP_V1, "Orca Swap V1", ProtocolType::Dex, true);
        self.add_protocol(ORCA_SWAP_V2, "Orca Swap V2", ProtocolType::Dex, true);
        self.add_protocol(JUPITER_V6, "Jupiter V6", ProtocolType::Dex, true);
        self.add_protocol(JUPITER_AGGREGATOR_V4, "Jupiter Aggregator V4", ProtocolType::Dex, true);
        self.add_protocol(OPENBOOK_V2, "Openbook V2", ProtocolType::Dex, true);
        self.add_protocol(OPENBOOK_DEX_V1, "Openbook DEX V1", ProtocolType::Dex, true);
        self.add_protocol(METEORA_POOL, "Meteora Pool", ProtocolType::Dex, true);
        self.add_protocol(PHOENIX_DEX, "Phoenix DEX", ProtocolType::Dex, true);
        self.add_protocol(DRIFT_PROTOCOL, "Drift Protocol", ProtocolType::Dex, true);
        self.add_protocol(MANGO_V4, "Mango V4", ProtocolType::Dex, true);
        
        // Add lending protocols
        self.add_protocol(SOLEND_PROGRAM, "Solend", ProtocolType::Lending, true);
        self.add_protocol(SOLEND_V2_PROGRAM, "Solend V2", ProtocolType::Lending, true);
        self.add_protocol(PORT_FINANCE, "Port Finance", ProtocolType::Lending, true);
        self.add_protocol(LARIX_LENDING, "Larix Lending", ProtocolType::Lending, true);
        self.add_protocol(JET_PROTOCOL, "Jet Protocol", ProtocolType::Lending, true);
        self.add_protocol(APRICOT_FINANCE, "Apricot Finance", ProtocolType::Lending, true);
        self.add_protocol(TULIP_PROTOCOL, "Tulip Protocol", ProtocolType::Lending, true);
        
        // Add staking and yield farming protocols
        self.add_protocol(MARINADE_FINANCE, "Marinade Finance", ProtocolType::Yield, true);
        self.add_protocol(LIDO_FINANCE, "Lido Finance", ProtocolType::Yield, true);
        self.add_protocol(QUARRY_MERGE_MINE, "Quarry Merge Mine", ProtocolType::Yield, true);
        
        // Add token programs
        self.add_protocol(TOKEN_PROGRAM, "SPL Token", ProtocolType::Other, true);
        self.add_protocol(TOKEN_2022_PROGRAM, "Token 2022", ProtocolType::Other, true);
        self.add_protocol(ASSOCIATED_TOKEN_PROGRAM, "Associated Token", ProtocolType::Other, true);
        
        // Add NFT marketplaces
        self.add_protocol(MAGIC_EDEN, "Magic Eden", ProtocolType::NftMarketplace, false);
        self.add_protocol(TENSOR_MARKETPLACE, "Tensor", ProtocolType::NftMarketplace, false);
        self.add_protocol(HYPERSPACE, "Hyperspace", ProtocolType::NftMarketplace, false);
        
        // Add MEV-specific protocols
        self.add_protocol(JITO_BLOCK_ENGINE, "Jito Block Engine", ProtocolType::Mev, true);
        self.add_protocol(JITO_SOLANA_ACCESS, "Jito Solana Access", ProtocolType::Mev, true);
    }
    
    /// Add a protocol to the registry
    fn add_protocol(&mut self, address: &str, name: &str, protocol_type: ProtocolType, supported_for_arbitrage: bool) {
        if let Ok(pubkey) = Pubkey::from_str(address) {
            self.protocol_map.insert(pubkey, ProtocolInfo {
                name: name.to_string(),
                protocol_type,
                id: pubkey,
                supported_for_arbitrage,
            });
        }
    }
    
    /// Get protocol info by pubkey
    pub fn get_protocol_info(&self, pubkey: &Pubkey) -> Option<&ProtocolInfo> {
        self.protocol_map.get(pubkey)
    }
    
    /// Get protocol info by address string
    pub fn get_protocol_info_by_address(&self, address: &str) -> Option<&ProtocolInfo> {
        if let Ok(pubkey) = Pubkey::from_str(address) {
            self.get_protocol_info(&pubkey)
        } else {
            None
        }
    }
    
    /// Check if a protocol is supported for arbitrage
    pub fn is_supported_for_arbitrage(&self, pubkey: &Pubkey) -> bool {
        self.get_protocol_info(pubkey)
            .map(|info| info.supported_for_arbitrage)
            .unwrap_or(false)
    }
    
    /// Get all protocols of a specific type
    pub fn get_protocols_by_type(&self, protocol_type: ProtocolType) -> Vec<&ProtocolInfo> {
        self.protocol_map.values()
            .filter(|info| info.protocol_type == protocol_type)
            .collect()
    }
    
    /// Get all DEX protocols
    pub fn get_dex_protocols(&self) -> Vec<&ProtocolInfo> {
        self.get_protocols_by_type(ProtocolType::Dex)
    }
    
    /// Get all lending protocols
    pub fn get_lending_protocols(&self) -> Vec<&ProtocolInfo> {
        self.get_protocols_by_type(ProtocolType::Lending)
    }
}