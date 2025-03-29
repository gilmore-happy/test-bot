//! Batch processing for RPC requests
//!
//! This module provides functionality for batching RPC requests
//! to improve throughput and reduce overhead.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::{future::join_all, StreamExt};
use serde_json::{json, Value};
use tokio::sync::oneshot;
use tracing::{debug, error, info, trace, warn};
use anyhow::{anyhow, Result};

use crate::metrics::RpcMetrics;
use crate::request::{RequestContext, RequestPriority};
use crate::RpcError;

/// Batch processing configuration
#[derive(Debug, Clone)]
pub struct BatchingConfig {
    /// Maximum batch size
    pub max_batch_size: usize,
    
    /// Maximum batch interval in milliseconds
    pub max_batch_interval_ms: u64,
    
    /// Whether to enable batching
    pub enable_batching: bool,
    
    /// Minimum batch size to process immediately
    pub min_batch_size: usize,
}

impl Default for BatchingConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 20,
            max_batch_interval_ms: 10,
            enable_batching: true,
            min_batch_size: 5,
        }
    }
}

/// Batch request
#[derive(Debug)]
pub struct BatchRequest {
    /// Request context
    pub context: RequestContext,
    
    /// RPC method
    pub method: String,
    
    /// RPC parameters
    pub params: Value,
    
    /// Response channel
    pub response_tx: oneshot::Sender<Result<Value, RpcError>>,
}

/// Batch response
#[derive(Debug)]
pub struct BatchResponse {
    /// Request ID
    pub request_id: u64,
    
    /// Response value
    pub value: Result<Value, RpcError>,
}

/// Batch processor for RPC requests
pub struct BatchProcessor {
    /// Batching configuration
    config: BatchingConfig,
    
    /// Metrics collector
    metrics: Arc<RpcMetrics>,
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(config: BatchingConfig, metrics: Arc<RpcMetrics>) -> Self {
        Self {
            config,
            metrics,
        }
    }
    
    /// Process a batch of requests
    pub async fn process_batch<C>(
        &self,
        requests: Vec<BatchRequest>,
        client: C,
    ) -> Result<()>
    where
        C: Clone + Send + Sync + 'static,
        C: Fn(&str, Vec<Value>, RequestPriority) -> Result<Vec<Value>>,
    {
        if requests.is_empty() {
            return Ok(());
        }
        
        let batch_id = self.metrics.generate_request_id();
        let start = Instant::now();
        let batch_size = requests.len();
        
        debug!("Processing batch {} with {} requests", batch_id, batch_size);
        
        // Group requests by method
        let mut method_groups: HashMap<String, Vec<BatchRequest>> = HashMap::new();
        
        for request in requests {
            method_groups
                .entry(request.method.clone())
                .or_insert_with(Vec::new)
                .push(request);
        }
        
        // Process each method group
        let mut futures = Vec::new();
        
        for (method, requests) in method_groups {
            let client = client.clone();
            let metrics = self.metrics.clone();
            
            let future = tokio::spawn(async move {
                let group_start = Instant::now();
                let group_size = requests.len();
                let priority = requests.first().map(|r| r.context.priority).unwrap_or(RequestPriority::Normal);
                
                // Extract parameters and create response channels
                let mut params = Vec::with_capacity(group_size);
                let mut response_channels = Vec::with_capacity(group_size);
                
                for request in requests {
                    params.push(request.params);
                    response_channels.push((request.context.id, request.response_tx));
                }
                
                // Execute the batch request
                match client(&method, params, priority) {
                    Ok(responses) => {
                        // Send responses to channels
                        for (i, (request_id, tx)) in response_channels.into_iter().enumerate() {
                            if i < responses.len() {
                                let _ = tx.send(Ok(responses[i].clone()));
                                metrics.record_request_success(request_id, group_start.elapsed());
                            } else {
                                let _ = tx.send(Err(RpcError::BatchProcessing(
                                    "Batch response count mismatch".to_string()
                                )));
                                metrics.record_request_error(request_id, "Batch response count mismatch");
                            }
                        }
                    },
                    Err(err) => {
                        // Send error to all channels
                        for (request_id, tx) in response_channels {
                            let _ = tx.send(Err(RpcError::BatchProcessing(
                                format!("Batch request failed: {}", err)
                            )));
                            metrics.record_request_error(request_id, &err.to_string());
                        }
                    }
                }
                
                debug!(
                    "Processed batch group for method {} with {} requests in {:?}",
                    method, group_size, group_start.elapsed()
                );
            });
            
            futures.push(future);
        }
        
        // Wait for all method groups to complete
        let results = join_all(futures).await;
        
        // Check for errors
        for result in results {
            if let Err(e) = result {
                error!("Batch processing task failed: {}", e);
            }
        }
        
        debug!(
            "Processed batch {} with {} requests in {:?}",
            batch_id, batch_size, start.elapsed()
        );
        
        Ok(())
    }
}