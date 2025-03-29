//! Transaction priority levels
//! 
//! This module defines priority levels for transaction execution.

use serde::{Deserialize, Serialize};

/// Priority level for transaction execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PriorityLevel {
    /// Critical priority - must execute as soon as possible
    /// 
    /// Used for time-sensitive transactions like liquidations or arbitrage
    Critical,
    
    /// High priority - should execute quickly
    /// 
    /// Used for important transactions like order placement
    High,
    
    /// Medium priority - normal execution
    /// 
    /// Used for standard transactions
    Medium,
    
    /// Low priority - can wait
    /// 
    /// Used for non-time-sensitive transactions like withdrawals
    Low,
}

impl PriorityLevel {
    /// Get the priority fee multiplier for this level
    pub fn fee_multiplier(&self) -> f64 {
        match self {
            PriorityLevel::Critical => 3.0,
            PriorityLevel::High => 2.0,
            PriorityLevel::Medium => 1.0,
            PriorityLevel::Low => 0.5,
        }
    }
    
    /// Get the compute unit multiplier for this level
    pub fn compute_unit_multiplier(&self) -> f64 {
        match self {
            PriorityLevel::Critical => 1.5,
            PriorityLevel::High => 1.2,
            PriorityLevel::Medium => 1.0,
            PriorityLevel::Low => 0.8,
        }
    }
    
    /// Get the retry count for this level
    pub fn retry_count(&self) -> usize {
        match self {
            PriorityLevel::Critical => 5,
            PriorityLevel::High => 3,
            PriorityLevel::Medium => 2,
            PriorityLevel::Low => 1,
        }
    }
    
    /// Get the timeout multiplier for this level
    pub fn timeout_multiplier(&self) -> f64 {
        match self {
            PriorityLevel::Critical => 0.5, // Shorter timeout for critical txs
            PriorityLevel::High => 0.8,
            PriorityLevel::Medium => 1.0,
            PriorityLevel::Low => 1.5,
        }
    }
    
    /// Whether this priority level should use Jito bundles
    pub fn use_jito(&self) -> bool {
        match self {
            PriorityLevel::Critical => true,
            PriorityLevel::High => true,
            PriorityLevel::Medium => false,
            PriorityLevel::Low => false,
        }
    }
    
    /// Whether this priority level should skip preflight checks
    pub fn skip_preflight(&self) -> bool {
        match self {
            PriorityLevel::Critical => true,
            PriorityLevel::High => true,
            PriorityLevel::Medium => false,
            PriorityLevel::Low => false,
        }
    }
    
    /// Get the queue position for this level (lower is higher priority)
    pub fn queue_position(&self) -> u8 {
        match self {
            PriorityLevel::Critical => 0,
            PriorityLevel::High => 1,
            PriorityLevel::Medium => 2,
            PriorityLevel::Low => 3,
        }
    }
}

impl Default for PriorityLevel {
    fn default() -> Self {
        PriorityLevel::Medium
    }
}

impl From<u8> for PriorityLevel {
    fn from(value: u8) -> Self {
        match value {
            0 => PriorityLevel::Critical,
            1 => PriorityLevel::High,
            2 => PriorityLevel::Medium,
            _ => PriorityLevel::Low,
        }
    }
}

impl From<PriorityLevel> for u8 {
    fn from(value: PriorityLevel) -> Self {
        match value {
            PriorityLevel::Critical => 0,
            PriorityLevel::High => 1,
            PriorityLevel::Medium => 2,
            PriorityLevel::Low => 3,
        }
    }
}
