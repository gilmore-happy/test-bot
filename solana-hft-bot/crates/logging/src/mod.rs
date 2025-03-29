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

// Re-export main components
pub use self::lib::*;
pub use self::context::*;
pub use self::timing::*;
pub use self::analysis::*;

// Internal modules
mod lib;
mod context;
mod timing;
mod analysis;
mod examples;

// Export examples for documentation
pub use self::examples::*;

// Re-export tracing for convenience
pub use tracing::{
    debug, error, info, trace, warn,
    Level, span, event,
    instrument,
};

/// Module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");