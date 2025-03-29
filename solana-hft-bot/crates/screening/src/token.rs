use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use chrono::{DateTime, Utc};

/// Token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    /// Token mint address
    pub mint: Pubkey,
    
    /// Token name
    pub name: String,
    
    /// Token symbol
    pub symbol: String,
    
    /// Token decimals
    pub decimals: u8,
    
    /// Total supply
    pub total_supply: u64,
    
    /// Circulating supply (if known)
    pub circulating_supply: Option<u64>,
    
    /// Token metadata
    pub metadata: TokenMetadata,
    
    /// Current price in USD (if known)
    pub price_usd: Option<f64>,
    
    /// Market cap in USD (if known)
    pub market_cap_usd: Option<f64>,
    
    /// 24h trading volume in USD (if known)
    pub volume_24h_usd: Option<f64>,
    
    /// Token score
    pub score: crate::scoring::TokenScore,
    
    /// Liquidity across different DEXes
    pub liquidity: HashMap<String, f64>,
    
    /// Top token holders
    pub holders: Vec<TokenHolder>,
    
    /// When the token was first tracked
    pub tracked_since: DateTime<Utc>,
    
    /// When the token information was last updated
    pub last_updated: DateTime<Utc>,
    
    /// Tags for the token
    pub tags: Vec<String>,
}

/// Token metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    /// Logo URI
    pub logo_uri: Option<String>,
    
    /// Website URL
    pub website: Option<String>,
    
    /// Twitter handle
    pub twitter: Option<String>,
    
    /// Whether the token is verified
    pub is_verified: bool,
    
    /// When the token was launched
    pub launch_timestamp: Option<DateTime<Utc>>,
    
    /// Tags for categorization
    pub tags: Vec<String>,
    
    /// Token description
    pub description: Option<String>,
}

/// Token holder information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenHolder {
    /// Holder address
    pub address: Pubkey,
    
    /// Amount held
    pub amount: u64,
    
    /// Percentage of total supply
    pub percentage: f64,
}

/// Additional token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Token mint address
    pub mint: Pubkey,
    
    /// Token price history
    pub price_history: Vec<PricePoint>,
    
    /// Volume history
    pub volume_history: Vec<VolumePoint>,
    
    /// Recent transactions
    pub recent_transactions: Vec<TokenTransaction>,
    
    /// Social metrics
    pub social: SocialMetrics,
}

/// Price point for historical data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Price in USD
    pub price_usd: f64,
}

/// Volume point for historical data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumePoint {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Volume in USD
    pub volume_usd: f64,
}

/// Token transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTransaction {
    /// Transaction signature
    pub signature: String,
    
    /// Transaction type
    pub transaction_type: TokenTransactionType,
    
    /// Amount involved
    pub amount: u64,
    
    /// USD value (if known)
    pub usd_value: Option<f64>,
    
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Token transaction type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenTransactionType {
    /// Token transfer
    Transfer,
    
    /// Token swap/trade
    Swap,
    
    /// Liquidity addition
    LiquidityAdd,
    
    /// Liquidity removal
    LiquidityRemove,
    
    /// Token mint
    Mint,
    
    /// Token burn
    Burn,
    
    /// Other transaction type
    Other,
}

/// Social metrics for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialMetrics {
    /// Twitter followers
    pub twitter_followers: Option<u32>,
    
    /// Discord members
    pub discord_members: Option<u32>,
    
    /// Telegram members
    pub telegram_members: Option<u32>,
    
    /// Recent mentions count
    pub recent_mentions: Option<u32>,
    
    /// Sentiment score (-1.0 to 1.0)
    pub sentiment_score: Option<f64>,
}

impl Token {
    /// Create a new token instance
    pub fn new(
        mint: Pubkey,
        name: String,
        symbol: String,
        decimals: u8,
        total_supply: u64,
    ) -> Self {
        Self {
            mint,
            name,
            symbol,
            decimals,
            total_supply,
            circulating_supply: None,
            metadata: TokenMetadata {
                logo_uri: None,
                website: None,
                twitter: None,
                is_verified: false,
                launch_timestamp: None,
                tags: Vec::new(),
                description: None,
            },
            price_usd: None,
            market_cap_usd: None,
            volume_24h_usd: None,
            score: crate::scoring::TokenScore::default(),
            liquidity: HashMap::new(),
            holders: Vec::new(),
            tracked_since: Utc::now(),
            last_updated: Utc::now(),
            tags: Vec::new(),
        }
    }

    /// Calculate total liquidity across all DEXes
    pub fn total_liquidity(&self) -> f64 {
        self.liquidity.values().sum()
    }

    /// Check if the token is verified
    pub fn is_verified(&self) -> bool {
        self.metadata.is_verified
    }

    /// Get token age in seconds
    pub fn age_seconds(&self) -> Option<i64> {
        self.metadata.launch_timestamp.map(|launch_time| {
            Utc::now().signed_duration_since(launch_time).num_seconds()
        })
    }

    /// Update token price
    pub fn update_price(&mut self, price_usd: f64) {
        self.price_usd = Some(price_usd);
        self.last_updated = Utc::now();
        
        // Update market cap if we have circulating supply
        if let Some(supply) = self.circulating_supply {
            self.market_cap_usd = Some(price_usd * (supply as f64) / 10f64.powi(self.decimals as i32));
        }
    }

    /// Add liquidity information for a DEX
    pub fn add_liquidity(&mut self, dex_name: &str, liquidity_usd: f64) {
        self.liquidity.insert(dex_name.to_string(), liquidity_usd);
        self.last_updated = Utc::now();
    }

    /// Add a tag to the token
    pub fn add_tag(&mut self, tag: &str) {
        if !self.tags.contains(&tag.to_string()) {
            self.tags.push(tag.to_string());
        }
    }
}