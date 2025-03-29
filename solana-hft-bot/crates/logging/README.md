# Solana HFT Bot Logging System

A comprehensive logging system for the Solana HFT Bot with advanced features designed for high-frequency trading environments.

## Features

- **Hierarchical Log Levels**: ERROR, WARN, INFO, DEBUG, TRACE
- **Rich Contextual Information**: Module path, file, line number
- **High-Precision Timestamps**: Microsecond resolution for accurate timing
- **Correlation IDs**: Track operations across module boundaries
- **Performance Metrics**: Execution latencies and timing statistics
- **Log Rotation**:
  - Time-based rotation (hourly/daily)
  - Size-based limits (configurable)
  - Compression of archived logs
  - Retention policies (configurable)
- **Multiple Output Formats**:
  - Pretty-printed console output with ANSI colors for development
  - JSON-structured logs for production and machine processing
  - File output with configurable paths and permissions
- **Real-Time Analysis**:
  - Export metrics to Prometheus via a /metrics endpoint
  - Critical error alerting via webhooks to Slack/Discord/Telegram
  - Performance anomaly detection based on moving averages
  - Automated log scanning for known error patterns
- **Context Propagation**:
  - Span context across async boundaries
  - Custom TraceLayer for all network requests
  - Decorator patterns for strategy execution paths
- **HFT-Specific Features**:
  - High-resolution timing logs for latency-critical paths
  - Circular buffers for ultra-high-frequency events
  - Adaptive log levels based on system load
  - Transaction-specific instrumentation
- **Configurable Profiles**:
  - Development (verbose, human-readable)
  - Testing (focused on test results and edge cases)
  - Production (efficient, security-conscious)

## Usage

### Basic Logging

```rust
use solana_hft_logging::{init_logging, info, debug, error};

fn main() {
    // Initialize with default configuration
    init_logging!();
    
    // Log messages at different levels
    info!("Application started");
    debug!("Processing transaction: {}", tx_id);
    error!("Failed to connect to RPC node: {}", err);
}
```

### Transaction Tracing

```rust
use solana_hft_logging::{init_logging, trace_tx};

fn process_transaction(tx_id: &str) {
    trace_tx!(tx_id, "Starting transaction processing");
    
    // ... transaction processing logic ...
    
    trace_tx!(tx_id, "Transaction confirmed in {} ms", confirmation_time);
}
```

### Performance Monitoring

```rust
use solana_hft_logging::{init_logging, perf_event, time_block};
use std::time::Instant;

fn execute_trade() {
    // Measure and log performance events
    let start = Instant::now();
    // ... execute trade ...
    let duration = start.elapsed();
    
    perf_event!("trade_execution", duration, 
        trade_type = "market",
        asset = "SOL/USDC",
        size = 100.0
    );
    
    // Or use the time_block macro
    let result = time_block!("order_placement", {
        // ... place order ...
        "order_id_123"
    });
}
```

### Strategy Logging

```rust
use solana_hft_logging::{init_logging, strategy_log};

fn run_arbitrage_strategy() {
    strategy_log!("triangular_arbitrage", "Strategy initialized");
    
    // ... strategy execution ...
    
    strategy_log!("triangular_arbitrage", "Found opportunity: SOL -> USDC -> BTC -> SOL");
    strategy_log!("triangular_arbitrage", "Executed trade with profit: 0.05 SOL");
}
```

### Custom Configuration

```rust
use solana_hft_logging::{
    init_logging, LoggingConfig, LoggingProfile, 
    OutputFormat, RotationPolicy, RetentionPolicy
};
use std::collections::HashMap;

fn main() {
    // Create a custom configuration
    let config = LoggingConfig {
        default_level: "debug".to_string(),
        profile: LoggingProfile::Development,
        // ... other configuration options ...
    };
    
    // Initialize with custom configuration
    init_logging!(config);
}
```

## Integration with Other Crates

The logging system integrates seamlessly with other crates in the Solana HFT Bot:

- **Core**: Logs bot lifecycle events and strategy execution
- **Network**: Logs connection events and latency metrics
- **RPC**: Logs request/response details and cache performance
- **Execution**: Logs transaction submission and confirmation
- **Arbitrage**: Logs opportunity detection and execution
- **Risk**: Logs risk checks and circuit breaker events

## Advanced Features

### Correlation IDs

```rust
use solana_hft_logging::{init_logging, info, Logger};

fn process_request() {
    // Generate and set a correlation ID
    let correlation_id = Logger::generate_correlation_id();
    Logger::set_correlation_id(&correlation_id);
    
    // All subsequent logs will include this correlation ID
    info!("Processing request with ID: {}", request_id);
    
    // Pass the correlation ID to other components
    execute_transaction(correlation_id);
}
```

### Context Propagation

```rust
use solana_hft_logging::{init_logging, info, with_context};

async fn process_transaction() {
    info!("Starting transaction processing");
    
    // Propagate context to async tasks
    let result = with_context!(async {
        // This async block will have the same correlation ID
        // and span context as the parent
        info!("Processing in async context");
        42
    }).await;
}
```

### Real-Time Analysis

```rust
use solana_hft_logging::{
    init_logging, AlertRule, AlertCondition, 
    AlertAction, AlertSeverity, ThresholdOperator
};
use std::time::Duration;

fn configure_alerts() {
    // Create alert rules
    let rules = vec![
        AlertRule {
            id: "high-latency".to_string(),
            name: "High Network Latency".to_string(),
            description: "Network latency exceeds threshold".to_string(),
            severity: AlertSeverity::Warning,
            condition: AlertCondition::MetricThreshold {
                metric: "network_latency_ms".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 100.0,
                duration: Duration::from_secs(60),
            },
            actions: vec![AlertAction::Log],
            enabled: true,
        },
    ];
    
    // Configure the analyzer with these rules
    // ...
}
```

## Performance Considerations

The logging system is designed for high-performance environments:

- Non-blocking I/O for all file operations
- Efficient context propagation across async boundaries
- Minimal overhead for disabled log levels
- Circular buffers for high-frequency events to prevent memory issues
- Adaptive log levels based on system load

## Configuration Reference

See the `LoggingConfig` struct for all available configuration options.
