//! Configuration schema definitions
//!
//! This module defines the structure of the configuration used by the Solana HFT Bot.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Environment type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentType {
    /// Cloud environment (AWS, GCP, Azure)
    Cloud,
    
    /// Virtual Private Server
    Vps,
    
    /// Bare metal server
    BareMetal,
    
    /// Development environment
    Development,
}

impl Default for EnvironmentType {
    fn default() -> Self {
        Self::Development
    }
}

/// Cloud provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudProvider {
    /// Amazon Web Services
    Aws,
    
    /// Google Cloud Platform
    Gcp,
    
    /// Microsoft Azure
    Azure,
    
    /// Digital Ocean
    DigitalOcean,
    
    /// Other cloud provider
    Other,
}

/// Main bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    /// Bot name
    #[serde(default = "default_bot_name")]
    pub name: String,
    
    /// Environment type
    #[serde(default)]
    pub environment: EnvironmentType,
    
    /// Cloud provider (if applicable)
    pub cloud_provider: Option<CloudProvider>,
    
    /// RPC URL
    #[serde(default = "default_rpc_url")]
    pub rpc_url: String,
    
    /// Websocket URL
    #[serde(default = "default_websocket_url")]
    pub websocket_url: String,
    
    /// Path to keypair file
    pub keypair_path: Option<PathBuf>,
    
    /// Hardware configuration
    #[serde(default)]
    pub hardware: HardwareConfig,
    
    /// Network configuration
    #[serde(default)]
    pub network: NetworkConfig,
    
    /// Module configurations
    #[serde(default)]
    pub modules: ModuleConfigs,
    
    /// Strategy configuration
    #[serde(default)]
    pub strategy: StrategyConfig,
    
    /// Feature flags
    #[serde(default)]
    pub feature_flags: FeatureFlags,
    
    /// Additional custom configuration
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            name: default_bot_name(),
            environment: EnvironmentType::default(),
            cloud_provider: None,
            rpc_url: default_rpc_url(),
            websocket_url: default_websocket_url(),
            keypair_path: None,
            hardware: HardwareConfig::default(),
            network: NetworkConfig::default(),
            modules: ModuleConfigs::default(),
            strategy: StrategyConfig::default(),
            feature_flags: FeatureFlags::default(),
            custom: HashMap::new(),
        }
    }
}

/// Hardware configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConfig {
    /// CPU configuration
    #[serde(default)]
    pub cpu: CpuConfig,
    
    /// Memory configuration
    #[serde(default)]
    pub memory: MemoryConfig,
    
    /// Storage configuration
    #[serde(default)]
    pub storage: StorageConfig,
    
    /// Network interface configuration
    #[serde(default)]
    pub network_interfaces: Vec<NetworkInterfaceConfig>,
    
    /// Hardware profile name
    pub profile: Option<String>,
}

impl Default for HardwareConfig {
    fn default() -> Self {
        Self {
            cpu: CpuConfig::default(),
            memory: MemoryConfig::default(),
            storage: StorageConfig::default(),
            network_interfaces: Vec::new(),
            profile: None,
        }
    }
}

/// CPU configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuConfig {
    /// Number of physical cores to use (null means auto-detect)
    pub physical_cores: Option<usize>,
    
    /// Number of logical threads to use (null means auto-detect)
    pub logical_threads: Option<usize>,
    
    /// CPU frequency in MHz (null means auto-detect)
    pub frequency_mhz: Option<u32>,
    
    /// Whether to use CPU pinning
    #[serde(default)]
    pub use_cpu_pinning: bool,
    
    /// CPU cores to isolate for the bot
    #[serde(default)]
    pub isolated_cores: Vec<usize>,
    
    /// Whether to use NUMA awareness
    #[serde(default)]
    pub numa_aware: bool,
    
    /// NUMA node to use (null means auto-select)
    pub numa_node: Option<i32>,
}

impl Default for CpuConfig {
    fn default() -> Self {
        Self {
            physical_cores: None,
            logical_threads: None,
            frequency_mhz: None,
            use_cpu_pinning: false,
            isolated_cores: Vec::new(),
            numa_aware: false,
            numa_node: None,
        }
    }
}

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Total memory to use in MB (null means auto-detect)
    pub total_mb: Option<usize>,
    
    /// Whether to use huge pages
    #[serde(default)]
    pub use_huge_pages: bool,
    
    /// Huge page size in KB (2048 or 1048576)
    #[serde(default = "default_huge_page_size")]
    pub huge_page_size_kb: usize,
    
    /// Number of huge pages to allocate
    pub huge_pages_count: Option<usize>,
    
    /// Memory allocation strategy
    #[serde(default)]
    pub allocation_strategy: MemoryAllocationStrategy,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            total_mb: None,
            use_huge_pages: false,
            huge_page_size_kb: default_huge_page_size(),
            huge_pages_count: None,
            allocation_strategy: MemoryAllocationStrategy::default(),
        }
    }
}

/// Memory allocation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryAllocationStrategy {
    /// Conservative memory allocation
    Conservative,
    
    /// Balanced memory allocation
    Balanced,
    
    /// Aggressive memory allocation
    Aggressive,
}

impl Default for MemoryAllocationStrategy {
    fn default() -> Self {
        Self::Balanced
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Data directory
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
    
    /// Log directory
    #[serde(default = "default_log_dir")]
    pub log_dir: PathBuf,
    
    /// Cache directory
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    
    /// Maximum disk usage in MB
    pub max_disk_usage_mb: Option<usize>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            log_dir: default_log_dir(),
            cache_dir: default_cache_dir(),
            max_disk_usage_mb: None,
        }
    }
}

/// Network interface configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceConfig {
    /// Interface name
    pub name: String,
    
    /// Whether to use this interface
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// IP address
    pub ip_address: Option<String>,
    
    /// MAC address
    pub mac_address: Option<String>,
    
    /// Link speed in Mbps
    pub speed_mbps: Option<u32>,
    
    /// Whether the interface supports hardware timestamping
    #[serde(default)]
    pub hardware_timestamping: bool,
    
    /// Whether the interface supports TCP/UDP offloading
    #[serde(default)]
    pub tcp_udp_offload: bool,
    
    /// Whether the interface supports RDMA
    #[serde(default)]
    pub rdma_support: bool,
    
    /// Whether the interface supports RSS/RFS
    #[serde(default)]
    pub rss_rfs_support: bool,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Kernel bypass mode
    #[serde(default)]
    pub kernel_bypass_mode: KernelBypassMode,
    
    /// DPDK configuration
    #[serde(default)]
    pub dpdk: DpdkConfig,
    
    /// io_uring configuration
    #[serde(default)]
    pub io_uring: IoUringConfig,
    
    /// TCP optimizations
    #[serde(default)]
    pub tcp_optimizations: TcpOptimizations,
    
    /// Socket buffer configuration
    #[serde(default)]
    pub socket_buffers: SocketBufferConfig,
    
    /// Zero-copy configuration
    #[serde(default)]
    pub zero_copy: ZeroCopyConfig,
    
    /// Endpoint configuration
    #[serde(default)]
    pub endpoints: EndpointConfig,
    
    /// Latency optimization configuration
    #[serde(default)]
    pub latency_optimization: LatencyOptimizationConfig,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            kernel_bypass_mode: KernelBypassMode::default(),
            dpdk: DpdkConfig::default(),
            io_uring: IoUringConfig::default(),
            tcp_optimizations: TcpOptimizations::default(),
            socket_buffers: SocketBufferConfig::default(),
            zero_copy: ZeroCopyConfig::default(),
            endpoints: EndpointConfig::default(),
            latency_optimization: LatencyOptimizationConfig::default(),
        }
    }
}

/// Kernel bypass mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KernelBypassMode {
    /// No kernel bypass
    None,
    
    /// DPDK for kernel bypass
    Dpdk,
    
    /// io_uring for async I/O
    IoUring,
    
    /// Both DPDK and io_uring
    DpdkAndIoUring,
}

impl Default for KernelBypassMode {
    fn default() -> Self {
        Self::None
    }
}

/// DPDK configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpdkConfig {
    /// Whether to use DPDK
    #[serde(default)]
    pub enabled: bool,
    
    /// EAL arguments
    #[serde(default)]
    pub eal_args: Vec<String>,
    
    /// Port ID to use
    #[serde(default)]
    pub port_id: u16,
    
    /// Number of RX queues
    #[serde(default = "default_dpdk_queues")]
    pub rx_queues: u16,
    
    /// Number of TX queues
    #[serde(default = "default_dpdk_queues")]
    pub tx_queues: u16,
    
    /// RX ring size
    #[serde(default = "default_dpdk_ring_size")]
    pub rx_ring_size: u16,
    
    /// TX ring size
    #[serde(default = "default_dpdk_ring_size")]
    pub tx_ring_size: u16,
    
    /// Memory pool size
    #[serde(default = "default_dpdk_mempool_size")]
    pub mempool_size: u32,
    
    /// Memory pool cache size
    #[serde(default = "default_dpdk_mempool_cache_size")]
    pub mempool_cache_size: u32,
    
    /// Memory pool socket ID
    #[serde(default)]
    pub mempool_socket_id: i32,
}

impl Default for DpdkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            eal_args: vec![
                "solana-hft-bot".to_string(),
                "-l".to_string(), "0-3".to_string(),
                "--huge-dir".to_string(), "/mnt/huge".to_string(),
                "--socket-mem".to_string(), "1024,0".to_string(),
            ],
            port_id: 0,
            rx_queues: default_dpdk_queues(),
            tx_queues: default_dpdk_queues(),
            rx_ring_size: default_dpdk_ring_size(),
            tx_ring_size: default_dpdk_ring_size(),
            mempool_size: default_dpdk_mempool_size(),
            mempool_cache_size: default_dpdk_mempool_cache_size(),
            mempool_socket_id: 0,
        }
    }
}

/// io_uring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoUringConfig {
    /// Whether to use io_uring
    #[serde(default)]
    pub enabled: bool,
    
    /// Number of entries in the submission queue
    #[serde(default = "default_io_uring_entries")]
    pub sq_entries: u32,
    
    /// Number of entries in the completion queue
    #[serde(default = "default_io_uring_entries")]
    pub cq_entries: u32,
    
    /// Flags for io_uring setup
    #[serde(default)]
    pub flags: u32,
}

impl Default for IoUringConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sq_entries: default_io_uring_entries(),
            cq_entries: default_io_uring_entries(),
            flags: 0,
        }
    }
}

/// TCP optimizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpOptimizations {
    /// Whether to enable TCP_NODELAY
    #[serde(default = "default_true")]
    pub tcp_nodelay: bool,
    
    /// Whether to enable TCP_QUICKACK
    #[serde(default = "default_true")]
    pub tcp_quickack: bool,
    
    /// Whether to enable TCP_FASTOPEN
    #[serde(default = "default_true")]
    pub tcp_fastopen: bool,
    
    /// Whether to enable TCP_CORK
    #[serde(default)]
    pub tcp_cork: bool,
    
    /// Whether to enable TCP_DEFER_ACCEPT
    #[serde(default)]
    pub tcp_defer_accept: bool,
    
    /// Keep-alive interval in seconds
    #[serde(default = "default_keepalive_interval")]
    pub keepalive_interval_secs: u32,
    
    /// TCP connection backlog
    #[serde(default = "default_tcp_backlog")]
    pub connection_backlog: i32,
}

impl Default for TcpOptimizations {
    fn default() -> Self {
        Self {
            tcp_nodelay: default_true(),
            tcp_quickack: default_true(),
            tcp_fastopen: default_true(),
            tcp_cork: false,
            tcp_defer_accept: false,
            keepalive_interval_secs: default_keepalive_interval(),
            connection_backlog: default_tcp_backlog(),
        }
    }
}

/// Socket buffer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketBufferConfig {
    /// Send buffer size in bytes
    #[serde(default = "default_socket_buffer_size")]
    pub send_buffer_size: usize,
    
    /// Receive buffer size in bytes
    #[serde(default = "default_socket_buffer_size")]
    pub recv_buffer_size: usize,
    
    /// Whether to use dynamic buffer sizing
    #[serde(default)]
    pub dynamic_buffer_sizing: bool,
    
    /// Maximum buffer size in bytes
    #[serde(default = "default_max_socket_buffer_size")]
    pub max_buffer_size: usize,
}

impl Default for SocketBufferConfig {
    fn default() -> Self {
        Self {
            send_buffer_size: default_socket_buffer_size(),
            recv_buffer_size: default_socket_buffer_size(),
            dynamic_buffer_sizing: false,
            max_buffer_size: default_max_socket_buffer_size(),
        }
    }
}

/// Zero-copy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroCopyConfig {
    /// Whether to enable zero-copy networking
    #[serde(default)]
    pub enabled: bool,
    
    /// Memory map size in bytes
    #[serde(default = "default_zero_copy_map_size")]
    pub memory_map_size: usize,
    
    /// Whether to use hardware offloading
    #[serde(default)]
    pub hardware_offloading: bool,
    
    /// Whether to use direct memory access
    #[serde(default)]
    pub direct_memory_access: bool,
}

impl Default for ZeroCopyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            memory_map_size: default_zero_copy_map_size(),
            hardware_offloading: false,
            direct_memory_access: false,
        }
    }
}

/// Endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// RPC endpoints
    #[serde(default)]
    pub rpc_endpoints: Vec<RpcEndpoint>,
    
    /// Websocket endpoints
    #[serde(default)]
    pub websocket_endpoints: Vec<String>,
    
    /// Endpoint selection strategy
    #[serde(default)]
    pub selection_strategy: EndpointSelectionStrategy,
    
    /// Connection pool size per endpoint
    #[serde(default = "default_connection_pool_size")]
    pub connection_pool_size: usize,
    
    /// Connection timeout in milliseconds
    #[serde(default = "default_connection_timeout_ms")]
    pub connection_timeout_ms: u64,
    
    /// Connection keep-alive interval in milliseconds
    #[serde(default = "default_keepalive_interval_ms")]
    pub keepalive_interval_ms: u64,
    
    /// Maximum concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    
    /// Endpoint health check interval in milliseconds
    #[serde(default = "default_health_check_interval_ms")]
    pub health_check_interval_ms: u64,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            rpc_endpoints: vec![
                RpcEndpoint {
                    url: "https://api.mainnet-beta.solana.com".to_string(),
                    weight: 1,
                    region: "global".to_string(),
                    features: Vec::new(),
                    tier: "free".to_string(),
                },
            ],
            websocket_endpoints: vec![
                "wss://api.mainnet-beta.solana.com".to_string(),
            ],
            selection_strategy: EndpointSelectionStrategy::default(),
            connection_pool_size: default_connection_pool_size(),
            connection_timeout_ms: default_connection_timeout_ms(),
            keepalive_interval_ms: default_keepalive_interval_ms(),
            max_connections: default_max_connections(),
            health_check_interval_ms: default_health_check_interval_ms(),
        }
    }
}

/// RPC endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcEndpoint {
    /// Endpoint URL
    pub url: String,
    
    /// Endpoint weight for load balancing
    #[serde(default = "default_endpoint_weight")]
    pub weight: u32,
    
    /// Geographic region
    #[serde(default = "default_endpoint_region")]
    pub region: String,
    
    /// Supported features
    #[serde(default)]
    pub features: Vec<String>,
    
    /// Service tier
    #[serde(default = "default_endpoint_tier")]
    pub tier: String,
}

/// Endpoint selection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EndpointSelectionStrategy {
    /// Round-robin selection
    RoundRobin,
    
    /// Random selection
    Random,
    
    /// Weighted random selection
    WeightedRandom,
    
    /// Lowest latency selection
    LowestLatency,
    
    /// Geographic proximity selection
    GeographicProximity,
}

impl Default for EndpointSelectionStrategy {
    fn default() -> Self {
        Self::WeightedRandom
    }
}

/// Latency optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyOptimizationConfig {
    /// Whether to enable latency optimization
    #[serde(default)]
    pub enabled: bool,
    
    /// Whether to build a validator proximity map
    #[serde(default)]
    pub build_validator_proximity_map: bool,
    
    /// Whether to optimize network routes
    #[serde(default)]
    pub optimize_routes: bool,
    
    /// Whether to use connection keep-alive management
    #[serde(default = "default_true")]
    pub connection_keepalive_management: bool,
    
    /// Whether to use request coalescing and batching
    #[serde(default)]
    pub request_coalescing: bool,
    
    /// Whether to use adaptive timeout configuration
    #[serde(default)]
    pub adaptive_timeouts: bool,
    
    /// Base timeout in milliseconds
    #[serde(default = "default_base_timeout_ms")]
    pub base_timeout_ms: u64,
    
    /// Maximum timeout in milliseconds
    #[serde(default = "default_max_timeout_ms")]
    pub max_timeout_ms: u64,
}

impl Default for LatencyOptimizationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            build_validator_proximity_map: false,
            optimize_routes: false,
            connection_keepalive_management: default_true(),
            request_coalescing: false,
            adaptive_timeouts: false,
            base_timeout_ms: default_base_timeout_ms(),
            max_timeout_ms: default_max_timeout_ms(),
        }
    }
}

/// Module configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfigs {
    /// Network module configuration
    #[serde(default)]
    pub network: ModuleConfig,
    
    /// RPC module configuration
    #[serde(default)]
    pub rpc: ModuleConfig,
    
    /// Screening module configuration
    #[serde(default)]
    pub screening: ModuleConfig,
    
    /// Execution module configuration
    #[serde(default)]
    pub execution: ModuleConfig,
    
    /// Arbitrage module configuration
    #[serde(default)]
    pub arbitrage: ModuleConfig,
    
    /// Risk module configuration
    #[serde(default)]
    pub risk: ModuleConfig,
}

impl Default for ModuleConfigs {
    fn default() -> Self {
        Self {
            network: ModuleConfig::default(),
            rpc: ModuleConfig::default(),
            screening: ModuleConfig::default(),
            execution: ModuleConfig::default(),
            arbitrage: ModuleConfig::default(),
            risk: ModuleConfig::default(),
        }
    }
}

/// Module configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// Whether the module is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Module-specific configuration
    pub config: Option<serde_json::Value>,
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            config: None,
        }
    }
}

/// Strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Whether strategies are enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Strategy run interval in milliseconds
    #[serde(default = "default_strategy_run_interval_ms")]
    pub run_interval_ms: u64,
    
    /// Strategies
    #[serde(default)]
    pub strategies: Vec<Strategy>,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            run_interval_ms: default_strategy_run_interval_ms(),
            strategies: Vec::new(),
        }
    }
}

/// Strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    /// Strategy ID
    pub id: String,
    
    /// Strategy name
    pub name: String,
    
    /// Whether the strategy is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Strategy type
    pub strategy_type: String,
    
    /// Strategy parameters
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Networking features
    #[serde(default)]
    pub networking: NetworkingFeatureFlags,
    
    /// CPU features
    #[serde(default)]
    pub cpu: CpuFeatureFlags,
    
    /// Memory features
    #[serde(default)]
    pub memory: MemoryFeatureFlags,
    
    /// Additional capabilities
    #[serde(default)]
    pub capabilities: CapabilityFeatureFlags,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            networking: NetworkingFeatureFlags::default(),
            cpu: CpuFeatureFlags::default(),
            memory: MemoryFeatureFlags::default(),
            capabilities: CapabilityFeatureFlags::default(),
        }
    }
}

/// Networking feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkingFeatureFlags {
    /// Whether kernel bypass networking is available
    #[serde(default)]
    pub kernel_bypass_networking: bool,
    
    /// Whether hardware timestamping is available
    #[serde(default)]
    pub hardware_timestamping: bool,
    
    /// Whether high-frequency networking is available
    #[serde(default)]
    pub high_frequency_networking: bool,
    
    /// Whether DPDK is supported
    #[serde(default)]
    pub dpdk_support: bool,
    
    /// Whether io_uring is supported
    #[serde(default)]
    pub io_uring_support: bool,
}

impl Default for NetworkingFeatureFlags {
    fn default() -> Self {
        Self {
            kernel_bypass_networking: false,
            hardware_timestamping: false,
            high_frequency_networking: false,
            dpdk_support: false,
            io_uring_support: false,
        }
    }
}

/// CPU feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuFeatureFlags {
    /// Whether CPU pinning is available
    #[serde(default)]
    pub cpu_pinning: bool,
    
    /// Whether SIMD optimizations are available
    #[serde(default)]
    pub simd_optimizations: bool,
    
    /// Whether AVX is supported
    #[serde(default)]
    pub avx_support: bool,
    
    /// Whether AVX2 is supported
    #[serde(default)]
    pub avx2_support: bool,
    
    /// Whether AVX-512 is supported
    #[serde(default)]
    pub avx512_support: bool,
}

impl Default for CpuFeatureFlags {
    fn default() -> Self {
        Self {
            cpu_pinning: false,
            simd_optimizations: false,
            avx_support: false,
            avx2_support: false,
            avx512_support: false,
        }
    }
}

/// Memory feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFeatureFlags {
    /// Whether huge pages are available
    #[serde(default)]
    pub huge_pages: bool,
    
    /// Whether 1GB huge pages are available
    #[serde(default)]
    pub huge_pages_1gb: bool,
}

impl Default for MemoryFeatureFlags {
    fn default() -> Self {
        Self {
            huge_pages: false,
            huge_pages_1gb: false,
        }
    }
}

/// Capability feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityFeatureFlags {
    /// Whether NUMA awareness is available
    #[serde(default)]
    pub numa_awareness: bool,
    
    /// Whether direct memory access is available
    #[serde(default)]
    pub direct_memory_access: bool,
    
    /// Whether CPU isolation is available
    #[serde(default)]
    pub isolated_cpus: bool,
}

impl Default for CapabilityFeatureFlags {
    fn default() -> Self {
        Self {
            numa_awareness: false,
            direct_memory_access: false,
            isolated_cpus: false,
        }
    }
}

// Default functions

fn default_bot_name() -> String {
    "Solana HFT Bot".to_string()
}

fn default_rpc_url() -> String {
    "https://api.mainnet-beta.solana.com".to_string()
}

fn default_websocket_url() -> String {
    "wss://api.mainnet-beta.solana.com".to_string()
}

fn default_huge_page_size() -> usize {
    2048
}

fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("solana-hft-bot")
}

fn default_log_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("solana-hft-bot")
        .join("logs")
}

fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("solana-hft-bot")
}

fn default_true() -> bool {
    true
}

fn default_dpdk_queues() -> u16 {
    4
}

fn default_dpdk_ring_size() -> u16 {
    1024
}

fn default_dpdk_mempool_size() -> u32 {
    8192
}

fn default_dpdk_mempool_cache_size() -> u32 {
    256
}

fn default_io_uring_entries() -> u32 {
    4096
}

fn default_keepalive_interval() -> u32 {
    15
}

fn default_tcp_backlog() -> i32 {
    128
}

fn default_socket_buffer_size() -> usize {
    1024 * 1024 // 1MB
}

fn default_max_socket_buffer_size() -> usize {
    16 * 1024 * 1024 // 16MB
}

fn default_zero_copy_map_size() -> usize {
    64 * 1024 * 1024 // 64MB
}

fn default_connection_pool_size() -> usize {
    5
}

fn default_connection_timeout_ms() -> u64 {
    30000 // 30 seconds
}

fn default_keepalive_interval_ms() -> u64 {
    15000 // 15 seconds
}

fn default_max_connections() -> usize {
    1000
}

fn default_health_check_interval_ms() -> u64 {
    30000 // 30 seconds
}

fn default_endpoint_weight() -> u32 {
    1
}

fn default_endpoint_region() -> String {
    "global".to_string()
}

fn default_endpoint_tier() -> String {
    "free".to_string()
}

fn default_base_timeout_ms() -> u64 {
    5000 // 5 seconds
}

fn default_max_timeout_ms() -> u64 {
    60000 // 60 seconds
}

fn default_strategy_run_interval_ms() -> u64 {
    1000 // 1 second
}