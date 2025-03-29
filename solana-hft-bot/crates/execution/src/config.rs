//! Configuration for the execution engine
//! 
//! This module provides configuration options for the execution engine.

use serde::{Deserialize, Serialize};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::Keypair,
};
use std::time::Duration;

/// Configuration for the execution engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Maximum number of concurrent transactions
    pub max_concurrent_transactions: usize,
    
    /// Queue capacity for execution requests
    pub queue_capacity: usize,
    
    /// Default commitment level
    pub commitment_config: CommitmentConfig,
    
    /// Whether to skip preflight transaction checks
    pub skip_preflight: bool,
    
    /// Number of retries for sending transactions
    pub send_transaction_retry_count: usize,
    
    /// Retry backoff base in milliseconds
    pub retry_backoff_ms: u64,
    
    /// Maximum retry backoff in milliseconds
    pub max_retry_backoff_ms: u64,
    
    /// Default priority fee in micro-lamports
    pub default_priority_fee: Option<u64>,
    
    /// Default compute units
    pub default_compute_units: Option<u32>,
    
    /// Whether to use Jito for MEV bundles
    pub use_jito: bool,
    
    /// Jito endpoint URL
    pub jito_endpoint: String,
    
    /// Jito authentication keypair (as string for serialization)
    pub jito_auth_keypair: String,
    
    /// Jito authentication token
    pub jito_auth_token: Option<String>,
    
    /// Jito tip account
    pub jito_tip_account: Option<String>,
    
    /// Jito tip amount in lamports
    pub jito_tip_amount: u64,
    
    /// Jito maximum bundle size
    pub jito_max_bundle_size: usize,
    
    /// Jito submission timeout in milliseconds
    pub jito_submission_timeout_ms: u64,
    
    /// Jito minimum tip in lamports
    pub jito_min_tip_lamports: u64,
    
    /// Jito maximum tip in lamports
    pub jito_max_tip_lamports: u64,
    
    /// Jito tip adjustment factor
    pub jito_tip_adjustment_factor: f64,
    
    /// Whether to use dynamic tips for Jito
    pub jito_use_dynamic_tips: bool,
    
    /// Whether to use the Jito searcher API
    pub jito_use_searcher_api: bool,
    
    /// Jito searcher API URL
    pub jito_searcher_api_url: Option<String>,
    
    /// Jito searcher API authentication token
    pub jito_searcher_api_auth_token: Option<String>,
    
    /// Whether to enable Jito ShredStream
    pub use_jito_shredstream: bool,
    
    /// Jito ShredStream block engine URL
    pub jito_shredstream_block_engine_url: String,
    
    /// Jito ShredStream auth keypair path
    pub jito_shredstream_auth_keypair_path: Option<String>,
    
    /// Jito ShredStream desired regions
    pub jito_shredstream_regions: Vec<String>,
    
    /// Jito ShredStream destination IP:Port combinations
    pub jito_shredstream_dest_ip_ports: Vec<String>,
    
    /// Jito ShredStream source bind port
    pub jito_shredstream_src_bind_port: Option<u16>,
    
    /// Whether to enable Jito ShredStream gRPC service
    pub jito_shredstream_enable_grpc: bool,
    
    /// Jito ShredStream gRPC service port
    pub jito_shredstream_grpc_port: Option<u16>,
    
    /// Whether to enable multi-path submission
    pub enable_multi_path_submission: bool,
    
    /// Primary RPC URL
    pub primary_rpc_url: String,
    
    /// Additional RPC endpoints for redundancy
    pub additional_rpc_endpoints: Vec<String>,
    
    /// RPC authentication tokens for additional endpoints
    pub rpc_auth_tokens: Vec<String>,
    
    /// Maximum number of parallel endpoints to use
    pub max_parallel_endpoints: usize,
    
    /// Whether to cancel in-flight transactions after first success
    pub cancel_after_first_success: bool,
    
    /// Maximum age of transactions in seconds before they're considered stale
    pub transaction_max_age_seconds: u64,
    
    /// Interval in milliseconds to check for transaction recovery
    pub recovery_check_interval_ms: u64,
    
    /// Interval in milliseconds to update endpoint rankings
    pub endpoint_ranking_update_interval_ms: u64,
    
    /// Whether to simulate transactions before sending
    pub simulate_transactions: bool,
    
    /// Whether to enable transaction prioritization
    pub enable_prioritization: bool,
    
    /// Default transaction timeout in milliseconds
    pub default_transaction_timeout_ms: u64,
    
    /// Maximum transaction fee in lamports
    pub max_transaction_fee: Option<u64>,
    
    /// Whether to enable automatic fee estimation
    pub enable_fee_estimation: bool,
    
    /// Transaction buffer size in bytes
    pub transaction_buffer_size: Option<usize>,
    
    /// Maximum number of transaction buffers to keep in the memory pool
    pub max_transaction_buffers: Option<usize>,
    
    /// Whether to enable speculative execution
    pub enable_speculative_execution: Option<bool>,
    
    /// Whether to enable SIMD optimizations
    pub enable_simd: Option<bool>,
    
    /// Whether to enable hardware-aware thread management
    pub enable_hardware_optimizations: Option<bool>,
    
    /// Number of worker threads
    pub worker_threads: Option<usize>,
    
    /// Whether to use NUMA-aware memory allocation
    pub use_numa_allocation: Option<bool>,
    
    /// Whether to use thread pinning
    pub use_thread_pinning: Option<bool>,
    
    /// Whether to use isolated CPUs for critical path processing
    pub use_isolated_cpus: Option<bool>,
    
    /// Whether to enable zero-copy serialization
    pub enable_zero_copy: Option<bool>,
    
    /// Whether to enable batch processing
    pub enable_batch_processing: Option<bool>,
    
    /// Batch size for signature verification
    pub signature_verification_batch_size: Option<usize>,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_transactions: 100,
            queue_capacity: 1000,
            commitment_config: CommitmentConfig::confirmed(),
            skip_preflight: true,
            send_transaction_retry_count: 3,
            retry_backoff_ms: 100,
            max_retry_backoff_ms: 1000,
            default_priority_fee: Some(1_000_000), // 1 SOL per million CU
            default_compute_units: Some(200_000),
            use_jito: false,
            jito_endpoint: "https://mainnet.block-engine.jito.io/api".to_string(),
            jito_auth_keypair: Keypair::new().to_base58_string(), // Generate a random keypair by default
            jito_auth_token: None,
            jito_tip_account: None,
            jito_tip_amount: 10_000, // 0.00001 SOL
            jito_max_bundle_size: 5,
            jito_submission_timeout_ms: 5000, // 5 seconds
            jito_min_tip_lamports: 10_000, // 0.00001 SOL
            jito_max_tip_lamports: 1_000_000, // 0.001 SOL
            jito_tip_adjustment_factor: 1.5,
            jito_use_dynamic_tips: true,
            jito_use_searcher_api: false,
            jito_searcher_api_url: None,
            jito_searcher_api_auth_token: None,
            
            // Jito ShredStream options
            use_jito_shredstream: false,
            jito_shredstream_block_engine_url: "https://mainnet.block-engine.jito.wtf".to_string(),
            jito_shredstream_auth_keypair_path: None,
            jito_shredstream_regions: vec!["amsterdam".to_string(), "ny".to_string()],
            jito_shredstream_dest_ip_ports: vec!["127.0.0.1:8001".to_string()],
            jito_shredstream_src_bind_port: Some(20000),
            jito_shredstream_enable_grpc: false,
            jito_shredstream_grpc_port: Some(50051),
            
            // Multi-path submission options
            enable_multi_path_submission: false, // Disabled by default
            primary_rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            additional_rpc_endpoints: vec![
                "https://solana-api.projectserum.com".to_string(),
                "https://rpc.ankr.com/solana".to_string(),
            ],
            rpc_auth_tokens: Vec::new(),
            max_parallel_endpoints: 3,
            cancel_after_first_success: true,
            transaction_max_age_seconds: 60, // 1 minute
            recovery_check_interval_ms: 500, // 500ms
            endpoint_ranking_update_interval_ms: 60_000, // 1 minute
            
            simulate_transactions: true,
            enable_prioritization: true,
            default_transaction_timeout_ms: 30_000, // 30 seconds
            max_transaction_fee: Some(10_000_000), // 0.01 SOL
            enable_fee_estimation: true,
            
            // New optimized execution options
            transaction_buffer_size: Some(8 * 1024),      // 8KB default
            max_transaction_buffers: Some(1024),          // 1024 buffers default
            enable_speculative_execution: Some(true),     // Enable by default
            enable_simd: Some(true),                      // Enable by default
            enable_hardware_optimizations: Some(true),    // Enable by default
            worker_threads: None,                         // Auto-detect
            use_numa_allocation: Some(true),              // Enable by default
            use_thread_pinning: Some(true),               // Enable by default
            use_isolated_cpus: Some(true),                // Enable by default
            enable_zero_copy: Some(true),                 // Enable by default
            enable_batch_processing: Some(true),          // Enable by default
            signature_verification_batch_size: Some(16),  // Default batch size
        }
    }
}

impl ExecutionConfig {
    /// Create a new execution config with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a new execution config for mainnet
    pub fn mainnet() -> Self {
        Self {
            commitment_config: CommitmentConfig::confirmed(),
            ..Self::default()
        }
    }
    
    /// Create a new execution config for devnet
    pub fn devnet() -> Self {
        Self {
            commitment_config: CommitmentConfig::confirmed(),
            jito_endpoint: "https://devnet.block-engine.jito.io/api".to_string(),
            ..Self::default()
        }
    }
    
    /// Create a new execution config for testnet
    pub fn testnet() -> Self {
        Self {
            commitment_config: CommitmentConfig::confirmed(),
            jito_endpoint: "https://testnet.block-engine.jito.io/api".to_string(),
            ..Self::default()
        }
    }
    
    /// Create a new execution config for localnet
    pub fn localnet() -> Self {
        Self {
            commitment_config: CommitmentConfig::confirmed(),
            use_jito: false,
            simulate_transactions: false,
            ..Self::default()
        }
    }
    
    /// Set the Jito authentication keypair
    pub fn with_jito_keypair(mut self, keypair: Keypair) -> Self {
        self.jito_auth_keypair = keypair.to_base58_string();
        self
    }
    
    /// Set the Jito tip account and amount
    pub fn with_jito_tip(mut self, account: String, amount: u64) -> Self {
        self.jito_tip_account = Some(account);
        self.jito_tip_amount = amount;
        self
    }
    
    /// Set the default priority fee
    pub fn with_priority_fee(mut self, fee: u64) -> Self {
        self.default_priority_fee = Some(fee);
        self
    }
    
    /// Set the default compute units
    pub fn with_compute_units(mut self, units: u32) -> Self {
        self.default_compute_units = Some(units);
        self
    }
    
    /// Set the maximum transaction fee
    pub fn with_max_fee(mut self, fee: u64) -> Self {
        self.max_transaction_fee = Some(fee);
        self
    }
    
    /// Get the default transaction timeout as a Duration
    pub fn default_timeout(&self) -> Duration {
        Duration::from_millis(self.default_transaction_timeout_ms)
    }
    
    /// Configure transaction buffer settings
    pub fn with_buffer_settings(mut self, buffer_size: usize, max_buffers: usize) -> Self {
        self.transaction_buffer_size = Some(buffer_size);
        self.max_transaction_buffers = Some(max_buffers);
        self
    }
    
    /// Configure SIMD optimizations
    pub fn with_simd(mut self, enable: bool) -> Self {
        self.enable_simd = Some(enable);
        self
    }
    
    /// Configure speculative execution
    pub fn with_speculative_execution(mut self, enable: bool) -> Self {
        self.enable_speculative_execution = Some(enable);
        self
    }
    
    /// Configure hardware optimizations
    pub fn with_hardware_optimizations(mut self, enable: bool) -> Self {
        self.enable_hardware_optimizations = Some(enable);
        self
    }
    
    /// Configure thread settings
    pub fn with_thread_settings(
        mut self,
        worker_threads: Option<usize>,
        use_thread_pinning: bool,
        use_numa_allocation: bool,
        use_isolated_cpus: bool,
    ) -> Self {
        self.worker_threads = worker_threads;
        self.use_thread_pinning = Some(use_thread_pinning);
        self.use_numa_allocation = Some(use_numa_allocation);
        self.use_isolated_cpus = Some(use_isolated_cpus);
        self
    }
    
    /// Configure batch processing
    pub fn with_batch_processing(mut self, enable: bool, batch_size: Option<usize>) -> Self {
        self.enable_batch_processing = Some(enable);
        if let Some(size) = batch_size {
            self.signature_verification_batch_size = Some(size);
        }
        self
    }
    
    /// Configure zero-copy serialization
    pub fn with_zero_copy(mut self, enable: bool) -> Self {
        self.enable_zero_copy = Some(enable);
        self
    }
    
    /// Configure high-performance mode (enables all optimizations)
    pub fn high_performance_mode(mut self) -> Self {
        self.enable_simd = Some(true);
        self.enable_speculative_execution = Some(true);
        self.enable_hardware_optimizations = Some(true);
        self.use_thread_pinning = Some(true);
        self.use_numa_allocation = Some(true);
        self.use_isolated_cpus = Some(true);
        self.enable_zero_copy = Some(true);
        self.enable_batch_processing = Some(true);
        self.signature_verification_batch_size = Some(32); // Larger batch size for high performance
        self.transaction_buffer_size = Some(16 * 1024);    // Larger buffer size
        self.max_transaction_buffers = Some(2048);         // More buffers
        self
    }
    
    /// Configure low-latency mode (optimized for minimal latency)
    pub fn low_latency_mode(mut self) -> Self {
        self.enable_simd = Some(true);
        self.enable_speculative_execution = Some(true);
        self.enable_hardware_optimizations = Some(true);
        self.use_thread_pinning = Some(true);
        self.use_numa_allocation = Some(true);
        self.use_isolated_cpus = Some(true);
        self.enable_zero_copy = Some(true);
        self.enable_batch_processing = Some(false);        // Disable batching for lower latency
        self.transaction_buffer_size = Some(4 * 1024);     // Smaller buffer size for lower latency
        self.max_transaction_buffers = Some(4096);         // More buffers for higher throughput
        self
    }
    
    /// Configure multi-path submission
    pub fn with_multi_path_submission(
        mut self,
        enable: bool,
        primary_url: Option<String>,
        additional_endpoints: Option<Vec<String>>,
        auth_tokens: Option<Vec<String>>,
        max_parallel: Option<usize>,
        cancel_after_first_success: Option<bool>,
    ) -> Self {
        self.enable_multi_path_submission = enable;
        
        if let Some(url) = primary_url {
            self.primary_rpc_url = url;
        }
        
        if let Some(endpoints) = additional_endpoints {
            self.additional_rpc_endpoints = endpoints;
        }
        
        if let Some(tokens) = auth_tokens {
            self.rpc_auth_tokens = tokens;
        }
        
        if let Some(max) = max_parallel {
            self.max_parallel_endpoints = max;
        }
        
        if let Some(cancel) = cancel_after_first_success {
            self.cancel_after_first_success = cancel;
        }
        
        self
    }
    
    /// Configure recovery and timeout settings for multi-path submission
    pub fn with_recovery_settings(
        mut self,
        max_age_seconds: Option<u64>,
        recovery_interval_ms: Option<u64>,
        ranking_update_interval_ms: Option<u64>,
        transaction_timeout_ms: Option<u64>,
    ) -> Self {
        if let Some(age) = max_age_seconds {
            self.transaction_max_age_seconds = age;
        }
        
        if let Some(interval) = recovery_interval_ms {
            self.recovery_check_interval_ms = interval;
        }
        
        if let Some(update) = ranking_update_interval_ms {
            self.endpoint_ranking_update_interval_ms = update;
        }
        
        if let Some(timeout) = transaction_timeout_ms {
            self.default_transaction_timeout_ms = timeout;
        }
        
        self
    }
    
    /// Configure Jito ShredStream settings
    pub fn with_shredstream(
        mut self,
        enable: bool,
        block_engine_url: Option<String>,
        auth_keypair_path: Option<String>,
        regions: Option<Vec<String>>,
        dest_ip_ports: Option<Vec<String>>,
        src_bind_port: Option<u16>,
        enable_grpc: Option<bool>,
        grpc_port: Option<u16>,
    ) -> Self {
        self.use_jito_shredstream = enable;
        
        if let Some(url) = block_engine_url {
            self.jito_shredstream_block_engine_url = url;
        }
        
        self.jito_shredstream_auth_keypair_path = auth_keypair_path;
        
        if let Some(r) = regions {
            self.jito_shredstream_regions = r;
        }
        
        if let Some(d) = dest_ip_ports {
            self.jito_shredstream_dest_ip_ports = d;
        }
        
        self.jito_shredstream_src_bind_port = src_bind_port;
        
        if let Some(e) = enable_grpc {
            self.jito_shredstream_enable_grpc = e;
        }
        
        self.jito_shredstream_grpc_port = grpc_port;
        
        self
    }
    
    /// Configure high-reliability mode (enables multi-path submission with optimal settings)
    pub fn high_reliability_mode(mut self) -> Self {
        // Enable multi-path submission
        self.enable_multi_path_submission = true;
        
        // Use more parallel endpoints
        self.max_parallel_endpoints = 5;
        
        // Cancel after first success to reduce network load
        self.cancel_after_first_success = true;
        
        // More aggressive recovery
        self.transaction_max_age_seconds = 30; // 30 seconds
        self.recovery_check_interval_ms = 250; // 250ms
        
        // More frequent endpoint ranking updates
        self.endpoint_ranking_update_interval_ms = 30_000; // 30 seconds
        
        // Shorter transaction timeout
        self.default_transaction_timeout_ms = 15_000; // 15 seconds
        
        self
    }
    
    /// Configure ultra-low-latency mode (enables Jito ShredStream and optimizes for lowest latency)
    pub fn ultra_low_latency_mode(mut self) -> Self {
        // Enable Jito ShredStream for lowest latency block updates
        self.use_jito_shredstream = true;
        
        // Enable Jito bundles for priority access
        self.use_jito = true;
        
        // Configure for low latency
        self = self.low_latency_mode();
        
        // Use more aggressive settings
        self.transaction_buffer_size = Some(2 * 1024);     // Even smaller buffer size
        self.max_transaction_buffers = Some(8192);         // More buffers
        self.default_transaction_timeout_ms = 5_000;       // Shorter timeout
        
        self
    }
}
