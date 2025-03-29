//! Order types and management
//! 
//! This module defines order types and management for the execution engine.

use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
};
use std::time::Duration;

use crate::priority::PriorityLevel;

/// Order side (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderSide {
    /// Buy order
    Buy,
    
    /// Sell order
    Sell,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    /// Market order - execute immediately at market price
    Market,
    
    /// Limit order - execute at specified price or better
    Limit,
    
    /// Post-only order - only execute as maker
    PostOnly,
    
    /// Immediate-or-cancel order - execute immediately or cancel
    IOC,
    
    /// Fill-or-kill order - execute completely or cancel
    FOK,
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order is created but not yet submitted
    Created,
    
    /// Order is pending execution
    Pending,
    
    /// Order is being processed
    Processing,
    
    /// Order is filled (completely executed)
    Filled,
    
    /// Order is partially filled
    PartiallyFilled,
    
    /// Order is cancelled
    Cancelled,
    
    /// Order execution failed
    Failed,
    
    /// Order expired
    Expired,
}

/// Order representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Unique order ID
    pub id: u64,
    
    /// Market ID
    pub market: Pubkey,
    
    /// Order side (buy or sell)
    pub side: OrderSide,
    
    /// Order type
    pub order_type: OrderType,
    
    /// Order size
    pub size: u64,
    
    /// Order price (for limit orders)
    pub price: Option<u64>,
    
    /// Client order ID
    pub client_id: Option<u64>,
    
    /// Order status
    pub status: OrderStatus,
    
    /// Priority level
    pub priority: PriorityLevel,
    
    /// Whether to use Jito bundles
    pub use_jito: bool,
    
    /// Payer account
    #[serde(skip)]
    pub payer: Keypair,
    
    /// Owner account
    pub owner: Pubkey,
    
    /// Compute units to request
    pub compute_units: Option<u32>,
    
    /// Priority fee in micro-lamports
    pub priority_fee: Option<u64>,
    
    /// Maximum fee to pay (in lamports)
    pub max_fee: Option<u64>,
    
    /// Order timeout
    pub timeout: Duration,
    
    /// Self-trade behavior
    pub self_trade_behavior: SelfTradeBehavior,
    
    /// Maximum number of orders to match against
    pub max_matches: Option<u64>,
    
    /// Oracle price
    pub oracle_price: Option<u64>,
    
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Expiration timestamp
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Self-trade behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelfTradeBehavior {
    /// Decrement take (maker gets priority)
    DecrementTake,
    
    /// Cancel provide (taker gets priority)
    CancelProvide,
    
    /// Abort transaction
    AbortTransaction,
}

impl Order {
    /// Create a new order
    pub fn new(
        market: Pubkey,
        side: OrderSide,
        order_type: OrderType,
        size: u64,
        payer: Keypair,
        owner: Pubkey,
    ) -> Self {
        Self {
            id: rand::random(),
            market,
            side,
            order_type,
            size,
            price: None,
            client_id: None,
            status: OrderStatus::Created,
            priority: PriorityLevel::Medium,
            use_jito: false,
            payer,
            owner,
            compute_units: None,
            priority_fee: None,
            max_fee: None,
            timeout: Duration::from_secs(30),
            self_trade_behavior: SelfTradeBehavior::DecrementTake,
            max_matches: None,
            oracle_price: None,
            created_at: chrono::Utc::now(),
            expires_at: None,
        }
    }
    
    /// Create a new market order
    pub fn market(
        market: Pubkey,
        side: OrderSide,
        size: u64,
        payer: Keypair,
        owner: Pubkey,
    ) -> Self {
        Self::new(market, side, OrderType::Market, size, payer, owner)
    }
    
    /// Create a new limit order
    pub fn limit(
        market: Pubkey,
        side: OrderSide,
        size: u64,
        price: u64,
        payer: Keypair,
        owner: Pubkey,
    ) -> Self {
        let mut order = Self::new(market, side, OrderType::Limit, size, payer, owner);
        order.price = Some(price);
        order
    }
    
    /// Set the order price
    pub fn with_price(mut self, price: u64) -> Self {
        self.price = Some(price);
        self
    }
    
    /// Set the client order ID
    pub fn with_client_id(mut self, client_id: u64) -> Self {
        self.client_id = Some(client_id);
        self
    }
    
    /// Set the priority level
    pub fn with_priority(mut self, priority: PriorityLevel) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set whether to use Jito bundles
    pub fn with_jito(mut self, use_jito: bool) -> Self {
        self.use_jito = use_jito;
        self
    }
    
    /// Set the compute units
    pub fn with_compute_units(mut self, compute_units: u32) -> Self {
        self.compute_units = Some(compute_units);
        self
    }
    
    /// Set the priority fee
    pub fn with_priority_fee(mut self, priority_fee: u64) -> Self {
        self.priority_fee = Some(priority_fee);
        self
    }
    
    /// Set the maximum fee
    pub fn with_max_fee(mut self, max_fee: u64) -> Self {
        self.max_fee = Some(max_fee);
        self
    }
    
    /// Set the order timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// Set the self-trade behavior
    pub fn with_self_trade_behavior(mut self, behavior: SelfTradeBehavior) -> Self {
        self.self_trade_behavior = behavior;
        self
    }
    
    /// Set the maximum number of orders to match against
    pub fn with_max_matches(mut self, max_matches: u64) -> Self {
        self.max_matches = Some(max_matches);
        self
    }
    
    /// Set the oracle price
    pub fn with_oracle_price(mut self, oracle_price: u64) -> Self {
        self.oracle_price = Some(oracle_price);
        self
    }
    
    /// Set the expiration timestamp
    pub fn with_expiration(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }
    
    /// Check if the order has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }
    
    /// Check if the order is active
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Created | OrderStatus::Pending | OrderStatus::Processing | OrderStatus::PartiallyFilled
        )
    }
    
    /// Check if the order is complete
    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Failed | OrderStatus::Expired
        )
    }
}
