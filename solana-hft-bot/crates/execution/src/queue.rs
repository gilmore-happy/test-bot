//! Lock-free, MPMC transaction queue with priority levels
//! 
//! This module provides a high-performance, lock-free, multi-producer multi-consumer
//! transaction queue with priority levels for the execution engine.

use crossbeam_queue::{ArrayQueue, SegQueue};
use crossbeam_utils::atomic::AtomicCell;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::mem;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use parking_lot::RwLock;
use tracing::{debug, error, info, trace, warn};

use crate::{ExecutionRequest, PriorityLevel, ExecutionError};
#[cfg(feature = "simd")]
use crate::simd::buffer_ops;

/// Memory pool for transaction structures optimized for HFT performance
pub struct TransactionMemoryPool {
    // Use a lock-free queue for high concurrency without mutex overhead
    buffers: Arc<ArrayQueue<Vec<u8>>>,
    buffer_size: usize,
    pre_allocation_size: usize,
}

impl TransactionMemoryPool {
    pub fn new(buffer_size: usize, pool_capacity: usize) -> Self {
        // Create a fixed-size pool with pre-allocated buffers
        let buffers = Arc::new(ArrayQueue::new(pool_capacity));
        
        // Pre-allocate all buffers upfront to avoid allocation during trading
        for _ in 0..pool_capacity {
            let buffer = Vec::with_capacity(buffer_size);
            buffers.push(buffer).ok(); // Queue is sized exactly to fit all buffers
        }
        
        Self {
            buffers,
            buffer_size,
            pre_allocation_size: pool_capacity,
        }
    }
    
    #[inline]
    pub fn get_buffer(&self) -> Vec<u8> {
        // Fast path - try to get from pool first
        match self.buffers.pop() {
            Some(buffer) => buffer,
            // If pool is exhausted, create a new buffer but don't track it
            // This handles burst scenarios without adding complexity
            None => Vec::with_capacity(self.buffer_size),
        }
    }
    
    #[inline]
    pub fn return_buffer(&self, mut buffer: Vec<u8>) {
        // Clear but preserve capacity
        #[cfg(feature = "simd")]
        {
            buffer_ops::clear_buffer_simd(&mut buffer);
        }
        #[cfg(not(feature = "simd"))]
        {
            buffer.clear();
        }
        
        // Only attempt to return to the pool, don't block if full
        // This is a "fire and forget" approach for simplicity and performance
        let _ = self.buffers.push(buffer);
    }
    
    /// Get the number of buffers in the pool
    pub fn available_buffers(&self) -> usize {
        self.buffers.len()
    }
    
    /// Get the total number of allocated buffers
    pub fn total_allocated(&self) -> u64 {
        self.pre_allocation_size as u64
    }
}

/// Transaction with zero-copy serialization
pub struct ZeroCopyTransaction {
    /// Transaction data buffer
    buffer: Vec<u8>,
    
    /// Transaction size
    size: usize,
    
    /// Memory pool reference
    pool: Arc<TransactionMemoryPool>,
}

impl ZeroCopyTransaction {
    /// Create a new zero-copy transaction
    pub fn new(transaction: &solana_sdk::transaction::Transaction, pool: Arc<TransactionMemoryPool>) -> Self {
        let mut buffer = pool.get_buffer();
        
        // Serialize the transaction into the buffer
        let size = Self::serialize_transaction(transaction, &mut buffer);
        
        Self {
            buffer,
            size,
            pool,
        }
    }
    
    /// Serialize a transaction into a buffer
    fn serialize_transaction(transaction: &solana_sdk::transaction::Transaction, buffer: &mut Vec<u8>) -> usize {
        // Use bincode for efficient serialization
        buffer.clear();
        match bincode::serialize_into(&mut *buffer, transaction) {
            Ok(_) => buffer.len(),
            Err(e) => {
                error!("Failed to serialize transaction: {}", e);
                0
            }
        }
    }
    
    /// Deserialize the transaction
    pub fn deserialize(&self) -> Result<solana_sdk::transaction::Transaction, ExecutionError> {
        bincode::deserialize(&self.buffer[..self.size])
            .map_err(|e| ExecutionError::Internal(format!("Failed to deserialize transaction: {}", e)))
    }
    
    /// Get a reference to the serialized data
    pub fn data(&self) -> &[u8] {
        &self.buffer[..self.size]
    }
    
    /// Get the size of the serialized transaction
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for ZeroCopyTransaction {
    fn drop(&mut self) {
        // Return the buffer to the pool when the transaction is dropped
        let buffer = mem::replace(&mut self.buffer, Vec::new());
        self.pool.return_buffer(buffer);
    }
}

/// Speculative execution candidate
pub struct SpeculativeCandidate {
    /// Transaction request
    pub request: ExecutionRequest,
    
    /// Estimated profit
    pub estimated_profit: u64,
    
    /// Probability of success
    pub success_probability: f64,
    
    /// Expiration time
    pub expires_at: Instant,
}

/// Priority queue for transaction execution
pub struct PriorityQueue {
    /// Queues for each priority level
    queues: [SegQueue<ExecutionRequest>; 4],
    
    /// Speculative execution candidates
    speculative_candidates: RwLock<Vec<SpeculativeCandidate>>,
    
    /// Memory pool for transaction structures
    memory_pool: Arc<TransactionMemoryPool>,
    
    /// Whether speculative execution is enabled
    speculative_execution_enabled: AtomicBool,
    
    /// Metrics
    metrics: PriorityQueueMetrics,
}

/// Priority queue metrics
#[derive(Debug, Default)]
pub struct PriorityQueueMetrics {
    /// Total enqueued transactions
    pub total_enqueued: AtomicU64,
    
    /// Total dequeued transactions
    pub total_dequeued: AtomicU64,
    
    /// Transactions enqueued by priority
    pub enqueued_by_priority: [AtomicU64; 4],
    
    /// Transactions dequeued by priority
    pub dequeued_by_priority: [AtomicU64; 4],
    
    /// Speculative transactions executed
    pub speculative_executed: AtomicU64,
    
    /// Speculative transactions succeeded
    pub speculative_succeeded: AtomicU64,
    
    /// Memory pool utilization
    pub memory_pool_utilization: AtomicCell<f64>,
}

impl PriorityQueueMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self {
            total_enqueued: AtomicU64::new(0),
            total_dequeued: AtomicU64::new(0),
            enqueued_by_priority: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
            dequeued_by_priority: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
            speculative_executed: AtomicU64::new(0),
            speculative_succeeded: AtomicU64::new(0),
            memory_pool_utilization: AtomicCell::new(0.0),
        }
    }
    
    /// Get a snapshot of the metrics
    pub fn snapshot(&self) -> PriorityQueueMetricsSnapshot {
        PriorityQueueMetricsSnapshot {
            total_enqueued: self.total_enqueued.load(Ordering::Relaxed),
            total_dequeued: self.total_dequeued.load(Ordering::Relaxed),
            enqueued_by_priority: [
                self.enqueued_by_priority[0].load(Ordering::Relaxed),
                self.enqueued_by_priority[1].load(Ordering::Relaxed),
                self.enqueued_by_priority[2].load(Ordering::Relaxed),
                self.enqueued_by_priority[3].load(Ordering::Relaxed),
            ],
            dequeued_by_priority: [
                self.dequeued_by_priority[0].load(Ordering::Relaxed),
                self.dequeued_by_priority[1].load(Ordering::Relaxed),
                self.dequeued_by_priority[2].load(Ordering::Relaxed),
                self.dequeued_by_priority[3].load(Ordering::Relaxed),
            ],
            speculative_executed: self.speculative_executed.load(Ordering::Relaxed),
            speculative_succeeded: self.speculative_succeeded.load(Ordering::Relaxed),
            memory_pool_utilization: self.memory_pool_utilization.load(),
        }
    }
}

/// Priority queue metrics snapshot
#[derive(Debug, Clone)]
pub struct PriorityQueueMetricsSnapshot {
    /// Total enqueued transactions
    pub total_enqueued: u64,
    
    /// Total dequeued transactions
    pub total_dequeued: u64,
    
    /// Transactions enqueued by priority
    pub enqueued_by_priority: [u64; 4],
    
    /// Transactions dequeued by priority
    pub dequeued_by_priority: [u64; 4],
    
    /// Speculative transactions executed
    pub speculative_executed: u64,
    
    /// Speculative transactions succeeded
    pub speculative_succeeded: u64,
    
    /// Memory pool utilization
    pub memory_pool_utilization: f64,
}

impl PriorityQueue {
    /// Create a new priority queue
    pub fn new(buffer_size: usize, pool_capacity: usize) -> Self {
        let memory_pool = Arc::new(TransactionMemoryPool::new(buffer_size, pool_capacity));
        
        Self {
            queues: [
                SegQueue::new(), // Critical
                SegQueue::new(), // High
                SegQueue::new(), // Medium
                SegQueue::new(), // Low
            ],
            speculative_candidates: RwLock::new(Vec::new()),
            memory_pool,
            speculative_execution_enabled: AtomicBool::new(true),
            metrics: PriorityQueueMetrics::new(),
        }
    }
    
    /// Enqueue a transaction request
    pub fn enqueue(&self, request: ExecutionRequest) {
        let priority_idx = request.priority.queue_position() as usize;
        
        // Update metrics
        self.metrics.total_enqueued.fetch_add(1, Ordering::Relaxed);
        self.metrics.enqueued_by_priority[priority_idx].fetch_add(1, Ordering::Relaxed);
        
        // Add to the appropriate queue
        self.queues[priority_idx].push(request);
    }
    
    /// Dequeue a transaction request
    pub fn dequeue(&self) -> Option<ExecutionRequest> {
        // First check for speculative candidates if enabled
        if self.speculative_execution_enabled.load(Ordering::Relaxed) {
            if let Some(candidate) = self.get_best_speculative_candidate() {
                self.metrics.speculative_executed.fetch_add(1, Ordering::Relaxed);
                self.metrics.total_dequeued.fetch_add(1, Ordering::Relaxed);
                return Some(candidate.request);
            }
        }
        
        // Then check each priority queue in order
        for (i, queue) in self.queues.iter().enumerate() {
            if let Some(request) = queue.pop() {
                // Update metrics
                self.metrics.total_dequeued.fetch_add(1, Ordering::Relaxed);
                self.metrics.dequeued_by_priority[i].fetch_add(1, Ordering::Relaxed);
                
                return Some(request);
            }
        }
        
        None
    }
    
    /// Add a speculative execution candidate
    pub fn add_speculative_candidate(
        &self,
        request: ExecutionRequest,
        estimated_profit: u64,
        success_probability: f64,
        ttl: Duration,
    ) {
        let candidate = SpeculativeCandidate {
            request,
            estimated_profit,
            success_probability,
            expires_at: Instant::now() + ttl,
        };
        
        let mut candidates = self.speculative_candidates.write();
        candidates.push(candidate);
        
        // Sort candidates by expected value (profit * probability)
        candidates.sort_by(|a, b| {
            let a_value = (a.estimated_profit as f64 * a.success_probability) as u64;
            let b_value = (b.estimated_profit as f64 * b.success_probability) as u64;
            b_value.cmp(&a_value) // Sort in descending order
        });
        
        // Prune expired candidates
        let now = Instant::now();
        candidates.retain(|c| c.expires_at > now);
    }
    
    /// Get the best speculative candidate
    fn get_best_speculative_candidate(&self) -> Option<SpeculativeCandidate> {
        let mut candidates = self.speculative_candidates.write();
        
        // Prune expired candidates
        let now = Instant::now();
        candidates.retain(|c| c.expires_at > now);
        
        // Get the best candidate
        if candidates.is_empty() {
            None
        } else {
            // Remove and return the best candidate
            Some(candidates.remove(0))
        }
    }
    
    /// Record a successful speculative execution
    pub fn record_speculative_success(&self) {
        self.metrics.speculative_succeeded.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Enable or disable speculative execution
    pub fn set_speculative_execution(&self, enabled: bool) {
        self.speculative_execution_enabled.store(enabled, Ordering::Relaxed);
    }
    
    /// Get the memory pool
    pub fn memory_pool(&self) -> Arc<TransactionMemoryPool> {
        self.memory_pool.clone()
    }
    
    /// Get queue lengths
    pub fn queue_lengths(&self) -> [usize; 4] {
        [
            self.queues[0].len(),
            self.queues[1].len(),
            self.queues[2].len(),
            self.queues[3].len(),
        ]
    }
    
    /// Get the number of speculative candidates
    pub fn speculative_candidate_count(&self) -> usize {
        self.speculative_candidates.read().len()
    }
    
    /// Update metrics
    pub fn update_metrics(&self) {
        // Update memory pool utilization
        let available = self.memory_pool.available_buffers() as f64;
        let total = self.memory_pool.total_allocated() as f64;
        
        if total > 0.0 {
            let utilization = 1.0 - (available / total);
            self.metrics.memory_pool_utilization.store(utilization);
        }
    }
    
    /// Get metrics
    pub fn metrics(&self) -> PriorityQueueMetricsSnapshot {
        self.update_metrics();
        self.metrics.snapshot()
    }
}

/// Default implementation
impl Default for PriorityQueue {
    fn default() -> Self {
        // Default to 8KB buffers with a pool capacity of 1024 buffers (8MB total)
        Self::new(8 * 1024, 1024)
    }
}

#[cfg(test)]
#[cfg(not(feature = "jito"))]
mod tests {
    use super::*;
    use crate::ExecutionRequest;
    use solana_sdk::signature::Keypair;
    use solana_sdk::transaction::Transaction;
    use solana_sdk::message::Message;
    use solana_sdk::commitment_config::CommitmentConfig;
    use tokio::sync::oneshot;
    use std::thread;
    use std::time::{Duration, Instant};
    
    #[test]
    fn test_memory_pool_basic() {
        let pool = TransactionMemoryPool::new(1024, 10);
        
        // Get a buffer
        let buffer = pool.get_buffer();
        assert_eq!(buffer.capacity(), 1024);
        assert_eq!(buffer.len(), 0);
        
        // Return the buffer
        pool.return_buffer(buffer);
        
        // Get another buffer (should be the same one)
        let buffer2 = pool.get_buffer();
        assert_eq!(buffer2.capacity(), 1024);
        assert_eq!(buffer2.len(), 0);
    }
    
    #[test]
    fn test_memory_pool_exhaustion() {
        let pool = TransactionMemoryPool::new(1024, 10);
        
        // Get more buffers than the pool capacity
        let mut buffers = Vec::new();
        for _ in 0..15 {
            buffers.push(pool.get_buffer());
        }
        
        // Verify all buffers have correct capacity
        for buffer in &buffers {
            assert_eq!(buffer.capacity(), 1024);
        }
        
        // Return all buffers
        for buffer in buffers {
            pool.return_buffer(buffer);
        }
        
        // Verify pool recovered
        assert!(pool.available_buffers() > 0, "Pool should have available buffers after stress test");
        assert!(pool.available_buffers() <= 10, "Pool should not exceed its capacity");
    }
    
    #[test]
    fn test_memory_pool_concurrent() {
        let pool = Arc::new(TransactionMemoryPool::new(1024, 100));
        let thread_count = 10;
        let ops_per_thread = 1000;
        
        let mut handles = Vec::new();
        
        // Spawn threads that get and return buffers concurrently
        for _ in 0..thread_count {
            let pool_clone = pool.clone();
            let handle = thread::spawn(move || {
                for _ in 0..ops_per_thread {
                    let buffer = pool_clone.get_buffer();
                    // Simulate some work
                    thread::sleep(Duration::from_nanos(10));
                    pool_clone.return_buffer(buffer);
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Verify pool is still functional
        let buffer = pool.get_buffer();
        assert_eq!(buffer.capacity(), 1024);
        pool.return_buffer(buffer);
    }
    
    #[test]
    fn test_memory_pool_performance() {
        let pool = TransactionMemoryPool::new(1024, 1000);
        
        // Measure time to get and return buffers
        let iterations = 10000;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let buffer = pool.get_buffer();
            pool.return_buffer(buffer);
        }
        
        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
        
        println!("Memory pool performance: {:.2} ops/sec", ops_per_sec);
        
        // This is not a strict assertion as performance depends on the system,
        // but we expect reasonable performance
        assert!(ops_per_sec > 10000.0, "Memory pool performance is too low");
    }
    
    #[test]
    fn test_priority_queue() {
        let queue = PriorityQueue::default();
        
        // Create some dummy requests
        let (tx1, _rx1) = oneshot::channel();
        let (tx2, _rx2) = oneshot::channel();
        let (tx3, _rx3) = oneshot::channel();
        
        let request1 = ExecutionRequest {
            id: 1,
            transaction: Transaction::new_with_payer(&[], None),
            priority: PriorityLevel::Critical,
            use_jito: false,
            max_fee: None,
            timeout: Duration::from_secs(1),
            commitment: CommitmentConfig::default(),
            response_tx: tx1,
        };
        
        let request2 = ExecutionRequest {
            id: 2,
            transaction: Transaction::new_with_payer(&[], None),
            priority: PriorityLevel::Medium,
            use_jito: false,
            max_fee: None,
            timeout: Duration::from_secs(1),
            commitment: CommitmentConfig::default(),
            response_tx: tx2,
        };
        
        let request3 = ExecutionRequest {
            id: 3,
            transaction: Transaction::new_with_payer(&[], None),
            priority: PriorityLevel::Low,
            use_jito: false,
            max_fee: None,
            timeout: Duration::from_secs(1),
            commitment: CommitmentConfig::default(),
            response_tx: tx3,
        };
        
        // Enqueue in reverse priority order
        queue.enqueue(request3);
        queue.enqueue(request2);
        queue.enqueue(request1);
        
        // Dequeue should return in priority order
        let dequeued1 = queue.dequeue().unwrap();
        let dequeued2 = queue.dequeue().unwrap();
        let dequeued3 = queue.dequeue().unwrap();
        
        assert_eq!(dequeued1.id, 1); // Critical
        assert_eq!(dequeued2.id, 2); // Medium
        assert_eq!(dequeued3.id, 3); // Low
    }
}