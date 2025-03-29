use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};
use dashmap::DashMap;
use crate::OpportunityType;

/// Snapshot of screening metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningMetricsSnapshot {
    /// Number of tokens being tracked
    pub tokens_tracked: u64,
    
    /// Number of liquidity pools being tracked
    pub liquidity_pools_tracked: u64,
    
    /// Number of subscription events received
    pub subscription_events: u64,
    
    /// Number of tokens added
    pub tokens_added: u64,
    
    /// Number of tokens updated
    pub tokens_updated: u64,
    
    /// Number of opportunities detected
    pub opportunities_detected: HashMap<OpportunityType, u64>,
    
    /// Current slot number
    pub current_slot: u64,
    
    /// Number of tokens screened
    pub tokens_screened: u64,
    
    /// Number of tokens rejected
    pub tokens_rejected: u64,
    
    /// Number of transactions screened
    pub transactions_screened: u64,
    
    /// Number of transactions rejected
    pub transactions_rejected: u64,
}

/// Metrics for the screening engine
pub struct ScreeningMetrics {
    /// Number of tokens being tracked
    tokens_tracked: AtomicU64,
    
    /// Number of liquidity pools being tracked
    liquidity_pools_tracked: AtomicU64,
    
    /// Number of subscription events received
    subscription_events: AtomicU64,
    
    /// Number of tokens added
    tokens_added: AtomicU64,
    
    /// Number of tokens updated
    tokens_updated: AtomicU64,
    
    /// Number of opportunities detected
    opportunities_detected: DashMap<OpportunityType, u64>,
    
    /// Current slot number
    current_slot: AtomicU64,
    
    /// Number of tokens screened
    tokens_screened: AtomicU64,
    
    /// Number of tokens rejected
    tokens_rejected: AtomicU64,
    
    /// Number of transactions screened
    transactions_screened: AtomicU64,
    
    /// Number of transactions rejected
    transactions_rejected: AtomicU64,
}

impl ScreeningMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            tokens_tracked: AtomicU64::new(0),
            liquidity_pools_tracked: AtomicU64::new(0),
            subscription_events: AtomicU64::new(0),
            tokens_added: AtomicU64::new(0),
            tokens_updated: AtomicU64::new(0),
            opportunities_detected: DashMap::new(),
            current_slot: AtomicU64::new(0),
            tokens_screened: AtomicU64::new(0),
            tokens_rejected: AtomicU64::new(0),
            transactions_screened: AtomicU64::new(0),
            transactions_rejected: AtomicU64::new(0),
        }
    }
    
    /// Record a new token being tracked
    pub fn record_token_added(&self) {
        self.tokens_added.fetch_add(1, Ordering::SeqCst);
        self.tokens_tracked.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Record a token being updated
    pub fn record_token_updated(&self) {
        self.tokens_updated.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Record a token being removed
    pub fn record_token_removed(&self) {
        self.tokens_tracked.fetch_sub(1, Ordering::SeqCst);
    }
    
    /// Record a new liquidity pool being tracked
    pub fn record_liquidity_pool_added(&self) {
        self.liquidity_pools_tracked.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Record a liquidity pool being removed
    pub fn record_liquidity_pool_removed(&self) {
        self.liquidity_pools_tracked.fetch_sub(1, Ordering::SeqCst);
    }
    
    /// Record a subscription event
    pub fn record_subscription_event(&self, _event_type: &str) {
        self.subscription_events.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Record an opportunity being detected
    pub fn record_opportunity_detected(&self, opportunity_type: OpportunityType) {
        *self.opportunities_detected
            .entry(opportunity_type)
            .or_insert(0) += 1;
    }
    
    /// Record current slot
    pub fn record_slot(&self, slot: u64) {
        self.current_slot.store(slot, Ordering::SeqCst);
    }
    
    /// Record a token being screened
    pub fn record_token_screened(&self, passed: bool) {
        self.tokens_screened.fetch_add(1, Ordering::SeqCst);
        if !passed {
            self.tokens_rejected.fetch_add(1, Ordering::SeqCst);
        }
    }
    
    /// Record a transaction being screened
    pub fn record_transaction_screened(&self, passed: bool) {
        self.transactions_screened.fetch_add(1, Ordering::SeqCst);
        if !passed {
            self.transactions_rejected.fetch_add(1, Ordering::SeqCst);
        }
    }
    
    /// Get a snapshot of the current metrics
    pub fn snapshot(&self) -> ScreeningMetricsSnapshot {
        let opportunities_detected = self.opportunities_detected
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect();
        
        ScreeningMetricsSnapshot {
            tokens_tracked: self.tokens_tracked.load(Ordering::SeqCst),
            liquidity_pools_tracked: self.liquidity_pools_tracked.load(Ordering::SeqCst),
            subscription_events: self.subscription_events.load(Ordering::SeqCst),
            tokens_added: self.tokens_added.load(Ordering::SeqCst),
            tokens_updated: self.tokens_updated.load(Ordering::SeqCst),
            opportunities_detected,
            current_slot: self.current_slot.load(Ordering::SeqCst),
            tokens_screened: self.tokens_screened.load(Ordering::SeqCst),
            tokens_rejected: self.tokens_rejected.load(Ordering::SeqCst),
            transactions_screened: self.transactions_screened.load(Ordering::SeqCst),
            transactions_rejected: self.transactions_rejected.load(Ordering::SeqCst),
        }
    }
    
    /// Reset all metrics
    pub fn reset(&self) {
        self.tokens_tracked.store(0, Ordering::SeqCst);
        self.liquidity_pools_tracked.store(0, Ordering::SeqCst);
        self.subscription_events.store(0, Ordering::SeqCst);
        self.tokens_added.store(0, Ordering::SeqCst);
        self.tokens_updated.store(0, Ordering::SeqCst);
        self.opportunities_detected.clear();
        self.current_slot.store(0, Ordering::SeqCst);
        self.tokens_screened.store(0, Ordering::SeqCst);
        self.tokens_rejected.store(0, Ordering::SeqCst);
        self.transactions_screened.store(0, Ordering::SeqCst);
        self.transactions_rejected.store(0, Ordering::SeqCst);
    }
}

impl Default for ScreeningMetrics {
    fn default() -> Self {
        Self::new()
    }
}