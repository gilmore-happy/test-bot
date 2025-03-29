#!/bin/bash
# Test script to verify that the code can be compiled and run with the full Jito features enabled

echo "Testing execution crate with full Jito features..."

# Change to the solana-hft-bot directory
cd "$(dirname "$0")"

# Build the execution crate with the Jito full feature
echo "Building execution crate with Jito full feature..."
cargo build --package solana-hft-execution --no-default-features --features="jito-full"

# Check if the build was successful
if [ $? -eq 0 ]; then
    echo -e "\033[0;32mBuild successful!\033[0m"
else
    echo -e "\033[0;31mBuild failed!\033[0m"
    exit 1
fi

# Run the memory pool test
echo "Running memory pool test..."
cargo run --package solana-hft-execution --no-default-features --features="jito-full" --bin memory_pool_test

# Check if the test was successful
if [ $? -eq 0 ]; then
    echo -e "\033[0;32mMemory pool test successful!\033[0m"
else
    echo -e "\033[0;31mMemory pool test failed!\033[0m"
    exit 1
fi

# Run the SIMD benchmarks
echo "Running SIMD benchmarks..."
cargo run --package solana-hft-execution --no-default-features --features="jito-full,simd" --bin simd_benchmark

# Check if the benchmarks were successful
if [ $? -eq 0 ]; then
    echo -e "\033[0;32mSIMD benchmarks successful!\033[0m"
else
    echo -e "\033[0;31mSIMD benchmarks failed!\033[0m"
    exit 1
fi

# Run the unit tests
echo "Running unit tests with Jito full feature..."
cargo test --package solana-hft-execution --no-default-features --features="jito-full"

# Check if the tests were successful
if [ $? -eq 0 ]; then
    echo -e "\033[0;32mUnit tests successful!\033[0m"
else
    echo -e "\033[0;31mUnit tests failed!\033[0m"
    exit 1
fi

# Run the SIMD tests
echo "Running SIMD tests..."
cargo test --package solana-hft-execution --no-default-features --features="jito-full,simd" -- --nocapture simd::tests

# Check if the SIMD tests were successful
if [ $? -eq 0 ]; then
    echo -e "\033[0;32mSIMD tests successful!\033[0m"
else
    echo -e "\033[0;31mSIMD tests failed!\033[0m"
    exit 1
fi

echo -e "\033[0;32mAll tests passed!\033[0m"