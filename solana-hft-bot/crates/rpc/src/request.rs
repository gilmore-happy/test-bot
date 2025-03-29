//! Request handling for RPC client
//!
//! This module provides functionality for handling RPC requests,
//! including request prioritization, context tracking, and options.

use std::time::Instant;
use std::fmt;

use solana_sdk::commitment_config::CommitmentLevel;
use serde::{Deserialize, Serialize};

use crate::endpoints::EndpointSelectionStrategy;

/// Request priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RequestPriority {
    /// Critical priority - for time-sensitive operations like transaction submission
    Critical,
    
    /// High priority - for important operations
    High,
    
    /// Normal priority - default for most operations
    Normal,
    
    /// Low priority - for non-urgent operations
    Low,
    
    /// Background priority - for prefetching and maintenance operations
    Background,
}

impl Default for RequestPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl fmt::Display for RequestPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Critical => write!(f, "Critical"),
            Self::High => write!(f, "High"),
            Self::Normal => write!(f, "Normal"),
            Self::Low => write!(f, "Low"),
            Self::Background => write!(f, "Background"),
        }
    }
}

/// Request context for tracking and managing requests
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Request ID
    pub id: u64,
    
    /// RPC method name
    pub method: String,
    
    /// Request priority
    pub priority: RequestPriority,
    
    /// Request start time
    pub start_time: Instant,
    
    /// Whether the request is cacheable
    pub cacheable: bool,
    
    /// Whether the request can be batched
    pub batchable: bool,
    
    /// Whether the request requires quorum
    pub requires_quorum: bool,
    
    /// Number of retries attempted
    pub retry_count: u32,
    
    /// Maximum number of retries allowed
    pub max_retries: u32,
}

/// Request options for customizing request behavior
#[derive(Debug, Clone)]
pub struct RequestOptions {
    /// Request priority
    pub priority: RequestPriority,
    
    /// Whether to bypass the cache
    pub bypass_cache: bool,
    
    /// Whether to bypass rate limiting
    pub bypass_rate_limit: bool,
    
    /// Whether to allow batching
    pub allow_batching: bool,
    
    /// Specific endpoint URL to use
    pub specific_endpoint: Option<String>,
    
    /// Endpoint selection strategy
    pub selection_strategy: Option<EndpointSelectionStrategy>,
    
    /// Request timeout in milliseconds
    pub timeout_ms: Option<u64>,
    
    /// Minimum commitment level
    pub min_commitment: Option<CommitmentLevel>,
    
    /// Maximum number of retries
    pub max_retries: Option<u32>,
    
    /// Whether to require quorum
    pub require_quorum: bool,
    
    /// Quorum size (number of endpoints that must agree)
    pub quorum_size: Option<usize>,
    
    /// Quorum threshold (percentage of endpoints that must agree)
    pub quorum_threshold: Option<f64>,
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            priority: RequestPriority::Normal,
            bypass_cache: false,
            bypass_rate_limit: false,
            allow_batching: true,
            specific_endpoint: None,
            selection_strategy: None,
            timeout_ms: None,
            min_commitment: None,
            max_retries: None,
            require_quorum: false,
            quorum_size: None,
            quorum_threshold: None,
        }
    }
}

impl RequestOptions {
    /// Create new options with critical priority
    pub fn critical() -> Self {
        Self {
            priority: RequestPriority::Critical,
            ..Default::default()
        }
    }
    
    /// Create new options with high priority
    pub fn high() -> Self {
        Self {
            priority: RequestPriority::High,
            ..Default::default()
        }
    }
    
    /// Create new options with low priority
    pub fn low() -> Self {
        Self {
            priority: RequestPriority::Low,
            ..Default::default()
        }
    }
    
    /// Create new options with background priority
    pub fn background() -> Self {
        Self {
            priority: RequestPriority::Background,
            ..Default::default()
        }
    }
    
    /// Set the priority
    pub fn with_priority(mut self, priority: RequestPriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set whether to bypass the cache
    pub fn with_bypass_cache(mut self, bypass: bool) -> Self {
        self.bypass_cache = bypass;
        self
    }
    
    /// Set whether to bypass rate limiting
    pub fn with_bypass_rate_limit(mut self, bypass: bool) -> Self {
        self.bypass_rate_limit = bypass;
        self
    }
    
    /// Set whether to allow batching
    pub fn with_allow_batching(mut self, allow: bool) -> Self {
        self.allow_batching = allow;
        self
    }
    
    /// Set a specific endpoint URL to use
    pub fn with_specific_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.specific_endpoint = Some(endpoint.into());
        self
    }
    
    /// Set the endpoint selection strategy
    pub fn with_selection_strategy(mut self, strategy: EndpointSelectionStrategy) -> Self {
        self.selection_strategy = Some(strategy);
        self
    }
    
    /// Set the request timeout in milliseconds
    pub fn with_timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = Some(timeout);
        self
    }
    
    /// Set the minimum commitment level
    pub fn with_min_commitment(mut self, commitment: CommitmentLevel) -> Self {
        self.min_commitment = Some(commitment);
        self
    }
    
    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }
    
    /// Set whether to require quorum
    pub fn with_require_quorum(mut self, require: bool) -> Self {
        self.require_quorum = require;
        self
    }
    
    /// Set the quorum size
    pub fn with_quorum_size(mut self, size: usize) -> Self {
        self.quorum_size = Some(size);
        self.require_quorum = true;
        self
    }
    
    /// Set the quorum threshold
    pub fn with_quorum_threshold(mut self, threshold: f64) -> Self {
        self.quorum_threshold = Some(threshold);
        self.require_quorum = true;
        self
    }
}