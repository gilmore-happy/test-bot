#!/bin/bash
# Script to run SIMD benchmarks for the Solana HFT Bot

echo "Building SIMD benchmark..."
cargo build --release --package solana-hft-execution --bin simd_benchmark

echo "Running SIMD benchmarks..."
./target/release/simd_benchmark

echo "Benchmarks complete!"