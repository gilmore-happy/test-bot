//! Memory management module
//! 
//! This module provides memory management utilities for high-performance networking,
//! including NUMA-aware memory allocation and hugepages support.

mod numa;
mod hugepages;

pub use numa::{NumaInfo, NumaMemory, get_numa_node_for_cpu};
pub use hugepages::{HugepagesInfo, HugepagesConfig, allocate_hugepages};

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Memory allocation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryStrategy {
    /// Standard memory allocation
    Standard,
    
    /// NUMA-aware memory allocation
    Numa,
    
    /// Hugepages memory allocation
    Hugepages,
    
    /// NUMA-aware hugepages memory allocation
    NumaHugepages,
}

/// Memory configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Memory allocation strategy
    pub strategy: MemoryStrategy,
    
    /// NUMA node to use
    pub numa_node: Option<i32>,
    
    /// Hugepages configuration
    pub hugepages_config: Option<HugepagesConfig>,
    
    /// Whether to use transparent hugepages
    pub use_transparent_hugepages: bool,
    
    /// Whether to lock memory
    pub lock_memory: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            strategy: MemoryStrategy::Standard,
            numa_node: None,
            hugepages_config: None,
            use_transparent_hugepages: false,
            lock_memory: false,
        }
    }
}

/// Memory manager
pub struct MemoryManager {
    /// Configuration
    config: MemoryConfig,
    
    /// NUMA information
    numa_info: Option<NumaInfo>,
    
    /// Hugepages information
    hugepages_info: Option<HugepagesInfo>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(config: MemoryConfig) -> Result<Self> {
        let numa_info = if config.strategy == MemoryStrategy::Numa || config.strategy == MemoryStrategy::NumaHugepages {
            Some(NumaInfo::new()?)
        } else {
            None
        };
        
        let hugepages_info = if config.strategy == MemoryStrategy::Hugepages || config.strategy == MemoryStrategy::NumaHugepages {
            Some(HugepagesInfo::new()?)
        } else {
            None
        };
        
        // Lock memory if requested
        if config.lock_memory {
            #[cfg(target_os = "linux")]
            unsafe {
                if libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) != 0 {
                    return Err(anyhow!("Failed to lock memory: {}", std::io::Error::last_os_error()));
                }
            }
            
            #[cfg(not(target_os = "linux"))]
            {
                warn!("Memory locking is only supported on Linux");
            }
        }
        
        Ok(Self {
            config,
            numa_info,
            hugepages_info,
        })
    }
    
    /// Allocate memory
    pub fn allocate(&self, size: usize) -> Result<*mut u8> {
        match self.config.strategy {
            MemoryStrategy::Standard => {
                // Standard memory allocation
                let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<u64>())?;
                let ptr = unsafe { std::alloc::alloc(layout) };
                if ptr.is_null() {
                    return Err(anyhow!("Memory allocation failed"));
                }
                Ok(ptr)
            }
            MemoryStrategy::Numa => {
                // NUMA-aware memory allocation
                let numa_node = self.config.numa_node.unwrap_or(0);
                
                #[cfg(target_os = "linux")]
                {
                    let ptr = unsafe { libc::numa_alloc_onnode(size, numa_node) };
                    if ptr.is_null() {
                        return Err(anyhow!("NUMA memory allocation failed"));
                    }
                    Ok(ptr as *mut u8)
                }
                
                #[cfg(not(target_os = "linux"))]
                {
                    warn!("NUMA-aware memory allocation is only supported on Linux");
                    let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<u64>())?;
                    let ptr = unsafe { std::alloc::alloc(layout) };
                    if ptr.is_null() {
                        return Err(anyhow!("Memory allocation failed"));
                    }
                    Ok(ptr)
                }
            }
            MemoryStrategy::Hugepages => {
                // Hugepages memory allocation
                let config = self.config.hugepages_config.clone().unwrap_or_default();
                allocate_hugepages(size, config)
            }
            MemoryStrategy::NumaHugepages => {
                // NUMA-aware hugepages memory allocation
                let numa_node = self.config.numa_node.unwrap_or(0);
                let config = self.config.hugepages_config.clone().unwrap_or_default();
                
                #[cfg(target_os = "linux")]
                {
                    let mut hugepages_config = config;
                    hugepages_config.numa_node = Some(numa_node);
                    allocate_hugepages(size, hugepages_config)
                }
                
                #[cfg(not(target_os = "linux"))]
                {
                    warn!("NUMA-aware hugepages allocation is only supported on Linux");
                    allocate_hugepages(size, config)
                }
            }
        }
    }
    
    /// Free memory
    pub fn free(&self, ptr: *mut u8, size: usize) {
        if ptr.is_null() {
            return;
        }
        
        match self.config.strategy {
            MemoryStrategy::Standard => {
                // Standard memory deallocation
                let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<u64>())
                    .expect("Invalid layout in free");
                unsafe { std::alloc::dealloc(ptr, layout) };
            }
            MemoryStrategy::Numa => {
                // NUMA-aware memory deallocation
                #[cfg(target_os = "linux")]
                unsafe {
                    libc::numa_free(ptr as *mut libc::c_void, size);
                }
                
                #[cfg(not(target_os = "linux"))]
                {
                    let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<u64>())
                        .expect("Invalid layout in free");
                    unsafe { std::alloc::dealloc(ptr, layout) };
                }
            }
            MemoryStrategy::Hugepages | MemoryStrategy::NumaHugepages => {
                // Hugepages memory deallocation
                #[cfg(target_os = "linux")]
                unsafe {
                    libc::munmap(ptr as *mut libc::c_void, size);
                }
                
                #[cfg(not(target_os = "linux"))]
                {
                    let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<u64>())
                        .expect("Invalid layout in free");
                    unsafe { std::alloc::dealloc(ptr, layout) };
                }
            }
        }
    }
    
    /// Get the NUMA information
    pub fn numa_info(&self) -> Option<&NumaInfo> {
        self.numa_info.as_ref()
    }
    
    /// Get the hugepages information
    pub fn hugepages_info(&self) -> Option<&HugepagesInfo> {
        self.hugepages_info.as_ref()
    }
    
    /// Get the memory configuration
    pub fn config(&self) -> &MemoryConfig {
        &self.config
    }
}

impl Drop for MemoryManager {
    fn drop(&mut self) {
        // Unlock memory if it was locked
        if self.config.lock_memory {
            #[cfg(target_os = "linux")]
            unsafe {
                libc::munlockall();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_standard() {
        let config = MemoryConfig {
            strategy: MemoryStrategy::Standard,
            ..Default::default()
        };
        
        let manager = MemoryManager::new(config).unwrap();
        
        // Allocate and free memory
        let size = 1024;
        let ptr = manager.allocate(size).unwrap();
        assert!(!ptr.is_null());
        
        // Write to memory
        unsafe {
            std::ptr::write_bytes(ptr, 0x42, size);
        }
        
        // Free memory
        manager.free(ptr, size);
    }
}