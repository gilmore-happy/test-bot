//! Transaction retry logic
//! 
//! This module provides retry logic for transaction execution.

use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, info, trace, warn};

use crate::ExecutionError;

/// Retry strategy for transaction execution
#[derive(Debug, Clone)]
pub enum RetryStrategy {
    /// Fixed interval retry
    Fixed {
        /// Retry interval
        interval: Duration,
        
        /// Maximum number of retries
        max_retries: usize,
    },
    
    /// Exponential backoff retry
    ExponentialBackoff {
        /// Initial retry interval
        initial_interval: Duration,
        
        /// Maximum retry interval
        max_interval: Duration,
        
        /// Backoff factor
        factor: f64,
        
        /// Maximum number of retries
        max_retries: usize,
    },
    
    /// Fibonacci backoff retry
    FibonacciBackoff {
        /// Initial retry interval
        initial_interval: Duration,
        
        /// Maximum retry interval
        max_interval: Duration,
        
        /// Maximum number of retries
        max_retries: usize,
    },
    
    /// No retry
    NoRetry,
}

impl RetryStrategy {
    /// Create a new fixed interval retry strategy
    pub fn fixed(interval: Duration, max_retries: usize) -> Self {
        RetryStrategy::Fixed {
            interval,
            max_retries,
        }
    }
    
    /// Create a new exponential backoff retry strategy
    pub fn exponential_backoff(
        initial_interval: Duration,
        max_interval: Duration,
        factor: f64,
        max_retries: usize,
    ) -> Self {
        RetryStrategy::ExponentialBackoff {
            initial_interval,
            max_interval,
            factor,
            max_retries,
        }
    }
    
    /// Create a new Fibonacci backoff retry strategy
    pub fn fibonacci_backoff(
        initial_interval: Duration,
        max_interval: Duration,
        max_retries: usize,
    ) -> Self {
        RetryStrategy::FibonacciBackoff {
            initial_interval,
            max_interval,
            max_retries,
        }
    }
    
    /// Create a new no retry strategy
    pub fn no_retry() -> Self {
        RetryStrategy::NoRetry
    }
    
    /// Get the maximum number of retries
    pub fn max_retries(&self) -> usize {
        match self {
            RetryStrategy::Fixed { max_retries, .. } => *max_retries,
            RetryStrategy::ExponentialBackoff { max_retries, .. } => *max_retries,
            RetryStrategy::FibonacciBackoff { max_retries, .. } => *max_retries,
            RetryStrategy::NoRetry => 0,
        }
    }
    
    /// Get the retry interval for a specific retry attempt
    pub fn get_retry_interval(&self, attempt: usize) -> Duration {
        match self {
            RetryStrategy::Fixed { interval, .. } => *interval,
            RetryStrategy::ExponentialBackoff {
                initial_interval,
                max_interval,
                factor,
                ..
            } => {
                let interval = initial_interval.as_millis() as f64 * factor.powf(attempt as f64);
                let interval = interval.min(max_interval.as_millis() as f64);
                Duration::from_millis(interval as u64)
            },
            RetryStrategy::FibonacciBackoff {
                initial_interval,
                max_interval,
                ..
            } => {
                let interval = initial_interval.as_millis() as u64 * Self::fibonacci(attempt + 1);
                Duration::from_millis(interval.min(max_interval.as_millis() as u64))
            },
            RetryStrategy::NoRetry => Duration::from_secs(0),
        }
    }
    
    /// Calculate the nth Fibonacci number
    fn fibonacci(n: usize) -> u64 {
        if n <= 1 {
            return 1;
        }
        
        let mut a = 1;
        let mut b = 1;
        
        for _ in 2..=n {
            let c = a + b;
            a = b;
            b = c;
        }
        
        b
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        RetryStrategy::exponential_backoff(
            Duration::from_millis(100),
            Duration::from_secs(5),
            2.0,
            3,
        )
    }
}

/// Retry a function with the specified retry strategy
pub async fn retry<F, Fut, T>(
    operation: F,
    strategy: RetryStrategy,
    is_retriable: fn(&ExecutionError) -> bool,
) -> Result<T, ExecutionError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ExecutionError>>,
{
    let max_retries = strategy.max_retries();
    let mut attempt = 0;
    let start = Instant::now();
    
    loop {
        let result = operation().await;
        
        match result {
            Ok(value) => {
                if attempt > 0 {
                    debug!("Operation succeeded after {} retries in {:?}", attempt, start.elapsed());
                }
                return Ok(value);
            },
            Err(err) => {
                if attempt >= max_retries || !is_retriable(&err) {
                    return Err(err);
                }
                
                attempt += 1;
                let retry_interval = strategy.get_retry_interval(attempt);
                
                warn!(
                    "Operation failed (attempt {}/{}), retrying in {:?}: {}",
                    attempt,
                    max_retries + 1,
                    retry_interval,
                    err
                );
                
                sleep(retry_interval).await;
            },
        }
    }
}

/// Check if an error is retriable
pub fn is_retriable_error(err: &ExecutionError) -> bool {
    match err {
        ExecutionError::Transaction(msg) => {
            // Check for retriable transaction errors
            msg.contains("blockhash not found") ||
            msg.contains("block height exceeded") ||
            msg.contains("insufficient funds") ||
            msg.contains("would exceed max vote cost") ||
            msg.contains("vote transaction not processed") ||
            msg.contains("transaction signature verification failure")
        },
        ExecutionError::Rpc(msg) => {
            // Check for retriable RPC errors
            msg.contains("429 Too Many Requests") ||
            msg.contains("503 Service Unavailable") ||
            msg.contains("504 Gateway Timeout") ||
            msg.contains("Connection reset by peer") ||
            msg.contains("connection closed before message completed")
        },
        ExecutionError::Timeout => true,
        _ => false,
    }
}

/// Retry a transaction with the specified retry strategy
pub async fn retry_transaction<F, Fut>(
    operation: F,
    strategy: RetryStrategy,
) -> Result<solana_sdk::signature::Signature, ExecutionError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<solana_sdk::signature::Signature, ExecutionError>>,
{
    retry(operation, strategy, is_retriable_error).await
}
