//! NUMA (Non-Uniform Memory Access) support
//! 
//! This module provides utilities for NUMA-aware memory allocation and CPU pinning.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// NUMA node information
#[derive(Debug, Clone)]
pub struct NumaInfo {
    /// Number of NUMA nodes
    pub num_nodes: i32,
    
    /// CPUs per NUMA node
    pub cpus_per_node: HashMap<i32, Vec<i32>>,
    
    /// Memory per NUMA node (in bytes)
    pub memory_per_node: HashMap<i32, u64>,
    
    /// Free memory per NUMA node (in bytes)
    pub free_memory_per_node: HashMap<i32, u64>,
}

impl NumaInfo {
    /// Create a new NUMA information object
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            // Check if NUMA is available
            let numa_available = unsafe { libc::numa_available() };
            if numa_available == -1 {
                return Err(anyhow!("NUMA is not available"));
            }
            
            // Get number of NUMA nodes
            let num_nodes = unsafe { libc::numa_num_configured_nodes() };
            
            // Get CPUs per NUMA node
            let mut cpus_per_node = HashMap::new();
            let max_cpus = unsafe { libc::numa_num_configured_cpus() };
            
            for node in 0..num_nodes {
                let mut cpu_mask = unsafe { std::mem::zeroed::<libc::bitmask>() };
                unsafe { libc::numa_node_to_cpus(node, &mut cpu_mask) };
                
                let mut cpus = Vec::new();
                for cpu in 0..max_cpus {
                    if unsafe { libc::numa_bitmask_isbitset(&cpu_mask, cpu) } != 0 {
                        cpus.push(cpu);
                    }
                }
                
                cpus_per_node.insert(node, cpus);
            }
            
            // Get memory per NUMA node
            let mut memory_per_node = HashMap::new();
            let mut free_memory_per_node = HashMap::new();
            
            for node in 0..num_nodes {
                let mut size: libc::c_longlong = 0;
                let mut free: libc::c_longlong = 0;
                
                unsafe {
                    libc::numa_node_size64(node, &mut free);
                    size = libc::numa_node_size64(node, std::ptr::null_mut());
                }
                
                memory_per_node.insert(node, size as u64);
                free_memory_per_node.insert(node, free as u64);
            }
            
            Ok(Self {
                num_nodes,
                cpus_per_node,
                memory_per_node,
                free_memory_per_node,
            })
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            // Simulate a single NUMA node on non-Linux systems
            let num_nodes = 1;
            let mut cpus_per_node = HashMap::new();
            let num_cpus = num_cpus::get() as i32;
            cpus_per_node.insert(0, (0..num_cpus).collect());
            
            let memory_per_node = HashMap::from([(0, 0)]);
            let free_memory_per_node = HashMap::from([(0, 0)]);
            
            Ok(Self {
                num_nodes,
                cpus_per_node,
                memory_per_node,
                free_memory_per_node,
            })
        }
    }
    
    /// Get the NUMA node for a CPU
    pub fn get_node_for_cpu(&self, cpu: i32) -> Option<i32> {
        for (node, cpus) in &self.cpus_per_node {
            if cpus.contains(&cpu) {
                return Some(*node);
            }
        }
        
        None
    }
    
    /// Get the CPUs for a NUMA node
    pub fn get_cpus_for_node(&self, node: i32) -> Option<&Vec<i32>> {
        self.cpus_per_node.get(&node)
    }
    
    /// Get the memory for a NUMA node
    pub fn get_memory_for_node(&self, node: i32) -> Option<u64> {
        self.memory_per_node.get(&node).copied()
    }
    
    /// Get the free memory for a NUMA node
    pub fn get_free_memory_for_node(&self, node: i32) -> Option<u64> {
        self.free_memory_per_node.get(&node).copied()
    }
    
    /// Get the NUMA node with the most free memory
    pub fn get_node_with_most_free_memory(&self) -> Option<i32> {
        self.free_memory_per_node
            .iter()
            .max_by_key(|(_, free)| *free)
            .map(|(node, _)| *node)
    }
    
    /// Get the NUMA node with the least free memory
    pub fn get_node_with_least_free_memory(&self) -> Option<i32> {
        self.free_memory_per_node
            .iter()
            .min_by_key(|(_, free)| *free)
            .map(|(node, _)| *node)
    }
}

/// Get the NUMA node for a CPU
pub fn get_numa_node_for_cpu(cpu: i32) -> Result<i32> {
    #[cfg(target_os = "linux")]
    {
        // Check if NUMA is available
        let numa_available = unsafe { libc::numa_available() };
        if numa_available == -1 {
            return Err(anyhow!("NUMA is not available"));
        }
        
        let node = unsafe { libc::numa_node_of_cpu(cpu) };
        if node == -1 {
            return Err(anyhow!("Failed to get NUMA node for CPU {}", cpu));
        }
        
        Ok(node)
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        // Simulate a single NUMA node on non-Linux systems
        Ok(0)
    }
}

/// NUMA-aware memory allocation
pub struct NumaMemory {
    /// NUMA node
    node: i32,
    
    /// Pointer to the memory
    ptr: *mut u8,
    
    /// Size of the memory
    size: usize,
}

impl NumaMemory {
    /// Allocate memory on a NUMA node
    pub fn new(size: usize, node: i32) -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            // Check if NUMA is available
            let numa_available = unsafe { libc::numa_available() };
            if numa_available == -1 {
                return Err(anyhow!("NUMA is not available"));
            }
            
            // Allocate memory on the specified NUMA node
            let ptr = unsafe { libc::numa_alloc_onnode(size, node) };
            if ptr.is_null() {
                return Err(anyhow!("Failed to allocate memory on NUMA node {}", node));
            }
            
            Ok(Self {
                node,
                ptr: ptr as *mut u8,
                size,
            })
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            // Fallback to standard allocation on non-Linux systems
            let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<u64>())?;
            let ptr = unsafe { std::alloc::alloc(layout) };
            if ptr.is_null() {
                return Err(anyhow!("Memory allocation failed"));
            }
            
            Ok(Self {
                node,
                ptr,
                size,
            })
        }
    }
    
    /// Get the NUMA node
    pub fn node(&self) -> i32 {
        self.node
    }
    
    /// Get a pointer to the memory
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }
    
    /// Get a mutable pointer to the memory
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }
    
    /// Get the size of the memory
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for NumaMemory {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            #[cfg(target_os = "linux")]
            unsafe {
                libc::numa_free(self.ptr as *mut libc::c_void, self.size);
            }
            
            #[cfg(not(target_os = "linux"))]
            unsafe {
                let layout = std::alloc::Layout::from_size_align(self.size, std::mem::align_of::<u64>())
                    .expect("Invalid layout in drop");
                std::alloc::dealloc(self.ptr, layout);
            }
        }
    }
}

unsafe impl Send for NumaMemory {}
unsafe impl Sync for NumaMemory {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numa_info() {
        let result = NumaInfo::new();
        
        // This test should pass on both Linux and non-Linux systems
        assert!(result.is_ok());
        
        let info = result.unwrap();
        assert!(info.num_nodes > 0);
        assert!(!info.cpus_per_node.is_empty());
    }

    #[test]
    fn test_numa_memory() {
        let result = NumaMemory::new(1024, 0);
        
        // This test should pass on both Linux and non-Linux systems
        assert!(result.is_ok());
        
        let mut memory = result.unwrap();
        assert_eq!(memory.node(), 0);
        assert_eq!(memory.size(), 1024);
        assert!(!memory.as_ptr().is_null());
        
        // Write to memory
        unsafe {
            std::ptr::write_bytes(memory.as_mut_ptr(), 0x42, memory.size());
        }
    }
}