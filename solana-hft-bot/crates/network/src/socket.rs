//! Socket optimizations
//! 
//! This module provides optimized socket implementations with various
//! performance enhancements for low-latency networking.

use anyhow::{anyhow, Result};
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream};
use tracing::{debug, error, info, warn};

use super::timestamps::{HardwareTimestamp, TimestampConfig, TimestampManager, TimestampSource};
use super::zero_copy::ZeroCopyBuffer;

/// Socket options for performance tuning
#[derive(Debug, Clone)]
pub struct SocketOptions {
    /// Enable TCP_NODELAY (disable Nagle's algorithm)
    pub tcp_nodelay: bool,
    
    /// Enable TCP_QUICKACK (disable delayed ACKs)
    pub tcp_quickack: bool,
    
    /// Enable SO_REUSEADDR
    pub so_reuseaddr: bool,
    
    /// Enable SO_REUSEPORT
    pub so_reuseport: bool,
    
    /// Send buffer size
    pub so_sndbuf: Option<usize>,
    
    /// Receive buffer size
    pub so_rcvbuf: Option<usize>,
    
    /// Enable TCP_FASTOPEN
    pub tcp_fastopen: bool,
    
    /// Socket priority
    pub priority: Option<i32>,
    
    /// IP Type of Service (TOS)
    pub tos: Option<u8>,
    
    /// Enable hardware timestamps
    pub hardware_timestamps: bool,
    
    /// Timestamp configuration
    pub timestamp_config: Option<TimestampConfig>,
    
    /// Enable TCP_CORK (coalesce small packets)
    pub tcp_cork: bool,
    
    /// Enable TCP_DEFER_ACCEPT (wait for data before accepting)
    pub tcp_defer_accept: bool,
    
    /// TCP_NOTSENT_LOWAT threshold
    pub tcp_notsent_lowat: Option<usize>,
    
    /// SO_BUSY_POLL timeout in microseconds
    pub so_busy_poll: Option<u32>,
    
    /// SO_INCOMING_CPU (direct interrupts to a specific CPU)
    pub so_incoming_cpu: Option<u32>,
    
    /// Enable TCP_THIN_LINEAR_TIMEOUTS
    pub tcp_thin_linear_timeouts: bool,
    
    /// Enable TCP_THIN_DUPACK
    pub tcp_thin_dupack: bool,
}

impl Default for SocketOptions {
    fn default() -> Self {
        Self {
            tcp_nodelay: true,
            tcp_quickack: true,
            so_reuseaddr: true,
            so_reuseport: true,
            so_sndbuf: Some(1024 * 1024),  // 1MB
            so_rcvbuf: Some(1024 * 1024),  // 1MB
            tcp_fastopen: true,
            priority: None,
            tos: Some(0x10), // IPTOS_LOWDELAY
            hardware_timestamps: false,
            timestamp_config: None,
            tcp_cork: false,
            tcp_defer_accept: false,
            tcp_notsent_lowat: None,
            so_busy_poll: None,
            so_incoming_cpu: None,
            tcp_thin_linear_timeouts: false,
            tcp_thin_dupack: false,
        }
    }
}

impl SocketOptions {
    /// Apply all optimizations for ultra-low latency
    pub fn optimize_for_ultra_low_latency(&mut self) {
        self.tcp_nodelay = true;
        self.tcp_quickack = true;
        self.so_reuseaddr = true;
        self.so_reuseport = true;
        self.tcp_fastopen = true;
        self.tos = Some(0x10); // IPTOS_LOWDELAY
        self.priority = Some(6); // TC_PRIO_INTERACTIVE
        
        // Increase buffer sizes for high throughput
        self.so_sndbuf = Some(4 * 1024 * 1024); // 4MB
        self.so_rcvbuf = Some(4 * 1024 * 1024); // 4MB
        
        // Enable hardware timestamps
        self.hardware_timestamps = true;
        self.timestamp_config = Some(TimestampConfig {
            source: TimestampSource::Hardware,
            enable_tx: true,
            enable_rx: true,
        });
        
        // Enable additional optimizations
        self.tcp_cork = true;
        self.tcp_defer_accept = true;
        self.tcp_notsent_lowat = Some(16384); // 16KB
        self.so_busy_poll = Some(50); // 50 microseconds
        self.so_incoming_cpu = Some(0); // CPU 0
        self.tcp_thin_linear_timeouts = true;
        self.tcp_thin_dupack = true;
    }
    
    /// Apply Linux-specific optimizations
    #[cfg(target_os = "linux")]
    pub fn apply_linux_specific_optimizations(&self, socket: &socket2::Socket) -> Result<()> {
        use std::os::unix::io::AsRawFd;
        
        let fd = socket.as_raw_fd();
        
        // Enable TCP_CORK for coalescing small packets
        if self.tcp_cork {
            unsafe {
                let optval: libc::c_int = 1;
                if libc::setsockopt(
                    fd,
                    libc::IPPROTO_TCP,
                    libc::TCP_CORK,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set TCP_CORK"));
                }
            }
        }
        
        // Enable TCP_DEFER_ACCEPT to wait for data before accepting
        if self.tcp_defer_accept {
            unsafe {
                let optval: libc::c_int = 1;
                if libc::setsockopt(
                    fd,
                    libc::IPPROTO_TCP,
                    libc::TCP_DEFER_ACCEPT,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set TCP_DEFER_ACCEPT"));
                }
            }
        }
        
        // Set TCP_NOTSENT_LOWAT to reduce memory pressure
        if let Some(lowat) = self.tcp_notsent_lowat {
            unsafe {
                let optval: libc::c_int = lowat as libc::c_int;
                if libc::setsockopt(
                    fd,
                    libc::IPPROTO_TCP,
                    libc::TCP_NOTSENT_LOWAT,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set TCP_NOTSENT_LOWAT"));
                }
            }
        }
        
        // Set SO_BUSY_POLL for low-latency polling
        if let Some(timeout) = self.so_busy_poll {
            unsafe {
                let optval: libc::c_int = timeout as libc::c_int;
                if libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_BUSY_POLL,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set SO_BUSY_POLL"));
                }
            }
        }
        
        // Set SO_INCOMING_CPU to direct interrupts to a specific CPU
        if let Some(cpu) = self.so_incoming_cpu {
            unsafe {
                let optval: libc::c_int = cpu as libc::c_int;
                if libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_INCOMING_CPU,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set SO_INCOMING_CPU"));
                }
            }
        }
        
        // Enable TCP_THIN_LINEAR_TIMEOUTS
        if self.tcp_thin_linear_timeouts {
            unsafe {
                let optval: libc::c_int = 1;
                if libc::setsockopt(
                    fd,
                    libc::IPPROTO_TCP,
                    libc::TCP_THIN_LINEAR_TIMEOUTS,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set TCP_THIN_LINEAR_TIMEOUTS"));
                }
            }
        }
        
        // Enable TCP_THIN_DUPACK
        if self.tcp_thin_dupack {
            unsafe {
                let optval: libc::c_int = 1;
                if libc::setsockopt(
                    fd,
                    libc::IPPROTO_TCP,
                    libc::TCP_THIN_DUPACK,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set TCP_THIN_DUPACK"));
                }
            }
        }
        
        Ok(())
    }
}

/// High-performance socket with optimizations
pub struct HighPerfSocket {
    /// Inner TCP stream
    inner: TcpStream,
    
    /// Socket options
    options: SocketOptions,
    
    /// Timestamp manager
    timestamp_manager: Option<TimestampManager>,
    
    /// Last TX timestamp
    last_tx_timestamp: Option<HardwareTimestamp>,
    
    /// Last RX timestamp
    last_rx_timestamp: Option<HardwareTimestamp>,
}

impl HighPerfSocket {
    /// Connect to a remote address with optimized options
    pub async fn connect(addr: std::net::SocketAddr, options: SocketOptions) -> Result<Self> {
        let socket = match addr {
            std::net::SocketAddr::V4(_) => TcpSocket::new_v4()?,
            std::net::SocketAddr::V6(_) => TcpSocket::new_v6()?,
        };
        
        Self::apply_socket_options(&socket, &options)?;
        
        let stream = socket.connect(addr).await?;
        
        // Further optimize connected socket
        let socket2 = socket2::Socket::from(std::net::TcpStream::from(stream.try_clone().await?));
        
        if options.tcp_nodelay {
            socket2.set_nodelay(true)?;
        }
        
        #[cfg(target_os = "linux")]
        if options.tcp_quickack {
            unsafe {
                let optval: libc::c_int = 1;
                if libc::setsockopt(
                    socket2.as_raw_fd(),
                    libc::IPPROTO_TCP,
                    libc::TCP_QUICKACK,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set TCP_QUICKACK"));
                }
            }
        }
        
        #[cfg(target_os = "linux")]
        if let Err(e) = options.apply_linux_specific_optimizations(&socket2) {
            warn!("Failed to apply Linux-specific optimizations: {}", e);
        }
        
        // Set up timestamp manager if hardware timestamps are enabled
        let timestamp_manager = if options.hardware_timestamps {
            let config = options.timestamp_config.unwrap_or_default();
            let manager = TimestampManager::new(config);
            
            #[cfg(target_os = "linux")]
            if let Err(e) = manager.enable_socket_timestamps(socket2.as_raw_fd()) {
                warn!("Failed to enable hardware timestamps: {}", e);
            }
            
            Some(manager)
        } else {
            None
        };
        
        Ok(Self {
            inner: stream,
            options,
            timestamp_manager,
            last_tx_timestamp: None,
            last_rx_timestamp: None,
        })
    }
    
    /// Apply socket options for optimization
    fn apply_socket_options(socket: &TcpSocket, options: &SocketOptions) -> Result<()> {
        let socket2 = socket2::Socket::from(socket.as_raw_fd());
        
        if options.so_reuseaddr {
            socket2.set_reuse_address(true)?;
        }
        
        #[cfg(target_os = "linux")]
        if options.so_reuseport {
            socket2.set_reuse_port(true)?;
        }
        
        if let Some(size) = options.so_sndbuf {
            socket2.set_send_buffer_size(size)?;
        }
        
        if let Some(size) = options.so_rcvbuf {
            socket2.set_recv_buffer_size(size)?;
        }
        
        #[cfg(target_os = "linux")]
        if options.tcp_fastopen {
            unsafe {
                let optval: libc::c_int = 1;
                if libc::setsockopt(
                    socket2.as_raw_fd(),
                    libc::IPPROTO_TCP,
                    libc::TCP_FASTOPEN,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set TCP_FASTOPEN"));
                }
            }
        }
        
        if let Some(prio) = options.priority {
            socket2.set_priority(prio)?;
        }
        
        if let Some(tos) = options.tos {
            #[cfg(target_os = "linux")]
            unsafe {
                if libc::setsockopt(
                    socket2.as_raw_fd(),
                    libc::IPPROTO_IP,
                    libc::IP_TOS,
                    &(tos as libc::c_int) as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set IP_TOS"));
                }
            }
        }
        
        Ok(())
    }
    
    /// Send data with zero-copy when possible
    pub async fn send(&mut self, data: &[u8]) -> Result<usize> {
        // Capture TX timestamp if hardware timestamps are enabled
        if let Some(ref manager) = self.timestamp_manager {
            self.last_tx_timestamp = Some(manager.now());
        }
        
        // In a real implementation, this would use zero-copy mechanisms
        // For now, use standard write
        Ok(self.inner.write(data).await?)
    }
    
    /// Receive data with zero-copy when possible
    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        // In a real implementation, this would use zero-copy mechanisms
        // For now, use standard read
        let n = self.inner.read(buf).await?;
        
        // Capture RX timestamp if hardware timestamps are enabled
        if let Some(ref manager) = self.timestamp_manager {
            self.last_rx_timestamp = Some(manager.now());
        }
        
        Ok(n)
    }
    
    /// Send data with zero-copy
    pub fn send_zero_copy(&mut self, buffer: &ZeroCopyBuffer) -> Result<usize> {
        use std::os::unix::io::AsRawFd;
        
        #[cfg(target_os = "linux")]
        {
            // Capture TX timestamp if hardware timestamps are enabled
            if let Some(ref manager) = self.timestamp_manager {
                self.last_tx_timestamp = Some(manager.now());
            }
            
            // Use sendmsg for zero-copy transfer
            let fd = self.inner.as_raw_fd();
            
            let mut iov = libc::iovec {
                iov_base: buffer.as_ptr() as *mut libc::c_void,
                iov_len: buffer.len(),
            };
            
            let mut msg = libc::msghdr {
                msg_name: std::ptr::null_mut(),
                msg_namelen: 0,
                msg_iov: &mut iov as *mut libc::iovec,
                msg_iovlen: 1,
                msg_control: std::ptr::null_mut(),
                msg_controllen: 0,
                msg_flags: 0,
            };
            
            let sent = unsafe {
                libc::sendmsg(fd, &msg, libc::MSG_ZEROCOPY)
            };
            
            if sent < 0 {
                Err(anyhow!("sendmsg failed: {}", std::io::Error::last_os_error()))
            } else {
                Ok(sent as usize)
            }
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            // Fallback to regular send on non-Linux systems
            futures::executor::block_on(self.send(buffer))
        }
    }
    
    /// Receive data with zero-copy
    pub fn recv_zero_copy(&mut self, buffer: &mut ZeroCopyBuffer) -> Result<usize> {
        use std::os::unix::io::AsRawFd;
        
        #[cfg(target_os = "linux")]
        {
            // Use recvmsg for zero-copy receive
            let fd = self.inner.as_raw_fd();
            
            let mut iov = libc::iovec {
                iov_base: buffer.as_mut_ptr() as *mut libc::c_void,
                iov_len: buffer.capacity(),
            };
            
            let mut msg = libc::msghdr {
                msg_name: std::ptr::null_mut(),
                msg_namelen: 0,
                msg_iov: &mut iov as *mut libc::iovec,
                msg_iovlen: 1,
                msg_control: std::ptr::null_mut(),
                msg_controllen: 0,
                msg_flags: 0,
            };
            
            let received = unsafe {
                libc::recvmsg(fd, &mut msg, 0)
            };
            
            if received < 0 {
                Err(anyhow!("recvmsg failed: {}", std::io::Error::last_os_error()))
            } else {
                unsafe {
                    buffer.set_len(received as usize);
                }
                
                // Capture RX timestamp if hardware timestamps are enabled
                if let Some(ref manager) = self.timestamp_manager {
                    self.last_rx_timestamp = Some(manager.now());
                }
                
                Ok(received as usize)
            }
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            // Fallback to regular recv on non-Linux systems
            let mut temp_buf = vec![0u8; buffer.capacity()];
            let n = futures::executor::block_on(self.recv(&mut temp_buf))?;
            buffer.copy_from_slice(&temp_buf[..n])?;
            Ok(n)
        }
    }
    
    /// Get the last TX timestamp
    pub fn last_tx_timestamp(&self) -> Option<HardwareTimestamp> {
        self.last_tx_timestamp
    }
    
    /// Get the last RX timestamp
    pub fn last_rx_timestamp(&self) -> Option<HardwareTimestamp> {
        self.last_rx_timestamp
    }
    
    /// Measure round-trip time
    pub fn measure_rtt(&self) -> Option<std::time::Duration> {
        match (self.last_tx_timestamp, self.last_rx_timestamp) {
            (Some(tx), Some(rx)) => {
                if let Some(ref manager) = self.timestamp_manager {
                    Some(manager.measure_latency(&tx, &rx))
                } else {
                    Some(rx.diff(&tx))
                }
            }
            _ => None,
        }
    }
    
    /// Get the inner TCP stream
    pub fn inner(&self) -> &TcpStream {
        &self.inner
    }
    
    /// Get a mutable reference to the inner TCP stream
    pub fn inner_mut(&mut self) -> &mut TcpStream {
        &mut self.inner
    }
    
    /// Get the socket options
    pub fn options(&self) -> &SocketOptions {
        &self.options
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_options_default() {
        let options = SocketOptions::default();
        assert!(options.tcp_nodelay);
        assert!(options.tcp_quickack);
        assert!(options.so_reuseaddr);
        assert!(options.so_reuseport);
        assert_eq!(options.so_sndbuf, Some(1024 * 1024));
        assert_eq!(options.so_rcvbuf, Some(1024 * 1024));
        assert!(options.tcp_fastopen);
        assert_eq!(options.priority, None);
        assert_eq!(options.tos, Some(0x10));
        assert!(!options.hardware_timestamps);
        assert_eq!(options.timestamp_config, None);
    }

    #[test]
    fn test_socket_options_optimize_for_ultra_low_latency() {
        let mut options = SocketOptions::default();
        options.optimize_for_ultra_low_latency();
        
        assert!(options.tcp_nodelay);
        assert!(options.tcp_quickack);
        assert!(options.so_reuseaddr);
        assert!(options.so_reuseport);
        assert_eq!(options.so_sndbuf, Some(4 * 1024 * 1024));
        assert_eq!(options.so_rcvbuf, Some(4 * 1024 * 1024));
        assert!(options.tcp_fastopen);
        assert_eq!(options.priority, Some(6));
        assert_eq!(options.tos, Some(0x10));
        assert!(options.hardware_timestamps);
        assert!(options.timestamp_config.is_some());
        assert!(options.tcp_cork);
        assert!(options.tcp_defer_accept);
        assert_eq!(options.tcp_notsent_lowat, Some(16384));
        assert_eq!(options.so_busy_poll, Some(50));
        assert_eq!(options.so_incoming_cpu, Some(0));
        assert!(options.tcp_thin_linear_timeouts);
        assert!(options.tcp_thin_dupack);
    }
}
