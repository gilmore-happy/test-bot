#!/bin/bash
# Test script to verify that the code can be compiled and run with the default features

echo "Testing execution crate with default features..."

# Change to the solana-hft-bot directory
cd "$(dirname "$0")"

# Build the execution crate with default features
echo "Building execution crate with default features..."
cargo build --package solana-hft-execution

# Check if the build was successful
if [ $? -eq 0 ]; then
    echo -e "\033[0;32mBuild successful!\033[0m"
else
    echo -e "\033[0;31mBuild failed!\033[0m"
    exit 1
fi

# Run the memory pool test
echo "Running memory pool test..."
cargo run --package solana-hft-execution --bin memory_pool_test

# Check if the test was successful
if [ $? -eq 0 ]; then
    echo -e "\033[0;32mMemory pool test successful!\033[0m"
else
    echo -e "\033[0;31mMemory pool test failed!\033[0m"
    exit 1
fi

# Run the unit tests
echo "Running unit tests with default features..."
cargo test --package solana-hft-execution

# Check if the tests were successful
if [ $? -eq 0 ]; then
    echo -e "\033[0;32mUnit tests successful!\033[0m"
else
    echo -e "\033[0;31mUnit tests failed!\033[0m"
    exit 1
fi

echo -e "\033[0;32mAll tests passed!\033[0m"