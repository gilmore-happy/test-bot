# Jito Integration Guide for Solana HFT Bot

This guide provides detailed information about the Jito integration in the Solana HFT Bot, including both the MEV Bundle support and the ShredStream low-latency block updates.

## Overview

Jito provides two key services that are integrated into the Solana HFT Bot:

1. **MEV Bundle Support**: Enables atomic execution of transactions and priority access to the Solana network through MEV bundles.
2. **ShredStream**: Provides low-latency block updates for faster transaction monitoring and execution.

## Prerequisites

To use the Jito integration, you need:

1. A Solana keypair for authentication
2. (Optional) A Jito authentication token
3. (Optional) A Jito searcher API token

## Configuration

The Jito integration can be configured through the `ExecutionConfig` struct:

```rust
// Enable Jito MEV bundles
config.use_jito = true;
config.jito_endpoint = "https://mainnet.block-engine.jito.wtf/api/v1/bundles";
config.jito_auth_keypair = keypair.to_base58_string();
config.jito_tip_amount = 10_000; // 0.00001 SOL

// Enable Jito ShredStream
config.use_jito_shredstream = true;
config.jito_shredstream_block_engine_url = "https://mainnet.block-engine.jito.wtf";
config.jito_shredstream_regions = vec!["amsterdam".to_string(), "ny".to_string()];
config.jito_shredstream_dest_ip_ports = vec!["127.0.0.1:8001".to_string()];
```

You can also use the builder pattern:

```rust
let config = ExecutionConfig::default()
    .with_jito_keypair(keypair)
    .with_jito_tip("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5".to_string(), 10_000)
    .with_shredstream(
        true,
        Some("https://mainnet.block-engine.jito.wtf".to_string()),
        Some("/path/to/keypair.json".to_string()),
        Some(vec!["amsterdam".to_string(), "ny".to_string()]),
        Some(vec!["127.0.0.1:8001".to_string()]),
        Some(20000),
        Some(false),
        Some(50051),
    );
```

## MEV Bundle Support

The Jito MEV Bundle support allows you to:

1. Submit single transactions with priority
2. Submit bundles of transactions for atomic execution
3. Optimize bundles for maximum MEV extraction
4. Simulate bundles before submission
5. Track bundle status

### Submitting a Transaction

```rust
// Create execution engine
let engine = ExecutionEngine::new(config, rpc_client).await?;

// Submit a transaction with Jito
let signature = engine.submit_transaction(
    transaction,
    PriorityLevel::High,
    true, // use_jito
    Some(Duration::from_secs(30)),
).await?;
```

### Submitting a Bundle

```rust
// Create a bundle builder
let mut builder = MevBundleBuilder::new(5);

// Add transactions to the bundle
builder.add_transaction(tx1)?;
builder.add_transaction(tx2)?;

// Set bundle options
let options = BundleOptions {
    tip_account: Some(tip_account),
    tip_amount: 10_000,
    wait_for_confirmation: true,
    ..BundleOptions::default()
};

// Build the bundle
let (transactions, options) = builder.with_options(options).build();

// Submit the bundle
let receipt = engine.jito_client.as_ref().unwrap().submit_bundle(
    transactions,
    options,
).await?;

// Check bundle status
let status = engine.jito_client.as_ref().unwrap().check_bundle_status(
    &receipt.uuid,
).await?;
```

## ShredStream Integration

The Jito ShredStream integration provides low-latency block updates, which can be used to:

1. Receive block updates with minimal latency
2. Decode transactions from shreds
3. Monitor the network for specific transactions
4. React to market events faster

### Starting ShredStream

```rust
// Create ShredStream client
let mut shredstream = ShredStreamClient::new(ShredStreamConfig {
    block_engine_url: "https://mainnet.block-engine.jito.wtf".to_string(),
    auth_keypair_path: Some("/path/to/keypair.json".to_string()),
    desired_regions: vec!["amsterdam".to_string(), "ny".to_string()],
    dest_ip_ports: vec!["127.0.0.1:8001".to_string()],
    src_bind_port: Some(20000),
    enable_grpc_service: false,
    grpc_service_port: Some(50051),
});

// Start ShredStream
shredstream.start().await?;

// Check status
let status = shredstream.get_status();
println!("ShredStream status: {:?}", status);

// Get metrics
let metrics = shredstream.get_metrics();
println!("ShredStream metrics: {:?}", metrics);
```

## Ultra-Low-Latency Mode

The Solana HFT Bot provides an ultra-low-latency mode that combines Jito MEV bundles and ShredStream for the lowest possible latency:

```rust
let config = ExecutionConfig::default().ultra_low_latency_mode();
```

This mode:

1. Enables Jito ShredStream for low-latency block updates
2. Enables Jito MEV bundles for priority access
3. Configures the execution engine for minimal latency
4. Uses smaller buffer sizes and more aggressive settings

## Troubleshooting

### MEV Bundle Issues

1. **Bundle not landing**: Check the bundle status using `check_bundle_status` or `get_inflight_bundle_statuses`.
2. **Tip too low**: Increase the tip amount, especially during high congestion periods.
3. **Bundle rejected**: Ensure all transactions in the bundle are valid and have proper signatures.

### ShredStream Issues

1. **Connection issues**: Ensure your firewall allows UDP traffic on the specified ports.
2. **Authentication issues**: Verify your keypair is valid and has the necessary permissions.
3. **No shreds received**: Check that the desired regions are correct and the ShredStream proxy is running.

## Feature Flags

The Jito integration is controlled by feature flags:

- `jito`: Enables basic Jito MEV bundle support
- `jito-full`: Enables both Jito MEV bundles and ShredStream
- `jito-mock`: Uses mock implementations for testing without actual Jito dependencies

## Testing

Use the provided test scripts to verify the Jito integration:

- `test-with-jito.sh` (Linux/macOS) or `test-with-jito.ps1` (Windows): Tests with basic Jito features
- `test-with-jito-full.sh` (Linux/macOS) or `test-with-jito-full.ps1` (Windows): Tests with full Jito features
- `test-without-jito.sh` (Linux/macOS) or `test-without-jito.ps1` (Windows): Tests with Jito mock feature

## References

- [Jito Documentation](https://docs.jito.wtf/)
- [Jito Block Engine API](https://docs.jito.wtf/api/v1/bundles)
- [Jito ShredStream Documentation](https://docs.jito.wtf/lowlatencytxnfeed/)