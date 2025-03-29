//! Metrics collection for the execution engine
//! 
//! This module provides metrics collection and reporting for the execution engine.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tracing::{debug, trace};

/// Snapshot of execution metrics
#[derive(Debug, Clone)]
pub struct ExecutionMetricsSnapshot {
    /// Total number of transactions
    pub total_transactions: u64,
    
    /// Number of successful transactions
    pub successful_transactions: u64,
    
    /// Number of failed transactions
    pub failed_transactions: u64,
    
    /// Number of timed out transactions
    pub timeout_transactions: u64,
    
    /// Number of retried transactions
    pub retried_transactions: u64,
    
    /// Average execution time in milliseconds
    pub avg_execution_time_ms: f64,
    
    /// 95th percentile execution time in milliseconds
    pub p95_execution_time_ms: f64,
    
    /// 99th percentile execution time in milliseconds
    pub p99_execution_time_ms: f64,
    
    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: u64,
    
    /// Average confirmation time in milliseconds
    pub avg_confirmation_time_ms: f64,
    
    /// Average fee paid in lamports
    pub avg_fee_paid: f64,
    
    /// Total fees paid in lamports
    pub total_fees_paid: u64,
    
    /// Number of transactions by priority level
    pub transactions_by_priority: HashMap<String, u64>,
    
    /// Success rate by priority level
    pub success_rate_by_priority: HashMap<String, f64>,
    
    /// Number of Jito bundles
    pub jito_bundles: u64,
    
    /// Number of successful Jito bundles
    pub successful_jito_bundles: u64,
}

/// Transaction metrics
#[derive(Debug)]
struct TransactionMetrics {
    /// Transaction ID
    id: u64,
    
    /// Priority level
    priority: String,
    
    /// Whether Jito was used
    used_jito: bool,
    
    /// Start time
    start_time: Instant,
    
    /// End time
    end_time: Option<Instant>,
    
    /// Confirmation time
    confirmation_time: Option<Instant>,
    
    /// Status (success, error, timeout)
    status: Option<String>,
    
    /// Error message if any
    error: Option<String>,
    
    /// Fee paid in lamports
    fee_paid: Option<u64>,
    
    /// Number of retries
    retries: u64,
}

/// Execution metrics collector
pub struct ExecutionMetrics {
    /// Total number of transactions
    total_transactions: AtomicU64,
    
    /// Number of successful transactions
    successful_transactions: AtomicU64,
    
    /// Number of failed transactions
    failed_transactions: AtomicU64,
    
    /// Number of timed out transactions
    timeout_transactions: AtomicU64,
    
    /// Number of retried transactions
    retried_transactions: AtomicU64,
    
    /// Total fees paid in lamports
    total_fees_paid: AtomicU64,
    
    /// Number of Jito bundles
    jito_bundles: AtomicU64,
    
    /// Number of successful Jito bundles
    successful_jito_bundles: AtomicU64,
    
    /// Transaction metrics
    transactions: RwLock<Vec<TransactionMetrics>>,
    
    /// Maximum number of transactions to keep
    max_transactions: usize,
}

impl ExecutionMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            total_transactions: AtomicU64::new(0),
            successful_transactions: AtomicU64::new(0),
            failed_transactions: AtomicU64::new(0),
            timeout_transactions: AtomicU64::new(0),
            retried_transactions: AtomicU64::new(0),
            total_fees_paid: AtomicU64::new(0),
            jito_bundles: AtomicU64::new(0),
            successful_jito_bundles: AtomicU64::new(0),
            transactions: RwLock::new(Vec::new()),
            max_transactions: 1000,
        }
    }
    
    /// Record a transaction start
    pub fn record_transaction_start(&self, id: u64, priority: &str, use_jito: bool) {
        let metrics = TransactionMetrics {
            id,
            priority: priority.to_string(),
            used_jito: use_jito,
            start_time: Instant::now(),
            end_time: None,
            confirmation_time: None,
            status: None,
            error: None,
            fee_paid: None,
            retries: 0,
        };
        
        self.total_transactions.fetch_add(1, Ordering::SeqCst);
        
        if use_jito {
            self.jito_bundles.fetch_add(1, Ordering::SeqCst);
        }
        
        let mut transactions = self.transactions.write();
        transactions.push(metrics);
        
        // Trim if needed
        if transactions.len() > self.max_transactions {
            transactions.remove(0);
        }
    }
    
    /// Record a transaction retry
    pub fn record_retry(&self, id: u64) {
        self.retried_transactions.fetch_add(1, Ordering::SeqCst);
        
        let mut transactions = self.transactions.write();
        
        for tx in transactions.iter_mut() {
            if tx.id == id {
                tx.retries += 1;
                break;
            }
        }
    }
    
    /// Record a successful transaction
    pub fn record_success(&self, duration: Duration) {
        self.successful_transactions.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Record a successful transaction with details
    pub fn record_success_with_details(&self, id: u64, fee_paid: Option<u64>) {
        self.successful_transactions.fetch_add(1, Ordering::SeqCst);
        
        let mut transactions = self.transactions.write();
        
        for tx in transactions.iter_mut() {
            if tx.id == id {
                tx.end_time = Some(Instant::now());
                tx.status = Some("success".to_string());
                tx.fee_paid = fee_paid;
                
                if let Some(fee) = fee_paid {
                    self.total_fees_paid.fetch_add(fee, Ordering::SeqCst);
                }
                
                if tx.used_jito {
                    self.successful_jito_bundles.fetch_add(1, Ordering::SeqCst);
                }
                
                break;
            }
        }
    }
    
    /// Record a failed transaction
    pub fn record_failure(&self, duration: Duration) {
        self.failed_transactions.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Record a failed transaction with details
    pub fn record_failure_with_details(&self, id: u64, error: &str, fee_paid: Option<u64>) {
        self.failed_transactions.fetch_add(1, Ordering::SeqCst);
        
        let mut transactions = self.transactions.write();
        
        for tx in transactions.iter_mut() {
            if tx.id == id {
                tx.end_time = Some(Instant::now());
                tx.status = Some("failure".to_string());
                tx.error = Some(error.to_string());
                tx.fee_paid = fee_paid;
                
                if let Some(fee) = fee_paid {
                    self.total_fees_paid.fetch_add(fee, Ordering::SeqCst);
                }
                
                break;
            }
        }
    }
    
    /// Record a timed out transaction
    pub fn record_timeout(&self) {
        self.timeout_transactions.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Record a timed out transaction with details
    pub fn record_timeout_with_details(&self, id: u64) {
        self.timeout_transactions.fetch_add(1, Ordering::SeqCst);
        
        let mut transactions = self.transactions.write();
        
        for tx in transactions.iter_mut() {
            if tx.id == id {
                tx.end_time = Some(Instant::now());
                tx.status = Some("timeout".to_string());
                tx.error = Some("Transaction timed out".to_string());
                break;
            }
        }
    }
    
    /// Record transaction confirmation
    pub fn record_confirmation(&self, id: u64, fee_paid: Option<u64>) {
        let mut transactions = self.transactions.write();
        
        for tx in transactions.iter_mut() {
            if tx.id == id {
                tx.confirmation_time = Some(Instant::now());
                
                if fee_paid.is_some() {
                    tx.fee_paid = fee_paid;
                    
                    if let Some(fee) = fee_paid {
                        // Update fee if it wasn't already recorded
                        if tx.fee_paid.is_none() {
                            self.total_fees_paid.fetch_add(fee, Ordering::SeqCst);
                        }
                    }
                }
                
                break;
            }
        }
    }
    
    /// Get a snapshot of the metrics
    pub fn snapshot(&self) -> ExecutionMetricsSnapshot {
        let transactions = self.transactions.read();
        
        // Calculate execution times
        let mut execution_times = Vec::new();
        let mut confirmation_times = Vec::new();
        let mut fees = Vec::new();
        let mut transactions_by_priority = HashMap::new();
        let mut success_by_priority: HashMap<String, u64> = HashMap::new();
        let mut total_by_priority: HashMap<String, u64> = HashMap::new();
        
        for tx in transactions.iter() {
            if let Some(end_time) = tx.end_time {
                let execution_time_ms = end_time.duration_since(tx.start_time).as_millis() as u64;
                execution_times.push(execution_time_ms);
                
                if let Some(fee) = tx.fee_paid {
                    fees.push(fee);
                }
                
                // Track by priority
                *total_by_priority.entry(tx.priority.clone()).or_insert(0) += 1;
                
                if let Some(status) = &tx.status {
                    if status == "success" {
                        *success_by_priority.entry(tx.priority.clone()).or_insert(0) += 1;
                    }
                }
                
                // Track confirmation times
                if let Some(confirmation_time) = tx.confirmation_time {
                    let confirmation_time_ms = confirmation_time.duration_since(tx.start_time).as_millis() as u64;
                    confirmation_times.push(confirmation_time_ms);
                }
            }
        }
        
        // Calculate transactions by priority
        for (priority, count) in total_by_priority.iter() {
            transactions_by_priority.insert(priority.clone(), *count);
        }
        
        // Calculate success rate by priority
        let mut success_rate_by_priority = HashMap::new();
        for (priority, total) in total_by_priority.iter() {
            let success = success_by_priority.get(priority).copied().unwrap_or(0);
            let rate = if *total > 0 {
                success as f64 / *total as f64
            } else {
                0.0
            };
            success_rate_by_priority.insert(priority.clone(), rate);
        }
        
        // Calculate average execution time
        let avg_execution_time_ms = if !execution_times.is_empty() {
            execution_times.iter().sum::<u64>() as f64 / execution_times.len() as f64
        } else {
            0.0
        };
        
        // Calculate percentiles
        let p95_execution_time_ms = Self::percentile(&execution_times, 95);
        let p99_execution_time_ms = Self::percentile(&execution_times, 99);
        
        // Calculate max execution time
        let max_execution_time_ms = execution_times.iter().max().copied().unwrap_or(0);
        
        // Calculate average confirmation time
        let avg_confirmation_time_ms = if !confirmation_times.is_empty() {
            confirmation_times.iter().sum::<u64>() as f64 / confirmation_times.len() as f64
        } else {
            0.0
        };
        
        // Calculate average fee
        let avg_fee_paid = if !fees.is_empty() {
            fees.iter().sum::<u64>() as f64 / fees.len() as f64
        } else {
            0.0
        };
        
        ExecutionMetricsSnapshot {
            total_transactions: self.total_transactions.load(Ordering::SeqCst),
            successful_transactions: self.successful_transactions.load(Ordering::SeqCst),
            failed_transactions: self.failed_transactions.load(Ordering::SeqCst),
            timeout_transactions: self.timeout_transactions.load(Ordering::SeqCst),
            retried_transactions: self.retried_transactions.load(Ordering::SeqCst),
            avg_execution_time_ms,
            p95_execution_time_ms,
            p99_execution_time_ms,
            max_execution_time_ms,
            avg_confirmation_time_ms,
            avg_fee_paid,
            total_fees_paid: self.total_fees_paid.load(Ordering::SeqCst),
            transactions_by_priority,
            success_rate_by_priority,
            jito_bundles: self.jito_bundles.load(Ordering::SeqCst),
            successful_jito_bundles: self.successful_jito_bundles.load(Ordering::SeqCst),
        }
    }
    
    /// Calculate a percentile value
    fn percentile(values: &[u64], percentile: usize) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        
        let mut sorted = values.to_vec();
        sorted.sort_unstable();
        
        let index = (percentile as f64 / 100.0 * sorted.len() as f64).ceil() as usize - 1;
        let index = index.min(sorted.len() - 1);
        
        sorted[index] as f64
    }
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self::new()
    }
}
