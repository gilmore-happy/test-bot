# Solana HFT Bot Integration and Testing Guide

This document provides instructions for integrating all modules of the Solana HFT Bot and setting up testing.

## Table of Contents

1. [Configuration Files](#configuration-files)
2. [Module Integration](#module-integration)
3. [Testing Strategy](#testing-strategy)
4. [Simulation](#simulation)
5. [Replacing Placeholder Code](#replacing-placeholder-code)
6. [Logging and Metrics](#logging-and-metrics)

## Configuration Files

The following configuration files have been created:

- `config.json`: Main configuration file for production use
- `config-test.json`: Configuration file for testing and simulation
- `logging-config.json`: Configuration for logging across all modules
- `metrics-config.json`: Configuration for metrics collection

### Using Configuration Files

To use these configuration files, pass them to the bot when starting:

```bash
# For production
cargo run --release -- --config config.json

# For testing
cargo run --release -- --config config-test.json
```

## Module Integration

All modules are integrated through the core `HftBot` class, which manages the lifecycle of each module. The integration flow is as follows:

1. The bot loads configuration from the specified file
2. Each module is initialized with its specific configuration
3. Modules are started in the correct order:
   - Network module first
   - RPC module second
   - Other modules after
4. Communication between modules happens through channels and shared state
5. The bot manages the shutdown sequence when stopping

### Module Dependencies

The modules have the following dependencies:

- **Core**: Central module that coordinates all others
- **Network**: No dependencies on other modules
- **RPC**: Depends on Network
- **Execution**: Depends on RPC
- **Screening**: Depends on RPC
- **Arbitrage**: Depends on RPC, Execution, and Screening
- **Risk**: Depends on Core

## Testing Strategy

The testing strategy consists of multiple levels:

1. **Unit Tests**: Each module has its own unit tests
2. **Integration Tests**: Test interactions between modules
3. **Simulation**: Full system testing in a controlled environment
4. **Devnet Testing**: Testing on Solana devnet with real transactions
5. **Mainnet Testing**: Final testing on mainnet with limited funds

### Running Tests

To run the unit tests:

```bash
cargo test
```

To run integration tests:

```bash
cargo test --features integration-tests
```

## Simulation

A simulation script has been created to test all components together:

```bash
# Run simulation with default settings (5 minutes, no real transactions)
cargo run --bin simulation

# Run simulation with custom duration
cargo run --bin simulation -- --duration 600

# Run simulation with real transactions on devnet
cargo run --bin simulation -- --real-transactions
```

The simulation:

1. Initializes all modules with test configuration
2. Runs multiple scenarios in parallel:
   - Token screening scenario
   - Arbitrage scenario
   - Risk management scenario
3. Collects and reports metrics at the end

## Replacing Placeholder Code

A script has been created to identify and replace placeholder code:

```bash
# Identify placeholders without replacing them
cargo run --bin replace-placeholders -- --dry-run

# Generate a configuration file with placeholders
cargo run --bin replace-placeholders -- --generate-config

# Replace placeholders using a configuration file
cargo run --bin replace-placeholders -- --config placeholder-replacements.json
```

### Placeholder Priorities

Placeholders are categorized and prioritized as follows:

1. **Network and Execution (Priority 1)**: Critical for performance
2. **RPC (Priority 2)**: Important for reliability
3. **Arbitrage (Priority 3)**: Important for profitability
4. **Screening (Priority 4)**: Important for opportunity detection
5. **Risk (Priority 5)**: Important for safety

## Logging and Metrics

### Logging

Logging is configured in `logging-config.json` and provides:

- Console logging for development
- File logging for production
- Separate error log for critical issues
- Separate metrics log for performance data

To view logs:

```bash
# View console logs
cargo run -- --config config.json

# View file logs
tail -f logs/solana-hft-bot.log

# View error logs
tail -f logs/error.log

# View metrics logs
tail -f logs/metrics.log
```

### Metrics

Metrics are collected and can be:

- Exported to Prometheus (port 9090)
- Logged to files
- Viewed in real-time through the CLI

To view metrics:

```bash
# View metrics through CLI
cargo run -- --config config.json status metrics

# Access Prometheus metrics
curl http://localhost:9090/metrics
```

## Integration Testing Checklist

Use this checklist to ensure all components are properly integrated:

- [ ] Configuration files are properly set up
- [ ] All modules initialize without errors
- [ ] Modules can communicate with each other
- [ ] Logging is working across all modules
- [ ] Metrics are being collected
- [ ] Simulation runs without errors
- [ ] Placeholder code has been replaced
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Devnet testing is successful

## Troubleshooting

If you encounter issues during integration:

1. Check the logs for error messages
2. Verify that all configuration files are correctly formatted
3. Ensure all dependencies are installed
4. Check that the correct Solana RPC endpoints are configured
5. Verify network connectivity to Solana nodes
6. Check that all modules are enabled in the configuration

For specific module issues:

- **Network**: Check socket configurations and permissions
- **RPC**: Verify endpoint health and rate limits
- **Execution**: Check transaction simulation results
- **Arbitrage**: Verify pool data and price feeds
- **Screening**: Check websocket subscriptions
- **Risk**: Verify circuit breaker configurations
