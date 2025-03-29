#!/bin/bash
# Installation script for Solana HFT Bot

set -e

echo "Installing Solana HFT Bot..."

# Check if Rust is installed
if ! command -v rustc &> /dev/null; then
    echo "Rust is not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# Check Rust version
RUST_VERSION=$(rustc --version | cut -d ' ' -f 2)
REQUIRED_VERSION="1.70.0"

if [ "$(printf '%s\n' "$REQUIRED_VERSION" "$RUST_VERSION" | sort -V | head -n1)" != "$REQUIRED_VERSION" ]; then
    echo "Rust version $RUST_VERSION is too old. Updating Rust..."
    rustup update
fi

# Install nightly for some features
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

# Check if Solana CLI is installed
if ! command -v solana &> /dev/null; then
    echo "Solana CLI is not installed. Installing Solana CLI..."
    sh -c "$(curl -sSfL https://release.solana.com/v1.17.22/install)"
    export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
fi

# Create config directory
mkdir -p config

# Create default config file if it doesn't exist
if [ ! -f config/default.toml ]; then
    echo "Creating default configuration file..."
    cat > config/default.toml << EOF
[network]
use_dpdk = false
use_io_uring = true
worker_threads = 4

[rpc]
endpoints = [
    "https://api.mainnet-beta.solana.com",
    "https://solana-api.projectserum.com"
]
connection_pool_size = 5

[execution]
use_jito = false
simulate_transactions = true

[arbitrage]
min_profit_threshold = 0.001
max_position_size = 1000
EOF
fi

# Check for DPDK (optional)
if command -v dpdk-devbind &> /dev/null; then
    echo "DPDK is installed. Enabling DPDK in configuration..."
    sed -i 's/use_dpdk = false/use_dpdk = true/' config/default.toml
else
    echo "DPDK is not installed. DPDK features will be disabled."
    echo "To install DPDK, follow the instructions at: https://doc.dpdk.org/guides/linux_gsg/build_dpdk.html"
fi

# Check for io_uring support (Linux only)
if [ "$(uname)" = "Linux" ]; then
    if [ -f /usr/include/liburing.h ] || [ -f /usr/local/include/liburing.h ]; then
        echo "io_uring is supported. Enabling io_uring in configuration..."
        sed -i 's/use_io_uring = false/use_io_uring = true/' config/default.toml
    else
        echo "io_uring headers not found. io_uring features will be disabled."
        echo "To enable io_uring, install liburing-dev package."
        sed -i 's/use_io_uring = true/use_io_uring = false/' config/default.toml
    fi
else
    echo "Not running on Linux. io_uring features will be disabled."
    sed -i 's/use_io_uring = true/use_io_uring = false/' config/default.toml
fi

# Build the project
echo "Building Solana HFT Bot..."
cargo build --release

echo "Installation complete!"
echo "To run the bot, use: cargo run --release -- --config config/default.toml"
