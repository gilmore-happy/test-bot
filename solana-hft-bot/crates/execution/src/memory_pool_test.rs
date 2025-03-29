//! Standalone test for the optimized TransactionMemoryPool implementation
//! 
//! This file provides a standalone test for the memory pool implementation
//! that can be run without the Jito feature enabled.

use std::sync::Arc;
use crossbeam::queue::ArrayQueue;
use std::thread;
use std::time::{Duration, Instant};

/// Optimized memory pool for transaction structures
pub struct OptimizedTransactionMemoryPool {
    // Use a lock-free queue for high concurrency without mutex overhead
    buffers: Arc<ArrayQueue<Vec<u8>>>,
    buffer_size: usize,
    pre_allocation_size: usize,
}

impl OptimizedTransactionMemoryPool {
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
        buffer.clear();
        
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_pool_basic() {
        let pool = OptimizedTransactionMemoryPool::new(1024, 10);
        
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
        let pool = OptimizedTransactionMemoryPool::new(1024, 10);
        
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
        let pool = Arc::new(OptimizedTransactionMemoryPool::new(1024, 100));
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
        let pool = OptimizedTransactionMemoryPool::new(1024, 1000);
        
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
}

/// Main function to run the tests directly
fn main() {
    println!("Running memory pool tests...");
    
    // Run tests directly
    tests::test_memory_pool_basic();
    tests::test_memory_pool_exhaustion();
    tests::test_memory_pool_concurrent();
    tests::test_memory_pool_performance();
    
    println!("All tests passed!");
}