//! High-resolution timing utilities for latency-critical paths
//!
//! This module provides utilities for measuring and logging high-resolution
//! timing information for latency-critical paths in the HFT bot.

use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;
use tracing::{debug, info, trace, warn};

use crate::Logger;

/// A high-resolution timer for measuring latency
pub struct HighResTimer {
    /// Name of the timer
    name: String,
    /// Start time
    start: Instant,
    /// Checkpoints
    checkpoints: Vec<(String, Instant)>,
    /// Whether to log checkpoints
    log_checkpoints: bool,
    /// Whether to log the final result
    log_result: bool,
    /// Log level for checkpoints
    checkpoint_level: TimingLogLevel,
    /// Log level for the final result
    result_level: TimingLogLevel,
    /// Additional context
    context: HashMap<String, String>,
}

/// Log level for timing logs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingLogLevel {
    /// Trace level
    Trace,
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Warn level
    Warn,
}

impl HighResTimer {
    /// Create a new high-resolution timer
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
            checkpoints: Vec::new(),
            log_checkpoints: true,
            log_result: true,
            checkpoint_level: TimingLogLevel::Debug,
            result_level: TimingLogLevel::Info,
            context: HashMap::new(),
        }
    }
    
    /// Add a checkpoint
    pub fn checkpoint(&mut self, name: impl Into<String>) -> &mut Self {
        let name = name.into();
        let now = Instant::now();
        
        if self.log_checkpoints {
            let elapsed = now.duration_since(self.start);
            let last_checkpoint = self.checkpoints.last().map(|(_, t)| *t).unwrap_or(self.start);
            let since_last = now.duration_since(last_checkpoint);
            
            match self.checkpoint_level {
                TimingLogLevel::Trace => trace!(
                    target: "timing",
                    timer = self.name,
                    checkpoint = name,
                    elapsed_us = elapsed.as_micros() as u64,
                    since_last_us = since_last.as_micros() as u64,
                    correlation_id = Logger::correlation_id().unwrap_or_default(),
                    context = ?self.context,
                ),
                TimingLogLevel::Debug => debug!(
                    target: "timing",
                    timer = self.name,
                    checkpoint = name,
                    elapsed_us = elapsed.as_micros() as u64,
                    since_last_us = since_last.as_micros() as u64,
                    correlation_id = Logger::correlation_id().unwrap_or_default(),
                    context = ?self.context,
                ),
                TimingLogLevel::Info => info!(
                    target: "timing",
                    timer = self.name,
                    checkpoint = name,
                    elapsed_us = elapsed.as_micros() as u64,
                    since_last_us = since_last.as_micros() as u64,
                    correlation_id = Logger::correlation_id().unwrap_or_default(),
                    context = ?self.context,
                ),
                TimingLogLevel::Warn => warn!(
                    target: "timing",
                    timer = self.name,
                    checkpoint = name,
                    elapsed_us = elapsed.as_micros() as u64,
                    since_last_us = since_last.as_micros() as u64,
                    correlation_id = Logger::correlation_id().unwrap_or_default(),
                    context = ?self.context,
                ),
            }
        }
        
        self.checkpoints.push((name, now));
        self
    }
    
    /// Add context to the timer
    pub fn with_context(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.context.insert(key.into(), value.into());
        self
    }
    
    /// Set whether to log checkpoints
    pub fn log_checkpoints(&mut self, log: bool) -> &mut Self {
        self.log_checkpoints = log;
        self
    }
    
    /// Set whether to log the final result
    pub fn log_result(&mut self, log: bool) -> &mut Self {
        self.log_result = log;
        self
    }
    
    /// Set the log level for checkpoints
    pub fn checkpoint_level(&mut self, level: TimingLogLevel) -> &mut Self {
        self.checkpoint_level = level;
        self
    }
    
    /// Set the log level for the final result
    pub fn result_level(&mut self, level: TimingLogLevel) -> &mut Self {
        self.result_level = level;
        self
    }
    
    /// Stop the timer and return the elapsed time
    pub fn stop(self) -> Duration {
        let now = Instant::now();
        let elapsed = now.duration_since(self.start);
        
        if self.log_result {
            match self.result_level {
                TimingLogLevel::Trace => trace!(
                    target: "timing",
                    timer = self.name,
                    elapsed_us = elapsed.as_micros() as u64,
                    checkpoints = self.checkpoints.len(),
                    correlation_id = Logger::correlation_id().unwrap_or_default(),
                    context = ?self.context,
                ),
                TimingLogLevel::Debug => debug!(
                    target: "timing",
                    timer = self.name,
                    elapsed_us = elapsed.as_micros() as u64,
                    checkpoints = self.checkpoints.len(),
                    correlation_id = Logger::correlation_id().unwrap_or_default(),
                    context = ?self.context,
                ),
                TimingLogLevel::Info => info!(
                    target: "timing",
                    timer = self.name,
                    elapsed_us = elapsed.as_micros() as u64,
                    checkpoints = self.checkpoints.len(),
                    correlation_id = Logger::correlation_id().unwrap_or_default(),
                    context = ?self.context,
                ),
                TimingLogLevel::Warn => warn!(
                    target: "timing",
                    timer = self.name,
                    elapsed_us = elapsed.as_micros() as u64,
                    checkpoints = self.checkpoints.len(),
                    correlation_id = Logger::correlation_id().unwrap_or_default(),
                    context = ?self.context,
                ),
            }
            
            // Record timing statistics
            TIMING_STATS.record(&self.name, elapsed);
        }
        
        elapsed
    }
    
    /// Get the elapsed time without stopping the timer
    pub fn elapsed(&self) -> Duration {
        Instant::now().duration_since(self.start)
    }
}

/// Timing statistics
struct TimingStats {
    /// Statistics for each timer
    stats: RwLock<HashMap<String, TimerStats>>,
}

/// Statistics for a single timer
struct TimerStats {
    /// Count of measurements
    count: AtomicU64,
    /// Minimum duration
    min: RwLock<Duration>,
    /// Maximum duration
    max: RwLock<Duration>,
    /// Total duration
    total: RwLock<Duration>,
    /// Moving average
    avg: RwLock<Duration>,
    /// Exponential moving average (alpha = 0.1)
    ema: RwLock<Duration>,
}

impl TimerStats {
    /// Create new timer statistics
    fn new(duration: Duration) -> Self {
        Self {
            count: AtomicU64::new(1),
            min: RwLock::new(duration),
            max: RwLock::new(duration),
            total: RwLock::new(duration),
            avg: RwLock::new(duration),
            ema: RwLock::new(duration),
        }
    }
    
    /// Update statistics with a new measurement
    fn update(&self, duration: Duration) {
        let count = self.count.fetch_add(1, Ordering::Relaxed) + 1;
        
        // Update min
        {
            let mut min = self.min.write();
            if duration < *min {
                *min = duration;
            }
        }
        
        // Update max
        {
            let mut max = self.max.write();
            if duration > *max {
                *max = duration;
            }
        }
        
        // Update total
        {
            let mut total = self.total.write();
            *total += duration;
        }
        
        // Update average
        {
            let mut avg = self.avg.write();
            *avg = Duration::from_nanos((self.total.read().as_nanos() / count as u128) as u64);
        }
        
        // Update EMA
        {
            let mut ema = self.ema.write();
            let alpha = 0.1;
            let ema_nanos = (1.0 - alpha) * ema.as_nanos() as f64 + alpha * duration.as_nanos() as f64;
            *ema = Duration::from_nanos(ema_nanos as u64);
        }
    }
}

/// Global timing statistics
static TIMING_STATS: TimingStats = TimingStats {
    stats: parking_lot::const_rwlock(HashMap::new()),
};

impl TimingStats {
    /// Record a timing measurement
    fn record(self: &TimingStats, name: &str, duration: Duration) {
        let mut stats = self.stats.write();
        
        if let Some(timer_stats) = stats.get(name) {
            timer_stats.update(duration);
        } else {
            stats.insert(name.to_string(), TimerStats::new(duration));
        }
    }
    
    /// Get statistics for a timer
    pub fn get(name: &str) -> Option<TimerStatsSnapshot> {
        let stats = TIMING_STATS.stats.read();
        
        stats.get(name).map(|s| TimerStatsSnapshot {
            name: name.to_string(),
            count: s.count.load(Ordering::Relaxed),
            min: *s.min.read(),
            max: *s.max.read(),
            avg: *s.avg.read(),
            ema: *s.ema.read(),
        })
    }
    
    /// Get all statistics
    pub fn get_all() -> Vec<TimerStatsSnapshot> {
        let stats = TIMING_STATS.stats.read();
        
        stats.iter().map(|(name, s)| TimerStatsSnapshot {
            name: name.clone(),
            count: s.count.load(Ordering::Relaxed),
            min: *s.min.read(),
            max: *s.max.read(),
            avg: *s.avg.read(),
            ema: *s.ema.read(),
        }).collect()
    }
}

/// Snapshot of timer statistics
#[derive(Debug, Clone)]
pub struct TimerStatsSnapshot {
    /// Timer name
    pub name: String,
    /// Count of measurements
    pub count: u64,
    /// Minimum duration
    pub min: Duration,
    /// Maximum duration
    pub max: Duration,
    /// Average duration
    pub avg: Duration,
    /// Exponential moving average
    pub ema: Duration,
}

/// Macro for timing a block of code
#[macro_export]
macro_rules! time_block {
    ($name:expr, $body:block) => {
        {
            let mut timer = $crate::timing::HighResTimer::new($name);
            let result = $body;
            timer.stop();
            result
        }
    };
    ($name:expr, $($key:ident = $value:expr),+, $body:block) => {
        {
            let mut timer = $crate::timing::HighResTimer::new($name);
            $(
                timer.with_context(stringify!($key), $value.to_string());
            )*
            let result = $body;
            timer.stop();
            result
        }
    };
}

/// Macro for timing a function call
#[macro_export]
macro_rules! time_call {
    ($name:expr, $func:expr) => {
        {
            let mut timer = $crate::timing::HighResTimer::new($name);
            let result = $func;
            timer.stop();
            result
        }
    };
    ($name:expr, $($key:ident = $value:expr),+; $func:expr) => {
        {
            let mut timer = $crate::timing::HighResTimer::new($name);
            $(
                timer.with_context(stringify!($key), $value.to_string());
            )*
            let result = $func;
            timer.stop();
            result
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    
    #[test]
    fn test_high_res_timer() {
        let mut timer = HighResTimer::new("test_timer");
        sleep(Duration::from_millis(1));
        
        timer.checkpoint("checkpoint1");
        sleep(Duration::from_millis(1));
        
        timer.checkpoint("checkpoint2");
        sleep(Duration::from_millis(1));
        
        let elapsed = timer.stop();
        assert!(elapsed.as_millis() >= 3);
    }
    
    #[test]
    fn test_timer_with_context() {
        let mut timer = HighResTimer::new("test_timer_context");
        timer.with_context("key1", "value1");
        timer.with_context("key2", "value2");
        
        sleep(Duration::from_millis(1));
        let elapsed = timer.stop();
        assert!(elapsed.as_millis() >= 1);
    }
    
    #[test]
    fn test_timing_stats() {
        let name = "test_timing_stats";
        
        // Record some measurements
        for i in 1..=5 {
            let duration = Duration::from_millis(i);
            TIMING_STATS.record(name, duration);
        }
        
        // Check statistics
        let stats = TimingStats::get(name).unwrap();
        assert_eq!(stats.count, 5);
        assert_eq!(stats.min, Duration::from_millis(1));
        assert_eq!(stats.max, Duration::from_millis(5));
        assert_eq!(stats.avg, Duration::from_millis(3));
    }
}