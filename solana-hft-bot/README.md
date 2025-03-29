# Solana HFT Bot

A high-frequency trading bot for Solana with ultra-low-latency and high-performance optimizations.

## Project Structure

The project is organized as a Rust workspace with multiple crates:

```text
solana-hft-bot/
├── Cargo.toml (workspace)
└── crates/
    ├── core/         # Core functionality and shared utilities
    ├── network/      # High-performance networking layer
    ├── rpc/          # Optimized RPC client for Solana
    ├── execution/    # Transaction execution engine
    ├── arbitrage/    # Arbitrage strategies
    ├── screening/    # Market screening and opportunity detection
    ├── risk/         # Risk management
    ├── metrics/      # Performance metrics and monitoring
    ├── security/     # Security features
    └── cli/          # Command-line interface
```

## Features

- **Ultra-low-latency networking**: Kernel bypass with DPDK and io_uring, zero-copy buffers, and CPU pinning
- **Optimized RPC client**: Connection pooling, request batching, dynamic endpoint selection, and caching
- **High-performance execution**: Transaction optimization, MEV bundle support via Jito, and retry strategies
  - **Enhanced Jito MEV Integration**: Bundle optimization, pre-flight simulation, and tip optimization
  - **Multi-Path Submission Strategy**: Parallel submission to multiple endpoints with dynamic selection
- **Advanced arbitrage strategies**: Cross-DEX and cross-market arbitrage opportunities
- **Real-time risk management**: Position limits, exposure tracking, and circuit breakers
- **Comprehensive metrics**: Latency tracking, success rates, and performance monitoring

## Performance Optimizations

- **Network optimizations**: Kernel bypass, zero-copy buffers, and hardware timestamps
- **CPU optimizations**: Core pinning, NUMA-aware memory allocation, and cache-friendly data structures
- **Memory optimizations**: Custom allocators, pre-allocated buffers, and memory pooling
- **Algorithmic optimizations**:
  - SIMD instructions (AVX2 and AVX-512) for buffer operations, transaction serialization/deserialization, signature verification, and hash calculation
  - Lock-free data structures for concurrent access
  - Parallel processing for compute-intensive operations

## Getting Started

### Prerequisites

- Rust 1.70+ (nightly required for some features)
- Solana CLI tools
- (Optional) DPDK for kernel bypass networking
- (Optional) io_uring for async I/O on Linux

### Documentation

- [Dependency Management](./DEPENDENCY_MANAGEMENT.md) - Information about the multi-repository structure and dependency management
- [Jito Integration](./solana-hft-bot/JITO_INTEGRATION_DOCUMENTATION.md) - Details about Jito MEV integration
- [Memory Management](./Optimized_Memory_Management.md) - Information about memory optimization techniques

### Building

```bash
# Clone the repository
git clone https://github.com/yourusername/solana-hft-bot.git
cd solana-hft-bot

# Build the project
cargo build --release
```

### Configuration

Create a configuration file at `config/default.toml` with your settings:

```toml
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
# General execution settings
simulate_transactions = true
max_concurrent_transactions = 100
default_transaction_timeout_ms = 30000

# Jito MEV integration
use_jito = true
jito_endpoint = "https://mainnet.block-engine.jito.io/api"
jito_auth_token = "your-auth-token"
jito_max_bundle_size = 5
jito_min_tip_lamports = 10000
jito_max_tip_lamports = 1000000
jito_use_dynamic_tips = true

# Multi-path submission
enable_multi_path_submission = true
primary_rpc_url = "https://api.mainnet-beta.solana.com"
additional_rpc_endpoints = [
    "https://solana-api.projectserum.com",
    "https://rpc.ankr.com/solana"
]
max_parallel_endpoints = 3
cancel_after_first_success = true
transaction_max_age_seconds = 60
recovery_check_interval_ms = 500
endpoint_ranking_update_interval_ms = 60000

[arbitrage]
min_profit_threshold = 0.001
max_position_size = 1000
```

### Running

```bash
# Run the bot
cargo run --release -- --config config/default.toml
```

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
