//! Utility functions for configuration
//!
//! This module provides utility functions for working with configuration.

use crate::error::{ConfigError, ConfigResult};
use crate::schema::BotConfig;
use std::path::Path;
use std::process::Command;
use tracing::{debug, info, warn};

/// Get system information
pub fn get_system_info() -> HashMap<String, String> {
    let mut info = HashMap::new();
    
    // Get OS information
    if let Ok(os_release) = std::fs::read_to_string("/etc/os-release") {
        for line in os_release.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let value = value.trim_matches('"');
                info.insert(key.to_string(), value.to_string());
            }
        }
    }
    
    // Get kernel information
    if let Ok(output) = Command::new("uname").args(["-r"]).output() {
        if output.status.success() {
            let kernel = String::from_utf8_lossy(&output.stdout).trim().to_string();
            info.insert("KERNEL_VERSION".to_string(), kernel);
        }
    }
    
    // Get CPU information
    if let Ok(cpu_info) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in cpu_info.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                
                if key == "model name" {
                    info.insert("CPU_MODEL".to_string(), value.to_string());
                    break;
                }
            }
        }
    }
    
    // Get memory information
    if let Ok(mem_info) = std::fs::read_to_string("/proc/meminfo") {
        for line in mem_info.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                
                if key == "MemTotal" {
                    info.insert("MEMORY_TOTAL".to_string(), value.to_string());
                    break;
                }
            }
        }
    }
    
    info
}

/// Check if a feature is supported by the hardware
pub fn is_feature_supported(feature: &str) -> bool {
    match feature {
        "kernel_bypass_networking" => is_kernel_bypass_supported(),
        "hardware_timestamping" => is_hardware_timestamping_supported(),
        "high_frequency_networking" => is_high_frequency_networking_supported(),
        "dpdk_support" => is_dpdk_supported(),
        "io_uring_support" => is_io_uring_supported(),
        "cpu_pinning" => is_cpu_pinning_supported(),
        "simd_optimizations" => is_simd_supported(),
        "avx_support" => is_avx_supported(),
        "avx2_support" => is_avx2_supported(),
        "avx512_support" => is_avx512_supported(),
        "huge_pages" => is_huge_pages_supported(),
        "huge_pages_1gb" => is_huge_pages_1gb_supported(),
        "numa_awareness" => is_numa_supported(),
        "direct_memory_access" => is_dma_supported(),
        "isolated_cpus" => is_cpu_isolation_supported(),
        _ => false,
    }
}

/// Check if kernel bypass is supported
fn is_kernel_bypass_supported() -> bool {
    is_dpdk_supported() || is_io_uring_supported()
}

/// Check if hardware timestamping is supported
fn is_hardware_timestamping_supported() -> bool {
    // Check if any network interface supports hardware timestamping
    if let Ok(interfaces) = std::fs::read_dir("/sys/class/net") {
        for interface in interfaces.flatten() {
            let interface_name = interface.file_name();
            let interface_name = interface_name.to_string_lossy();
            
            // Skip loopback interface
            if interface_name == "lo" {
                continue;
            }
            
            // Check if interface supports hardware timestamping
            if let Ok(output) = Command::new("ethtool")
                .args(["-T", &interface_name])
                .output()
            {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    if output_str.contains("hardware-transmit") || output_str.contains("hardware-receive") {
                        return true;
                    }
                }
            }
        }
    }
    
    false
}

/// Check if high-frequency networking is supported
fn is_high_frequency_networking_supported() -> bool {
    // Check if any network interface has a speed of at least 10 Gbps
    if let Ok(interfaces) = std::fs::read_dir("/sys/class/net") {
        for interface in interfaces.flatten() {
            let interface_name = interface.file_name();
            let interface_name = interface_name.to_string_lossy();
            
            // Skip loopback interface
            if interface_name == "lo" {
                continue;
            }
            
            // Check interface speed
            let speed_path = format!("/sys/class/net/{}/speed", interface_name);
            if let Ok(speed) = std::fs::read_to_string(speed_path) {
                if let Ok(speed) = speed.trim().parse::<u32>() {
                    if speed >= 10000 {
                        return true;
                    }
                }
            }
        }
    }
    
    false
}

/// Check if DPDK is supported
fn is_dpdk_supported() -> bool {
    // Check if DPDK is installed
    Command::new("dpdk-devbind.py")
        .arg("--status")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if io_uring is supported
fn is_io_uring_supported() -> bool {
    // Check if io_uring is supported by the kernel
    if let Ok(output) = Command::new("uname").args(["-r"]).output() {
        if output.status.success() {
            let kernel = String::from_utf8_lossy(&output.stdout);
            let kernel_version = kernel.trim().split('.').collect::<Vec<_>>();
            
            if kernel_version.len() >= 2 {
                if let (Ok(major), Ok(minor)) = (kernel_version[0].parse::<u32>(), kernel_version[1].parse::<u32>()) {
                    // io_uring was introduced in Linux 5.1
                    return major > 5 || (major == 5 && minor >= 1);
                }
            }
        }
    }
    
    // Check if liburing is installed
    std::path::Path::new("/usr/include/liburing.h").exists()
}

/// Check if CPU pinning is supported
fn is_cpu_pinning_supported() -> bool {
    // CPU pinning is supported on most systems
    true
}

/// Check if SIMD is supported
fn is_simd_supported() -> bool {
    is_avx_supported() || is_sse_supported()
}

/// Check if AVX is supported
fn is_avx_supported() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        std::is_x86_feature_detected!("avx")
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Check if AVX2 is supported
fn is_avx2_supported() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        std::is_x86_feature_detected!("avx2")
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Check if AVX-512 is supported
fn is_avx512_supported() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        std::is_x86_feature_detected!("avx512f")
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Check if SSE is supported
fn is_sse_supported() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        std::is_x86_feature_detected!("sse") &&
        std::is_x86_feature_detected!("sse2") &&
        std::is_x86_feature_detected!("sse3") &&
        std::is_x86_feature_detected!("sse4.1") &&
        std::is_x86_feature_detected!("sse4.2")
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Check if huge pages are supported
fn is_huge_pages_supported() -> bool {
    std::path::Path::new("/sys/kernel/mm/hugepages").exists()
}

/// Check if 1GB huge pages are supported
fn is_huge_pages_1gb_supported() -> bool {
    std::path::Path::new("/sys/kernel/mm/hugepages/hugepages-1048576kB").exists()
}

/// Check if NUMA is supported
fn is_numa_supported() -> bool {
    std::path::Path::new("/sys/devices/system/node/node1").exists()
}

/// Check if DMA is supported
fn is_dma_supported() -> bool {
    // Check if any network interface supports RDMA
    if let Ok(interfaces) = std::fs::read_dir("/sys/class/net") {
        for interface in interfaces.flatten() {
            let interface_name = interface.file_name();
            let interface_name = interface_name.to_string_lossy();
            
            // Check if interface supports RDMA
            let rdma_path = format!("/sys/class/infiniband/{}", interface_name);
            if std::path::Path::new(&rdma_path).exists() {
                return true;
            }
        }
    }
    
    false
}

/// Check if CPU isolation is supported
fn is_cpu_isolation_supported() -> bool {
    std::path::Path::new("/sys/devices/system/cpu/isolated").exists()
}

/// Compare two configurations and return the differences
pub fn diff_configs(old_config: &BotConfig, new_config: &BotConfig) -> HashMap<String, (String, String)> {
    let mut diffs = HashMap::new();
    
    // Compare RPC URL
    if old_config.rpc_url != new_config.rpc_url {
        diffs.insert(
            "rpc_url".to_string(),
            (old_config.rpc_url.clone(), new_config.rpc_url.clone()),
        );
    }
    
    // Compare websocket URL
    if old_config.websocket_url != new_config.websocket_url {
        diffs.insert(
            "websocket_url".to_string(),
            (old_config.websocket_url.clone(), new_config.websocket_url.clone()),
        );
    }
    
    // Compare environment
    if old_config.environment != new_config.environment {
        diffs.insert(
            "environment".to_string(),
            (format!("{:?}", old_config.environment), format!("{:?}", new_config.environment)),
        );
    }
    
    // Compare hardware profile
    if old_config.hardware.profile != new_config.hardware.profile {
        diffs.insert(
            "hardware.profile".to_string(),
            (
                format!("{:?}", old_config.hardware.profile),
                format!("{:?}", new_config.hardware.profile),
            ),
        );
    }
    
    // Compare network configuration
    if old_config.network.kernel_bypass_mode != new_config.network.kernel_bypass_mode {
        diffs.insert(
            "network.kernel_bypass_mode".to_string(),
            (
                format!("{:?}", old_config.network.kernel_bypass_mode),
                format!("{:?}", new_config.network.kernel_bypass_mode),
            ),
        );
    }
    
    // Compare feature flags
    let old_flags = old_config.feature_flags;
    let new_flags = new_config.feature_flags;
    
    // Networking features
    if old_flags.networking.kernel_bypass_networking != new_flags.networking.kernel_bypass_networking {
        diffs.insert(
            "feature_flags.networking.kernel_bypass_networking".to_string(),
            (
                format!("{}", old_flags.networking.kernel_bypass_networking),
                format!("{}", new_flags.networking.kernel_bypass_networking),
            ),
        );
    }
    
    if old_flags.networking.hardware_timestamping != new_flags.networking.hardware_timestamping {
        diffs.insert(
            "feature_flags.networking.hardware_timestamping".to_string(),
            (
                format!("{}", old_flags.networking.hardware_timestamping),
                format!("{}", new_flags.networking.hardware_timestamping),
            ),
        );
    }
    
    if old_flags.networking.high_frequency_networking != new_flags.networking.high_frequency_networking {
        diffs.insert(
            "feature_flags.networking.high_frequency_networking".to_string(),
            (
                format!("{}", old_flags.networking.high_frequency_networking),
                format!("{}", new_flags.networking.high_frequency_networking),
            ),
        );
    }
    
    // CPU features
    if old_flags.cpu.cpu_pinning != new_flags.cpu.cpu_pinning {
        diffs.insert(
            "feature_flags.cpu.cpu_pinning".to_string(),
            (
                format!("{}", old_flags.cpu.cpu_pinning),
                format!("{}", new_flags.cpu.cpu_pinning),
            ),
        );
    }
    
    if old_flags.cpu.simd_optimizations != new_flags.cpu.simd_optimizations {
        diffs.insert(
            "feature_flags.cpu.simd_optimizations".to_string(),
            (
                format!("{}", old_flags.cpu.simd_optimizations),
                format!("{}", new_flags.cpu.simd_optimizations),
            ),
        );
    }
    
    // Memory features
    if old_flags.memory.huge_pages != new_flags.memory.huge_pages {
        diffs.insert(
            "feature_flags.memory.huge_pages".to_string(),
            (
                format!("{}", old_flags.memory.huge_pages),
                format!("{}", new_flags.memory.huge_pages),
            ),
        );
    }
    
    if old_flags.memory.huge_pages_1gb != new_flags.memory.huge_pages_1gb {
        diffs.insert(
            "feature_flags.memory.huge_pages_1gb".to_string(),
            (
                format!("{}", old_flags.memory.huge_pages_1gb),
                format!("{}", new_flags.memory.huge_pages_1gb),
            ),
        );
    }
    
    // Additional capabilities
    if old_flags.capabilities.numa_awareness != new_flags.capabilities.numa_awareness {
        diffs.insert(
            "feature_flags.capabilities.numa_awareness".to_string(),
            (
                format!("{}", old_flags.capabilities.numa_awareness),
                format!("{}", new_flags.capabilities.numa_awareness),
            ),
        );
    }
    
    if old_flags.capabilities.direct_memory_access != new_flags.capabilities.direct_memory_access {
        diffs.insert(
            "feature_flags.capabilities.direct_memory_access".to_string(),
            (
                format!("{}", old_flags.capabilities.direct_memory_access),
                format!("{}", new_flags.capabilities.direct_memory_access),
            ),
        );
    }
    
    if old_flags.capabilities.isolated_cpus != new_flags.capabilities.isolated_cpus {
        diffs.insert(
            "feature_flags.capabilities.isolated_cpus".to_string(),
            (
                format!("{}", old_flags.capabilities.isolated_cpus),
                format!("{}", new_flags.capabilities.isolated_cpus),
            ),
        );
    }
    
    diffs
}

/// Generate a default configuration file
pub fn generate_default_config(path: &Path) -> ConfigResult<()> {
    let config = BotConfig::default();
    
    // Determine file format
    let format = match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => "json",
        Some("yaml") | Some("yml") => "yaml",
        _ => "json", // Default to JSON
    };
    
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Serialize and save
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&config)?;
            std::fs::write(path, json)?;
        }
        "yaml" => {
            let yaml = serde_yaml::to_string(&config)?;
            std::fs::write(path, yaml)?;
        }
        _ => unreachable!(),
    }
    
    info!("Default configuration generated at: {:?}", path);
    
    Ok(())
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_diff_configs() {
        let mut old_config = BotConfig::default();
        let mut new_config = BotConfig::default();
        
        // No differences initially
        let diffs = diff_configs(&old_config, &new_config);
        assert!(diffs.is_empty());
        
        // Change RPC URL
        new_config.rpc_url = "https://new-rpc.example.com".to_string();
        let diffs = diff_configs(&old_config, &new_config);
        assert_eq!(diffs.len(), 1);
        assert!(diffs.contains_key("rpc_url"));
        
        // Change feature flag
        new_config.feature_flags.networking.kernel_bypass_networking = true;
        let diffs = diff_configs(&old_config, &new_config);
        assert_eq!(diffs.len(), 2);
        assert!(diffs.contains_key("feature_flags.networking.kernel_bypass_networking"));
    }
    
    #[test]
    fn test_generate_default_config() -> ConfigResult<()> {
        // Create a temporary directory
        let dir = tempdir()?;
        let config_path = dir.path().join("config.json");
        
        // Generate default config
        generate_default_config(&config_path)?;
        
        // Check that the file exists
        assert!(config_path.exists());
        
        // Clean up
        dir.close()?;
        
        Ok(())
    }
}