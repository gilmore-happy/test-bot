# Jito Integration Documentation

This document provides detailed information about the Jito MEV (Maximal Extractable Value) integration in the Solana HFT Bot.

## Overview

Jito is a Solana MEV infrastructure provider that offers priority access to the Solana network through MEV bundles. The Solana HFT Bot integrates with Jito to enable atomic execution of transactions and priority access to the Solana network, which is crucial for high-frequency trading strategies.

## Repository Structure

The Jito integration spans multiple repositories:

1. **jito-solana**: Contains the tip distributor package
   - Repository: [https://github.com/jito-foundation/jito-solana.git](https://github.com/jito-foundation/jito-solana.git)
   - Package: tip-distributor

2. **jito-rust-rpc**: Contains the bundle package for creating and submitting MEV bundles
   - Repository: [https://github.com/jito-foundation/jito-rust-rpc.git](https://github.com/jito-foundation/jito-rust-rpc.git)

3. **jito-labs**: Contains the searcher client package for interacting with the Jito searcher API
   - Repository: [https://github.com/jito-foundation/jito-labs.git](https://github.com/jito-foundation/jito-labs.git)
   - Package: searcher_client

## Dependency Configuration

The Jito dependencies are configured in the workspace Cargo.toml file:

```toml
# Jito MEV libraries - using branches for development
jito-tip-distribution = { git = "https://github.com/jito-foundation/jito-solana.git", branch = "master", package = "tip-distributor" }
jito-bundle = { git = "https://github.com/jito-foundation/jito-rust-rpc.git", branch = "master" }
jito-searcher-client = { git = "https://github.com/jito-foundation/jito-labs.git", branch = "master", package = "searcher_client" }
```

For production use, it's recommended to pin these dependencies to specific commits to ensure stability and reproducibility. For example:

```toml
jito-tip-distribution = { git = "https://github.com/jito-foundation/jito-solana.git", rev = "specific-commit-hash", package = "tip-distributor" }
```

## Feature Flags

The Jito integration is controlled by feature flags in the execution crate's Cargo.toml file:

```toml
[features]
default = ["jito"]
simulation = []  # Enable simulation mode
jito = ["dep:jito-tip-distribution", "dep:jito-bundle", "dep:jito-searcher-client"]  # Enable Jito MEV features
jito-mock = []  # Use mock implementations instead of actual Jito dependencies
hardware-optimizations = ["dep:hwloc", "dep:libc"]  # Enable hardware-aware optimizations
simd = []  # Enable SIMD optimizations
```

- The `jito` feature enables the actual Jito dependencies
- The `jito-mock` feature enables mock implementations of Jito types for testing and development without the actual Jito dependencies

## Jito Client

The `JitoClient` struct in `jito.rs` provides the main interface for interacting with Jito:

```rust
pub struct JitoClient {
    /// Configuration
    config: JitoConfig,
    
    /// RPC client for fallback
    rpc_client: Arc<RpcClient>,
    
    /// Jito bundle publisher
    bundle_publisher: Option<Arc<BundlePublisher>>,
    
    /// Jito searcher client
    searcher_client: Option<Arc<SearcherClient>>,
    
    /// Keypair for signing
    keypair: Arc<Keypair>,
    
    /// Submission semaphore to limit concurrent submissions
    submission_semaphore: Arc<Semaphore>,
    
    /// Bundle result cache
    bundle_results: Arc<DashMap<String, BundleReceipt>>,
    
    /// Recent bundle UUIDs
    recent_bundles: Arc<RwLock<VecDeque<(String, Instant)>>>,
    
    /// Network congestion tracker
    congestion_tracker: Arc<CongestionTracker>,
    
    /// Bundle optimizer
    bundle_optimizer: Option<Arc<JitoBundleOptimizer>>,
    
    /// Tip optimizer
    tip_optimizer: Option<Arc<TipOptimizer>>,
    
    /// Bundle simulator
    bundle_simulator: Option<Arc<BundleSimulator>>,
    
    /// Relay endpoints for redundancy
    relay_endpoints: Arc<RwLock<Vec<String>>>,
    
    /// Current active relay index
    active_relay_index: Arc<RwLock<usize>>,
    
    /// Bundle status tracking
    bundle_status_tracker: Arc<DashMap<String, BundleStatus>>,
    
    /// Bundle monitoring channel
    bundle_monitor_tx: Option<mpsc::Sender<String>>,
}
```

## Configuration

The `JitoConfig` struct in `jito.rs` provides configuration options for the Jito integration:

```rust
pub struct JitoConfig {
    /// Jito bundle relay URL
    pub bundle_relay_url: String,
    
    /// Jito auth token (if required)
    pub auth_token: Option<String>,
    
    /// Whether to use Jito bundles
    pub enabled: bool,
    
    /// Maximum bundle size (number of transactions)
    pub max_bundle_size: usize,
    
    /// Bundle submission timeout in milliseconds
    pub submission_timeout_ms: u64,
    
    /// Minimum tip in lamports
    pub min_tip_lamports: u64,
    
    /// Maximum tip in lamports
    pub max_tip_lamports: u64,
    
    /// Tip adjustment factor based on bundle size
    pub tip_adjustment_factor: f64,
    
    /// Whether to use dynamic tip calculation
    pub use_dynamic_tips: bool,
    
    /// Whether to use the Jito searcher API
    pub use_searcher_api: bool,
    
    /// Jito searcher API URL
    pub searcher_api_url: Option<String>,
    
    /// Jito searcher API auth token
    pub searcher_api_auth_token: Option<String>,
    
    /// Whether to fallback to regular RPC if Jito fails
    pub fallback_to_rpc: bool,
    
    /// Maximum number of concurrent bundle submissions
    pub max_concurrent_submissions: usize,
    
    /// Whether to cache recent bundle results
    pub cache_bundle_results: bool,
    
    /// Maximum size of the bundle result cache
    pub bundle_result_cache_size: usize,
    
    /// Additional relay endpoints for redundancy
    pub additional_relay_endpoints: Vec<String>,
    
    /// Whether to enable bundle optimization
    pub enable_bundle_optimization: bool,
    
    /// Whether to enable pre-flight simulation
    pub enable_preflight_simulation: bool,
    
    /// Whether to enable tip optimization
    pub enable_tip_optimization: bool,
    
    /// Whether to enable bundle status tracking
    pub enable_bundle_tracking: bool,
    
    /// Bundle monitoring interval in milliseconds
    pub bundle_monitoring_interval_ms: u64,
    
    /// Whether to enable automatic retry for failed bundles
    pub enable_auto_retry: bool,
    
    /// Maximum number of retries for failed bundles
    pub max_bundle_retries: u32,
    
    /// Whether to enable transaction grouping for atomic execution
    pub enable_transaction_grouping: bool,
}
```

## Bundle Optimization

The `JitoBundleOptimizer` struct in `jito_optimizer.rs` provides optimization capabilities for Jito MEV bundles:

```rust
pub struct JitoBundleOptimizer {
    /// Maximum bundle size
    max_bundle_size: usize,
    
    /// Minimum tip in lamports
    min_tip_lamports: u64,
    
    /// Maximum tip in lamports
    max_tip_lamports: u64,
    
    /// Pending opportunities
    opportunities: Arc<RwLock<VecDeque<TransactionOpportunity>>>,
    
    /// Transaction dependencies
    dependencies: Arc<RwLock<HashMap<Signature, HashSet<Signature>>>>,
}
```

The optimizer provides the following capabilities:

1. **Transaction opportunity analysis**: Analyzes transactions to identify opportunities for MEV extraction
2. **Bundle optimization**: Optimizes bundles for maximum MEV extraction
3. **Transaction grouping**: Groups related transactions for atomic execution
4. **Pre-flight simulation**: Simulates bundles before submission to predict outcomes
5. **Tip optimization**: Optimizes tip amounts based on historical data

## Bundle Simulation

The `BundleSimulator` struct in `jito_optimizer.rs` provides simulation capabilities for Jito MEV bundles:

```rust
pub struct BundleSimulator {
    /// RPC client
    rpc_client: Arc<RpcClient>,
    
    /// Searcher client (optional)
    searcher_client: Option<Arc<SearcherClient>>,
}
```

The simulator allows simulating bundles before submission to predict outcomes and avoid wasting resources on bundles that would fail.

## Tip Optimization

The `TipOptimizer` struct in `jito_optimizer.rs` provides tip optimization capabilities for Jito MEV bundles:

```rust
pub struct TipOptimizer {
    /// Minimum tip in lamports
    min_tip_lamports: u64,
    
    /// Maximum tip in lamports
    max_tip_lamports: u64,
    
    /// Historical tip data by opportunity type
    historical_tips: Arc<RwLock<HashMap<OpportunityType, VecDeque<(u64, bool)>>>>,
    
    /// Network congestion level
    congestion_level: Arc<RwLock<f64>>,
}
```

The tip optimizer uses historical data and network congestion levels to calculate optimal tip amounts for different types of opportunities.

## Bundle Building

The `MevBundleBuilder` struct in `jito.rs` provides a builder pattern for creating Jito MEV bundles:

```rust
pub struct MevBundleBuilder {
    /// Transactions to include in the bundle
    transactions: Vec<Transaction>,
    
    /// Maximum bundle size
    max_bundle_size: usize,
    
    /// Bundle options
    options: BundleOptions,
}
```

The builder allows adding transactions to a bundle and configuring bundle options such as tip amount and timeout.

## Market Making

The `MarketMakingBundleBuilder` struct in `jito.rs` provides a specialized builder for market making bundles:

```rust
pub struct MarketMakingBundleBuilder {
    /// Buy transactions
    buy_transactions: Vec<Transaction>,
    
    /// Sell transactions
    sell_transactions: Vec<Transaction>,
    
    /// Maximum bundle size
    max_bundle_size: usize,
    
    /// Bundle options
    options: BundleOptions,
    
    /// Market making strategy
    strategy: MarketMakingStrategy,
}
```

The market making builder allows creating bundles that include both buy and sell transactions, which is useful for market making strategies.

## Testing

The Jito integration includes test scripts for testing with and without Jito:

- `test-with-jito.sh` (Linux/macOS) or `test-with-jito.ps1` (Windows): Tests with the actual Jito dependencies
- `test-without-jito.sh` (Linux/macOS) or `test-without-jito.ps1` (Windows): Tests with the Jito mock feature enabled

## Mock Implementation

The Jito integration includes mock implementations of Jito types for testing and development without the actual Jito dependencies. The mock implementations are enabled by the `jito-mock` feature flag.

## Conditional Compilation

The Jito integration uses conditional compilation directives to handle both the presence and absence of Jito dependencies:

```rust
#[cfg(any(not(feature = "jito"), feature = "jito-mock"))]
mod jito_mock;
#[cfg(feature = "jito")]
mod jito_optimizer;
#[cfg(any(not(feature = "jito"), feature = "jito-mock"))]
mod jito_optimizer_mock;
```

This allows the code to be compiled and run with or without the actual Jito dependencies, which is useful for testing and development.

## References

- [Jito Documentation](https://jito.network/docs)
- [Jito Solana Repository](https://github.com/jito-foundation/jito-solana)
- [Jito Rust RPC Repository](https://github.com/jito-foundation/jito-rust-rpc)
- [Jito Labs Repository](https://github.com/jito-foundation/jito-labs)
