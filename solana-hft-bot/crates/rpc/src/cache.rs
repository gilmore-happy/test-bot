//! Caching system for RPC responses
//!
//! This module provides a caching system for RPC responses to reduce
//! the number of requests to the Solana network and improve performance.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{debug, error, info, trace, warn};
use anyhow::{anyhow, Result};

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether to enable caching
    pub enable_cache: bool,
    
    /// Maximum size of the cache in items
    pub max_cache_size: usize,
    
    /// Time-to-live for account data in milliseconds
    pub account_ttl_ms: u64,
    
    /// Time-to-live for blockhash in milliseconds
    pub blockhash_ttl_ms: u64,
    
    /// Time-to-live for slot data in milliseconds
    pub slot_ttl_ms: u64,
    
    /// Time-to-live for program accounts in milliseconds
    pub program_accounts_ttl_ms: u64,
    
    /// Whether to enable prefetching
    pub enable_prefetching: bool,
    
    /// Prefetch interval in milliseconds
    pub prefetch_interval_ms: u64,
    
    /// Maximum number of items to prefetch
    pub max_prefetch_items: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            max_cache_size: 10_000,
            account_ttl_ms: 2_000,    // 2 seconds
            blockhash_ttl_ms: 1_000,  // 1 second
            slot_ttl_ms: 500,         // 500 milliseconds
            program_accounts_ttl_ms: 5_000, // 5 seconds
            enable_prefetching: true,
            prefetch_interval_ms: 1_000, // 1 second
            max_prefetch_items: 10,
        }
    }
}

/// Cache entry
struct CacheEntry {
    /// Value stored in the cache
    value: Value,
    
    /// Expiration time
    expires_at: Instant,
    
    /// Access count
    access_count: u64,
    
    /// Last access time
    last_access: Instant,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total cache hits
    pub hits: u64,
    
    /// Total cache misses
    pub misses: u64,
    
    /// Total cache evictions
    pub evictions: u64,
    
    /// Total cache insertions
    pub insertions: u64,
    
    /// Current cache size
    pub size: usize,
    
    /// Maximum cache size
    pub max_size: usize,
    
    /// Hit rate (0.0 - 1.0)
    pub hit_rate: f64,
    
    /// Average TTL in milliseconds
    pub avg_ttl_ms: u64,
    
    /// Most frequently accessed keys
    pub top_keys: Vec<(String, u64)>,
}

/// Cache for RPC responses
pub struct ResponseCache {
    /// Cache entries
    entries: DashMap<String, CacheEntry>,
    
    /// Configuration
    config: CacheConfig,
    
    /// Cache statistics
    stats: Mutex<CacheStats>,
    
    /// Prefetch queue
    prefetch_queue: Mutex<VecDeque<String>>,
    
    /// Shutdown signal sender
    shutdown_tx: Mutex<Option<broadcast::Sender<()>>>,
}

impl ResponseCache {
    /// Create a new response cache
    pub fn new(config: CacheConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        
        let cache = Self {
            entries: DashMap::new(),
            config,
            stats: Mutex::new(CacheStats {
                hits: 0,
                misses: 0,
                evictions: 0,
                insertions: 0,
                size: 0,
                max_size: 0,
                hit_rate: 0.0,
                avg_ttl_ms: 0,
                top_keys: Vec::new(),
            }),
            prefetch_queue: Mutex::new(VecDeque::new()),
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
        };
        
        // Start the cache cleaner
        cache.start_cache_cleaner();
        
        // Start the prefetch worker if enabled
        if cache.config.enable_prefetching {
            cache.start_prefetch_worker();
        }
        
        cache
    }
    
    /// Start the cache cleaner
    fn start_cache_cleaner(&self) {
        let this = self.clone();
        let shutdown_rx = self.shutdown_tx.lock().as_ref().unwrap().subscribe();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        this.clean_expired().await;
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Cache cleaner shutting down");
                        break;
                    }
                }
            }
        });
    }
    
    /// Start the prefetch worker
    fn start_prefetch_worker(&self) {
        let this = self.clone();
        let interval_ms = self.config.prefetch_interval_ms;
        let shutdown_rx = self.shutdown_tx.lock().as_ref().unwrap().subscribe();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(interval_ms));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Get the next key to prefetch
                        if let Some(key) = this.get_next_prefetch_key().await {
                            debug!("Prefetching key: {}", key);
                            // The actual prefetching is done by the client
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Prefetch worker shutting down");
                        break;
                    }
                }
            }
        });
    }
    
    /// Get a value from the cache
    pub async fn get(&self, key: &str) -> Option<Value> {
        if !self.config.enable_cache {
            return None;
        }
        
        if let Some(mut entry) = self.entries.get_mut(key) {
            if entry.expires_at > Instant::now() {
                // Cache hit
                entry.access_count += 1;
                entry.last_access = Instant::now();
                
                // Update stats
                let mut stats = self.stats.lock();
                stats.hits += 1;
                
                // Calculate hit rate
                let total = stats.hits + stats.misses;
                if total > 0 {
                    stats.hit_rate = stats.hits as f64 / total as f64;
                }
                
                // Add to prefetch queue if prefetching is enabled
                if self.config.enable_prefetching {
                    let mut prefetch_queue = self.prefetch_queue.lock();
                    if !prefetch_queue.contains(&key.to_string()) {
                        prefetch_queue.push_back(key.to_string());
                        
                        // Keep queue size limited
                        while prefetch_queue.len() > self.config.max_prefetch_items {
                            prefetch_queue.pop_front();
                        }
                    }
                }
                
                return Some(entry.value.clone());
            } else {
                // Cache entry expired
                self.entries.remove(key);
                
                // Update stats
                let mut stats = self.stats.lock();
                stats.misses += 1;
                stats.size = self.entries.len();
                
                // Calculate hit rate
                let total = stats.hits + stats.misses;
                if total > 0 {
                    stats.hit_rate = stats.hits as f64 / total as f64;
                }
            }
        } else {
            // Cache miss
            let mut stats = self.stats.lock();
            stats.misses += 1;
            
            // Calculate hit rate
            let total = stats.hits + stats.misses;
            if total > 0 {
                stats.hit_rate = stats.hits as f64 / total as f64;
            }
        }
        
        None
    }
    
    /// Set a value in the cache
    pub async fn set(&self, key: &str, value: Value, ttl_ms: u64) {
        if !self.config.enable_cache {
            return;
        }
        
        // Check if we need to evict entries
        if self.entries.len() >= self.config.max_cache_size {
            self.evict_entries().await;
        }
        
        // Insert the new entry
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + Duration::from_millis(ttl_ms),
            access_count: 0,
            last_access: Instant::now(),
        };
        
        self.entries.insert(key.to_string(), entry);
        
        // Update stats
        let mut stats = self.stats.lock();
        stats.insertions += 1;
        stats.size = self.entries.len();
        stats.max_size = stats.max_size.max(stats.size);
        
        // Update average TTL
        stats.avg_ttl_ms = (stats.avg_ttl_ms * (stats.insertions - 1) + ttl_ms) / stats.insertions;
    }
    
    /// Evict entries from the cache
    async fn evict_entries(&self) {
        // Evict 10% of entries or at least 1
        let evict_count = (self.config.max_cache_size / 10).max(1);
        
        // Find the least recently accessed entries
        let mut entries_to_evict = Vec::with_capacity(evict_count);
        
        for entry in self.entries.iter() {
            entries_to_evict.push((entry.key().clone(), entry.value().last_access));
            
            if entries_to_evict.len() >= evict_count {
                break;
            }
        }
        
        // Sort by last access time (oldest first)
        entries_to_evict.sort_by(|a, b| a.1.cmp(&b.1));
        
        // Evict entries
        for (key, _) in entries_to_evict {
            self.entries.remove(&key);
        }
        
        // Update stats
        let mut stats = self.stats.lock();
        stats.evictions += evict_count as u64;
        stats.size = self.entries.len();
    }
    
    /// Clear all entries from the cache
    pub async fn clear(&self) {
        self.entries.clear();
        
        // Update stats
        let mut stats = self.stats.lock();
        stats.size = 0;
    }
    
    /// Remove expired entries from the cache
    pub async fn clean_expired(&self) {
        let now = Instant::now();
        let mut expired_count = 0;
        
        // Find and remove expired entries
        let keys_to_remove: Vec<_> = self.entries.iter()
            .filter(|entry| entry.value().expires_at <= now)
            .map(|entry| entry.key().clone())
            .collect();
        
        for key in keys_to_remove {
            self.entries.remove(&key);
            expired_count += 1;
        }
        
        if expired_count > 0 {
            debug!("Removed {} expired cache entries", expired_count);
            
            // Update stats
            let mut stats = self.stats.lock();
            stats.size = self.entries.len();
        }
    }
    
    /// Get the next key to prefetch
    pub async fn get_next_prefetch_key(&self) -> Option<String> {
        if !self.config.enable_prefetching {
            return None;
        }
        
        let mut prefetch_queue = self.prefetch_queue.lock();
        prefetch_queue.pop_front()
    }
    
    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let mut stats = self.stats.lock().clone();
        
        // Calculate top keys by access count
        let mut top_keys = Vec::new();
        
        for entry in self.entries.iter() {
            top_keys.push((entry.key().clone(), entry.value().access_count));
        }
        
        // Sort by access count (highest first)
        top_keys.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Keep only the top 10
        top_keys.truncate(10);
        
        stats.top_keys = top_keys;
        stats
    }
    
    /// Stop the cache
    pub async fn stop(&self) {
        info!("Stopping response cache");
        
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.lock().take() {
            let _ = tx.send(());
        }
    }
}

impl Clone for ResponseCache {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            config: self.config.clone(),
            stats: Mutex::new(self.stats.lock().clone()),
            prefetch_queue: Mutex::new(self.prefetch_queue.lock().clone()),
            shutdown_tx: Mutex::new(None),
        }
    }
}
