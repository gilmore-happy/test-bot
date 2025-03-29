use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Network configuration for the high-performance network engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Number of worker threads (defaults to number of CPU cores)
    pub worker_threads: Option<usize>,
    
    /// Whether to use DPDK for kernel bypass networking
    pub use_dpdk: bool,
    
    /// Whether to use io_uring for async IO
    pub use_io_uring: bool,
    
    /// Size of the send buffer in bytes
    pub send_buffer_size: usize,
    
    /// Size of the receive buffer in bytes
    pub recv_buffer_size: usize,
    
    /// Connection timeout
    pub connection_timeout: Duration,
    
    /// Keep-alive interval
    pub keepalive_interval: Option<Duration>,
    
    /// Maximum number of connections
    pub max_connections: usize,
    
    /// Local addresses to bind to
    pub bind_addresses: Vec<std::net::IpAddr>,
    
    /// Advanced socket options
    pub socket_options: super::socket::SocketOptions,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            worker_threads: None, // Will use number of CPU cores
            use_dpdk: false,     // Requires system setup; disable by default
            use_io_uring: true,  // Enable by default on supported systems
            send_buffer_size: 1024 * 1024, // 1MB
            recv_buffer_size: 1024 * 1024, // 1MB
            connection_timeout: Duration::from_secs(30),
            keepalive_interval: Some(Duration::from_secs(15)),
            max_connections: 1000,
            bind_addresses: vec![],
            socket_options: super::socket::SocketOptions::default(),
        }
    }
}