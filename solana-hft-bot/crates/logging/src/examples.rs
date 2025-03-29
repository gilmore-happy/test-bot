//! Examples of using the Solana HFT Bot logging system
//!
//! This file contains examples of how to use the logging system in different scenarios.

use std::time::Duration;
use std::collections::HashMap;

use tracing::{debug, error, info, trace, warn, Level};

use crate::{
    init_logging, trace_tx, perf_event, strategy_log,
    time_block, time_call,
    Logger, LoggingConfig, LoggingProfile, OutputFormat,
    RotationPolicy, RetentionPolicy, LogTargetConfig,
};

/// Example of basic logging
pub fn basic_logging_example() {
    // Initialize logging with default configuration
    init_logging!();
    
    // Log messages at different levels
    trace!("This is a trace message");
    debug!("This is a debug message");
    info!("This is an info message");
    warn!("This is a warning message");
    error!("This is an error message");
    
    // Log with additional context
    info!(
        target: "example",
        user_id = "user123",
        request_id = "req456",
        latency_ms = 42,
        "Request processed successfully"
    );
}

/// Example of using correlation IDs
pub fn correlation_id_example() {
    // Initialize logging with default configuration
    init_logging!();
    
    // Generate and set a correlation ID
    let correlation_id = Logger::generate_correlation_id();
    Logger::set_correlation_id(&correlation_id);
    
    // Log messages with the correlation ID
    info!("Starting operation with correlation ID: {}", correlation_id);
    
    // The correlation ID will be automatically included in all log messages
    debug!("Processing step 1");
    info!("Processing step 2");
    warn!("Processing step 3 encountered a warning");
    
    // Get the current correlation ID
    let current_id = Logger::correlation_id().unwrap();
    assert_eq!(current_id, correlation_id);
}

/// Example of using transaction tracing
pub fn transaction_tracing_example() {
    // Initialize logging with default configuration
    init_logging!();
    
    // Trace a transaction
    let tx_id = "tx_abc123";
    trace_tx!(tx_id, "Starting transaction");
    
    // Perform transaction steps
    // ...
    
    // Log more transaction events
    trace_tx!(tx_id, "Transaction step 1 completed");
    trace_tx!(tx_id, "Transaction step 2 completed");
    
    // Log transaction completion
    trace_tx!(tx_id, "Transaction completed successfully");
}

/// Example of performance event logging
pub fn performance_event_example() {
    // Initialize logging with default configuration
    init_logging!();
    
    // Simulate a performance-sensitive operation
    let start = std::time::Instant::now();
    // ... perform operation
    std::thread::sleep(Duration::from_millis(10));
    let duration = start.elapsed();
    
    // Log the performance event
    perf_event!("example_operation", duration);
    
    // Log with additional context
    perf_event!("example_operation_with_context", duration,
        operation_type = "query",
        data_size = 1024,
        cache_hit = true
    );
}

/// Example of strategy logging
pub fn strategy_logging_example() {
    // Initialize logging with default configuration
    init_logging!();
    
    // Log strategy events
    strategy_log!("triangular_arbitrage", "Strategy initialized");
    strategy_log!("triangular_arbitrage", "Found opportunity: SOL -> USDC -> BTC -> SOL");
    strategy_log!("triangular_arbitrage", "Executed trade with profit: 0.05 SOL");
}

/// Example of high-resolution timing
pub fn high_resolution_timing_example() {
    // Initialize logging with default configuration
    init_logging!();
    
    // Time a block of code
    let result = time_block!("example_operation", {
        // ... perform operation
        std::thread::sleep(Duration::from_millis(10));
        42
    });
    
    assert_eq!(result, 42);
    
    // Time a block with context
    let result = time_block!("example_operation_with_context", 
        operation_type = "query",
        data_size = 1024,
        {
            // ... perform operation
            std::thread::sleep(Duration::from_millis(10));
            "result"
        }
    );
    
    assert_eq!(result, "result");
    
    // Time a function call
    let result = time_call!("example_function", || {
        // ... perform operation
        std::thread::sleep(Duration::from_millis(10));
        true
    });
    
    assert!(result);
    
    // Time a function call with context
    let result = time_call!("example_function_with_context", 
        operation_type = "query",
        data_size = 1024;
        || {
            // ... perform operation
            std::thread::sleep(Duration::from_millis(10));
            vec![1, 2, 3]
        }
    );
    
    assert_eq!(result, vec![1, 2, 3]);
}

/// Example of custom logging configuration
pub fn custom_logging_configuration_example() {
    // Create a custom logging configuration
    let config = LoggingConfig {
        default_level: "debug".to_string(),
        include_file_line: true,
        include_module_path: true,
        include_thread_id: true,
        include_correlation_id: true,
        include_high_precision_timestamps: true,
        targets: vec![
            LogTargetConfig {
                name: "console".to_string(),
                enabled: true,
                level: "info".to_string(),
                format: OutputFormat::PrettyConsole,
                path: None,
                rotation: None,
                retention: None,
                settings: HashMap::new(),
            },
            LogTargetConfig {
                name: "file".to_string(),
                enabled: true,
                level: "debug".to_string(),
                format: OutputFormat::Json,
                path: Some("logs/custom.log".to_string()),
                rotation: Some(RotationPolicy::Compound {
                    time: Box::new(RotationPolicy::Daily),
                    size: Box::new(RotationPolicy::Size(50 * 1024 * 1024)), // 50 MB
                }),
                retention: Some(RetentionPolicy {
                    days: 14,
                    max_files: Some(100),
                    max_size: Some(1024 * 1024 * 1024), // 1 GB
                }),
                settings: HashMap::new(),
            },
            LogTargetConfig {
                name: "performance".to_string(),
                enabled: true,
                level: "trace".to_string(),
                format: OutputFormat::Json,
                path: Some("logs/performance.log".to_string()),
                rotation: Some(RotationPolicy::Hourly),
                retention: Some(RetentionPolicy {
                    days: 7,
                    max_files: Some(168), // 7 days * 24 hours
                    max_size: None,
                }),
                settings: HashMap::new(),
            },
        ],
        log_dir: "logs".to_string(),
        enable_prometheus_metrics: true,
        enable_real_time_analysis: true,
        enable_critical_error_alerting: true,
        alert_webhooks: vec![
            "https://example.com/webhook".to_string(),
        ],
        enable_performance_anomaly_detection: true,
        enable_automated_log_scanning: true,
        known_error_patterns: vec![
            "connection refused".to_string(),
            "timeout".to_string(),
            "rate limit".to_string(),
        ],
        enable_adaptive_log_levels: true,
        enable_circular_buffers: true,
        circular_buffer_size: 10000,
        enable_transaction_instrumentation: true,
        profile: LoggingProfile::Development,
    };
    
    // Initialize logging with the custom configuration
    init_logging!(config);
    
    // Log messages
    info!("Logging initialized with custom configuration");
    debug!("This is a debug message");
    trace!("This is a trace message");
}

/// Example of using different logging profiles
pub fn logging_profiles_example() {
    // Development profile (verbose, human-readable)
    let dev_config = LoggingConfig {
        default_level: "debug".to_string(),
        profile: LoggingProfile::Development,
        ..LoggingConfig::default()
    };
    
    // Testing profile (focused on test results and edge cases)
    let test_config = LoggingConfig {
        default_level: "info".to_string(),
        profile: LoggingProfile::Testing,
        ..LoggingConfig::default()
    };
    
    // Production profile (efficient, security-conscious)
    let prod_config = LoggingConfig {
        default_level: "warn".to_string(),
        profile: LoggingProfile::Production,
        ..LoggingConfig::default()
    };
    
    // Use the appropriate profile based on the environment
    let env = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
    
    let config = match env.as_str() {
        "development" => dev_config,
        "testing" => test_config,
        "production" => prod_config,
        _ => dev_config,
    };
    
    // Initialize logging with the selected profile
    init_logging!(config);
    
    info!("Logging initialized with {} profile", env);
}

/// Run all examples
pub fn run_all_examples() {
    basic_logging_example();
    correlation_id_example();
    transaction_tracing_example();
    performance_event_example();
    strategy_logging_example();
    high_resolution_timing_example();
    custom_logging_configuration_example();
    logging_profiles_example();
}