//! SIMD Benchmarks for Solana HFT Bot
//!
//! This module provides benchmarks for SIMD-optimized operations to measure
//! performance improvements compared to standard implementations.

use std::time::{Duration, Instant};
use rand::{Rng, thread_rng};

use crate::simd::buffer_ops::{copy_buffer_simd, clear_buffer_simd};
use crate::simd::serialization::{serialize_transaction_simd, deserialize_transaction_simd};
use crate::simd::signature_ops::verify_signatures_simd;
use crate::simd::hash_ops::calculate_hash_simd;
use crate::simd::CpuFeatures;

/// Buffer size for benchmarks
const BUFFER_SIZE: usize = 1024 * 1024; // 1MB

/// Number of iterations for each benchmark
const ITERATIONS: usize = 100;

/// Number of transactions for batch benchmarks
const TRANSACTION_COUNT: usize = 1000;

/// Benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Operation name
    pub operation: String,
    
    /// SIMD implementation time
    pub simd_time: Duration,
    
    /// Standard implementation time
    pub standard_time: Duration,
    
    /// Speedup factor (standard / SIMD)
    pub speedup: f64,
}

/// Run all benchmarks
pub fn run_all_benchmarks() -> Vec<BenchmarkResult> {
    println!("Running SIMD benchmarks...");
    
    // Detect CPU features
    let features = CpuFeatures::detect();
    println!("Detected CPU features: {:?}", features);
    println!("Best SIMD version: {:?}", features.best_simd_version());
    
    let mut results = Vec::new();
    
    // Buffer copy benchmark
    results.push(benchmark_buffer_copy());
    
    // Buffer clear benchmark
    results.push(benchmark_buffer_clear());
    
    // Transaction serialization benchmark
    results.push(benchmark_transaction_serialization());
    
    // Transaction deserialization benchmark
    results.push(benchmark_transaction_deserialization());
    
    // Signature verification benchmark
    results.push(benchmark_signature_verification());
    
    // Hash calculation benchmark
    results.push(benchmark_hash_calculation());
    
    // Print results
    println!("\nBenchmark Results:");
    println!("{:-^80}", "");
    println!("{:<30} | {:>12} | {:>12} | {:>12}", "Operation", "SIMD (ms)", "Standard (ms)", "Speedup");
    println!("{:-^80}", "");
    
    for result in &results {
        println!(
            "{:<30} | {:>12.2} | {:>12.2} | {:>12.2}x",
            result.operation,
            result.simd_time.as_secs_f64() * 1000.0,
            result.standard_time.as_secs_f64() * 1000.0,
            result.speedup
        );
    }
    
    println!("{:-^80}", "");
    
    results
}

/// Benchmark buffer copy operations
fn benchmark_buffer_copy() -> BenchmarkResult {
    // Create random source buffer
    let mut rng = thread_rng();
    let src: Vec<u8> = (0..BUFFER_SIZE).map(|_| rng.gen()).collect();
    let mut dst1 = vec![0u8; BUFFER_SIZE];
    let mut dst2 = vec![0u8; BUFFER_SIZE];
    
    // Benchmark SIMD implementation
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        copy_buffer_simd(&src, &mut dst1);
    }
    let simd_time = start.elapsed() / ITERATIONS as u32;
    
    // Benchmark standard implementation
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        dst2.copy_from_slice(&src);
    }
    let standard_time = start.elapsed() / ITERATIONS as u32;
    
    // Calculate speedup
    let speedup = standard_time.as_secs_f64() / simd_time.as_secs_f64();
    
    BenchmarkResult {
        operation: "Buffer Copy".to_string(),
        simd_time,
        standard_time,
        speedup,
    }
}

/// Benchmark buffer clear operations
fn benchmark_buffer_clear() -> BenchmarkResult {
    // Create buffers
    let mut buffer1 = vec![0xFFu8; BUFFER_SIZE];
    let mut buffer2 = vec![0xFFu8; BUFFER_SIZE];
    
    // Benchmark SIMD implementation
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        clear_buffer_simd(&mut buffer1);
    }
    let simd_time = start.elapsed() / ITERATIONS as u32;
    
    // Benchmark standard implementation
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        buffer2.fill(0);
    }
    let standard_time = start.elapsed() / ITERATIONS as u32;
    
    // Calculate speedup
    let speedup = standard_time.as_secs_f64() / simd_time.as_secs_f64();
    
    BenchmarkResult {
        operation: "Buffer Clear".to_string(),
        simd_time,
        standard_time,
        speedup,
    }
}

/// Benchmark transaction serialization
fn benchmark_transaction_serialization() -> BenchmarkResult {
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
        message::Message,
        pubkey::Pubkey,
        system_instruction,
    };
    
    // Create sample transactions
    let mut rng = thread_rng();
    let from_keypair = Keypair::new();
    let to_pubkey = Pubkey::new_unique();
    
    let transactions: Vec<_> = (0..TRANSACTION_COUNT)
        .map(|_| {
            let amount = rng.gen_range(1..1000);
            let instruction = system_instruction::transfer(&from_keypair.pubkey(), &to_pubkey, amount);
            let message = Message::new(&[instruction], Some(&from_keypair.pubkey()));
            Transaction::new(&[&from_keypair], message, rng.gen())
        })
        .collect();
    
    let mut buffer = Vec::with_capacity(1024);
    
    // Benchmark SIMD implementation
    let start = Instant::now();
    for tx in &transactions {
        serialize_transaction_simd(tx, &mut buffer);
    }
    let simd_time = start.elapsed() / TRANSACTION_COUNT as u32;
    
    // Benchmark standard implementation
    let start = Instant::now();
    for tx in &transactions {
        buffer.clear();
        bincode::serialize_into(&mut buffer, tx).unwrap();
    }
    let standard_time = start.elapsed() / TRANSACTION_COUNT as u32;
    
    // Calculate speedup
    let speedup = standard_time.as_secs_f64() / simd_time.as_secs_f64();
    
    BenchmarkResult {
        operation: "Transaction Serialization".to_string(),
        simd_time,
        standard_time,
        speedup,
    }
}

/// Benchmark transaction deserialization
fn benchmark_transaction_deserialization() -> BenchmarkResult {
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
        message::Message,
        pubkey::Pubkey,
        system_instruction,
    };
    
    // Create sample transactions
    let mut rng = thread_rng();
    let from_keypair = Keypair::new();
    let to_pubkey = Pubkey::new_unique();
    
    let transaction = {
        let amount = rng.gen_range(1..1000);
        let instruction = system_instruction::transfer(&from_keypair.pubkey(), &to_pubkey, amount);
        let message = Message::new(&[instruction], Some(&from_keypair.pubkey()));
        Transaction::new(&[&from_keypair], message, rng.gen())
    };
    
    // Serialize transaction
    let serialized = bincode::serialize(&transaction).unwrap();
    
    // Benchmark SIMD implementation
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = deserialize_transaction_simd(&serialized).unwrap();
    }
    let simd_time = start.elapsed() / ITERATIONS as u32;
    
    // Benchmark standard implementation
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _: Transaction = bincode::deserialize(&serialized).unwrap();
    }
    let standard_time = start.elapsed() / ITERATIONS as u32;
    
    // Calculate speedup
    let speedup = standard_time.as_secs_f64() / simd_time.as_secs_f64();
    
    BenchmarkResult {
        operation: "Transaction Deserialization".to_string(),
        simd_time,
        standard_time,
        speedup,
    }
}

/// Benchmark signature verification
fn benchmark_signature_verification() -> BenchmarkResult {
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
        message::Message,
        pubkey::Pubkey,
        system_instruction,
    };
    
    // Create sample transactions
    let mut rng = thread_rng();
    let from_keypair = Keypair::new();
    let to_pubkey = Pubkey::new_unique();
    
    let transactions: Vec<_> = (0..TRANSACTION_COUNT / 10) // Use fewer transactions for this benchmark
        .map(|_| {
            let amount = rng.gen_range(1..1000);
            let instruction = system_instruction::transfer(&from_keypair.pubkey(), &to_pubkey, amount);
            let message = Message::new(&[instruction], Some(&from_keypair.pubkey()));
            Transaction::new(&[&from_keypair], message, rng.gen())
        })
        .collect();
    
    // Benchmark SIMD implementation
    let start = Instant::now();
    let _ = verify_signatures_simd(&transactions);
    let simd_time = start.elapsed();
    
    // Benchmark standard implementation
    let start = Instant::now();
    let _ = transactions
        .iter()
        .map(|tx| tx.verify_with_results().iter().all(|&result| result))
        .collect::<Vec<_>>();
    let standard_time = start.elapsed();
    
    // Calculate speedup
    let speedup = standard_time.as_secs_f64() / simd_time.as_secs_f64();
    
    BenchmarkResult {
        operation: "Signature Verification".to_string(),
        simd_time,
        standard_time,
        speedup,
    }
}

/// Benchmark hash calculation
fn benchmark_hash_calculation() -> BenchmarkResult {
    // Create random data
    let mut rng = thread_rng();
    let data: Vec<u8> = (0..BUFFER_SIZE).map(|_| rng.gen()).collect();
    
    // Benchmark SIMD implementation
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = calculate_hash_simd(&data);
    }
    let simd_time = start.elapsed() / ITERATIONS as u32;
    
    // Benchmark standard implementation
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = solana_sdk::hash::hash(&data);
    }
    let standard_time = start.elapsed() / ITERATIONS as u32;
    
    // Calculate speedup
    let speedup = standard_time.as_secs_f64() / simd_time.as_secs_f64();
    
    BenchmarkResult {
        operation: "Hash Calculation".to_string(),
        simd_time,
        standard_time,
        speedup,
    }
}

/// Main function to run benchmarks
pub fn main() {
    run_all_benchmarks();
}