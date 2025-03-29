// System Types
export interface SystemStatus {
  status: 'online' | 'offline' | 'degraded';
  uptime: number; // in seconds
  lastRestart: string; // ISO date string
  version: string;
  nodeCount: number;
  activeStrategies: number;
  cpuUsage: number; // percentage
  memoryUsage: number; // percentage
  networkUsage: number; // in Mbps
}

// Hardware Configuration Types
export interface BotConfig {
  name: string;
  environment: 'cloud' | 'vps' | 'bareMetal' | 'development';
  cloudProvider?: 'aws' | 'gcp' | 'azure' | 'digitalOcean' | 'other';
  rpc_url: string;
  websocket_url: string;
  keypair_path?: string;
  hardware: HardwareConfig;
  network: NetworkConfig;
  feature_flags: FeatureFlags;
}

export interface HardwareConfig {
  cpu: CpuConfig;
  memory: MemoryConfig;
  storage: StorageConfig;
  network_interfaces: NetworkInterfaceConfig[];
  profile?: string;
}

export interface CpuConfig {
  physical_cores?: number;
  logical_threads?: number;
  frequency_mhz?: number;
  use_cpu_pinning: boolean;
  isolated_cores: number[];
  numa_aware: boolean;
  numa_node?: number;
}

export interface MemoryConfig {
  total_mb?: number;
  use_huge_pages: boolean;
  huge_page_size_kb: number;
  huge_pages_count?: number;
  allocation_strategy: 'conservative' | 'balanced' | 'aggressive';
}

export interface StorageConfig {
  data_dir: string;
  log_dir: string;
  cache_dir: string;
  max_disk_usage_mb?: number;
}

export interface NetworkInterfaceConfig {
  name: string;
  enabled: boolean;
  ip_address?: string;
  mac_address?: string;
  speed_mbps?: number;
  hardware_timestamping: boolean;
  tcp_udp_offload: boolean;
  rdma_support: boolean;
  rss_rfs_support: boolean;
}

export interface NetworkConfig {
  kernel_bypass_mode: 'none' | 'dpdk' | 'ioUring' | 'dpdkAndIoUring';
  dpdk: DpdkConfig;
  io_uring: IoUringConfig;
  endpoints: EndpointConfig;
}

export interface DpdkConfig {
  enabled: boolean;
  port_id: number;
  rx_queues: number;
  tx_queues: number;
}

export interface IoUringConfig {
  enabled: boolean;
  sq_entries: number;
  cq_entries: number;
}

export interface EndpointConfig {
  rpc_endpoints: RpcEndpoint[];
  websocket_endpoints: string[];
  selection_strategy: 'roundRobin' | 'random' | 'weightedRandom' | 'lowestLatency' | 'geographicProximity';
}

export interface RpcEndpoint {
  url: string;
  weight: number;
  region: string;
  features: string[];
  tier: string;
}

export interface FeatureFlags {
  networking: NetworkingFeatureFlags;
  cpu: CpuFeatureFlags;
  memory: MemoryFeatureFlags;
  capabilities: CapabilityFeatureFlags;
}

export interface NetworkingFeatureFlags {
  kernel_bypass_networking: boolean;
  hardware_timestamping: boolean;
  high_frequency_networking: boolean;
  dpdk_support: boolean;
  io_uring_support: boolean;
}

export interface CpuFeatureFlags {
  cpu_pinning: boolean;
  simd_optimizations: boolean;
  avx_support: boolean;
  avx2_support: boolean;
  avx512_support: boolean;
}

export interface MemoryFeatureFlags {
  huge_pages: boolean;
  huge_pages_1gb: boolean;
}

export interface CapabilityFeatureFlags {
  numa_awareness: boolean;
  direct_memory_access: boolean;
  isolated_cpus: boolean;
}

export interface HardwareCapabilities {
  cpu: CpuInfo;
  memory: MemoryInfo;
  storage: StorageInfo;
  network_interfaces: NetworkInterfaceInfo[];
  virtualization: VirtualizationInfo;
  feature_flags: HardwareFeatureFlags;
}

export interface CpuInfo {
  vendor: string;
  model: string;
  architecture: string;
  physical_cores: number;
  logical_threads: number;
  frequency_mhz: number;
  flags: string[];
}

export interface MemoryInfo {
  total_mb: number;
  available_mb: number;
  used_mb: number;
  huge_pages_supported: boolean;
  huge_pages_1gb_supported: boolean;
}

export interface StorageInfo {
  disks: DiskInfo[];
  total_mb: number;
  available_mb: number;
}

export interface DiskInfo {
  name: string;
  mount_point: string;
  total_mb: number;
  available_mb: number;
  disk_type: 'fixed' | 'removable';
}

export interface NetworkInterfaceInfo {
  name: string;
  mac_address: string;
  ip_address?: string;
  speed_mbps?: number;
  hardware_timestamping: boolean;
  tcp_udp_offload: boolean;
  rdma_support: boolean;
  rss_rfs_support: boolean;
}

export interface VirtualizationInfo {
  is_virtualized: boolean;
  virtualization_type?: string;
  has_paravirt_drivers: boolean;
}

export interface HardwareFeatureFlags {
  kernel_bypass_networking: boolean;
  hardware_timestamping: boolean;
  high_frequency_networking: boolean;
  dpdk_support: boolean;
  io_uring_support: boolean;
  cpu_pinning: boolean;
  simd_optimizations: boolean;
  avx_support: boolean;
  avx2_support: boolean;
  avx512_support: boolean;
  huge_pages: boolean;
  huge_pages_1gb: boolean;
  numa_awareness: boolean;
  direct_memory_access: boolean;
  isolated_cpus: boolean;
}

// Performance Types
export interface LatencyMetrics {
  network: number; // in ms
  execution: number; // in ms
  confirmation: number; // in ms
  average: number; // in ms
}

export interface PerformanceMetrics {
  latency: LatencyMetrics;
  successRate: number; // percentage
  failureRate: number; // percentage
  throughput: number; // transactions per second
  resourceUtilization: {
    cpu: number; // percentage
    memory: number; // percentage
    network: number; // in Mbps
  };
}

export interface TimeSeriesDataPoint {
  timestamp: string; // ISO date string
  value: number;
}

export interface PerformanceHistory {
  latency: TimeSeriesDataPoint[];
  successRate: TimeSeriesDataPoint[];
  throughput: TimeSeriesDataPoint[];
}

// Position Types
export interface Position {
  id: string;
  asset: string;
  strategy: string;
  entryPrice: number;
  currentPrice: number;
  size: number;
  side: 'long' | 'short';
  pnl: number;
  pnlPercentage: number;
  openTime: string; // ISO date string
  status: 'open' | 'closed' | 'liquidated';
  risk: 'low' | 'medium' | 'high';
}

// Strategy Types
export interface Strategy {
  id: string;
  name: string;
  description: string;
  status: 'active' | 'paused' | 'stopped';
  performance: {
    daily: number;
    weekly: number;
    monthly: number;
    allTime: number;
  };
  parameters: {
    [key: string]: number | string | boolean;
  };
  schedule: {
    active: boolean;
    startTime: string; // ISO date string
    endTime: string; // ISO date string
    days: ('monday' | 'tuesday' | 'wednesday' | 'thursday' | 'friday' | 'saturday' | 'sunday')[];
  };
}

// Asset Types
export interface Asset {
  symbol: string;
  name: string;
  balance: number;
  valueUsd: number;
  allocation: number; // percentage of portfolio
  price: number;
  priceChange24h: number; // percentage
  liquidity: number; // in USD
}

export interface AssetHistory {
  timestamp: string; // ISO date string
  totalValue: number;
  assets: {
    [symbol: string]: number; // value in USD
  };
}

// Notification Types
export interface Notification {
  id: string;
  type: 'info' | 'success' | 'warning' | 'error';
  title: string;
  message: string;
  timestamp: string; // ISO date string
  read: boolean;
  link?: string;
}

// User Types
export interface User {
  id: string;
  name: string;
  email: string;
  role: 'admin' | 'user' | 'viewer';
  preferences: {
    theme: 'dark' | 'light';
    notifications: boolean;
    refreshRate: number; // in seconds
  };
}

// API Response Types
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: string; // ISO date string
}

// WebSocket Message Types
export interface WebSocketMessage<T> {
  type: string;
  data: T;
  timestamp: string; // ISO date string
}