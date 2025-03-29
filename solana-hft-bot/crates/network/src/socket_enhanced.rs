//! Enhanced socket implementation for ultra-low latency networking
//! 
//! This module provides a custom socket implementation optimized for
//! high-frequency trading with busy-polling, direct memory access,
//! and reduced context switching.

use anyhow::{anyhow, Result};
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream};
use tracing::{debug, error, info, warn};

use super::timestamps::{HardwareTimestamp, TimestampConfig, TimestampManager, TimestampSource};
use super::zero_copy::ZeroCopyBuffer;

/// Enhanced socket options for ultra-low latency
#[derive(Debug, Clone)]
pub struct EnhancedSocketOptions {
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
    
    /// Enable busy polling mode instead of interrupt-driven
    pub busy_polling_mode: bool,
    
    /// Busy polling interval in microseconds (0 = continuous)
    pub busy_polling_interval_us: u32,
    
    /// Enable zero-copy for send/receive operations
    pub zero_copy: bool,
    
    /// Enable direct memory access (DMA)
    pub direct_memory_access: bool,
    
    /// Enable kernel bypass (requires DPDK)
    pub kernel_bypass: bool,
    
    /// Custom buffer alignment in bytes
    pub buffer_alignment: Option<usize>,
    
    /// Enable TCP_KEEPALIVE
    pub tcp_keepalive: bool,
    
    /// TCP keepalive time in seconds
    pub tcp_keepalive_time: Option<u32>,
    
    /// TCP keepalive interval in seconds
    pub tcp_keepalive_intvl: Option<u32>,
    
    /// TCP keepalive probe count
    pub tcp_keepalive_probes: Option<u32>,
    
    /// Enable TCP_USER_TIMEOUT
    pub tcp_user_timeout: Option<u32>,
    
    /// Enable TCP_CONGESTION control algorithm
    pub tcp_congestion_algorithm: Option<String>,
    
    /// Enable SO_TIMESTAMPING
    pub so_timestamping: bool,
    
    /// Enable SO_TIMESTAMPNS
    pub so_timestampns: bool,
    
    /// Enable SO_BINDTODEVICE
    pub so_bindtodevice: Option<String>,
    
    /// Enable SO_LOCK_FILTER
    pub so_lock_filter: bool,
    
    /// Enable TCP_MAXSEG (Maximum Segment Size)
    pub tcp_maxseg: Option<u32>,
    
    /// Enable TCP_SYNCNT (SYN retries)
    pub tcp_syncnt: Option<u32>,
}

impl Default for EnhancedSocketOptions {
    fn default() -> Self {
        Self {
            tcp_nodelay: true,
            tcp_quickack: true,
            so_reuseaddr: true,
            so_reuseport: true,
            so_sndbuf: Some(4 * 1024 * 1024),  // 4MB
            so_rcvbuf: Some(4 * 1024 * 1024),  // 4MB
            tcp_fastopen: true,
            priority: Some(6), // TC_PRIO_INTERACTIVE
            tos: Some(0x10), // IPTOS_LOWDELAY
            hardware_timestamps: true,
            timestamp_config: Some(TimestampConfig {
                source: TimestampSource::Hardware,
                enable_tx: true,
                enable_rx: true,
            }),
            tcp_cork: false, // Disable by default for low latency
            tcp_defer_accept: true,
            tcp_notsent_lowat: Some(16384), // 16KB
            so_busy_poll: Some(0), // Continuous polling
            so_incoming_cpu: Some(0), // CPU 0
            tcp_thin_linear_timeouts: true,
            tcp_thin_dupack: true,
            busy_polling_mode: true,
            busy_polling_interval_us: 0, // Continuous polling
            zero_copy: true,
            direct_memory_access: true,
            kernel_bypass: false, // Requires DPDK, disabled by default
            buffer_alignment: Some(4096), // Page size alignment
            tcp_keepalive: true,
            tcp_keepalive_time: Some(60), // 60 seconds
            tcp_keepalive_intvl: Some(10), // 10 seconds
            tcp_keepalive_probes: Some(6), // 6 probes
            tcp_user_timeout: Some(30000), // 30 seconds
            tcp_congestion_algorithm: Some("bbr".to_string()), // BBR congestion control
            so_timestamping: true,
            so_timestampns: true,
            so_bindtodevice: None,
            so_lock_filter: true,
            tcp_maxseg: Some(1448), // Optimized MSS
            tcp_syncnt: Some(3), // 3 SYN retries
        }
    }
}

impl EnhancedSocketOptions {
    /// Create options optimized for ultra-low latency
    pub fn ultra_low_latency() -> Self {
        let mut options = Self::default();
        
        // Optimize for ultra-low latency
        options.tcp_nodelay = true;
        options.tcp_quickack = true;
        options.tcp_cork = false;
        options.busy_polling_mode = true;
        options.busy_polling_interval_us = 0; // Continuous polling
        options.zero_copy = true;
        options.direct_memory_access = true;
        options.so_busy_poll = Some(0); // Continuous polling
        options.tcp_congestion_algorithm = Some("bbr".to_string());
        options.tcp_maxseg = Some(1448);
        
        // Increase buffer sizes for high throughput
        options.so_sndbuf = Some(8 * 1024 * 1024); // 8MB
        options.so_rcvbuf = Some(8 * 1024 * 1024); // 8MB
        
        options
    }
    
    /// Create options optimized for high throughput
    pub fn high_throughput() -> Self {
        let mut options = Self::default();
        
        // Optimize for high throughput
        options.tcp_nodelay = false;
        options.tcp_cork = true;
        options.tcp_defer_accept = true;
        options.busy_polling_mode = false;
        options.zero_copy = true;
        options.direct_memory_access = true;
        options.tcp_congestion_algorithm = Some("cubic".to_string());
        options.tcp_maxseg = Some(9000); // Jumbo frames
        
        // Increase buffer sizes for high throughput
        options.so_sndbuf = Some(16 * 1024 * 1024); // 16MB
        options.so_rcvbuf = Some(16 * 1024 * 1024); // 16MB
        
        options
    }
    
    /// Apply all socket options to a socket
    pub fn apply_all(&self, socket: &socket2::Socket) -> Result<()> {
        // Apply basic socket options
        self.apply_basic_options(socket)?;
        
        // Apply Linux-specific options
        #[cfg(target_os = "linux")]
        self.apply_linux_specific_options(socket)?;
        
        Ok(())
    }
    
    /// Apply basic socket options
    fn apply_basic_options(&self, socket: &socket2::Socket) -> Result<()> {
        if self.tcp_nodelay {
            socket.set_nodelay(true)?;
        }
        
        if self.so_reuseaddr {
            socket.set_reuse_address(true)?;
        }
        
        #[cfg(target_os = "linux")]
        if self.so_reuseport {
            socket.set_reuse_port(true)?;
        }
        
        if let Some(size) = self.so_sndbuf {
            socket.set_send_buffer_size(size)?;
        }
        
        if let Some(size) = self.so_rcvbuf {
            socket.set_recv_buffer_size(size)?;
        }
        
        if let Some(prio) = self.priority {
            socket.set_priority(prio)?;
        }
        
        if self.tcp_keepalive {
            socket.set_keepalive(true)?;
        }
        
        Ok(())
    }
    
    /// Apply Linux-specific socket options
    #[cfg(target_os = "linux")]
    fn apply_linux_specific_options(&self, socket: &socket2::Socket) -> Result<()> {
        use std::os::unix::io::AsRawFd;
        
        let fd = socket.as_raw_fd();
        
        // TCP_QUICKACK
        if self.tcp_quickack {
            unsafe {
                let optval: libc::c_int = 1;
                if libc::setsockopt(
                    fd,
                    libc::IPPROTO_TCP,
                    libc::TCP_QUICKACK,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set TCP_QUICKACK"));
                }
            }
        }
        
        // TCP_FASTOPEN
        if self.tcp_fastopen {
            unsafe {
                let optval: libc::c_int = 1;
                if libc::setsockopt(
                    fd,
                    libc::IPPROTO_TCP,
                    libc::TCP_FASTOPEN,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set TCP_FASTOPEN"));
                }
            }
        }
        
        // IP_TOS
        if let Some(tos) = self.tos {
            unsafe {
                if libc::setsockopt(
                    fd,
                    libc::IPPROTO_IP,
                    libc::IP_TOS,
                    &(tos as libc::c_int) as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set IP_TOS"));
                }
            }
        }
        
        // TCP_CORK
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
        
        // TCP_DEFER_ACCEPT
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
        
        // TCP_NOTSENT_LOWAT
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
        
        // SO_BUSY_POLL
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
        
        // SO_INCOMING_CPU
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
        
        // TCP_THIN_LINEAR_TIMEOUTS
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
        
        // TCP_THIN_DUPACK
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
        
        // TCP_KEEPALIVE options
        if self.tcp_keepalive {
            if let Some(time) = self.tcp_keepalive_time {
                unsafe {
                    let optval: libc::c_int = time as libc::c_int;
                    if libc::setsockopt(
                        fd,
                        libc::IPPROTO_TCP,
                        libc::TCP_KEEPIDLE,
                        &optval as *const _ as *const libc::c_void,
                        std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                    ) < 0 {
                        return Err(anyhow!("Failed to set TCP_KEEPIDLE"));
                    }
                }
            }
            
            if let Some(interval) = self.tcp_keepalive_intvl {
                unsafe {
                    let optval: libc::c_int = interval as libc::c_int;
                    if libc::setsockopt(
                        fd,
                        libc::IPPROTO_TCP,
                        libc::TCP_KEEPINTVL,
                        &optval as *const _ as *const libc::c_void,
                        std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                    ) < 0 {
                        return Err(anyhow!("Failed to set TCP_KEEPINTVL"));
                    }
                }
            }
            
            if let Some(probes) = self.tcp_keepalive_probes {
                unsafe {
                    let optval: libc::c_int = probes as libc::c_int;
                    if libc::setsockopt(
                        fd,
                        libc::IPPROTO_TCP,
                        libc::TCP_KEEPCNT,
                        &optval as *const _ as *const libc::c_void,
                        std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                    ) < 0 {
                        return Err(anyhow!("Failed to set TCP_KEEPCNT"));
                    }
                }
            }
        }
        
        // TCP_USER_TIMEOUT
        if let Some(timeout) = self.tcp_user_timeout {
            unsafe {
                let optval: libc::c_int = timeout as libc::c_int;
                if libc::setsockopt(
                    fd,
                    libc::IPPROTO_TCP,
                    libc::TCP_USER_TIMEOUT,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set TCP_USER_TIMEOUT"));
                }
            }
        }
        
        // TCP_CONGESTION
        if let Some(ref algo) = self.tcp_congestion_algorithm {
            unsafe {
                let algo_cstr = std::ffi::CString::new(algo.as_str()).unwrap();
                if libc::setsockopt(
                    fd,
                    libc::IPPROTO_TCP,
                    libc::TCP_CONGESTION,
                    algo_cstr.as_ptr() as *const libc::c_void,
                    algo.len() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set TCP_CONGESTION"));
                }
            }
        }
        
        // SO_TIMESTAMPING
        if self.so_timestamping {
            unsafe {
                let optval: libc::c_int = (libc::SOF_TIMESTAMPING_RX_HARDWARE | 
                                          libc::SOF_TIMESTAMPING_TX_HARDWARE | 
                                          libc::SOF_TIMESTAMPING_RAW_HARDWARE) as libc::c_int;
                if libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_TIMESTAMPING,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set SO_TIMESTAMPING"));
                }
            }
        }
        
        // SO_TIMESTAMPNS
        if self.so_timestampns {
            unsafe {
                let optval: libc::c_int = 1;
                if libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_TIMESTAMPNS,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set SO_TIMESTAMPNS"));
                }
            }
        }
        
        // SO_BINDTODEVICE
        if let Some(ref device) = self.so_bindtodevice {
            unsafe {
                let device_cstr = std::ffi::CString::new(device.as_str()).unwrap();
                if libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_BINDTODEVICE,
                    device_cstr.as_ptr() as *const libc::c_void,
                    device.len() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set SO_BINDTODEVICE"));
                }
            }
        }
        
        // SO_LOCK_FILTER
        if self.so_lock_filter {
            unsafe {
                let optval: libc::c_int = 1;
                if libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_LOCK_FILTER,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set SO_LOCK_FILTER"));
                }
            }
        }
        
        // TCP_MAXSEG
        if let Some(mss) = self.tcp_maxseg {
            unsafe {
                let optval: libc::c_int = mss as libc::c_int;
                if libc::setsockopt(
                    fd,
                    libc::IPPROTO_TCP,
                    libc::TCP_MAXSEG,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set TCP_MAXSEG"));
                }
            }
        }
        
        // TCP_SYNCNT
        if let Some(syncnt) = self.tcp_syncnt {
            unsafe {
                let optval: libc::c_int = syncnt as libc::c_int;
                if libc::setsockopt(
                    fd,
                    libc::IPPROTO_TCP,
                    libc::TCP_SYNCNT,
                    &optval as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                ) < 0 {
                    return Err(anyhow!("Failed to set TCP_SYNCNT"));
                }
            }
        }
        
        Ok(())
    }
}

/// Enhanced socket implementation for ultra-low latency
pub struct EnhancedSocket {
    /// Inner TCP stream
    inner: TcpStream,
    
    /// Socket options
    options: EnhancedSocketOptions,
    
    /// Timestamp manager
    timestamp_manager: Option<TimestampManager>,
    
    /// Last TX timestamp
    last_tx_timestamp: Option<HardwareTimestamp>,
    
    /// Last RX timestamp
    last_rx_timestamp: Option<HardwareTimestamp>,
    
    /// Busy polling thread handle
    busy_polling_thread: Option<std::thread::JoinHandle<()>>,
    
    /// Busy polling stop flag
    busy_polling_stop: Arc<std::sync::atomic::AtomicBool>,
    
    /// Receive buffer
    rx_buffer: Option<ZeroCopyBuffer>,
    
    /// Send buffer
    tx_buffer: Option<ZeroCopyBuffer>,
}

impl EnhancedSocket {
    /// Connect to a remote address with enhanced options
    pub async fn connect(addr: std::net::SocketAddr, options: EnhancedSocketOptions) -> Result<Self> {
        let socket = match addr {
            std::net::SocketAddr::V4(_) => TcpSocket::new_v4()?,
            std::net::SocketAddr::V6(_) => TcpSocket::new_v6()?,
        };
        
        Self::apply_socket_options(&socket, &options)?;
        
        let stream = socket.connect(addr).await?;
        
        // Further optimize connected socket
        let socket2 = socket2::Socket::from(std::net::TcpStream::from(stream.try_clone().await?));
        
        // Apply all socket options
        options.apply_all(&socket2)?;
        
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
        
        // Create zero-copy buffers if enabled
        let (rx_buffer, tx_buffer) = if options.zero_copy {
            let rx_size = options.so_rcvbuf.unwrap_or(4 * 1024 * 1024);
            let tx_size = options.so_sndbuf.unwrap_or(4 * 1024 * 1024);
            
            let rx_buffer = ZeroCopyBuffer::new(rx_size)?;
            let tx_buffer = ZeroCopyBuffer::new(tx_size)?;
            
            (Some(rx_buffer), Some(tx_buffer))
        } else {
            (None, None)
        };
        
        let busy_polling_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        
        // Create socket
        let mut socket = Self {
            inner: stream,
            options,
            timestamp_manager,
            last_tx_timestamp: None,
            last_rx_timestamp: None,
            busy_polling_thread: None,
            busy_polling_stop,
            rx_buffer,
            tx_buffer,
        };
        
        // Start busy polling if enabled
        if socket.options.busy_polling_mode {
            socket.start_busy_polling()?;
        }
        
        Ok(socket)
    }
    
    /// Apply socket options for optimization
    fn apply_socket_options(socket: &TcpSocket, options: &EnhancedSocketOptions) -> Result<()> {
        let socket2 = socket2::Socket::from(socket.as_raw_fd());
        
        // Apply all socket options
        options.apply_all(&socket2)?;
        
        Ok(())
    }
    
    /// Start busy polling
    fn start_busy_polling(&mut self) -> Result<()> {
        if self.busy_polling_thread.is_some() {
            return Ok(());
        }
        
        let fd = self.inner.as_raw_fd();
        let interval_us = self.options.busy_polling_interval_us;
        let stop_flag = self.busy_polling_stop.clone();
        
        // Create busy polling thread
        let handle = std::thread::Builder::new()
            .name("socket-busy-poll".to_string())
            .spawn(move || {
                info!("Starting busy polling thread for socket {}", fd);
                
                #[cfg(target_os = "linux")]
                {
                    // Pin to CPU 0 by default
                    unsafe {
                        let mut cpu_set = libc::cpu_set_t::default();
                        libc::CPU_ZERO(&mut cpu_set);
                        libc::CPU_SET(0, &mut cpu_set);
                        libc::pthread_setaffinity_np(
                            libc::pthread_self(),
                            std::mem::size_of::<libc::cpu_set_t>(),
                            &cpu_set,
                        );
                    }
                    
                    // Set thread priority to real-time
                    unsafe {
                        let mut param = libc::sched_param {
                            sched_priority: 99, // Maximum real-time priority
                        };
                        libc::pthread_setschedparam(
                            libc::pthread_self(),
                            libc::SCHED_FIFO,
                            &param,
                        );
                    }
                    
                    // Busy polling loop
                    while !stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        // Poll the socket
                        let mut pollfd = libc::pollfd {
                            fd,
                            events: libc::POLLIN | libc::POLLOUT,
                            revents: 0,
                        };
                        
                        unsafe {
                            libc::poll(&mut pollfd, 1, 0);
                        }
                        
                        // Sleep for the specified interval if not continuous
                        if interval_us > 0 {
                            std::thread::sleep(std::time::Duration::from_micros(interval_us as u64));
                        } else {
                            // Yield to other threads
                            std::thread::yield_now();
                        }
                    }
                }
                
                info!("Stopping busy polling thread for socket {}", fd);
            })?;
        
        self.busy_polling_thread = Some(handle);
        
        Ok(())
    }
    
    /// Stop busy polling
    fn stop_busy_polling(&mut self) {
        if let Some(handle) = self.busy_polling_thread.take() {
            // Set stop flag
            self.busy_polling_stop.store(true, std::sync::atomic::Ordering::Relaxed);
            
            // Wait for thread to finish
            if let Err(e) = handle.join() {
                error!("Failed to join busy polling thread: {:?}", e);
            }
        }
    }
    
    /// Send data with zero-copy when possible
    pub async fn send(&mut self, data: &[u8]) -> Result<usize> {
        // Capture TX timestamp if hardware timestamps are enabled
        if let Some(ref manager) = self.timestamp_manager {
            self.last_tx_timestamp = Some(manager.now());
        }
        
        // Use zero-copy if enabled
        if self.options.zero_copy && self.tx_buffer.is_some() {
            let buffer = self.tx_buffer.as_mut().unwrap();
            
            // Copy data to zero-copy buffer
            buffer.clear();
            buffer.extend_from_slice(data)?;
            
            // Send with zero-copy
            self.send_zero_copy(buffer)
        } else {
            // Use standard write
            Ok(self.inner.write(data).await?)
        }
    }
    
    /// Receive data with zero-copy when possible
    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Use zero-copy if enabled
        if self.options.zero_copy && self.rx_buffer.is_some() {
            let buffer = self.rx_buffer.as_mut().unwrap();
            
            // Receive with zero-copy
            let n = self.recv_zero_copy(buffer)?;
            
            // Copy data from zero-copy buffer to output buffer
            let n = std::cmp::min(n, buf.len());
            buf[..n].copy_from_slice(&buffer[..n]);
            
            // Capture RX timestamp if hardware timestamps are enabled
            if let Some(ref manager) = self.timestamp_manager {
                self.last_rx_timestamp = Some(manager.now());
            }
            
            Ok(n)
        } else {
            // Use standard read
            let n = self.inner.read(buf).await?;
            
            // Capture RX timestamp if hardware timestamps are enabled
            if let Some(ref manager) = self.timestamp_manager {
                self.last_rx_timestamp = Some(manager.now());
            }
            
            Ok(n)
        }
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
            futures::executor::block_on(self.inner.write(buffer))
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
            let n = futures::executor::block_on(self.inner.read(&mut temp_buf))?;
            buffer.clear();
            buffer.extend_from_slice(&temp_buf[..n])?;
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
    pub fn options(&self) -> &EnhancedSocketOptions {
        &self.options
    }
    
    /// Set TCP_NODELAY dynamically
    pub fn set_nodelay(&self, nodelay: bool) -> Result<()> {
        let socket = socket2::Socket::from(self.inner.as_raw_fd());
        socket.set_nodelay(nodelay)?;
        Ok(())
    }
    
    /// Set TCP_QUICKACK dynamically
    #[cfg(target_os = "linux")]
    pub fn set_quickack(&self, quickack: bool) -> Result<()> {
        let fd = self.inner.as_raw_fd();
        
        unsafe {
            let optval: libc::c_int = if quickack { 1 } else { 0 };
            if libc::setsockopt(
                fd,
                libc::IPPROTO_TCP,
                libc::TCP_QUICKACK,
                &optval as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            ) < 0 {
                return Err(anyhow!("Failed to set TCP_QUICKACK"));
            }
        }
        
        Ok(())
    }
    
    /// Set TCP_CORK dynamically
    #[cfg(target_os = "linux")]
    pub fn set_cork(&self, cork: bool) -> Result<()> {
        let fd = self.inner.as_raw_fd();
        
        unsafe {
            let optval: libc::c_int = if cork { 1 } else { 0 };
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
        
        Ok(())
    }
}

impl Drop for EnhancedSocket {
    fn drop(&mut self) {
        // Stop busy polling
        self.stop_busy_polling();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_socket_options_default() {
        let options = EnhancedSocketOptions::default();
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
        assert!(!options.tcp_cork);
        assert!(options.tcp_defer_accept);
        assert_eq!(options.tcp_notsent_lowat, Some(16384));
        assert_eq!(options.so_busy_poll, Some(0));
        assert_eq!(options.so_incoming_cpu, Some(0));
        assert!(options.tcp_thin_linear_timeouts);
        assert!(options.tcp_thin_dupack);
        assert!(options.busy_polling_mode);
        assert_eq!(options.busy_polling_interval_us, 0);
        assert!(options.zero_copy);
        assert!(options.direct_memory_access);
        assert!(!options.kernel_bypass);
    }

    #[test]
    fn test_enhanced_socket_options_ultra_low_latency() {
        let options = EnhancedSocketOptions::ultra_low_latency();
        assert!(options.tcp_nodelay);
        assert!(options.tcp_quickack);
        assert!(!options.tcp_cork);
        assert!(options.busy_polling_mode);
        assert_eq!(options.busy_polling_interval_us, 0);
        assert!(options.zero_copy);
        assert!(options.direct_memory_access);
        assert_eq!(options.so_busy_poll, Some(0));
        assert_eq!(options.tcp_congestion_algorithm, Some("bbr".to_string()));
        assert_eq!(options.tcp_maxseg, Some(1448));
        assert_eq!(options.so_sndbuf, Some(8 * 1024 * 1024));
        assert_eq!(options.so_rcvbuf, Some(8 * 1024 * 1024));
    }

    #[test]
    fn test_enhanced_socket_options_high_throughput() {
        let options = EnhancedSocketOptions::high_throughput();
        assert!(!options.tcp_nodelay);
        assert!(options.tcp_cork);
        assert!(options.tcp_defer_accept);
        assert!(!options.busy_polling_mode);
        assert!(options.zero_copy);
        assert!(options.direct_memory_access);
        assert_eq!(options.tcp_congestion_algorithm, Some("cubic".to_string()));
        assert_eq!(options.tcp_maxseg, Some(9000));
        assert_eq!(options.so_sndbuf, Some(16 * 1024 * 1024));
        assert_eq!(options.so_rcvbuf, Some(16 * 1024 * 1024));
    }
}