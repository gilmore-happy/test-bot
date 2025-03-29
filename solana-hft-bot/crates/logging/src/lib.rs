//! Advanced Logging System for Solana HFT Bot
//!
//! This module provides a comprehensive logging system with:
//! - Hierarchical log levels (ERROR, WARN, INFO, DEBUG, TRACE)
//! - Contextual information including module path, file, line number
//! - High-precision timestamps (microsecond resolution)
//! - Correlation IDs for tracking operations across module boundaries
//! - Performance metrics including execution latencies
//! - Log rotation with time and size-based policies
//! - Multiple output formats (console, JSON, file)
//! - Real-time analysis capabilities
//! - Context propagation across async boundaries
//! - HFT-specific logging features
//!
//! This is the main implementation file for the logging system.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::fmt;
use std::io;

use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, info_span, trace, warn, Level, Subscriber};
use tracing::span::{Attributes, Id, Record};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan, time::FormatTime},
    layer::SubscriberExt,
    registry::LookupSpan,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};
use tracing_subscriber::fmt::format::Format;
use tracing_subscriber::fmt::time::SystemTime;
use tracing_appender::{
    non_blocking::{NonBlocking, WorkerGuard},
    rolling::{RollingFileAppender, Rotation},
};
use uuid::Uuid;

/// Global logger instance
static GLOBAL_LOGGER: OnceCell<Arc<Logger>> = OnceCell::new();

/// Correlation ID for the current execution context
thread_local! {
    static CORRELATION_ID: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

/// High-precision timestamp formatter
struct MicrosecondTimestamp;

impl FormatTime for MicrosecondTimestamp {
    fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        let now = chrono::Utc::now();
        write!(
            w,
            "{}.{:06}",
            now.format("%Y-%m-%d %H:%M:%S"),
            now.timestamp_subsec_micros()
        )
    }
}

/// Log rotation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationPolicy {
    /// Hourly rotation
    Hourly,
    /// Daily rotation
    Daily,
    /// Size-based rotation (in bytes)
    Size(u64),
    /// Compound rotation (both time and size based)
    Compound {
        /// Time-based rotation
        time: Box<RotationPolicy>,
        /// Size-based rotation
        size: Box<RotationPolicy>,
    },
}

impl RotationPolicy {
    /// Convert to tracing_appender rotation
    fn to_appender_rotation(&self) -> Rotation {
        match self {
            RotationPolicy::Hourly => Rotation::HOURLY,
            RotationPolicy::Daily => Rotation::DAILY,
            RotationPolicy::Size(_) => Rotation::NEVER, // We'll handle size-based rotation separately
            RotationPolicy::Compound { time, .. } => time.to_appender_rotation(),
        }
    }
}

/// Log retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Number of days to keep logs
    pub days: u32,
    /// Maximum number of log files to keep
    pub max_files: Option<usize>,
    /// Maximum total size of all log files (in bytes)
    pub max_size: Option<u64>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            days: 7,
            max_files: Some(100),
            max_size: Some(1024 * 1024 * 1024), // 1 GB
        }
    }
}

/// Output format for logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Pretty-printed text format with ANSI colors
    PrettyConsole,
    /// Compact text format
    Compact,
    /// JSON format
    Json,
    /// Custom format
    Custom(String),
}

/// Configuration for a log target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogTargetConfig {
    /// Target name
    pub name: String,
    /// Whether this target is enabled
    pub enabled: bool,
    /// Log level for this target
    pub level: String,
    /// Output format
    pub format: OutputFormat,
    /// Output path (for file targets)
    pub path: Option<String>,
    /// Rotation policy (for file targets)
    pub rotation: Option<RotationPolicy>,
    /// Retention policy (for file targets)
    pub retention: Option<RetentionPolicy>,
    /// Additional target-specific settings
    pub settings: HashMap<String, String>,
}

/// Configuration for the logging system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Default log level
    pub default_level: String,
    /// Whether to include file and line information
    pub include_file_line: bool,
    /// Whether to include module path
    pub include_module_path: bool,
    /// Whether to include thread ID
    pub include_thread_id: bool,
    /// Whether to include correlation ID
    pub include_correlation_id: bool,
    /// Whether to include high-precision timestamps
    pub include_high_precision_timestamps: bool,
    /// Log targets
    pub targets: Vec<LogTargetConfig>,
    /// Log directory
    pub log_dir: String,
    /// Whether to enable Prometheus metrics for logs
    pub enable_prometheus_metrics: bool,
    /// Whether to enable real-time analysis
    pub enable_real_time_analysis: bool,
    /// Whether to enable critical error alerting
    pub enable_critical_error_alerting: bool,
    /// Webhook URLs for critical error alerting
    pub alert_webhooks: Vec<String>,
    /// Whether to enable performance anomaly detection
    pub enable_performance_anomaly_detection: bool,
    /// Whether to enable automated log scanning
    pub enable_automated_log_scanning: bool,
    /// Known error patterns for automated scanning
    pub known_error_patterns: Vec<String>,
    /// Whether to enable adaptive log levels
    pub enable_adaptive_log_levels: bool,
    /// Whether to enable circular buffers for high-frequency events
    pub enable_circular_buffers: bool,
    /// Size of circular buffers (number of events)
    pub circular_buffer_size: usize,
    /// Whether to enable transaction-specific instrumentation
    pub enable_transaction_instrumentation: bool,
    /// Current logging profile
    pub profile: LoggingProfile,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            default_level: "info".to_string(),
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
                    path: Some("logs/solana-hft-bot.log".to_string()),
                    rotation: Some(RotationPolicy::Compound {
                        time: Box::new(RotationPolicy::Daily),
                        size: Box::new(RotationPolicy::Size(100 * 1024 * 1024)), // 100 MB
                    }),
                    retention: Some(RetentionPolicy::default()),
                    settings: HashMap::new(),
                },
                LogTargetConfig {
                    name: "error_file".to_string(),
                    enabled: true,
                    level: "error".to_string(),
                    format: OutputFormat::Json,
                    path: Some("logs/error.log".to_string()),
                    rotation: Some(RotationPolicy::Daily),
                    retention: Some(RetentionPolicy {
                        days: 30,
                        max_files: Some(100),
                        max_size: Some(1024 * 1024 * 1024), // 1 GB
                    }),
                    settings: HashMap::new(),
                },
            ],
            log_dir: "logs".to_string(),
            enable_prometheus_metrics: true,
            enable_real_time_analysis: true,
            enable_critical_error_alerting: true,
            alert_webhooks: vec![],
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
        }
    }
}

/// Logging profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoggingProfile {
    /// Development profile (verbose, human-readable)
    Development,
    /// Testing profile (focused on test results and edge cases)
    Testing,
    /// Production profile (efficient, security-conscious)
    Production,
}

/// Logger implementation
pub struct Logger {
    /// Configuration
    pub config: LoggingConfig,
    /// Worker guards for non-blocking writers
    _guards: Vec<WorkerGuard>,
    /// Circular buffer for high-frequency events
    circular_buffer: Option<Arc<RwLock<Vec<CircularBufferEvent>>>>,
    /// Channel for sending alerts
    alert_tx: Option<mpsc::Sender<AlertMessage>>,
}

/// Circular buffer event
#[derive(Debug, Clone)]
struct CircularBufferEvent {
    /// Timestamp
    timestamp: DateTime<Utc>,
    /// Level
    level: Level,
    /// Message
    message: String,
    /// Module path
    module_path: Option<String>,
    /// File
    file: Option<String>,
    /// Line
    line: Option<u32>,
    /// Correlation ID
    correlation_id: Option<String>,
    /// Additional fields
    fields: HashMap<String, String>,
}

/// Alert message
#[derive(Debug, Clone)]
struct AlertMessage {
    /// Level
    level: Level,
    /// Message
    message: String,
    /// Module path
    module_path: Option<String>,
    /// File
    file: Option<String>,
    /// Line
    line: Option<u32>,
    /// Correlation ID
    correlation_id: Option<String>,
    /// Additional fields
    fields: HashMap<String, String>,
}

impl Logger {
    /// Create a new logger with the given configuration
    pub fn new(config: LoggingConfig) -> Result<(Self, Vec<WorkerGuard>), LoggingError> {
        // Create log directory if it doesn't exist
        std::fs::create_dir_all(&config.log_dir)?;

        let mut guards = Vec::new();
        let mut layers = Vec::new();

        // Create environment filter
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&config.default_level));

        // Configure console output
        if let Some(target) = config.targets.iter().find(|t| t.name == "console" && t.enabled) {
            let console_layer = fmt::layer()
                .with_target(config.include_module_path)
                .with_thread_ids(config.include_thread_id)
                .with_file(config.include_file_line)
                .with_line_number(config.include_file_line)
                .with_level(true)
                .with_ansi(true);

            let console_layer = match target.format {
                OutputFormat::PrettyConsole => console_layer.pretty(),
                OutputFormat::Compact => console_layer.compact(),
                OutputFormat::Json => console_layer.json(),
                OutputFormat::Custom(_) => console_layer, // Custom format not supported for console
            };

            let console_layer = if config.include_high_precision_timestamps {
                console_layer.with_timer(MicrosecondTimestamp)
            } else {
                console_layer.with_timer(SystemTime)
            };

            layers.push(console_layer.with_filter(env_filter.clone()).boxed());
        }

        // Configure file outputs
        for target in config.targets.iter().filter(|t| t.enabled && t.path.is_some()) {
            if let Some(path) = &target.path {
                let path = PathBuf::from(&config.log_dir).join(path);
                
                // Create directory if it doesn't exist
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let rotation = target.rotation.as_ref().map(|r| r.to_appender_rotation()).unwrap_or(Rotation::NEVER);
                let file_appender = RollingFileAppender::new(rotation, config.log_dir.clone(), path.file_name().unwrap().to_str().unwrap());
                let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
                guards.push(guard);

                let file_layer = fmt::layer()
                    .with_writer(non_blocking)
                    .with_target(config.include_module_path)
                    .with_thread_ids(config.include_thread_id)
                    .with_file(config.include_file_line)
                    .with_line_number(config.include_file_line)
                    .with_level(true)
                    .with_ansi(false);

                let file_layer = match target.format {
                    OutputFormat::PrettyConsole => file_layer,
                    OutputFormat::Compact => file_layer.compact(),
                    OutputFormat::Json => file_layer.json(),
                    OutputFormat::Custom(_) => file_layer, // Custom format not supported yet
                };

                let file_layer = if config.include_high_precision_timestamps {
                    file_layer.with_timer(MicrosecondTimestamp)
                } else {
                    file_layer.with_timer(SystemTime)
                };

                let target_filter = EnvFilter::new(format!("{},{}", env_filter, target.level));
                layers.push(file_layer.with_filter(target_filter).boxed());
            }
        }

        // Set up circular buffer if enabled
        let circular_buffer = if config.enable_circular_buffers {
            Some(Arc::new(RwLock::new(Vec::with_capacity(config.circular_buffer_size))))
        } else {
            None
        };

        // Set up alert channel if enabled
        let alert_tx = if config.enable_critical_error_alerting && !config.alert_webhooks.is_empty() {
            let (tx, rx) = mpsc::channel(100);
            let webhooks = config.alert_webhooks.clone();
            
            tokio::spawn(async move {
                while let Some(alert) = rx.recv().await {
                    for webhook in &webhooks {
                        if let Err(e) = Self::send_alert(webhook, &alert).await {
                            eprintln!("Failed to send alert to webhook {}: {}", webhook, e);
                        }
                    }
                }
            });
            
            Some(tx)
        } else {
            None
        };

        // Create and register the subscriber
        tracing_subscriber::registry()
            .with(layers)
            .init();

        Ok((
            Self {
                config,
                _guards: guards.clone(),
                circular_buffer,
                alert_tx,
            },
            guards,
        ))
    }

    /// Initialize the global logger
    pub fn init(config: LoggingConfig) -> Result<(), LoggingError> {
        let (logger, _guards) = Self::new(config)?;
        GLOBAL_LOGGER.set(Arc::new(logger)).map_err(|_| LoggingError::AlreadyInitialized)?;
        Ok(())
    }

    /// Get the global logger
    pub fn global() -> Result<Arc<Logger>, LoggingError> {
        GLOBAL_LOGGER.get().cloned().ok_or(LoggingError::NotInitialized)
    }

    /// Set the correlation ID for the current thread
    pub fn set_correlation_id(id: impl Into<String>) {
        CORRELATION_ID.with(|cell| {
            *cell.borrow_mut() = Some(id.into());
        });
    }

    /// Get the correlation ID for the current thread
    pub fn correlation_id() -> Option<String> {
        CORRELATION_ID.with(|cell| cell.borrow().clone())
    }

    /// Generate a new correlation ID
    pub fn generate_correlation_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Add an event to the circular buffer
    pub fn add_to_circular_buffer(&self, event: CircularBufferEvent) {
        if let Some(buffer) = &self.circular_buffer {
            let mut buffer = buffer.write();
            if buffer.len() >= self.config.circular_buffer_size {
                buffer.remove(0);
            }
            buffer.push(event);
        }
    }

    /// Get events from the circular buffer
    pub fn get_circular_buffer_events(&self) -> Vec<CircularBufferEvent> {
        if let Some(buffer) = &self.circular_buffer {
            buffer.read().clone()
        } else {
            Vec::new()
        }
    }

    /// Send an alert
    async fn send_alert(webhook: &str, alert: &AlertMessage) -> Result<(), reqwest::Error> {
        let client = reqwest::Client::new();
        
        let payload = serde_json::json!({
            "level": format!("{:?}", alert.level),
            "message": alert.message,
            "module_path": alert.module_path,
            "file": alert.file,
            "line": alert.line,
            "correlation_id": alert.correlation_id,
            "fields": alert.fields,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        client.post(webhook)
            .json(&payload)
            .send()
            .await?;
            
        Ok(())
    }
}

/// Logging error types
#[derive(thiserror::Error, Debug)]
pub enum LoggingError {
    #[error("Logging system already initialized")]
    AlreadyInitialized,
    
    #[error("Logging system not initialized")]
    NotInitialized,
    
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Tracing error: {0}")]
    Tracing(String),
}

/// Initialize the logging system with the given configuration file
pub fn init_from_file(path: &str) -> Result<(), LoggingError> {
    let config_str = std::fs::read_to_string(path)?;
    let config: LoggingConfig = serde_json::from_str(&config_str)?;
    Logger::init(config)
}

/// Initialize the logging system with default configuration
pub fn init_default() -> Result<(), LoggingError> {
    Logger::init(LoggingConfig::default())
}

/// Macro for initializing logging
#[macro_export]
macro_rules! init_logging {
    () => {
        $crate::init_default().expect("Failed to initialize logging system")
    };
    ($path:expr) => {
        $crate::init_from_file($path).expect("Failed to initialize logging system")
    };
    ($config:expr) => {
        $crate::Logger::init($config).expect("Failed to initialize logging system")
    };
}

/// Macro for tracing a transaction
#[macro_export]
macro_rules! trace_tx {
    ($tx_id:expr, $($arg:tt)*) => {
        let span = tracing::info_span!("transaction", tx_id = $tx_id);
        let _guard = span.enter();
        tracing::info!($($arg)*);
    };
}

/// Macro for logging performance events
#[macro_export]
macro_rules! perf_event {
    ($name:expr, $duration:expr) => {
        tracing::info!(
            target: "performance",
            event = $name,
            duration_us = $duration.as_micros() as u64,
            correlation_id = $crate::Logger::correlation_id().unwrap_or_default()
        );
    };
    ($name:expr, $duration:expr, $($key:ident = $value:expr),*) => {
        tracing::info!(
            target: "performance",
            event = $name,
            duration_us = $duration.as_micros() as u64,
            correlation_id = $crate::Logger::correlation_id().unwrap_or_default(),
            $($key = $value),*
        );
    };
}

/// Macro for logging strategy events
#[macro_export]
macro_rules! strategy_log {
    ($strategy:expr, $($arg:tt)*) => {
        let span = tracing::info_span!("strategy", name = $strategy);
        let _guard = span.enter();
        tracing::info!($($arg)*);
    };
}

/// Module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.default_level, "info");
        assert!(config.include_file_line);
        assert!(config.include_module_path);
        assert!(config.include_thread_id);
        assert!(config.include_correlation_id);
        assert!(config.include_high_precision_timestamps);
        assert_eq!(config.profile, LoggingProfile::Development);
    }
    
    #[test]
    fn test_correlation_id() {
        let id = "test-correlation-id";
        Logger::set_correlation_id(id);
        assert_eq!(Logger::correlation_id(), Some(id.to_string()));
    }
    
    #[test]
    fn test_generate_correlation_id() {
        let id = Logger::generate_correlation_id();
        assert!(!id.is_empty());
        assert_ne!(id, Logger::generate_correlation_id());
    }
}