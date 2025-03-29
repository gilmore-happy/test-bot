//! Protocol identifiers for Solana DeFi
//!
//! This module contains program IDs and constants for various
//! Solana DeFi protocols used in arbitrage detection.

use solana_program::pubkey::Pubkey;
use solana_program::instruction::CompiledInstruction;
use solana_program::hash::Hash;
use solana_program::{pubkey, program_error::ProgramError};
use std::str::FromStr;

// DEX and AMM Programs
pub const RAYDIUM_SWAP_V2: &str = "27haf8L6oxUeXrHrgEgsexjSY5hbVUWEmvv9Nyxg8vQv";
pub const RAYDIUM_LIQUIDITY_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const RAYDIUM_CONCENTRATED_LIQUIDITY: &str = "9rpQHSyFVM1dkkHFQ2TtTzPEYPJyy7HPNeEWHKPJEicB";

pub const ORCA_WHIRLPOOL: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
pub const ORCA_SWAP_V1: &str = "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1";
pub const ORCA_SWAP_V2: &str = "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP";

pub const JUPITER_V6: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
pub const JUPITER_AGGREGATOR_V4: &str = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB";

pub const OPENBOOK_V2: &str = "opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb";
pub const OPENBOOK_DEX_V1: &str = "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX";

pub const METEORA_POOL: &str = "M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K";
pub const PHOENIX_DEX: &str = "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY";
pub const DRIFT_PROTOCOL: &str = "dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH";
pub const MANGO_V4: &str = "4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg";

// Lending Platforms
pub const SOLEND_PROGRAM: &str = "So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo";
pub const SOLEND_V2_PROGRAM: &str = "SLDx3BJ3NR8ksQgeCLNjKnYY6L4RRZzJyZntMnH3U41";
pub const PORT_FINANCE: &str = "Port7uDYB3wk6GJAw4KT1WpTeMtSu9bTcChBHkX2LfR";
pub const LARIX_LENDING: &str = "7Zb1bGi32pfsrBkzWdqd4dFhUXwp5Nybr1zuS7g6rNLE";
pub const JET_PROTOCOL: &str = "JPv1rCqrhagNNmJVM5J1he7msQ5ybtvE1nNuHpDHMNU";
pub const APRICOT_FINANCE: &str = "7Ne8S2j4qhEL4vQX7dVT4NWKc3nYKMkeEeLB3B2GoXZ6";
pub const TULIP_PROTOCOL: &str = "4bcFeLv4nydFrsZqV5CgwCVrPhkQKsXtzfy2KyMz7ozM";

// Staking and Yield Farming
pub const MARINADE_FINANCE: &str = "MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD";
pub const LIDO_FINANCE: &str = "CrX7kMhLC3cSsXJdT7JDgqrRVWGnUpX3gfEfxxU2NVLi";
pub const QUARRY_MERGE_MINE: &str = "QMMD16kjauP5knBwxNUJRZ1Z5o3deBuFrqVjBVmmqto";

// Token Programs
pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
pub const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

// Governance & Launch Programs
pub const METAPLEX_TOKEN_METADATA: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";
pub const SERUM_GOV: &str = "GovHgfDPyQ1GwazJTDY2avSVY8GGcpmCapmmCsymRaGe";
pub const REALMS_PROGRAM: &str = "GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw";

// MEV-Specific Programs
pub const JITO_BLOCK_ENGINE: &str = "JitoEJpr2PKXuEY91HqcpYJ5xS4r4fNVAi7foCbQmdL";
pub const JITO_SOLANA_ACCESS: &str = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn";

// NFT Marketplaces (for NFT MEV)
pub const MAGIC_EDEN: &str = "M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K";
pub const TENSOR_MARKETPLACE: &str = "TSWAPaqyCSx2KABk68Shruf4rp7CxcNi8hAsbdwmHbN";
pub const HYPERSPACE: &str = "HYPERfwdTjyJ2SCaKHmpF2MtrXqWxrsotYDsTrshHWq8";

// Instruction Tags - used for identifying specific instruction types
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InstructionTag {
    Unknown,
    Swap,
    Liquidity,
    Liquidation,
    TokenTransfer,
    NftTransaction,
    Staking,
    Unstaking,
    MevBundle,
    Arbitrage,
}

// SPL Token Program Instructions (documented and well-known)
pub enum TokenInstruction {
    InitializeMint = 0,
    InitializeAccount = 1,
    InitializeMultisig = 2,
    Transfer = 3,
    Approve = 4,
    Revoke = 5,
    SetAuthority = 6,
    MintTo = 7,
    Burn = 8,
    CloseAccount = 9,
    FreezeAccount = 10,
    ThawAccount = 11,
    TransferChecked = 12,
    ApproveChecked = 13,
    MintToChecked = 14,
    BurnChecked = 15,
    InitializeAccount2 = 16,
    SyncNative = 17,
    InitializeAccount3 = 18,
    InitializeMultisig2 = 19,
    InitializeMint2 = 20,
    GetAccountDataSize = 21,
    InitializeImmutableOwner = 22,
    AmountToUiAmount = 23,
    UiAmountToAmount = 24,
    InitializeMintCloseAuthority = 25,
    TransferFeeExtension = 26,
    ConfidentialTransferExtension = 27,
    DefaultAccountStateExtension = 28,
    Reallocate = 29,
    MemoTransferExtension = 30,
    CreateNativeMint = 31,
    InitializeNonTransferableMint = 32,
    InterestBearingMintExtension = 33,
}

// Solend instructions (based on their program)
pub enum SolendInstruction {
    InitMarket = 0,
    SetMarketOwner = 1,
    UpdateMarket = 2,
    AddReserve = 3,
    ActivateReserve = 4,
    Deposit = 5,
    Withdraw = 6,
    Borrow = 7,
    Repay = 8,
    LiquidateObligor = 9,  // This is the liquidation instruction
    FlashLoan = 10,
    DepositReserveLiquidity = 11,
    RefreshReserve = 12,
    WithdrawFee = 13,
    UpdateReserveConfig = 14,
    RedeemReserveCollateral = 15,
    // There are more instructions, but these are the most relevant for MEV
}

// Raydium instruction types (based on their program)
pub enum RaydiumInstruction {
    Initialize = 0,
    Swap = 1,
    DepositAll = 2,
    WithdrawAll = 3,
    Deposit = 4,
    Withdraw = 5,
    // More instructions exist, these are the key ones for MEV
}

// Orca Whirlpool instruction types
pub enum OrcaWhirlpoolInstruction {
    Initialize = 0,
    InitializeConfig = 1,
    InitializePool = 2,
    InitializeTickArray = 3,
    InitializeFeeTier = 4,
    OpenPosition = 5,
    OpenPositionWithMetadata = 6,
    IncreaseLiquidity = 7,
    DecreaseLiquidity = 8,
    Swap = 9,  // This is the key swap instruction
    ClosePosition = 10,
    CollectFees = 11,
    CollectReward = 12,
    CollectProtocolFees = 13,
    // More instructions exist
}

// Jupiter instruction (simplified, as Jupiter is an aggregator with complex patterns)
pub enum JupiterInstruction {
    Exchange = 0,
    ExactOutExchange = 1,
    // Jupiter has many more patterns as it aggregates multiple protocols
}

// PhoenixDEX instruction types (simplified)
pub enum PhoenixInstruction {
    PlaceOrder = 0,
    CancelOrder = 1,
    CancelAllOrders = 2,
    Swap = 3,  // Direct swap
    // More instructions exist
}

// Mango v4 instruction types (simplified)
pub enum MangoV4Instruction {
    Initialize = 0,
    Borrow = 3,
    Deposit = 4,
    Withdraw = 5,
    PlaceOrder = 6,
    CancelOrder = 7,
    Liquidate = 8,  // Liquidation instruction
    // Many more instructions exist
}

/// Utility function to get Pubkey from string
pub fn pubkey_from_str(s: &str) -> Result<Pubkey, ProgramError> {
    Pubkey::from_str(s).map_err(|_| ProgramError::InvalidArgument)
}

/// Check if a program is in our known MEV-relevant list
pub fn is_mev_relevant_program(program_id: &Pubkey) -> bool {
    let program_id_str = program_id.to_string();
    
    matches!(program_id_str.as_str(),
        // DEXs and AMMs
        RAYDIUM_SWAP_V2 | RAYDIUM_LIQUIDITY_V4 | RAYDIUM_CONCENTRATED_LIQUIDITY |
        ORCA_WHIRLPOOL | ORCA_SWAP_V1 | ORCA_SWAP_V2 |
        JUPITER_V6 | JUPITER_AGGREGATOR_V4 |
        OPENBOOK_V2 | OPENBOOK_DEX_V1 |
        METEORA_POOL | PHOENIX_DEX | DRIFT_PROTOCOL | MANGO_V4 |
        
        // Lending Platforms
        SOLEND_PROGRAM | SOLEND_V2_PROGRAM | PORT_FINANCE |
        LARIX_LENDING | JET_PROTOCOL | APRICOT_FINANCE | TULIP_PROTOCOL |
        
        // Token Programs (for transfers)
        TOKEN_PROGRAM | TOKEN_2022_PROGRAM |
        
        // NFT Marketplaces
        MAGIC_EDEN | TENSOR_MARKETPLACE | HYPERSPACE | METAPLEX_TOKEN_METADATA |
        
        // MEV-Specific
        JITO_BLOCK_ENGINE | JITO_SOLANA_ACCESS
    )
}