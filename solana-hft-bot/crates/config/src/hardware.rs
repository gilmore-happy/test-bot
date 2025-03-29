//! Hardware detection and capability management
//!
//! This module provides functionality to detect hardware capabilities
//! and manage hardware-dependent feature flags.

use crate::error::{ConfigError, ConfigResult};
use crate::schema::{
    CpuConfig, CpuFeatureFlags, FeatureFlags, HardwareConfig, MemoryConfig,
    MemoryFeatureFlags, NetworkingFeatureFlags, NetworkInterfaceConfig,
    CapabilityFeatureFlags,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::{CpuExt, DiskExt, NetworkExt, NetworksExt, System, SystemExt};
use tracing::{debug, info, warn};

/// Hardware capabilities detected on the system
#[derive(Debug, Clone)]
pub struct HardwareCapabilities {
    /// CPU information
    pub cpu: CpuInfo,
    
    /// Memory information
    pub memory: MemoryInfo,
    
    /// Storage information
    pub storage: StorageInfo,
    
    /// Network interface information
    pub network_interfaces: Vec<NetworkInterfaceInfo>,
    
    /// Virtualization information
    pub virtualization: VirtualizationInfo,
    
    /// Feature flags based on detected capabilities
    pub feature_flags: HardwareFeatureFlags,
}

impl HardwareCapabilities {
    /// Detect hardware capabilities
    pub fn detect() -> ConfigResult<Self> {
        info!("Detecting hardware capabilities...");
        
        let mut system = System::new_all();
        system.refresh_all();
        
        // Detect CPU information
        let cpu = CpuInfo::detect(&system)?;
        debug!("Detected CPU: {:?}", cpu);
        
        // Detect memory information
        let memory = MemoryInfo::detect(&system)?;
        debug!("Detected memory: {:?}", memory);
        
        // Detect storage information
        let storage = StorageInfo::detect(&system)?;
        debug!("Detected storage: {:?}", storage);
        
        // Detect network interfaces
        let network_interfaces = NetworkInterfaceInfo::detect_all(&system)?;
        debug!("Detected network interfaces: {:?}", network_interfaces);
        
        // Detect virtualization
        let virtualization = VirtualizationInfo::detect()?;
        debug!("Detected virtualization: {:?}", virtualization);
        
        // Generate feature flags based on detected hardware
        let feature_flags = HardwareFeatureFlags::from_detected_hardware(
            &cpu, &memory, &network_interfaces, &virtualization,
        );
        debug!("Generated feature flags: {:?}", feature_flags);
        
        info!("Hardware detection complete");
        
        Ok(Self {
            cpu,
            memory,
            storage,
            network_interfaces,
            virtualization,
            feature_flags,
        })
    }
    
    /// Create a hardware configuration based on detected capabilities
    pub fn to_hardware_config(&self) -> HardwareConfig {
        HardwareConfig {
            cpu: CpuConfig {
                physical_cores: Some(self.cpu.physical_cores),
                logical_threads: Some(self.cpu.logical_threads),
                frequency_mhz: Some(self.cpu.frequency_mhz),
                use_cpu_pinning: self.feature_flags.cpu_pinning,
                isolated_cores: Vec::new(),
                numa_aware: self.feature_flags.numa_awareness,
                numa_node: None,
            },
            memory: MemoryConfig {
                total_mb: Some(self.memory.total_mb),
                use_huge_pages: self.feature_flags.huge_pages,
                huge_page_size_kb: if self.feature_flags.huge_pages_1gb { 1048576 } else { 2048 },
                huge_pages_count: None,
                allocation_strategy: Default::default(),
            },
            storage: Default::default(),
            network_interfaces: self.network_interfaces
                .iter()
                .map(|iface| NetworkInterfaceConfig {
                    name: iface.name.clone(),
                    enabled: true,
                    ip_address: iface.ip_address.clone(),
                    mac_address: Some(iface.mac_address.clone()),
                    speed_mbps: iface.speed_mbps,
                    hardware_timestamping: iface.hardware_timestamping,
                    tcp_udp_offload: iface.tcp_udp_offload,
                    rdma_support: iface.rdma_support,
                    rss_rfs_support: iface.rss_rfs_support,
                })
                .collect(),
            profile: None,
        }
    }
    
    /// Create feature flags based on detected capabilities
    pub fn to_feature_flags(&self) -> FeatureFlags {
        FeatureFlags {
            networking: NetworkingFeatureFlags {
                kernel_bypass_networking: self.feature_flags.kernel_bypass_networking,
                hardware_timestamping: self.feature_flags.hardware_timestamping,
                high_frequency_networking: self.feature_flags.high_frequency_networking,
                dpdk_support: self.feature_flags.dpdk_support,
                io_uring_support: self.feature_flags.io_uring_support,
            },
            cpu: CpuFeatureFlags {
                cpu_pinning: self.feature_flags.cpu_pinning,
                simd_optimizations: self.feature_flags.simd_optimizations,
                avx_support: self.feature_flags.avx_support,
                avx2_support: self.feature_flags.avx2_support,
                avx512_support: self.feature_flags.avx512_support,
            },
            memory: MemoryFeatureFlags {
                huge_pages: self.feature_flags.huge_pages,
                huge_pages_1gb: self.feature_flags.huge_pages_1gb,
            },
            capabilities: CapabilityFeatureFlags {
                numa_awareness: self.feature_flags.numa_awareness,
                direct_memory_access: self.feature_flags.direct_memory_access,
                isolated_cpus: self.feature_flags.isolated_cpus,
            },
        }
    }
}

/// CPU information
#[derive(Debug, Clone)]
pub struct CpuInfo {
    /// CPU vendor
    pub vendor: String,
    
    /// CPU model
    pub model: String,
    
    /// CPU architecture
    pub architecture: String,
    
    /// Number of physical cores
    pub physical_cores: usize,
    
    /// Number of logical threads
    pub logical_threads: usize,
    
    /// CPU frequency in MHz
    pub frequency_mhz: u32,
    
    /// L1 cache size in KB
    pub l1_cache_kb: Option<usize>,
    
    /// L2 cache size in KB
    pub l2_cache_kb: Option<usize>,
    
    /// L3 cache size in KB
    pub l3_cache_kb: Option<usize>,
    
    /// CPU flags
    pub flags: Vec<String>,
    
    /// NUMA topology
    pub numa_topology: Option<NumaTopology>,
}

impl CpuInfo {
    /// Detect CPU information
    pub fn detect(system: &System) -> ConfigResult<Self> {
        let cpus = system.cpus();
        
        if cpus.is_empty() {
            return Err(ConfigError::HardwareDetectionError(
                "No CPUs detected".to_string(),
            ));
        }
        
        // Get CPU model from the first CPU
        let model = cpus[0].brand().to_string();
        
        // Determine vendor
        let vendor = if model.contains("Intel") {
            "Intel".to_string()
        } else if model.contains("AMD") {
            "AMD".to_string()
        } else {
            "Unknown".to_string()
        };
        
        // Determine architecture
        let architecture = if cfg!(target_arch = "x86_64") {
            "x86_64".to_string()
        } else if cfg!(target_arch = "aarch64") {
            "aarch64".to_string()
        } else {
            "unknown".to_string()
        };
        
        // Get physical cores and logical threads
        let logical_threads = cpus.len();
        let physical_cores = num_cpus::get_physical();
        
        // Get CPU frequency
        let frequency_mhz = cpus[0].frequency() as u32;
        
        // Get CPU flags
        let flags = Self::detect_cpu_flags();
        
        // Detect NUMA topology
        let numa_topology = Self::detect_numa_topology();
        
        Ok(Self {
            vendor,
            model,
            architecture,
            physical_cores,
            logical_threads,
            frequency_mhz,
            l1_cache_kb: None, // Would require additional detection
            l2_cache_kb: None, // Would require additional detection
            l3_cache_kb: None, // Would require additional detection
            flags,
            numa_topology,
        })
    }
    
    /// Detect CPU flags
    fn detect_cpu_flags() -> Vec<String> {
        let mut flags = Vec::new();
        
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx") {
                flags.push("avx".to_string());
            }
            if is_x86_feature_detected!("avx2") {
                flags.push("avx2".to_string());
            }
            if is_x86_feature_detected!("avx512f") {
                flags.push("avx512f".to_string());
            }
            if is_x86_feature_detected!("sse") {
                flags.push("sse".to_string());
            }
            if is_x86_feature_detected!("sse2") {
                flags.push("sse2".to_string());
            }
            if is_x86_feature_detected!("sse3") {
                flags.push("sse3".to_string());
            }
            if is_x86_feature_detected!("sse4.1") {
                flags.push("sse4.1".to_string());
            }
            if is_x86_feature_detected!("sse4.2") {
                flags.push("sse4.2".to_string());
            }
            if is_x86_feature_detected!("aes") {
                flags.push("aes".to_string());
            }
            if is_x86_feature_detected!("pclmulqdq") {
                flags.push("pclmulqdq".to_string());
            }
            if is_x86_feature_detected!("rdrand") {
                flags.push("rdrand".to_string());
            }
        }
        
        flags
    }
    
    /// Detect NUMA topology
    fn detect_numa_topology() -> Option<NumaTopology> {
        #[cfg(target_os = "linux")]
        {
            // On Linux, we could parse /sys/devices/system/node/
            // For now, just check if there are multiple NUMA nodes
            if std::path::Path::new("/sys/devices/system/node/node1").exists() {
                // Simple detection - just count the number of NUMA nodes
                let mut node_count = 0;
                while std::path::Path::new(&format!("/sys/devices/system/node/node{}", node_count)).exists() {
                    node_count += 1;
                }
                
                if node_count > 1 {
                    return Some(NumaTopology {
                        node_count,
                        nodes: Vec::new(), // Detailed node info would require more parsing
                    });
                }
            }
        }
        
        None
    }
    
    /// Check if CPU supports AVX
    pub fn has_avx(&self) -> bool {
        self.flags.contains(&"avx".to_string())
    }
    
    /// Check if CPU supports AVX2
    pub fn has_avx2(&self) -> bool {
        self.flags.contains(&"avx2".to_string())
    }
    
    /// Check if CPU supports AVX-512
    pub fn has_avx512(&self) -> bool {
        self.flags.contains(&"avx512f".to_string())
    }
    
    /// Check if system has NUMA architecture
    pub fn has_numa(&self) -> bool {
        self.numa_topology.is_some()
    }
}

/// NUMA topology
#[derive(Debug, Clone)]
pub struct NumaTopology {
    /// Number of NUMA nodes
    pub node_count: usize,
    
    /// NUMA nodes
    pub nodes: Vec<NumaNode>,
}

/// NUMA node
#[derive(Debug, Clone)]
pub struct NumaNode {
    /// Node ID
    pub id: usize,
    
    /// CPU cores in this node
    pub cpu_cores: Vec<usize>,
    
    /// Memory in MB
    pub memory_mb: usize,
}

/// Memory information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    /// Total memory in MB
    pub total_mb: usize,
    
    /// Available memory in MB
    pub available_mb: usize,
    
    /// Used memory in MB
    pub used_mb: usize,
    
    /// Whether huge pages are supported
    pub huge_pages_supported: bool,
    
    /// Whether 1GB huge pages are supported
    pub huge_pages_1gb_supported: bool,
    
    /// Number of configured huge pages
    pub huge_pages_count: Option<usize>,
    
    /// Huge page size in KB
    pub huge_page_size_kb: Option<usize>,
}

impl MemoryInfo {
    /// Detect memory information
    pub fn detect(system: &System) -> ConfigResult<Self> {
        let total_kb = system.total_memory() / 1024;
        let used_kb = system.used_memory() / 1024;
        let available_kb = total_kb - used_kb;
        
        // Check for huge pages support
        let huge_pages_supported = Self::detect_huge_pages_support();
        let huge_pages_1gb_supported = Self::detect_huge_pages_1gb_support();
        let (huge_pages_count, huge_page_size_kb) = Self::detect_huge_pages_config();
        
        Ok(Self {
            total_mb: (total_kb / 1024) as usize,
            available_mb: (available_kb / 1024) as usize,
            used_mb: (used_kb / 1024) as usize,
            huge_pages_supported,
            huge_pages_1gb_supported,
            huge_pages_count,
            huge_page_size_kb,
        })
    }
    
    /// Detect huge pages support
    fn detect_huge_pages_support() -> bool {
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new("/sys/kernel/mm/hugepages").exists()
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }
    
    /// Detect 1GB huge pages support
    fn detect_huge_pages_1gb_support() -> bool {
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new("/sys/kernel/mm/hugepages/hugepages-1048576kB").exists()
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }
    
    /// Detect huge pages configuration
    fn detect_huge_pages_config() -> (Option<usize>, Option<usize>) {
        #[cfg(target_os = "linux")]
        {
            // Try to read the number of huge pages
            if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
                let mut huge_pages_count = None;
                let mut huge_page_size_kb = None;
                
                for line in content.lines() {
                    if line.starts_with("HugePages_Total:") {
                        if let Some(count_str) = line.split_whitespace().nth(1) {
                            if let Ok(count) = count_str.parse::<usize>() {
                                huge_pages_count = Some(count);
                            }
                        }
                    } else if line.starts_with("Hugepagesize:") {
                        if let Some(size_str) = line.split_whitespace().nth(1) {
                            if let Ok(size) = size_str.parse::<usize>() {
                                huge_page_size_kb = Some(size);
                            }
                        }
                    }
                }
                
                return (huge_pages_count, huge_page_size_kb);
            }
        }
        
        (None, None)
    }
}

/// Storage information
#[derive(Debug, Clone)]
pub struct StorageInfo {
    /// Disks
    pub disks: Vec<DiskInfo>,
    
    /// Total storage in MB
    pub total_mb: usize,
    
    /// Available storage in MB
    pub available_mb: usize,
}

impl StorageInfo {
    /// Detect storage information
    pub fn detect(system: &System) -> ConfigResult<Self> {
        let mut disks = Vec::new();
        let mut total_mb = 0;
        let mut available_mb = 0;
        
        for disk in system.disks() {
            let disk_info = DiskInfo {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total_mb: (disk.total_space() / (1024 * 1024)) as usize,
                available_mb: (disk.available_space() / (1024 * 1024)) as usize,
                disk_type: if disk.is_removable() {
                    DiskType::Removable
                } else {
                    DiskType::Fixed
                },
            };
            
            total_mb += disk_info.total_mb;
            available_mb += disk_info.available_mb;
            
            disks.push(disk_info);
        }
        
        Ok(Self {
            disks,
            total_mb,
            available_mb,
        })
    }
}

/// Disk information
#[derive(Debug, Clone)]
pub struct DiskInfo {
    /// Disk name
    pub name: String,
    
    /// Mount point
    pub mount_point: String,
    
    /// Total space in MB
    pub total_mb: usize,
    
    /// Available space in MB
    pub available_mb: usize,
    
    /// Disk type
    pub disk_type: DiskType,
}

/// Disk type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskType {
    /// Fixed disk
    Fixed,
    
    /// Removable disk
    Removable,
}

/// Network interface information
#[derive(Debug, Clone)]
pub struct NetworkInterfaceInfo {
    /// Interface name
    pub name: String,
    
    /// MAC address
    pub mac_address: String,
    
    /// IP address
    pub ip_address: Option<String>,
    
    /// Link speed in Mbps
    pub speed_mbps: Option<u32>,
    
    /// Whether the interface supports hardware timestamping
    pub hardware_timestamping: bool,
    
    /// Whether the interface supports TCP/UDP offloading
    pub tcp_udp_offload: bool,
    
    /// Whether the interface supports RDMA
    pub rdma_support: bool,
    
    /// Whether the interface supports RSS/RFS
    pub rss_rfs_support: bool,
}

impl NetworkInterfaceInfo {
    /// Detect all network interfaces
    pub fn detect_all(system: &System) -> ConfigResult<Vec<Self>> {
        let mut interfaces = Vec::new();
        
        for (name, data) in system.networks() {
            let mac_address = data.mac_address().to_string();
            
            // Get the first IP address (prefer IPv4)
            let ip_address = if !data.addresses().is_empty() {
                let addr = data.addresses().iter()
                    .find(|addr| addr.ip().is_ipv4())
                    .or_else(|| data.addresses().first());
                
                addr.map(|a| a.ip().to_string())
            } else {
                None
            };
            
            // Detect interface capabilities
            let (
                speed_mbps,
                hardware_timestamping,
                tcp_udp_offload,
                rdma_support,
                rss_rfs_support,
            ) = Self::detect_interface_capabilities(name);
            
            interfaces.push(Self {
                name: name.to_string(),
                mac_address,
                ip_address,
                speed_mbps,
                hardware_timestamping,
                tcp_udp_offload,
                rdma_support,
                rss_rfs_support,
            });
        }
        
        Ok(interfaces)
    }
    
    /// Detect interface capabilities
    fn detect_interface_capabilities(interface_name: &str) -> (Option<u32>, bool, bool, bool, bool) {
        #[cfg(target_os = "linux")]
        {
            // Try to get link speed
            let speed_mbps = if let Ok(content) = std::fs::read_to_string(
                format!("/sys/class/net/{}/speed", interface_name)
            ) {
                content.trim().parse::<u32>().ok()
            } else {
                None
            };
            
            // Check for hardware timestamping
            let hardware_timestamping = std::process::Command::new("ethtool")
                .args(["-T", interface_name])
                .output()
                .map(|output| {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    output_str.contains("hardware-transmit") || output_str.contains("hardware-receive")
                })
                .unwrap_or(false);
            
            // Check for TCP/UDP offloading
            let tcp_udp_offload = std::process::Command::new("ethtool")
                .args(["-k", interface_name])
                .output()
                .map(|output| {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    output_str.contains("tcp-segmentation-offload: on") ||
                    output_str.contains("udp-fragmentation-offload: on")
                })
                .unwrap_or(false);
            
            // Check for RDMA support
            let rdma_support = std::path::Path::new(&format!("/sys/class/infiniband/{}", interface_name)).exists();
            
            // Check for RSS/RFS support
            let rss_rfs_support = std::path::Path::new(&format!("/sys/class/net/{}/queues", interface_name)).exists();
            
            (speed_mbps, hardware_timestamping, tcp_udp_offload, rdma_support, rss_rfs_support)
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            (None, false, false, false, false)
        }
    }
}

/// Virtualization information
#[derive(Debug, Clone)]
pub struct VirtualizationInfo {
    /// Whether the system is virtualized
    pub is_virtualized: bool,
    
    /// Virtualization technology
    pub virtualization_type: Option<VirtualizationType>,
    
    /// Whether paravirtualized drivers are available
    pub has_paravirt_drivers: bool,
}

impl VirtualizationInfo {
    /// Detect virtualization
    pub fn detect() -> ConfigResult<Self> {
        #[cfg(target_os = "linux")]
        {
            // Check for virtualization using systemd-detect-virt
            let virt_type = std::process::Command::new("systemd-detect-virt")
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        let virt_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if virt_str != "none" {
                            Some(virt_str)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });
            
            // If systemd-detect-virt is not available, try other methods
            let virt_type = virt_type.or_else(|| {
                if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
                    if content.contains("hypervisor") {
                        // Generic virtualization detected
                        Some("unknown".to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            
            let (is_virtualized, virtualization_type) = if let Some(virt_str) = virt_type {
                let vtype = match virt_str.as_str() {
                    "kvm" => VirtualizationType::Kvm,
                    "xen" => VirtualizationType::Xen,
                    "vmware" => VirtualizationType::Vmware,
                    "microsoft" => VirtualizationType::Hyper_v,
                    "oracle" => VirtualizationType::VirtualBox,
                    "amazon" => VirtualizationType::Aws,
                    "qemu" => VirtualizationType::Qemu,
                    _ => VirtualizationType::Other(virt_str),
                };
                
                (true, Some(vtype))
            } else {
                (false, None)
            };
            
            // Check for paravirtualized drivers
            let has_paravirt_drivers = if is_virtualized {
                if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
                    content.contains("paravirt") || content.contains("pv_ops")
                } else {
                    false
                }
            } else {
                false
            };
            
            Ok(Self {
                is_virtualized,
                virtualization_type,
                has_paravirt_drivers,
            })
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            // Simple detection for non-Linux platforms
            // This is not very reliable
            let is_virtualized = std::env::var("PROCESSOR_IDENTIFIER")
                .map(|id| id.contains("Virtual") || id.contains("VMware"))
                .unwrap_or(false);
            
            Ok(Self {
                is_virtualized,
                virtualization_type: if is_virtualized {
                    Some(VirtualizationType::Other("unknown".to_string()))
                } else {
                    None
                },
                has_paravirt_drivers: false,
            })
        }
    }
}

/// Virtualization type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VirtualizationType {
    /// KVM
    Kvm,
    
    /// Xen
    Xen,
    
    /// VMware
    Vmware,
    
    /// Hyper-V
    Hyper_v,
    
    /// VirtualBox
    VirtualBox,
    
    /// AWS
    Aws,
    
    /// QEMU
    Qemu,
    
    /// Other virtualization technology
    Other(String),
}

/// Hardware feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareFeatureFlags {
    // Networking features
    /// Whether kernel bypass networking is available
    pub kernel_bypass_networking: bool,
    
    /// Whether hardware timestamping is available
    pub hardware_timestamping: bool,
    
    /// Whether high-frequency networking is available
    pub high_frequency_networking: bool,
    
    /// Whether DPDK is supported
    pub dpdk_support: bool,
    
    /// Whether io_uring is supported
    pub io_uring_support: bool,
    
    // CPU features
    /// Whether CPU pinning is available
    pub cpu_pinning: bool,
    
    /// Whether SIMD optimizations are available
    pub simd_optimizations: bool,
    
    /// Whether AVX is supported
    pub avx_support: bool,
    
    /// Whether AVX2 is supported
    pub avx2_support: bool,
    
    /// Whether AVX-512 is supported
    pub avx512_support: bool,
    
    // Memory features
    /// Whether huge pages are available
    pub huge_pages: bool,
    
    /// Whether 1GB huge pages are available
    pub huge_pages_1gb: bool,
    
    // Additional capabilities
    /// Whether NUMA awareness is available
    pub numa_awareness: bool,
    
    /// Whether direct memory access is available
    pub direct_memory_access: bool,
    
    /// Whether CPU isolation is available
    pub isolated_cpus: bool,
}

impl Default for HardwareFeatureFlags {
    fn default() -> Self {
        Self {
            // Networking features
            kernel_bypass_networking: false,
            hardware_timestamping: false,
            high_frequency_networking: false,
            dpdk_support: false,
            io_uring_support: false,
            
            // CPU features
            cpu_pinning: false,
            simd_optimizations: false,
            avx_support: false,
            avx2_support: false,
            avx512_support: false,
            
            // Memory features
            huge_pages: false,
            huge_pages_1gb: false,
            
            // Additional capabilities
            numa_awareness: false,
            direct_memory_access: false,
            isolated_cpus: false,
        }
    }
}

impl HardwareFeatureFlags {
    /// Create feature flags from detected hardware
    pub fn from_detected_hardware(
        cpu: &CpuInfo,
        memory: &MemoryInfo,
        network_interfaces: &[NetworkInterfaceInfo],
        virtualization: &VirtualizationInfo,
    ) -> Self {
        // Check for hardware timestamping support
        let hardware_timestamping = network_interfaces.iter()
            .any(|iface| iface.hardware_timestamping);
        
        // Check for high-frequency networking support
        let high_frequency_networking = network_interfaces.iter()
            .any(|iface| iface.speed_mbps.unwrap_or(0) >= 10000); // 10 Gbps or higher
        
        // Check for DPDK support
        let dpdk_support = !virtualization.is_virtualized && 
            network_interfaces.iter().any(|iface| iface.tcp_udp_offload);
        
        // Check for io_uring support
        let io_uring_support = cfg!(target_os = "linux") && 
            std::path::Path::new("/usr/include/liburing.h").exists();
        
        // Check for CPU pinning support
        let cpu_pinning = !virtualization.is_virtualized || 
            virtualization.virtualization_type == Some(VirtualizationType::Kvm);
        
        // Check for SIMD optimizations
        let simd_optimizations = cpu.flags.iter().any(|flag| 
            flag.starts_with("sse") || flag.starts_with("avx"));
        
        // Check for AVX support
        let avx_support = cpu.has_avx();
        
        // Check for AVX2 support
        let avx2_support = cpu.has_avx2();
        
        // Check for AVX-512 support
        let avx512_support = cpu.has_avx512();
        
        // Check for huge pages support
        let huge_pages = memory.huge_pages_supported;
        
        // Check for 1GB huge pages support
        let huge_pages_1gb = memory.huge_pages_1gb_supported;
        
        // Check for NUMA awareness
        let numa_awareness = cpu.has_numa();
        
        // Check for direct memory access
        let direct_memory_access = network_interfaces.iter()
            .any(|iface| iface.rdma_support);
        
        // Check for CPU isolation
        let isolated_cpus = cfg!(target_os = "linux") && 
            std::path::Path::new("/sys/devices/system/cpu/isolated").exists();
        
        // Check for kernel bypass networking
        let kernel_bypass_networking = dpdk_support || io_uring_support;
        
        Self {
            kernel_bypass_networking,
            hardware_timestamping,
            high_frequency_networking,
            dpdk_support,
            io_uring_support,
            cpu_pinning,
            simd_optimizations,
            avx_support,
            avx2_support,
            avx512_support,
            huge_pages,
            huge_pages_1gb,
            numa_awareness,
            direct_memory_access,
            isolated_cpus,
        }
    }
    
    /// Check if a feature is enabled
    pub fn is_feature_enabled(&self, feature_name: &str) -> bool {
        match feature_name {
            "kernel_bypass_networking" => self.kernel_bypass_networking,
            "hardware_timestamping" => self.hardware_timestamping,
            "high_frequency_networking" => self.high_frequency_networking,
            "dpdk_support" => self.dpdk_support,
            "io_uring_support" => self.io_uring_support,
            "cpu_pinning" => self.cpu_pinning,
            "simd_optimizations" => self.simd_optimizations,
            "avx_support" => self.avx_support,
            "avx2_support" => self.avx2_support,
            "avx512_support" => self.avx512_support,
            "huge_pages" => self.huge_pages,
            "huge_pages_1gb" => self.huge_pages_1gb,
            "numa_awareness" => self.numa_awareness,
            "direct_memory_access" => self.direct_memory_access,
            "isolated_cpus" => self.isolated_cpus,
            _ => false,
        }
    }
    
    /// Enable a feature
    pub fn enable_feature(&mut self, feature_name: &str) {
        match feature_name {
            "kernel_bypass_networking" => self.kernel_bypass_networking = true,
            "hardware_timestamping" => self.hardware_timestamping = true,
            "high_frequency_networking" => self.high_frequency_networking = true,
            "dpdk_support" => self.dpdk_support = true,
            "io_uring_support" => self.io_uring_support = true,
            "cpu_pinning" => self.cpu_pinning = true,
            "simd_optimizations" => self.simd_optimizations = true,
            "avx_support" => self.avx_support = true,
            "avx2_support" => self.avx2_support = true,
            "avx512_support" => self.avx512_support = true,
            "huge_pages" => self.huge_pages = true,
            "huge_pages_1gb" => self.huge_pages_1gb = true,
            "numa_awareness" => self.numa_awareness = true,
            "direct_memory_access" => self.direct_memory_access = true,
            "isolated_cpus" => self.isolated_cpus = true,
            _ => {},
        }
    }
    
    /// Disable a feature
    pub fn disable_feature(&mut self, feature_name: &str) {
        match feature_name {
            "kernel_bypass_networking" => self.kernel_bypass_networking = false,
            "hardware_timestamping" => self.hardware_timestamping = false,
            "high_frequency_networking" => self.high_frequency_networking = false,
            "dpdk_support" => self.dpdk_support = false,
            "io_uring_support" => self.io_uring_support = false,
            "cpu_pinning" => self.cpu_pinning = false,
            "simd_optimizations" => self.simd_optimizations = false,
            "avx_support" => self.avx_support = false,
            "avx2_support" => self.avx2_support = false,
            "avx512_support" => self.avx512_support = false,
            "huge_pages" => self.huge_pages = false,
            "huge_pages_1gb" => self.huge_pages_1gb = false,
            "numa_awareness" => self.numa_awareness = false,
            "direct_memory_access" => self.direct_memory_access = false,
            "isolated_cpus" => self.isolated_cpus = false,
            _ => {},
        }
    }
    
    /// Get all enabled features
    pub fn enabled_features(&self) -> Vec<String> {
        let mut features = Vec::new();
        
        if self.kernel_bypass_networking { features.push("kernel_bypass_networking".to_string()); }
        if self.hardware_timestamping { features.push("hardware_timestamping".to_string()); }
        if self.high_frequency_networking { features.push("high_frequency_networking".to_string()); }
        if self.dpdk_support { features.push("dpdk_support".to_string()); }
        if self.io_uring_support { features.push("io_uring_support".to_string()); }
        if self.cpu_pinning { features.push("cpu_pinning".to_string()); }
        if self.simd_optimizations { features.push("simd_optimizations".to_string()); }
        if self.avx_support { features.push("avx_support".to_string()); }
        if self.avx2_support { features.push("avx2_support".to_string()); }
        if self.avx512_support { features.push("avx512_support".to_string()); }
        if self.huge_pages { features.push("huge_pages".to_string()); }
        if self.huge_pages_1gb { features.push("huge_pages_1gb".to_string()); }
        if self.numa_awareness { features.push("numa_awareness".to_string()); }
        if self.direct_memory_access { features.push("direct_memory_access".to_string()); }
        if self.isolated_cpus { features.push("isolated_cpus".to_string()); }
        
        features
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hardware_feature_flags_default() {
        let flags = HardwareFeatureFlags::default();
        
        assert!(!flags.kernel_bypass_networking);
        assert!(!flags.hardware_timestamping);
        assert!(!flags.high_frequency_networking);
        assert!(!flags.dpdk_support);
        assert!(!flags.io_uring_support);
        assert!(!flags.cpu_pinning);
        assert!(!flags.simd_optimizations);
        assert!(!flags.avx_support);
        assert!(!flags.avx2_support);
        assert!(!flags.avx512_support);
        assert!(!flags.huge_pages);
        assert!(!flags.huge_pages_1gb);
        assert!(!flags.numa_awareness);
        assert!(!flags.direct_memory_access);
        assert!(!flags.isolated_cpus);
    }
    
    #[test]
    fn test_hardware_feature_flags_enable_disable() {
        let mut flags = HardwareFeatureFlags::default();
        
        // Enable some features
        flags.enable_feature("kernel_bypass_networking");
        flags.enable_feature("hardware_timestamping");
        flags.enable_feature("cpu_pinning");
        
        assert!(flags.kernel_bypass_networking);
        assert!(flags.hardware_timestamping);
        assert!(flags.cpu_pinning);
        assert!(!flags.huge_pages);
        
        // Disable a feature
        flags.disable_feature("hardware_timestamping");
        
        assert!(flags.kernel_bypass_networking);
        assert!(!flags.hardware_timestamping);
        assert!(flags.cpu_pinning);
        
        // Check enabled features
        let enabled = flags.enabled_features();
        assert!(enabled.contains(&"kernel_bypass_networking".to_string()));
        assert!(enabled.contains(&"cpu_pinning".to_string()));
        assert!(!enabled.contains(&"hardware_timestamping".to_string()));
    }
}