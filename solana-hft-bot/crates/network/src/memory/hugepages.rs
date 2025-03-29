//! Hugepages support
//! 
//! This module provides utilities for hugepages memory allocation.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// Hugepages size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HugepagesSize {
    /// 2MB hugepages
    Size2MB,
    
    /// 1GB hugepages
    Size1GB,
}

impl HugepagesSize {
    /// Get the size in bytes
    pub fn size_in_bytes(&self) -> usize {
        match self {
            HugepagesSize::Size2MB => 2 * 1024 * 1024,
            HugepagesSize::Size1GB => 1024 * 1024 * 1024,
        }
    }
    
    /// Get the mmap flag
    #[cfg(target_os = "linux")]
    pub fn mmap_flag(&self) -> i32 {
        match self {
            HugepagesSize::Size2MB => libc::MAP_HUGETLB | (21 << libc::MAP_HUGE_SHIFT),
            HugepagesSize::Size1GB => libc::MAP_HUGETLB | (30 << libc::MAP_HUGE_SHIFT),
        }
    }
}

/// Hugepages configuration
#[derive(Debug, Clone)]
pub struct HugepagesConfig {
    /// Hugepages size
    pub size: HugepagesSize,
    
    /// NUMA node
    pub numa_node: Option<i32>,
    
    /// Whether to use transparent hugepages
    pub use_transparent: bool,
}

impl Default for HugepagesConfig {
    fn default() -> Self {
        Self {
            size: HugepagesSize::Size2MB,
            numa_node: None,
            use_transparent: false,
        }
    }
}

/// Hugepages information
#[derive(Debug, Clone)]
pub struct HugepagesInfo {
    /// Total number of hugepages
    pub total: HashMap<HugepagesSize, u64>,
    
    /// Free number of hugepages
    pub free: HashMap<HugepagesSize, u64>,
    
    /// Reserved number of hugepages
    pub reserved: HashMap<HugepagesSize, u64>,
    
    /// Surplus number of hugepages
    pub surplus: HashMap<HugepagesSize, u64>,
}

impl HugepagesInfo {
    /// Create a new hugepages information object
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            let mut total = HashMap::new();
            let mut free = HashMap::new();
            let mut reserved = HashMap::new();
            let mut surplus = HashMap::new();
            
            // Read hugepages information from /proc/meminfo
            let meminfo = std::fs::read_to_string("/proc/meminfo")?;
            
            // Parse 2MB hugepages
            let total_2mb = parse_meminfo_value(&meminfo, "HugePages_Total:")?;
            let free_2mb = parse_meminfo_value(&meminfo, "HugePages_Free:")?;
            let reserved_2mb = parse_meminfo_value(&meminfo, "HugePages_Rsvd:")?;
            let surplus_2mb = parse_meminfo_value(&meminfo, "HugePages_Surp:")?;
            
            total.insert(HugepagesSize::Size2MB, total_2mb);
            free.insert(HugepagesSize::Size2MB, free_2mb);
            reserved.insert(HugepagesSize::Size2MB, reserved_2mb);
            surplus.insert(HugepagesSize::Size2MB, surplus_2mb);
            
            // Parse 1GB hugepages if available
            if let Ok(total_1gb) = parse_meminfo_value(&meminfo, "Hugepagesize:") {
                if total_1gb == 1048576 {
                    // 1GB hugepages are available
                    let total_1gb = parse_meminfo_value(&meminfo, "HugePages_Total:")?;
                    let free_1gb = parse_meminfo_value(&meminfo, "HugePages_Free:")?;
                    let reserved_1gb = parse_meminfo_value(&meminfo, "HugePages_Rsvd:")?;
                    let surplus_1gb = parse_meminfo_value(&meminfo, "HugePages_Surp:")?;
                    
                    total.insert(HugepagesSize::Size1GB, total_1gb);
                    free.insert(HugepagesSize::Size1GB, free_1gb);
                    reserved.insert(HugepagesSize::Size1GB, reserved_1gb);
                    surplus.insert(HugepagesSize::Size1GB, surplus_1gb);
                }
            }
            
            Ok(Self {
                total,
                free,
                reserved,
                surplus,
            })
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            // Simulate hugepages on non-Linux systems
            let mut total = HashMap::new();
            let mut free = HashMap::new();
            let mut reserved = HashMap::new();
            let mut surplus = HashMap::new();
            
            total.insert(HugepagesSize::Size2MB, 0);
            free.insert(HugepagesSize::Size2MB, 0);
            reserved.insert(HugepagesSize::Size2MB, 0);
            surplus.insert(HugepagesSize::Size2MB, 0);
            
            total.insert(HugepagesSize::Size1GB, 0);
            free.insert(HugepagesSize::Size1GB, 0);
            reserved.insert(HugepagesSize::Size1GB, 0);
            surplus.insert(HugepagesSize::Size1GB, 0);
            
            Ok(Self {
                total,
                free,
                reserved,
                surplus,
            })
        }
    }
    
    /// Get the total number of hugepages
    pub fn get_total(&self, size: HugepagesSize) -> u64 {
        self.total.get(&size).copied().unwrap_or(0)
    }
    
    /// Get the free number of hugepages
    pub fn get_free(&self, size: HugepagesSize) -> u64 {
        self.free.get(&size).copied().unwrap_or(0)
    }
    
    /// Get the reserved number of hugepages
    pub fn get_reserved(&self, size: HugepagesSize) -> u64 {
        self.reserved.get(&size).copied().unwrap_or(0)
    }
    
    /// Get the surplus number of hugepages
    pub fn get_surplus(&self, size: HugepagesSize) -> u64 {
        self.surplus.get(&size).copied().unwrap_or(0)
    }
}

/// Parse a value from /proc/meminfo
#[cfg(target_os = "linux")]
fn parse_meminfo_value(meminfo: &str, key: &str) -> Result<u64> {
    for line in meminfo.lines() {
        if line.starts_with(key) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return Ok(parts[1].parse()?);
            }
        }
    }
    
    Err(anyhow!("Failed to parse {} from /proc/meminfo", key))
}

/// Allocate memory using hugepages
pub fn allocate_hugepages(size: usize, config: HugepagesConfig) -> Result<*mut u8> {
    #[cfg(target_os = "linux")]
    {
        // Align size to hugepage size
        let hugepage_size = config.size.size_in_bytes();
        let aligned_size = (size + hugepage_size - 1) & !(hugepage_size - 1);
        
        // Set up mmap flags
        let mut flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS;
        
        if !config.use_transparent {
            flags |= config.size.mmap_flag();
        }
        
        // Allocate memory
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                aligned_size,
                libc::PROT_READ | libc::PROT_WRITE,
                flags,
                -1,
                0,
            )
        };
        
        if ptr == libc::MAP_FAILED {
            return Err(anyhow!("Failed to allocate hugepages: {}", std::io::Error::last_os_error()));
        }
        
        // If NUMA node is specified, bind memory to it
        if let Some(node) = config.numa_node {
            let numa_available = unsafe { libc::numa_available() };
            if numa_available == -1 {
                warn!("NUMA is not available, ignoring NUMA node");
            } else {
                let ret = unsafe {
                    libc::mbind(
                        ptr,
                        aligned_size,
                        libc::MPOL_BIND,
                        &(1u64 << node) as *const _ as *const libc::c_ulong,
                        64,
                        libc::MPOL_MF_STRICT,
                    )
                };
                
                if ret != 0 {
                    warn!("Failed to bind memory to NUMA node {}: {}", node, std::io::Error::last_os_error());
                }
            }
        }
        
        Ok(ptr as *mut u8)
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        // Fallback to standard allocation on non-Linux systems
        let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<u64>())?;
        let ptr = unsafe { std::alloc::alloc(layout) };
        if ptr.is_null() {
            return Err(anyhow!("Memory allocation failed"));
        }
        
        Ok(ptr)
    }
}

/// Free hugepages memory
pub fn free_hugepages(ptr: *mut u8, size: usize, config: &HugepagesConfig) {
    if ptr.is_null() {
        return;
    }
    
    #[cfg(target_os = "linux")]
    {
        // Align size to hugepage size
        let hugepage_size = config.size.size_in_bytes();
        let aligned_size = (size + hugepage_size - 1) & !(hugepage_size - 1);
        
        unsafe {
            libc::munmap(ptr as *mut libc::c_void, aligned_size);
        }
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<u64>())
            .expect("Invalid layout in free_hugepages");
        unsafe { std::alloc::dealloc(ptr, layout) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hugepages_size() {
        assert_eq!(HugepagesSize::Size2MB.size_in_bytes(), 2 * 1024 * 1024);
        assert_eq!(HugepagesSize::Size1GB.size_in_bytes(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_hugepages_config_default() {
        let config = HugepagesConfig::default();
        assert_eq!(config.size, HugepagesSize::Size2MB);
        assert_eq!(config.numa_node, None);
        assert_eq!(config.use_transparent, false);
    }

    #[test]
    fn test_allocate_hugepages() {
        let config = HugepagesConfig::default();
        let size = 4 * 1024 * 1024; // 4MB
        
        let result = allocate_hugepages(size, config.clone());
        assert!(result.is_ok());
        
        let ptr = result.unwrap();
        assert!(!ptr.is_null());
        
        // Write to memory
        unsafe {
            std::ptr::write_bytes(ptr, 0x42, size);
        }
        
        // Free memory
        free_hugepages(ptr, size, &config);
    }
}