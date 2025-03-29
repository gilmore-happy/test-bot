//! Hardware timestamp support
//! 
//! This module provides support for hardware timestamps from network cards,
//! particularly Mellanox NICs, for precise timing measurements.

use anyhow::{anyhow, Result};
use std::os::unix::io::RawFd;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

/// Hardware timestamp source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampSource {
    /// Software timestamps
    Software,
    
    /// Hardware timestamps from NIC
    Hardware,
    
    /// PTP hardware clock
    Ptp,
}

/// Timestamp configuration
#[derive(Debug, Clone)]
pub struct TimestampConfig {
    /// Timestamp source
    pub source: TimestampSource,
    
    /// Whether to enable TX timestamps
    pub enable_tx: bool,
    
    /// Whether to enable RX timestamps
    pub enable_rx: bool,
}

impl Default for TimestampConfig {
    fn default() -> Self {
        Self {
            source: TimestampSource::Hardware,
            enable_tx: true,
            enable_rx: true,
        }
    }
}

/// Hardware timestamp
#[derive(Debug, Clone, Copy)]
pub struct HardwareTimestamp {
    /// Seconds
    pub seconds: u64,
    
    /// Nanoseconds
    pub nanoseconds: u32,
    
    /// Source of the timestamp
    pub source: TimestampSource,
}

impl HardwareTimestamp {
    /// Create a new hardware timestamp
    pub fn new(seconds: u64, nanoseconds: u32, source: TimestampSource) -> Self {
        Self {
            seconds,
            nanoseconds,
            source,
        }
    }
    
    /// Create a timestamp from the current system time
    pub fn now() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        
        Self {
            seconds: now.as_secs(),
            nanoseconds: now.subsec_nanos(),
            source: TimestampSource::Software,
        }
    }
    
    /// Convert to Duration since UNIX epoch
    pub fn as_duration(&self) -> Duration {
        Duration::new(self.seconds, self.nanoseconds)
    }
    
    /// Calculate the difference between two timestamps
    pub fn diff(&self, other: &Self) -> Duration {
        if self.seconds >= other.seconds {
            let secs = self.seconds - other.seconds;
            let nanos = if self.nanoseconds >= other.nanoseconds {
                self.nanoseconds - other.nanoseconds
            } else {
                1_000_000_000 + self.nanoseconds - other.nanoseconds - 1
            };
            
            Duration::new(secs, nanos)
        } else {
            other.diff(self)
        }
    }
    
    /// Convert to nanoseconds
    pub fn as_nanos(&self) -> u128 {
        (self.seconds as u128) * 1_000_000_000 + (self.nanoseconds as u128)
    }
    
    /// Format as a string
    pub fn format(&self) -> String {
        format!("{}.{:09} ({:?})", self.seconds, self.nanoseconds, self.source)
    }
}

/// Hardware timestamp manager
pub struct TimestampManager {
    /// Configuration
    config: TimestampConfig,
}

impl TimestampManager {
    /// Create a new timestamp manager
    pub fn new(config: TimestampConfig) -> Self {
        Self { config }
    }
    
    /// Enable hardware timestamps on a socket
    pub fn enable_socket_timestamps(&self, fd: RawFd) -> Result<()> {
        if self.config.source == TimestampSource::Software {
            return Ok(());
        }
        
        #[cfg(target_os = "linux")]
        {
            use std::mem;
            
            // Enable hardware timestamps
            let val: libc::c_int = match self.config.source {
                TimestampSource::Hardware => {
                    if self.config.enable_tx && self.config.enable_rx {
                        libc::SOF_TIMESTAMPING_RAW_HARDWARE | libc::SOF_TIMESTAMPING_TX_HARDWARE | libc::SOF_TIMESTAMPING_RX_HARDWARE
                    } else if self.config.enable_tx {
                        libc::SOF_TIMESTAMPING_RAW_HARDWARE | libc::SOF_TIMESTAMPING_TX_HARDWARE
                    } else if self.config.enable_rx {
                        libc::SOF_TIMESTAMPING_RAW_HARDWARE | libc::SOF_TIMESTAMPING_RX_HARDWARE
                    } else {
                        return Ok(());
                    }
                }
                TimestampSource::Ptp => {
                    if self.config.enable_tx && self.config.enable_rx {
                        libc::SOF_TIMESTAMPING_TX_HARDWARE | libc::SOF_TIMESTAMPING_RX_HARDWARE |
                        libc::SOF_TIMESTAMPING_RAW_HARDWARE | libc::SOF_TIMESTAMPING_SYS_HARDWARE |
                        libc::SOF_TIMESTAMPING_TX_SOFTWARE | libc::SOF_TIMESTAMPING_RX_SOFTWARE
                    } else if self.config.enable_tx {
                        libc::SOF_TIMESTAMPING_TX_HARDWARE | libc::SOF_TIMESTAMPING_RAW_HARDWARE |
                        libc::SOF_TIMESTAMPING_SYS_HARDWARE | libc::SOF_TIMESTAMPING_TX_SOFTWARE
                    } else if self.config.enable_rx {
                        libc::SOF_TIMESTAMPING_RX_HARDWARE | libc::SOF_TIMESTAMPING_RAW_HARDWARE |
                        libc::SOF_TIMESTAMPING_SYS_HARDWARE | libc::SOF_TIMESTAMPING_RX_SOFTWARE
                    } else {
                        return Ok(());
                    }
                }
                TimestampSource::Software => {
                    if self.config.enable_tx && self.config.enable_rx {
                        libc::SOF_TIMESTAMPING_TX_SOFTWARE | libc::SOF_TIMESTAMPING_RX_SOFTWARE
                    } else if self.config.enable_tx {
                        libc::SOF_TIMESTAMPING_TX_SOFTWARE
                    } else if self.config.enable_rx {
                        libc::SOF_TIMESTAMPING_RX_SOFTWARE
                    } else {
                        return Ok(());
                    }
                }
            };
            
            // Add flags for retrieving timestamps
            let val = val | libc::SOF_TIMESTAMPING_SOFTWARE | libc::SOF_TIMESTAMPING_OPT_TSONLY;
            
            let ret = unsafe {
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_TIMESTAMPING,
                    &val as *const _ as *const libc::c_void,
                    mem::size_of::<libc::c_int>() as libc::socklen_t,
                )
            };
            
            if ret < 0 {
                return Err(anyhow!("Failed to enable hardware timestamps: {}", std::io::Error::last_os_error()));
            }
            
            Ok(())
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            warn!("Hardware timestamps are only supported on Linux");
            Ok(())
        }
    }
    
    /// Extract hardware timestamp from a control message
    #[cfg(target_os = "linux")]
    pub fn extract_timestamp(&self, cmsg: &libc::cmsghdr) -> Option<HardwareTimestamp> {
        if cmsg.cmsg_level == libc::SOL_SOCKET && cmsg.cmsg_type == libc::SO_TIMESTAMPING {
            let timestamps = unsafe {
                std::slice::from_raw_parts(
                    libc::CMSG_DATA(cmsg) as *const libc::timespec,
                    3,
                )
            };
            
            // The timestamps array contains:
            // [0]: Software timestamp
            // [1]: Hardware timestamp transformed to system time
            // [2]: Hardware timestamp
            
            let idx = match self.config.source {
                TimestampSource::Software => 0,
                TimestampSource::Hardware => 2,
                TimestampSource::Ptp => 1,
            };
            
            let ts = &timestamps[idx];
            
            if ts.tv_sec != 0 || ts.tv_nsec != 0 {
                return Some(HardwareTimestamp {
                    seconds: ts.tv_sec as u64,
                    nanoseconds: ts.tv_nsec as u32,
                    source: self.config.source,
                });
            }
        }
        
        None
    }
    
    #[cfg(not(target_os = "linux"))]
    pub fn extract_timestamp(&self, _cmsg: &std::os::raw::c_void) -> Option<HardwareTimestamp> {
        None
    }
    
    /// Get the current hardware timestamp
    pub fn now(&self) -> HardwareTimestamp {
        HardwareTimestamp::now()
    }
    
    /// Measure latency between two timestamps
    pub fn measure_latency(&self, start: &HardwareTimestamp, end: &HardwareTimestamp) -> Duration {
        end.diff(start)
    }
    
    /// Measure latency in nanoseconds
    pub fn measure_latency_ns(&self, start: &HardwareTimestamp, end: &HardwareTimestamp) -> u64 {
        let duration = self.measure_latency(start, end);
        duration.as_secs() * 1_000_000_000 + duration.subsec_nanos() as u64
    }
}

/// Timestamp statistics
#[derive(Debug, Clone, Default)]
pub struct TimestampStats {
    /// Minimum latency in nanoseconds
    pub min_latency_ns: u64,
    
    /// Maximum latency in nanoseconds
    pub max_latency_ns: u64,
    
    /// Average latency in nanoseconds
    pub avg_latency_ns: f64,
    
    /// Number of samples
    pub samples: u64,
    
    /// Total latency in nanoseconds
    pub total_latency_ns: u64,
}

impl TimestampStats {
    /// Create new timestamp statistics
    pub fn new() -> Self {
        Self {
            min_latency_ns: u64::MAX,
            max_latency_ns: 0,
            avg_latency_ns: 0.0,
            samples: 0,
            total_latency_ns: 0,
        }
    }
    
    /// Add a latency sample
    pub fn add_sample(&mut self, latency_ns: u64) {
        self.min_latency_ns = self.min_latency_ns.min(latency_ns);
        self.max_latency_ns = self.max_latency_ns.max(latency_ns);
        self.total_latency_ns += latency_ns;
        self.samples += 1;
        self.avg_latency_ns = self.total_latency_ns as f64 / self.samples as f64;
    }
    
    /// Reset statistics
    pub fn reset(&mut self) {
        self.min_latency_ns = u64::MAX;
        self.max_latency_ns = 0;
        self.avg_latency_ns = 0.0;
        self.samples = 0;
        self.total_latency_ns = 0;
    }
    
    /// Get minimum latency in microseconds
    pub fn min_latency_us(&self) -> f64 {
        self.min_latency_ns as f64 / 1000.0
    }
    
    /// Get maximum latency in microseconds
    pub fn max_latency_us(&self) -> f64 {
        self.max_latency_ns as f64 / 1000.0
    }
    
    /// Get average latency in microseconds
    pub fn avg_latency_us(&self) -> f64 {
        self.avg_latency_ns / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_timestamp_now() {
        let ts = HardwareTimestamp::now();
        assert_eq!(ts.source, TimestampSource::Software);
        assert!(ts.seconds > 0);
    }

    #[test]
    fn test_hardware_timestamp_diff() {
        let ts1 = HardwareTimestamp::new(100, 500_000_000, TimestampSource::Software);
        let ts2 = HardwareTimestamp::new(101, 700_000_000, TimestampSource::Software);
        
        let diff = ts2.diff(&ts1);
        assert_eq!(diff.as_secs(), 1);
        assert_eq!(diff.subsec_nanos(), 200_000_000);
        
        let diff = ts1.diff(&ts2);
        assert_eq!(diff.as_secs(), 1);
        assert_eq!(diff.subsec_nanos(), 200_000_000);
    }

    #[test]
    fn test_timestamp_stats() {
        let mut stats = TimestampStats::new();
        
        stats.add_sample(100);
        stats.add_sample(200);
        stats.add_sample(300);
        
        assert_eq!(stats.min_latency_ns, 100);
        assert_eq!(stats.max_latency_ns, 300);
        assert_eq!(stats.avg_latency_ns, 200.0);
        assert_eq!(stats.samples, 3);
        assert_eq!(stats.total_latency_ns, 600);
        
        stats.reset();
        
        assert_eq!(stats.min_latency_ns, u64::MAX);
        assert_eq!(stats.max_latency_ns, 0);
        assert_eq!(stats.avg_latency_ns, 0.0);
        assert_eq!(stats.samples, 0);
        assert_eq!(stats.total_latency_ns, 0);
    }
}
