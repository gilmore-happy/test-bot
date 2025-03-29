//! Hardware-aware thread management
//! 
//! This module provides hardware-aware thread management for the execution engine,
//! including NUMA awareness, thread-to-core pinning, and isolated CPU configurations.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use anyhow::{anyhow, Result};
use crossbeam_channel::{bounded, Receiver, Sender};
use num_cpus;
use parking_lot::RwLock;
use tracing::{debug, error, info, trace, warn};

#[cfg(target_os = "linux")]
use hwloc::{Topology, TopologyObject, ObjectType, CpuSet, CPUBIND_THREAD};

/// CPU core information
#[derive(Debug, Clone)]
pub struct CoreInfo {
    /// Core ID
    pub id: usize,
    
    /// NUMA node ID
    pub numa_node: usize,
    
    /// Whether this is a hyperthreaded core
    pub is_hyperthread: bool,
    
    /// Whether this core is isolated
    pub is_isolated: bool,
    
    /// CPU frequency in MHz
    pub frequency_mhz: Option<u64>,
}

/// NUMA node information
#[derive(Debug, Clone)]
pub struct NumaNode {
    /// Node ID
    pub id: usize,
    
    /// Cores in this NUMA node
    pub cores: Vec<usize>,
    
    /// Memory size in bytes
    pub memory_bytes: u64,
    
    /// Memory bandwidth in MB/s
    pub memory_bandwidth: Option<u64>,
}

/// Thread affinity configuration
#[derive(Debug, Clone)]
pub struct ThreadAffinity {
    /// Core ID to pin to
    pub core_id: usize,
    
    /// Whether to use strict pinning
    pub strict: bool,
}

/// Hardware topology information
#[derive(Debug, Clone)]
pub struct HardwareTopology {
    /// Number of physical cores
    pub physical_cores: usize,
    
    /// Number of logical cores (including hyperthreads)
    pub logical_cores: usize,
    
    /// Number of NUMA nodes
    pub numa_nodes: usize,
    
    /// Core information
    pub cores: HashMap<usize, CoreInfo>,
    
    /// NUMA node information
    pub numa_info: HashMap<usize, NumaNode>,
    
    /// Isolated cores
    pub isolated_cores: HashSet<usize>,
    
    /// Whether NUMA is available
    pub numa_available: bool,
    
    /// Whether thread pinning is available
    pub thread_pinning_available: bool,
}

impl HardwareTopology {
    /// Detect hardware topology
    pub fn detect() -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            Self::detect_linux()
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            Self::detect_generic()
        }
    }
    
    /// Detect hardware topology on Linux
    #[cfg(target_os = "linux")]
    fn detect_linux() -> Result<Self> {
        let topo = Topology::new();
        if let Err(e) = topo {
            warn!("Failed to initialize hwloc: {}", e);
            return Self::detect_generic();
        }
        
        let topo = topo.unwrap();
        let mut cores = HashMap::new();
        let mut numa_info = HashMap::new();
        let mut isolated_cores = HashSet::new();
        
        // Get NUMA nodes
        let numa_nodes = topo.objects_with_type(&ObjectType::NUMANode).len();
        
        // Initialize NUMA node info
        for i in 0..numa_nodes {
            numa_info.insert(i, NumaNode {
                id: i,
                cores: Vec::new(),
                memory_bytes: 0,
                memory_bandwidth: None,
            });
        }
        
        // Get isolated cores from kernel cmdline
        if let Ok(cmdline) = std::fs::read_to_string("/proc/cmdline") {
            if let Some(isolcpus) = cmdline.split_whitespace()
                .find(|arg| arg.starts_with("isolcpus="))
                .map(|arg| arg.trim_start_matches("isolcpus="))
            {
                for range in isolcpus.split(',') {
                    if range.contains('-') {
                        let parts: Vec<&str> = range.split('-').collect();
                        if parts.len() == 2 {
                            if let (Ok(start), Ok(end)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                                for core in start..=end {
                                    isolated_cores.insert(core);
                                }
                            }
                        }
                    } else if let Ok(core) = range.parse::<usize>() {
                        isolated_cores.insert(core);
                    }
                }
            }
        }
        
        // Get core information
        let mut physical_cores = 0;
        let logical_cores = num_cpus::get();
        
        // Process each core
        for i in 0..logical_cores {
            let mut core_info = CoreInfo {
                id: i,
                numa_node: 0, // Default to NUMA node 0
                is_hyperthread: false,
                is_isolated: isolated_cores.contains(&i),
                frequency_mhz: None,
            };
            
            // Try to get CPU frequency
            if let Ok(freq_str) = std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu{}/cpufreq/cpuinfo_max_freq", i)) {
                if let Ok(freq_khz) = freq_str.trim().parse::<u64>() {
                    core_info.frequency_mhz = Some(freq_khz / 1000);
                }
            }
            
            // Try to determine if this is a hyperthread
            if let Ok(thread_siblings) = std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu{}/topology/thread_siblings_list", i)) {
                let siblings: Vec<usize> = thread_siblings.trim()
                    .split(',')
                    .filter_map(|s| s.parse::<usize>().ok())
                    .collect();
                
                if siblings.len() > 1 && siblings[0] != i {
                    core_info.is_hyperthread = true;
                } else {
                    physical_cores += 1;
                }
            } else {
                // Can't determine, assume it's a physical core
                physical_cores += 1;
            }
            
            // Try to determine NUMA node
            if let Ok(numa_node_str) = std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu{}/topology/physical_package_id", i)) {
                if let Ok(numa_node) = numa_node_str.trim().parse::<usize>() {
                    core_info.numa_node = numa_node;
                    
                    // Add this core to the NUMA node
                    if let Some(node_info) = numa_info.get_mut(&numa_node) {
                        node_info.cores.push(i);
                    }
                }
            }
            
            cores.insert(i, core_info);
        }
        
        // Get memory information for each NUMA node
        for (node_id, node_info) in numa_info.iter_mut() {
            if let Ok(meminfo) = std::fs::read_to_string(format!("/sys/devices/system/node/node{}/meminfo", node_id)) {
                for line in meminfo.lines() {
                    if line.contains("MemTotal") {
                        if let Some(mem_kb_str) = line.split_whitespace().nth(3) {
                            if let Ok(mem_kb) = mem_kb_str.parse::<u64>() {
                                node_info.memory_bytes = mem_kb * 1024;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(Self {
            physical_cores,
            logical_cores,
            numa_nodes,
            cores,
            numa_info,
            isolated_cores,
            numa_available: numa_nodes > 1,
            thread_pinning_available: true,
        })
    }
    
    /// Detect hardware topology on non-Linux platforms
    fn detect_generic() -> Result<Self> {
        let logical_cores = num_cpus::get();
        let physical_cores = num_cpus::get_physical();
        
        let mut cores = HashMap::new();
        let mut numa_info = HashMap::new();
        
        // Create a single NUMA node
        numa_info.insert(0, NumaNode {
            id: 0,
            cores: (0..logical_cores).collect(),
            memory_bytes: 0, // Unknown
            memory_bandwidth: None,
        });
        
        // Create core information
        for i in 0..logical_cores {
            let is_hyperthread = i >= physical_cores;
            
            cores.insert(i, CoreInfo {
                id: i,
                numa_node: 0,
                is_hyperthread,
                is_isolated: false,
                frequency_mhz: None,
            });
        }
        
        Ok(Self {
            physical_cores,
            logical_cores,
            numa_nodes: 1,
            cores,
            numa_info,
            isolated_cores: HashSet::new(),
            numa_available: false,
            thread_pinning_available: false,
        })
    }
    
    /// Get cores for a specific NUMA node
    pub fn get_numa_cores(&self, node: usize) -> Vec<usize> {
        self.numa_info.get(&node)
            .map(|node_info| node_info.cores.clone())
            .unwrap_or_default()
    }
    
    /// Get physical cores (excluding hyperthreads)
    pub fn get_physical_cores(&self) -> Vec<usize> {
        self.cores.iter()
            .filter(|(_, info)| !info.is_hyperthread)
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Get isolated cores
    pub fn get_isolated_cores(&self) -> Vec<usize> {
        self.isolated_cores.iter().copied().collect()
    }
    
    /// Get non-isolated cores
    pub fn get_non_isolated_cores(&self) -> Vec<usize> {
        self.cores.iter()
            .filter(|(id, _)| !self.isolated_cores.contains(id))
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Get physical cores for a specific NUMA node
    pub fn get_numa_physical_cores(&self, node: usize) -> Vec<usize> {
        self.cores.iter()
            .filter(|(_, info)| info.numa_node == node && !info.is_hyperthread)
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Get isolated cores for a specific NUMA node
    pub fn get_numa_isolated_cores(&self, node: usize) -> Vec<usize> {
        self.cores.iter()
            .filter(|(id, info)| info.numa_node == node && self.isolated_cores.contains(id))
            .map(|(id, _)| *id)
            .collect()
    }
}

/// Thread manager for hardware-aware thread management
pub struct ThreadManager {
    /// Hardware topology
    topology: Arc<HardwareTopology>,
    
    /// Thread handles
    threads: RwLock<HashMap<String, thread::JoinHandle<()>>>,
    
    /// Thread affinities
    affinities: RwLock<HashMap<String, ThreadAffinity>>,
    
    /// Shutdown channel
    shutdown: (Sender<()>, Receiver<()>),
}

impl ThreadManager {
    /// Create a new thread manager
    pub fn new() -> Result<Self> {
        let topology = HardwareTopology::detect()?;
        
        Ok(Self {
            topology: Arc::new(topology),
            threads: RwLock::new(HashMap::new()),
            affinities: RwLock::new(HashMap::new()),
            shutdown: bounded(1),
        })
    }
    
    /// Get the hardware topology
    pub fn topology(&self) -> Arc<HardwareTopology> {
        self.topology.clone()
    }
    
    /// Spawn a thread with specific affinity
    pub fn spawn_with_affinity<F, T>(
        &self,
        name: &str,
        core_id: usize,
        strict: bool,
        f: F,
    ) -> Result<thread::JoinHandle<T>>
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        // Check if the core exists
        if !self.topology.cores.contains_key(&core_id) {
            return Err(anyhow!("Core {} does not exist", core_id));
        }
        
        // Create thread builder
        let builder = thread::Builder::new().name(name.to_string());
        
        // Spawn the thread
        let handle = builder.spawn(move || {
            // Set thread affinity
            #[cfg(target_os = "linux")]
            {
                let mut cpu_set = CpuSet::new();
                cpu_set.set(core_id);
                
                if let Ok(topo) = Topology::new() {
                    if let Err(e) = topo.set_cpubind_for_thread(
                        thread::current().id().as_u64().unwrap() as u32,
                        cpu_set,
                        CPUBIND_THREAD,
                    ) {
                        warn!("Failed to set thread affinity: {}", e);
                    }
                }
            }
            
            // Execute the function
            f()
        })?;
        
        // Store the thread handle and affinity
        {
            let mut threads = self.threads.write();
            threads.insert(name.to_string(), handle.clone());
            
            let mut affinities = self.affinities.write();
            affinities.insert(name.to_string(), ThreadAffinity {
                core_id,
                strict,
            });
        }
        
        Ok(handle)
    }
    
    /// Spawn a thread on a specific NUMA node
    pub fn spawn_on_numa<F, T>(
        &self,
        name: &str,
        numa_node: usize,
        prefer_isolated: bool,
        f: F,
    ) -> Result<thread::JoinHandle<T>>
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        // Get cores for the NUMA node
        let cores = if prefer_isolated {
            self.topology.get_numa_isolated_cores(numa_node)
        } else {
            self.topology.get_numa_physical_cores(numa_node)
        };
        
        if cores.is_empty() {
            return Err(anyhow!("No suitable cores found on NUMA node {}", numa_node));
        }
        
        // Use the first available core
        self.spawn_with_affinity(name, cores[0], true, f)
    }
    
    /// Spawn a thread for interrupt handling
    pub fn spawn_interrupt_handler<F, T>(
        &self,
        name: &str,
        f: F,
    ) -> Result<thread::JoinHandle<T>>
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        // For interrupt handling, use non-isolated cores
        let cores = self.topology.get_non_isolated_cores();
        
        if cores.is_empty() {
            return Err(anyhow!("No suitable cores found for interrupt handling"));
        }
        
        // Use the first available core
        self.spawn_with_affinity(name, cores[0], false, f)
    }
    
    /// Spawn a thread for critical path processing
    pub fn spawn_critical_path<F, T>(
        &self,
        name: &str,
        f: F,
    ) -> Result<thread::JoinHandle<T>>
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        // For critical path processing, prefer isolated cores
        let cores = self.topology.get_isolated_cores();
        
        if cores.is_empty() {
            // Fall back to physical cores if no isolated cores are available
            let cores = self.topology.get_physical_cores();
            
            if cores.is_empty() {
                return Err(anyhow!("No suitable cores found for critical path processing"));
            }
            
            // Use the first available core
            self.spawn_with_affinity(name, cores[0], true, f)
        } else {
            // Use the first isolated core
            self.spawn_with_affinity(name, cores[0], true, f)
        }
    }
    
    /// Allocate memory on a specific NUMA node
    #[cfg(target_os = "linux")]
    pub fn allocate_numa_memory(&self, node: usize, size: usize) -> Result<Vec<u8>> {
        use libc::{c_void, size_t};
        use std::ptr;
        
        extern "C" {
            fn numa_alloc_onnode(size: size_t, node: i32) -> *mut c_void;
            fn numa_free(start: *mut c_void, size: size_t);
        }
        
        unsafe {
            let ptr = numa_alloc_onnode(size, node as i32);
            if ptr.is_null() {
                return Err(anyhow!("Failed to allocate memory on NUMA node {}", node));
            }
            
            // Create a Vec that will free the memory when dropped
            let mut vec = Vec::with_capacity(size);
            vec.set_len(size);
            
            // Copy the memory to the Vec
            ptr::copy_nonoverlapping(ptr as *const u8, vec.as_mut_ptr(), size);
            
            // Free the original memory
            numa_free(ptr, size);
            
            Ok(vec)
        }
    }
    
    /// Allocate memory on a specific NUMA node (generic implementation)
    #[cfg(not(target_os = "linux"))]
    pub fn allocate_numa_memory(&self, _node: usize, size: usize) -> Result<Vec<u8>> {
        // On non-Linux platforms, just allocate regular memory
        Ok(vec![0; size])
    }
    
    /// Shutdown all threads
    pub fn shutdown(&self) {
        // Send shutdown signal
        let _ = self.shutdown.0.send(());
        
        // Join all threads
        let mut threads = self.threads.write();
        for (name, handle) in threads.drain() {
            if let Err(e) = handle.join() {
                error!("Failed to join thread {}: {:?}", name, e);
            }
        }
    }
}

impl Drop for ThreadManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// NUMA-aware memory allocator
pub struct NumaAllocator {
    /// Hardware topology
    topology: Arc<HardwareTopology>,
    
    /// Thread manager
    thread_manager: Arc<ThreadManager>,
}

impl NumaAllocator {
    /// Create a new NUMA-aware allocator
    pub fn new(thread_manager: Arc<ThreadManager>) -> Self {
        Self {
            topology: thread_manager.topology(),
            thread_manager,
        }
    }
    
    /// Allocate memory on the local NUMA node
    pub fn allocate_local(&self, size: usize) -> Result<Vec<u8>> {
        // Get the current thread's CPU
        #[cfg(target_os = "linux")]
        {
            if let Ok(cpu) = Self::get_current_cpu() {
                if let Some(core_info) = self.topology.cores.get(&cpu) {
                    return self.thread_manager.allocate_numa_memory(core_info.numa_node, size);
                }
            }
        }
        
        // Fall back to regular allocation
        Ok(vec![0; size])
    }
    
    /// Allocate memory on a specific NUMA node
    pub fn allocate_on_node(&self, node: usize, size: usize) -> Result<Vec<u8>> {
        self.thread_manager.allocate_numa_memory(node, size)
    }
    
    /// Get the current CPU
    #[cfg(target_os = "linux")]
    fn get_current_cpu() -> Result<usize> {
        use std::fs::File;
        use std::io::Read;
        
        let mut file = File::open("/proc/self/stat")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        let parts: Vec<&str> = contents.split_whitespace().collect();
        if parts.len() >= 39 {
            if let Ok(cpu) = parts[38].parse::<usize>() {
                return Ok(cpu);
            }
        }
        
        Err(anyhow!("Failed to get current CPU"))
    }
}

/// Thread configuration for the execution engine
#[derive(Debug, Clone)]
pub struct ThreadConfig {
    /// Number of worker threads
    pub worker_threads: usize,
    
    /// Number of critical path threads
    pub critical_path_threads: usize,
    
    /// Number of interrupt handling threads
    pub interrupt_threads: usize,
    
    /// Whether to use NUMA-aware memory allocation
    pub use_numa_allocation: bool,
    
    /// Whether to use thread pinning
    pub use_thread_pinning: bool,
    
    /// Whether to use isolated CPUs
    pub use_isolated_cpus: bool,
}

impl Default for ThreadConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get_physical(),
            critical_path_threads: 2,
            interrupt_threads: 1,
            use_numa_allocation: true,
            use_thread_pinning: true,
            use_isolated_cpus: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hardware_topology_detection() {
        let topology = HardwareTopology::detect().unwrap();
        println!("Hardware topology: {:?}", topology);
        
        // Just make sure detection runs without errors
        assert!(topology.logical_cores > 0);
        assert!(topology.physical_cores > 0);
    }
    
    #[test]
    fn test_thread_manager() {
        let thread_manager = ThreadManager::new().unwrap();
        
        // Spawn a thread that just sleeps
        let handle = thread_manager.spawn_with_affinity(
            "test_thread",
            0, // Use core 0
            false,
            || {
                thread::sleep(Duration::from_millis(100));
                42
            },
        );
        
        // This should succeed on any platform
        assert!(handle.is_ok());
        
        // Join the thread
        let result = handle.unwrap().join();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}