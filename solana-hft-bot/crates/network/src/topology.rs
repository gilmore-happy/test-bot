//! Network topology optimization for ultra-low latency trading
//! 
//! This module provides network path discovery, latency mapping, and route optimization
//! to ensure the lowest possible latency for high-frequency trading operations.

use anyhow::{anyhow, Context, Result};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::time;
use tracing::{debug, error, info, instrument, trace, warn};

use super::socket_enhanced::{EnhancedSocket, EnhancedSocketOptions};

/// Network node representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkNode {
    /// Node identifier
    pub id: String,
    
    /// Node address
    pub address: SocketAddr,
    
    /// Node type
    pub node_type: NodeType,
    
    /// Node region
    pub region: Option<String>,
    
    /// Node datacenter
    pub datacenter: Option<String>,
    
    /// Node provider
    pub provider: Option<String>,
    
    /// Node capabilities
    pub capabilities: Vec<NodeCapability>,
}

/// Network node type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeType {
    /// Validator node
    Validator,
    
    /// RPC node
    Rpc,
    
    /// Relay node
    Relay,
    
    /// Edge node
    Edge,
    
    /// Custom node
    Custom,
}

/// Network node capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeCapability {
    /// Supports hardware timestamps
    HardwareTimestamps,
    
    /// Supports kernel bypass
    KernelBypass,
    
    /// Supports zero-copy
    ZeroCopy,
    
    /// Supports direct memory access
    DirectMemoryAccess,
    
    /// Supports jumbo frames
    JumboFrames,
    
    /// Supports TCP BBR
    TcpBbr,
    
    /// Supports QUIC protocol
    Quic,
    
    /// Supports TLS offload
    TlsOffload,
}

/// Path status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStatus {
    /// Path is active and healthy
    Active,
    
    /// Path is degraded but usable
    Degraded,
    
    /// Path is down
    Down,
    
    /// Path status is unknown
    Unknown,
}

/// Latency statistics
#[derive(Debug, Clone, Copy)]
pub struct LatencyStats {
    /// Minimum latency
    pub min_latency: Duration,
    
    /// Maximum latency
    pub max_latency: Duration,
    
    /// Average latency
    pub avg_latency: Duration,
    
    /// Median latency
    pub median_latency: Duration,
    
    /// 99th percentile latency
    pub p99_latency: Duration,
    
    /// Jitter (standard deviation)
    pub jitter: Duration,
    
    /// Number of samples
    pub samples: usize,
    
    /// Last measurement time
    pub last_measured: Instant,
}

impl Default for LatencyStats {
    fn default() -> Self {
        Self {
            min_latency: Duration::MAX,
            max_latency: Duration::ZERO,
            avg_latency: Duration::ZERO,
            median_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            jitter: Duration::ZERO,
            samples: 0,
            last_measured: Instant::now(),
        }
    }
}

/// Network path between two nodes
#[derive(Debug, Clone)]
pub struct NetworkPath {
    /// Source node
    pub source: NetworkNode,
    
    /// Destination node
    pub destination: NetworkNode,
    
    /// Intermediate hops
    pub hops: Vec<NetworkNode>,
    
    /// Path latency statistics
    pub latency_stats: LatencyStats,
    
    /// Path status
    pub status: PathStatus,
    
    /// Path creation time
    pub created_at: Instant,
    
    /// Path last updated time
    pub updated_at: Instant,
    
    /// Path priority (lower is better)
    pub priority: u32,
    
    /// Path metadata
    pub metadata: HashMap<String, String>,
}

/// Network topology configuration
#[derive(Debug, Clone)]
pub struct TopologyConfig {
    /// Known nodes
    pub known_nodes: Vec<NetworkNode>,
    
    /// Preferred regions
    pub preferred_regions: Vec<String>,
    
    /// Preferred providers
    pub preferred_providers: Vec<String>,
    
    /// Required node capabilities
    pub required_capabilities: Vec<NodeCapability>,
    
    /// Preferred node capabilities
    pub preferred_capabilities: Vec<NodeCapability>,
    
    /// Latency threshold for path selection (in microseconds)
    pub latency_threshold_us: u64,
    
    /// Jitter threshold for path selection (in microseconds)
    pub jitter_threshold_us: u64,
    
    /// Path discovery interval
    pub discovery_interval: Duration,
    
    /// Path monitoring interval
    pub monitoring_interval: Duration,
    
    /// Number of probes per path
    pub probes_per_path: usize,
    
    /// Enable path failover
    pub enable_failover: bool,
    
    /// Failover threshold (in microseconds)
    pub failover_threshold_us: u64,
    
    /// Enable multipath routing
    pub enable_multipath: bool,
    
    /// Maximum number of paths to maintain
    pub max_paths: usize,
    
    /// Enable dynamic path optimization
    pub enable_dynamic_optimization: bool,
    
    /// Dynamic optimization interval
    pub dynamic_optimization_interval: Duration,
    
    /// Socket options for path probing
    pub probe_socket_options: EnhancedSocketOptions,
    
    /// Socket options for data transmission
    pub data_socket_options: EnhancedSocketOptions,
}

impl Default for TopologyConfig {
    fn default() -> Self {
        Self {
            known_nodes: Vec::new(),
            preferred_regions: Vec::new(),
            preferred_providers: Vec::new(),
            required_capabilities: Vec::new(),
            preferred_capabilities: vec![
                NodeCapability::HardwareTimestamps,
                NodeCapability::TcpBbr,
                NodeCapability::ZeroCopy,
            ],
            latency_threshold_us: 500, // 500 microseconds
            jitter_threshold_us: 50,   // 50 microseconds
            discovery_interval: Duration::from_secs(60),
            monitoring_interval: Duration::from_secs(1),
            probes_per_path: 10,
            enable_failover: true,
            failover_threshold_us: 1000, // 1 millisecond
            enable_multipath: true,
            max_paths: 3,
            enable_dynamic_optimization: true,
            dynamic_optimization_interval: Duration::from_secs(10),
            probe_socket_options: EnhancedSocketOptions::ultra_low_latency(),
            data_socket_options: EnhancedSocketOptions::ultra_low_latency(),
        }
    }
}

/// Network topology manager
pub struct TopologyManager {
    /// Configuration
    config: TopologyConfig,
    
    /// All known nodes
    nodes: Arc<RwLock<HashMap<String, NetworkNode>>>,
    
    /// All known paths
    paths: Arc<RwLock<HashMap<(String, String), Vec<NetworkPath>>>>,
    
    /// Active paths for each destination
    active_paths: Arc<RwLock<HashMap<String, NetworkPath>>>,
    
    /// Backup paths for each destination
    backup_paths: Arc<RwLock<HashMap<String, Vec<NetworkPath>>>>,
    
    /// Latency matrix between nodes
    latency_matrix: Arc<RwLock<HashMap<(String, String), LatencyStats>>>,
    
    /// Path discovery task handle
    discovery_task: Option<tokio::task::JoinHandle<()>>,
    
    /// Path monitoring task handle
    monitoring_task: Option<tokio::task::JoinHandle<()>>,
    
    /// Dynamic optimization task handle
    optimization_task: Option<tokio::task::JoinHandle<()>>,
    
    /// Running flag
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl TopologyManager {
    /// Create a new topology manager
    pub fn new(config: TopologyConfig) -> Self {
        let running = Arc::new(std::sync::atomic::AtomicBool::new(false));
        
        Self {
            config,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            paths: Arc::new(RwLock::new(HashMap::new())),
            active_paths: Arc::new(RwLock::new(HashMap::new())),
            backup_paths: Arc::new(RwLock::new(HashMap::new())),
            latency_matrix: Arc::new(RwLock::new(HashMap::new())),
            discovery_task: None,
            monitoring_task: None,
            optimization_task: None,
            running,
        }
    }
    
    /// Start the topology manager
    pub async fn start(&mut self) -> Result<()> {
        if self.running.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        
        info!("Starting network topology manager");
        
        // Initialize known nodes
        self.initialize_nodes();
        
        // Start path discovery
        self.start_path_discovery().await?;
        
        // Start path monitoring
        self.start_path_monitoring().await?;
        
        // Start dynamic optimization if enabled
        if self.config.enable_dynamic_optimization {
            self.start_dynamic_optimization().await?;
        }
        
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);
        
        Ok(())
    }
    
    /// Stop the topology manager
    pub async fn stop(&mut self) -> Result<()> {
        if !self.running.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        
        info!("Stopping network topology manager");
        
        // Stop running flag
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        
        // Abort discovery task
        if let Some(task) = self.discovery_task.take() {
            task.abort();
        }
        
        // Abort monitoring task
        if let Some(task) = self.monitoring_task.take() {
            task.abort();
        }
        
        // Abort optimization task
        if let Some(task) = self.optimization_task.take() {
            task.abort();
        }
        
        Ok(())
    }
    
    /// Initialize known nodes
    fn initialize_nodes(&self) {
        let mut nodes = self.nodes.write().unwrap();
        
        for node in &self.config.known_nodes {
            nodes.insert(node.id.clone(), node.clone());
        }
        
        info!("Initialized {} known nodes", nodes.len());
    }
    
    /// Calculate latency statistics from a list of latency measurements
    fn calculate_latency_stats(latencies: &[Duration]) -> LatencyStats {
        if latencies.is_empty() {
            return LatencyStats::default();
        }
        
        // Sort latencies for percentile calculations
        let mut sorted_latencies = latencies.to_vec();
        sorted_latencies.sort();
        
        // Calculate statistics
        let min_latency = *sorted_latencies.first().unwrap();
        let max_latency = *sorted_latencies.last().unwrap();
        
        let sum: Duration = sorted_latencies.iter().sum();
        let avg_latency = sum / sorted_latencies.len() as u32;
        
        let median_idx = sorted_latencies.len() / 2;
        let median_latency = sorted_latencies[median_idx];
        
        let p99_idx = (sorted_latencies.len() as f64 * 0.99) as usize;
        let p99_latency = sorted_latencies[p99_idx];
        
        // Calculate jitter (standard deviation)
        let variance_sum: u128 = sorted_latencies.iter()
            .map(|&latency| {
                let diff = if latency > avg_latency {
                    latency - avg_latency
                } else {
                    avg_latency - latency
                };
                diff.as_nanos().pow(2)
            })
            .sum();
        
        let variance = variance_sum as f64 / sorted_latencies.len() as f64;
        let std_dev_ns = variance.sqrt();
        let jitter = Duration::from_nanos(std_dev_ns as u64);
        
        LatencyStats {
            min_latency,
            max_latency,
            avg_latency,
            median_latency,
            p99_latency,
            jitter,
            samples: latencies.len(),
            last_measured: Instant::now(),
        }
    }
    
    /// Get the best path to a destination
    pub fn get_best_path(&self, destination_id: &str) -> Option<NetworkPath> {
        let active_paths = self.active_paths.read().unwrap();
        active_paths.get(destination_id).cloned()
    }
    
    /// Get all paths to a destination
    pub fn get_all_paths(&self, destination_id: &str) -> Vec<NetworkPath> {
        let mut result = Vec::new();
        
        // Add active path if available
        let active_paths = self.active_paths.read().unwrap();
        if let Some(path) = active_paths.get(destination_id) {
            result.push(path.clone());
        }
        
        // Add backup paths
        let backup_paths = self.backup_paths.read().unwrap();
        if let Some(paths) = backup_paths.get(destination_id) {
            result.extend(paths.clone());
        }
        
        result
    }
    
    /// Add a new node to the topology
    pub fn add_node(&self, node: NetworkNode) -> Result<()> {
        let mut nodes = self.nodes.write().unwrap();
        
        // Check if node already exists
        if nodes.contains_key(&node.id) {
            return Err(anyhow!("Node {} already exists", node.id));
        }
        
        // Add node
        nodes.insert(node.id.clone(), node);
        
        info!("Added new node: {}", node.id);
        
        Ok(())
    }
    
    /// Remove a node from the topology
    pub fn remove_node(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write().unwrap();
        
        // Check if node exists
        if !nodes.contains_key(node_id) {
            return Err(anyhow!("Node {} does not exist", node_id));
        }
        
        // Remove node
        nodes.remove(node_id);
        
        // Remove paths involving this node
        let mut paths = self.paths.write().unwrap();
        paths.retain(|(src, dst), _| src != node_id && dst != node_id);
        
        // Remove from latency matrix
        let mut matrix = self.latency_matrix.write().unwrap();
        matrix.retain(|(src, dst), _| src != node_id && dst != node_id);
        
        // Remove from active paths
        let mut active_paths = self.active_paths.write().unwrap();
        active_paths.retain(|_, path| path.source.id != node_id && path.destination.id != node_id);
        
        // Remove from backup paths
        let mut backup_paths = self.backup_paths.write().unwrap();
        backup_paths.retain(|_, paths| {
            paths.retain(|path| path.source.id != node_id && path.destination.id != node_id);
            !paths.is_empty()
        });
        
        info!("Removed node: {}", node_id);
        
        Ok(())
    }
    
    /// Start path discovery task
    async fn start_path_discovery(&mut self) -> Result<()> {
        let config = self.config.clone();
        let nodes = self.nodes.clone();
        let paths = self.paths.clone();
        let latency_matrix = self.latency_matrix.clone();
        let running = self.running.clone();
        
        let handle = tokio::spawn(async move {
            info!("Starting path discovery task");
            
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                // Discover paths between nodes
                if let Err(e) = Self::discover_paths(&config, &nodes, &paths, &latency_matrix).await {
                    error!("Path discovery error: {}", e);
                }
                
                // Wait for next discovery interval
                time::sleep(config.discovery_interval).await;
            }
            
            info!("Path discovery task stopped");
        });
        
        self.discovery_task = Some(handle);
        
        Ok(())
    }
    
    /// Discover paths between nodes
    async fn discover_paths(
        config: &TopologyConfig,
        nodes: &Arc<RwLock<HashMap<String, NetworkNode>>>,
        paths: &Arc<RwLock<HashMap<(String, String), Vec<NetworkPath>>>>,
        latency_matrix: &Arc<RwLock<HashMap<(String, String), LatencyStats>>>,
    ) -> Result<()> {
        let nodes_read = nodes.read().unwrap();
        
        // Skip if no nodes
        if nodes_read.is_empty() {
            return Ok(());
        }
        
        info!("Discovering paths between {} nodes", nodes_read.len());
        
        // Create a list of node pairs to measure
        let mut node_pairs = Vec::new();
        for (src_id, src_node) in nodes_read.iter() {
            for (dst_id, dst_node) in nodes_read.iter() {
                if src_id != dst_id {
                    node_pairs.push((src_node.clone(), dst_node.clone()));
                }
            }
        }
        
        // Measure latency between node pairs
        for (src_node, dst_node) in node_pairs {
            if let Err(e) = Self::measure_path_latency(config, &src_node, &dst_node, latency_matrix).await {
                warn!("Failed to measure latency between {} and {}: {}", src_node.id, dst_node.id, e);
                continue;
            }
        }
        
        // Update paths based on latency measurements
        Self::update_paths(config, nodes, paths, latency_matrix)?;
        
        Ok(())
    }
    
    /// Measure latency between two nodes
    async fn measure_path_latency(
        config: &TopologyConfig,
        src_node: &NetworkNode,
        dst_node: &NetworkNode,
        latency_matrix: &Arc<RwLock<HashMap<(String, String), LatencyStats>>>,
    ) -> Result<()> {
        debug!("Measuring latency from {} to {}", src_node.id, dst_node.id);
        
        // Connect to destination node with enhanced socket
        let mut socket = match EnhancedSocket::connect(dst_node.address, config.probe_socket_options.clone()).await {
            Ok(socket) => socket,
            Err(e) => {
                return Err(anyhow!("Failed to connect to {}: {}", dst_node.id, e));
            }
        };
        
        // Perform latency measurements
        let mut latencies = Vec::with_capacity(config.probes_per_path);
        
        for _ in 0..config.probes_per_path {
            // Send probe packet
            let probe_time = Instant::now();
            let probe_data = b"PROBE";
            
            if let Err(e) = socket.send(probe_data).await {
                warn!("Failed to send probe to {}: {}", dst_node.id, e);
                continue;
            }
            
            // Receive response
            let mut response = [0u8; 5];
            if let Err(e) = socket.recv(&mut response).await {
                warn!("Failed to receive probe response from {}: {}", dst_node.id, e);
                continue;
            }
            
            // Calculate latency
            let latency = Instant::now().duration_since(probe_time);
            latencies.push(latency);
            
            // Small delay between probes
            time::sleep(Duration::from_millis(10)).await;
        }
        
        // Calculate latency statistics
        if !latencies.is_empty() {
            let stats = Self::calculate_latency_stats(&latencies);
            
            // Update latency matrix
            let mut matrix = latency_matrix.write().unwrap();
            matrix.insert((src_node.id.clone(), dst_node.id.clone()), stats);
            
            debug!("Latency from {} to {}: min={}µs, avg={}µs, max={}µs, p99={}µs, jitter={}µs",
                src_node.id, dst_node.id,
                stats.min_latency.as_micros(),
                stats.avg_latency.as_micros(),
                stats.max_latency.as_micros(),
                stats.p99_latency.as_micros(),
                stats.jitter.as_micros());
        }
        
        Ok(())
    }
    
    /// Update paths based on latency measurements
    fn update_paths(
        config: &TopologyConfig,
        nodes: &Arc<RwLock<HashMap<String, NetworkNode>>>,
        paths: &Arc<RwLock<HashMap<(String, String), Vec<NetworkPath>>>>,
        latency_matrix: &Arc<RwLock<HashMap<(String, String), LatencyStats>>>,
    ) -> Result<()> {
        let nodes_read = nodes.read().unwrap();
        let matrix_read = latency_matrix.read().unwrap();
        let mut paths_write = paths.write().unwrap();
        
        info!("Updating paths based on latency measurements");
        
        // Clear existing paths
        paths_write.clear();
        
        // Create direct paths based on latency measurements
        for ((src_id, dst_id), stats) in matrix_read.iter() {
            let src_node = match nodes_read.get(src_id) {
                Some(node) => node.clone(),
                None => continue,
            };
            
            let dst_node = match nodes_read.get(dst_id) {
                Some(node) => node.clone(),
                None => continue,
            };
            
            // Create direct path
            let path = NetworkPath {
                source: src_node.clone(),
                destination: dst_node.clone(),
                hops: Vec::new(),
                latency_stats: *stats,
                status: if stats.avg_latency.as_micros() <= config.latency_threshold_us as u128 {
                    PathStatus::Active
                } else {
                    PathStatus::Degraded
                },
                created_at: Instant::now(),
                updated_at: Instant::now(),
                priority: 0,
                metadata: HashMap::new(),
            };
            
            // Add path
            let key = (src_id.clone(), dst_id.clone());
            paths_write.entry(key).or_insert_with(Vec::new).push(path);
        }
        
        // Find indirect paths (with one intermediate hop)
        for src_id in nodes_read.keys() {
            for dst_id in nodes_read.keys() {
                if src_id == dst_id {
                    continue;
                }
                
                // Skip if we already have enough paths
                let key = (src_id.clone(), dst_id.clone());
                let existing_paths = paths_write.entry(key.clone()).or_insert_with(Vec::new);
                if existing_paths.len() >= config.max_paths {
                    continue;
                }
                
                // Try to find indirect paths through intermediate nodes
                for relay_id in nodes_read.keys() {
                    if relay_id == src_id || relay_id == dst_id {
                        continue;
                    }
                    
                    // Check if we have latency measurements for both hops
                    let src_relay_key = (src_id.clone(), relay_id.clone());
                    let relay_dst_key = (relay_id.clone(), dst_id.clone());
                    
                    let src_relay_stats = match matrix_read.get(&src_relay_key) {
                        Some(stats) => stats,
                        None => continue,
                    };
                    
                    let relay_dst_stats = match matrix_read.get(&relay_dst_key) {
                        Some(stats) => stats,
                        None => continue,
                    };
                    
                    // Calculate end-to-end latency
                    let total_latency = src_relay_stats.avg_latency + relay_dst_stats.avg_latency;
                    let total_jitter = src_relay_stats.jitter + relay_dst_stats.jitter;
                    
                    // Skip if latency is too high
                    if total_latency.as_micros() > (config.latency_threshold_us * 2) as u128 {
                        continue;
                    }
                    
                    // Create indirect path
                    let src_node = nodes_read.get(src_id).unwrap().clone();
                    let dst_node = nodes_read.get(dst_id).unwrap().clone();
                    let relay_node = nodes_read.get(relay_id).unwrap().clone();
                    
                    let path = NetworkPath {
                        source: src_node,
                        destination: dst_node,
                        hops: vec![relay_node],
                        latency_stats: LatencyStats {
                            min_latency: src_relay_stats.min_latency + relay_dst_stats.min_latency,
                            max_latency: src_relay_stats.max_latency + relay_dst_stats.max_latency,
                            avg_latency: total_latency,
                            median_latency: src_relay_stats.median_latency + relay_dst_stats.median_latency,
                            p99_latency: src_relay_stats.p99_latency + relay_dst_stats.p99_latency,
                            jitter: total_jitter,
                            samples: std::cmp::min(src_relay_stats.samples, relay_dst_stats.samples),
                            last_measured: Instant::now(),
                        },
                        status: if total_latency.as_micros() <= (config.latency_threshold_us * 2) as u128 {
                            PathStatus::Active
                        } else {
                            PathStatus::Degraded
                        },
                        created_at: Instant::now(),
                        updated_at: Instant::now(),
                        priority: 1, // Lower priority than direct paths
                        metadata: HashMap::new(),
                    };
                    
                    // Add path
                    existing_paths.push(path);
                    
                    // Stop if we have enough paths
                    if existing_paths.len() >= config.max_paths {
                        break;
                    }
                }
            }
        }
        
        // Sort paths by latency
        for paths_vec in paths_write.values_mut() {
            paths_vec.sort_by(|a, b| {
                a.latency_stats.avg_latency.cmp(&b.latency_stats.avg_latency)
            });
            
            // Limit to max_paths
            if paths_vec.len() > config.max_paths {
                paths_vec.truncate(config.max_paths);
            }
        }
        
        info!("Updated paths for {} node pairs", paths_write.len());
        
        Ok(())
    }
    
    /// Start path monitoring task
    async fn start_path_monitoring(&mut self) -> Result<()> {
        let config = self.config.clone();
        let paths = self.paths.clone();
        let active_paths = self.active_paths.clone();
        let backup_paths = self.backup_paths.clone();
        let running = self.running.clone();
        
        let handle = tokio::spawn(async move {
            info!("Starting path monitoring task");
            
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                // Monitor paths and update active/backup paths
                if let Err(e) = Self::monitor_paths(&config, &paths, &active_paths, &backup_paths).await {
                    error!("Path monitoring error: {}", e);
                }
                
                // Wait for next monitoring interval
                time::sleep(config.monitoring_interval).await;
            }
            
            info!("Path monitoring task stopped");
        });
        
        self.monitoring_task = Some(handle);
        
        Ok(())
    }
    
    /// Monitor paths and update active/backup paths
    async fn monitor_paths(
        config: &TopologyConfig,
        paths: &Arc<RwLock<HashMap<(String, String), Vec<NetworkPath>>>>,
        active_paths: &Arc<RwLock<HashMap<String, NetworkPath>>>,
        backup_paths: &Arc<RwLock<HashMap<String, Vec<NetworkPath>>>>,
    ) -> Result<()> {
        let paths_read = paths.read().unwrap();
        
        // Skip if no paths
        if paths_read.is_empty() {
            return Ok(());
        }
        
        debug!("Monitoring paths for {} node pairs", paths_read.len());
        
        let mut new_active_paths = HashMap::new();
        let mut new_backup_paths = HashMap::new();
        
        // Select best paths for each destination
        for ((src_id, dst_id), path_list) in paths_read.iter() {
            if path_list.is_empty() {
                continue;
            }
            
            // Find best path (lowest latency)
            let mut best_path = None;
            let mut backup_list = Vec::new();
            
            for path in path_list {
                if path.status == PathStatus::Active {
                    if best_path.is_none() || path.latency_stats.avg_latency < best_path.as_ref().unwrap().latency_stats.avg_latency {
                        if let Some(old_best) = best_path.take() {
                            backup_list.push(old_best);
                        }
                        best_path = Some(path.clone());
                    } else {
                        backup_list.push(path.clone());
                    }
                } else if path.status == PathStatus::Degraded {
                    backup_list.push(path.clone());
                }
            }
            
            // Update active and backup paths
            if let Some(path) = best_path {
                new_active_paths.insert(dst_id.clone(), path);
                
                if !backup_list.is_empty() {
                    // Sort backup paths by latency
                    backup_list.sort_by(|a, b| {
                        a.latency_stats.avg_latency.cmp(&b.latency_stats.avg_latency)
                    });
                    
                    new_backup_paths.insert(dst_id.clone(), backup_list);
                }
            }
        }
        
        // Update active paths
        let mut active_write = active_paths.write().unwrap();
        *active_write = new_active_paths;
        
        // Update backup paths
        let mut backup_write = backup_paths.write().unwrap();
        *backup_write = new_backup_paths;
        
        debug!("Updated active paths for {} destinations", active_write.len());
        
        Ok(())
    }
    
    /// Start dynamic optimization task
    async fn start_dynamic_optimization(&mut self) -> Result<()> {
        let config = self.config.clone();
        let nodes = self.nodes.clone();
        let paths = self.paths.clone();
        let latency_matrix = self.latency_matrix.clone();
        let running = self.running.clone();
        
        let handle = tokio::spawn(async move {
            info!("Starting dynamic optimization task");
            
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                // Perform dynamic optimization
                if let Err(e) = Self::optimize_paths(&config, &nodes, &paths, &latency_matrix).await {
                    error!("Dynamic optimization error: {}", e);
                }
                
                // Wait for next optimization interval
                time::sleep(config.dynamic_optimization_interval).await;
            }
            
            info!("Dynamic optimization task stopped");
        });
        
        self.optimization_task = Some(handle);
        
        Ok(())
    }
    
    /// Optimize paths based on current network conditions
    async fn optimize_paths(
        config: &TopologyConfig,
        nodes: &Arc<RwLock<HashMap<String, NetworkNode>>>,
        paths: &Arc<RwLock<HashMap<(String, String), Vec<NetworkPath>>>>,
        latency_matrix: &Arc<RwLock<HashMap<(String, String), LatencyStats>>>,
    ) -> Result<()> {
        info!("Performing dynamic path optimization");
        
        // Re-measure latency for critical paths
        let critical_paths = Self::identify_critical_paths(paths)?;
        
        for (src_node, dst_node) in critical_paths {
            if let Err(e) = Self::measure_path_latency(config, &src_node, &dst_node, latency_matrix).await {
                warn!("Failed to re-measure latency for critical path {}-{}: {}", src_node.id, dst_node.id, e);
            }
        }
        
        // Update paths based on new measurements
        Self::update_paths(config, nodes, paths, latency_matrix)?;
        
        Ok(())
    }
    
    /// Identify critical paths that need re-measurement
    fn identify_critical_paths(
        paths: &Arc<RwLock<HashMap<(String, String), Vec<NetworkPath>>>>,
    ) -> Result<Vec<(NetworkNode, NetworkNode)>> {
        let paths_read = paths.read().unwrap();
        let mut critical_paths = Vec::new();
        
        for path_list in paths_read.values() {
            for path in path_list {
                // Consider paths with high jitter or recent status changes as critical
                if path.status == PathStatus::Degraded ||
                   path.latency_stats.jitter.as_micros() > 100 || // High jitter
                   path.latency_stats.last_measured.elapsed() > Duration::from_secs(30) // Not measured recently
                {
                    critical_paths.push((path.source.clone(), path.destination.clone()));
                }
            }
        }
        
        Ok(critical_paths)
    }
    
    /// Get network topology statistics
    pub fn get_stats(&self) -> TopologyStats {
        let nodes = self.nodes.read().unwrap();
        let paths = self.paths.read().unwrap();
        let active_paths = self.active_paths.read().unwrap();
        let backup_paths = self.backup_paths.read().unwrap();
        
        // Calculate average latency across all active paths
        let mut total_latency = Duration::ZERO;
        let mut path_count = 0;
        
        for path in active_paths.values() {
            total_latency += path.latency_stats.avg_latency;
            path_count += 1;
        }
        
        let avg_latency = if path_count > 0 {
            total_latency / path_count as u32
        } else {
            Duration::ZERO
        };
        
        // Count paths by status
        let mut active_count = 0;
        let mut degraded_count = 0;
        let mut down_count = 0;
        
        for path_list in paths.values() {
            for path in path_list {
                match path.status {
                    PathStatus::Active => active_count += 1,
                    PathStatus::Degraded => degraded_count += 1,
                    PathStatus::Down => down_count += 1,
                    _ => {}
                }
            }
        }
        
        TopologyStats {
            node_count: nodes.len(),
            path_count: paths.values().map(|v| v.len()).sum(),
            active_path_count: active_count,
            degraded_path_count: degraded_count,
            down_path_count: down_count,
            avg_latency,
            last_discovery: Instant::now(), // TODO: Track actual last discovery time
            last_optimization: Instant::now(), // TODO: Track actual last optimization time
        }
    }
}

/// Network topology statistics
#[derive(Debug, Clone)]
pub struct TopologyStats {
    /// Number of nodes
    pub node_count: usize,
    
    /// Number of paths
    pub path_count: usize,
    
    /// Number of active paths
    pub active_path_count: usize,
    
    /// Number of degraded paths
    pub degraded_path_count: usize,
    
    /// Number of down paths
    pub down_path_count: usize,
    
    /// Average latency across all active paths
    pub avg_latency: Duration,
    
    /// Last path discovery time
    pub last_discovery: Instant,
    
    /// Last path optimization time
    pub last_optimization: Instant,
}

/// Network path finder for optimal routing
pub struct PathFinder {
    /// Topology manager
    topology: Arc<TopologyManager>,
}

impl PathFinder {
    /// Create a new path finder
    pub fn new(topology: Arc<TopologyManager>) -> Self {
        Self { topology }
    }
    
    /// Find the best path to a destination
    pub fn find_best_path(&self, destination_id: &str) -> Option<NetworkPath> {
        self.topology.get_best_path(destination_id)
    }
    
    /// Find all paths to a destination
    pub fn find_all_paths(&self, destination_id: &str) -> Vec<NetworkPath> {
        self.topology.get_all_paths(destination_id)
    }
    
    /// Find a path with specific requirements
    pub fn find_path_with_requirements(
        &self,
        destination_id: &str,
        max_latency: Option<Duration>,
        max_jitter: Option<Duration>,
        required_capabilities: &[NodeCapability],
    ) -> Option<NetworkPath> {
        let paths = self.topology.get_all_paths(destination_id);
        
        for path in paths {
            // Check latency requirement
            if let Some(max_latency) = max_latency {
                if path.latency_stats.avg_latency > max_latency {
                    continue;
                }
            }
            
            // Check jitter requirement
            if let Some(max_jitter) = max_jitter {
                if path.latency_stats.jitter > max_jitter {
                    continue;
                }
            }
            
            // Check capability requirements
            let mut has_all_capabilities = true;
            for &capability in required_capabilities {
                if !path.destination.capabilities.contains(&capability) {
                    has_all_capabilities = false;
                    break;
                }
            }
            
            if !has_all_capabilities {
                continue;
            }
            
            // Path meets all requirements
            return Some(path);
        }
        
        None
    }
    
    /// Find the lowest latency path
    pub fn find_lowest_latency_path(&self, destination_id: &str) -> Option<NetworkPath> {
        let paths = self.topology.get_all_paths(destination_id);
        
        paths.into_iter().min_by_key(|path| path.latency_stats.avg_latency)
    }
    
    /// Find the most stable path (lowest jitter)
    pub fn find_most_stable_path(&self, destination_id: &str) -> Option<NetworkPath> {
        let paths = self.topology.get_all_paths(destination_id);
        
        paths.into_iter().min_by_key(|path| path.latency_stats.jitter)
    }
}

/// Path selection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSelectionStrategy {
    /// Always use the lowest latency path
    LowestLatency,
    
    /// Always use the most stable path (lowest jitter)
    MostStable,
    
    /// Round-robin between available paths
    RoundRobin,
    
    /// Weighted round-robin based on latency
    WeightedRoundRobin,
    
    /// Random selection
    Random,
    
    /// Adaptive selection based on network conditions
    Adaptive,
}

/// Network path selector for multipath routing
pub struct PathSelector {
    /// Topology manager
    topology: Arc<TopologyManager>,
    
    /// Current active paths
    active_paths: HashMap<String, NetworkPath>,
    
    /// Path selection strategy
    strategy: PathSelectionStrategy,
    
    /// Last path switch time
    last_switch: HashMap<String, Instant>,
    
    /// Minimum time between path switches
    min_switch_interval: Duration,
}

impl PathSelector {
    /// Create a new path selector
    pub fn new(
        topology: Arc<TopologyManager>,
        strategy: PathSelectionStrategy,
        min_switch_interval: Duration,
    ) -> Self {
        Self {
            topology,
            active_paths: HashMap::new(),
            strategy,
            last_switch: HashMap::new(),
            min_switch_interval,
        }
    }
    
    /// Select a path to a destination
    pub fn select_path(&mut self, destination_id: &str) -> Option<NetworkPath> {
        // Get all available paths
        let paths = self.topology.get_all_paths(destination_id);
        if paths.is_empty() {
            return None;
        }
        
        // Check if we need to switch paths
        let now = Instant::now();
        let last_switch = self.last_switch.get(destination_id).cloned().unwrap_or(Instant::now() - self.min_switch_interval * 2);
        
        if now.duration_since(last_switch) < self.min_switch_interval {
            // Use current active path if available
            if let Some(path) = self.active_paths.get(destination_id) {
                return Some(path.clone());
            }
        }
        
        // Select path based on strategy
        let selected_path = match self.strategy {
            PathSelectionStrategy::LowestLatency => {
                paths.into_iter().min_by_key(|path| path.latency_stats.avg_latency)
            }
            PathSelectionStrategy::MostStable => {
                paths.into_iter().min_by_key(|path| path.latency_stats.jitter)
            }
            PathSelectionStrategy::RoundRobin => {
                // Simple round-robin: just take the next path
                let current_idx = self.active_paths.get(destination_id)
                    .and_then(|current| paths.iter().position(|p| p.source.id == current.source.id && p.destination.id == current.destination.id))
                    .unwrap_or(0);
                
                let next_idx = (current_idx + 1) % paths.len();
                Some(paths[next_idx].clone())
            }
            PathSelectionStrategy::WeightedRoundRobin => {
                // Weight paths by inverse of latency
                let total_weight: f64 = paths.iter()
                    .map(|path| 1.0 / path.latency_stats.avg_latency.as_secs_f64())
                    .sum();
                
                let mut rng = rand::thread_rng();
                let r: f64 = rand::Rng::gen_range(&mut rng, 0.0..total_weight);
                
                let mut cumulative_weight = 0.0;
                for path in &paths {
                    let weight = 1.0 / path.latency_stats.avg_latency.as_secs_f64();
                    cumulative_weight += weight;
                    if cumulative_weight >= r {
                        return Some(path.clone());
                    }
                }
                
                Some(paths[0].clone())
            }
            PathSelectionStrategy::Random => {
                // Random selection
                let mut rng = rand::thread_rng();
                let idx = rand::Rng::gen_range(&mut rng, 0..paths.len());
                Some(paths[idx].clone())
            }
            PathSelectionStrategy::Adaptive => {
                // Adaptive selection based on network conditions
                // If current path is degraded, switch to a better one
                let current_path = self.active_paths.get(destination_id);
                
                if let Some(current) = current_path {
                    if current.status == PathStatus::Degraded || current.latency_stats.jitter.as_micros() > 100 {
                        // Current path is degraded, find a better one
                        paths.into_iter().min_by_key(|path| path.latency_stats.avg_latency)
                    } else {
                        // Current path is good, keep using it
                        Some(current.clone())
                    }
                } else {
                    // No current path, select the best one
                    paths.into_iter().min_by_key(|path| path.latency_stats.avg_latency)
                }
            }
        };
        
        // Update active path and last switch time
        if let Some(ref path) = selected_path {
            let current_path = self.active_paths.get(destination_id);
            
            // Only update if path changed
            if current_path.is_none() || current_path.unwrap().source.id != path.source.id || current_path.unwrap().destination.id != path.destination.id {
                self.active_paths.insert(destination_id.to_string(), path.clone());
                self.last_switch.insert(destination_id.to_string(), now);
            }
        }
        
        selected_path
    }
    
    /// Set path selection strategy
    pub fn set_strategy(&mut self, strategy: PathSelectionStrategy) {
        self.strategy = strategy;
    }
    
    /// Get current path selection strategy
    pub fn get_strategy(&self) -> PathSelectionStrategy {
        self.strategy
    }
    
    /// Set minimum time between path switches
    pub fn set_min_switch_interval(&mut self, interval: Duration) {
        self.min_switch_interval = interval;
    }
    
    /// Get minimum time between path switches
    pub fn get_min_switch_interval(&self) -> Duration {
        self.min_switch_interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_topology_manager_initialization() {
        let config = TopologyConfig::default();
        let mut manager = TopologyManager::new(config);
        
        assert!(!manager.running.load(std::sync::atomic::Ordering::SeqCst));
        
        // Start and stop
        manager.start().await.unwrap();
        assert!(manager.running.load(std::sync::atomic::Ordering::SeqCst));
        
        manager.stop().await.unwrap();
        assert!(!manager.running.load(std::sync::atomic::Ordering::SeqCst));
    }
    
    #[test]
    fn test_latency_stats_calculation() {
        let latencies = vec![
            Duration::from_micros(100),
            Duration::from_micros(150),
            Duration::from_micros(120),
            Duration::from_micros(200),
            Duration::from_micros(180),
        ];
        
        let stats = TopologyManager::calculate_latency_stats(&latencies);
        
        assert_eq!(stats.min_latency, Duration::from_micros(100));
        assert_eq!(stats.max_latency, Duration::from_micros(200));
        assert_eq!(stats.avg_latency, Duration::from_micros(150));
        assert_eq!(stats.median_latency, Duration::from_micros(150));
        assert_eq!(stats.samples, 5);
    }
    
    #[test]
    fn test_path_selection_strategies() {
        let config = TopologyConfig::default();
        let manager = Arc::new(TopologyManager::new(config));
        
        let mut selector = PathSelector::new(
            manager.clone(),
            PathSelectionStrategy::LowestLatency,
            Duration::from_secs(1),
        );
        
        assert_eq!(selector.get_strategy(), PathSelectionStrategy::LowestLatency);
        
        selector.set_strategy(PathSelectionStrategy::MostStable);
        assert_eq!(selector.get_strategy(), PathSelectionStrategy::MostStable);
        
        selector.set_min_switch_interval(Duration::from_secs(5));
        assert_eq!(selector.get_min_switch_interval(), Duration::from_secs(5));
    }
}