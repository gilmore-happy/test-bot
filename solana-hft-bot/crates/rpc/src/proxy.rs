//! RPC proxy for Solana
//!
//! This module provides a proxy for RPC requests to Solana validators,
//! allowing for request interception, modification, and routing.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::future::join_all;
use parking_lot::RwLock;
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, trace, warn};
use anyhow::{anyhow, Result};

use crate::EnhancedRpcClient;
use crate::request::{RequestOptions, RequestPriority};
use crate::RpcError;

/// RPC proxy configuration
#[derive(Debug, Clone)]
pub struct RpcProxyConfig {
    /// Whether to enable request logging
    pub enable_request_logging: bool,
    
    /// Whether to enable response logging
    pub enable_response_logging: bool,
    
    /// Whether to enable request modification
    pub enable_request_modification: bool,
    
    /// Whether to enable response modification
    pub enable_response_modification: bool,
    
    /// Whether to enable request routing
    pub enable_request_routing: bool,
    
    /// Whether to enable quorum verification
    pub enable_quorum_verification: bool,
    
    /// Quorum size (number of endpoints that must agree)
    pub quorum_size: usize,
    
    /// Quorum threshold (percentage of endpoints that must agree)
    pub quorum_threshold: f64,
    
    /// Method-specific configurations
    pub method_configs: HashMap<String, RpcProxyMethodConfig>,
}

impl Default for RpcProxyConfig {
    fn default() -> Self {
        let mut method_configs = HashMap::new();
        
        // Configure common methods
        method_configs.insert("getAccountInfo".to_string(), RpcProxyMethodConfig {
            require_quorum: false,
            quorum_size: 2,
            quorum_threshold: 0.66,
            priority: RequestPriority::Normal,
            timeout_ms: 15_000,
            max_retries: 3,
        });
        
        method_configs.insert("getBalance".to_string(), RpcProxyMethodConfig {
            require_quorum: false,
            quorum_size: 2,
            quorum_threshold: 0.66,
            priority: RequestPriority::Normal,
            timeout_ms: 10_000,
            max_retries: 3,
        });
        
        method_configs.insert("getLatestBlockhash".to_string(), RpcProxyMethodConfig {
            require_quorum: true,
            quorum_size: 2,
            quorum_threshold: 0.66,
            priority: RequestPriority::High,
            timeout_ms: 10_000,
            max_retries: 3,
        });
        
        method_configs.insert("sendTransaction".to_string(), RpcProxyMethodConfig {
            require_quorum: false,
            quorum_size: 1,
            quorum_threshold: 1.0,
            priority: RequestPriority::Critical,
            timeout_ms: 15_000,
            max_retries: 3,
        });
        
        Self {
            enable_request_logging: true,
            enable_response_logging: true,
            enable_request_modification: true,
            enable_response_modification: true,
            enable_request_routing: true,
            enable_quorum_verification: true,
            quorum_size: 2,
            quorum_threshold: 0.66,
            method_configs,
        }
    }
}

/// RPC proxy method configuration
#[derive(Debug, Clone)]
pub struct RpcProxyMethodConfig {
    /// Whether to require quorum for this method
    pub require_quorum: bool,
    
    /// Quorum size (number of endpoints that must agree)
    pub quorum_size: usize,
    
    /// Quorum threshold (percentage of endpoints that must agree)
    pub quorum_threshold: f64,
    
    /// Request priority
    pub priority: RequestPriority,
    
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    
    /// Maximum retries
    pub max_retries: u32,
}

/// RPC proxy command
enum RpcProxyCommand {
    /// Execute a request
    Execute {
        /// Method name
        method: String,
        
        /// Parameters
        params: Value,
        
        /// Request options
        options: RequestOptions,
        
        /// Response channel
        response_tx: oneshot::Sender<Result<Value, RpcError>>,
    },
    
    /// Shutdown the proxy
    Shutdown {
        /// Response channel
        response_tx: oneshot::Sender<()>,
    },
}

/// RPC proxy for Solana
pub struct RpcProxy {
    /// Enhanced RPC client
    client: Arc<EnhancedRpcClient>,
    
    /// Proxy configuration
    config: RpcProxyConfig,
    
    /// Request interceptors
    request_interceptors: RwLock<Vec<Box<dyn Fn(&str, &Value) -> Option<Value> + Send + Sync>>>,
    
    /// Response interceptors
    response_interceptors: RwLock<Vec<Box<dyn Fn(&str, &Value) -> Option<Value> + Send + Sync>>>,
    
    /// Command channel sender
    command_tx: mpsc::Sender<RpcProxyCommand>,
    
    /// Command channel receiver
    command_rx: Option<mpsc::Receiver<RpcProxyCommand>>,
}

impl RpcProxy {
    /// Create a new RPC proxy
    pub fn new(client: Arc<EnhancedRpcClient>, config: RpcProxyConfig) -> Self {
        let (command_tx, command_rx) = mpsc::channel(100);
        
        Self {
            client,
            config,
            request_interceptors: RwLock::new(Vec::new()),
            response_interceptors: RwLock::new(Vec::new()),
            command_tx,
            command_rx: Some(command_rx),
        }
    }
    
    /// Start the proxy
    pub async fn start(&mut self) -> Result<()> {
        let mut command_rx = self.command_rx.take().ok_or_else(|| anyhow!("Proxy already started"))?;
        
        let client = self.client.clone();
        let config = self.config.clone();
        let request_interceptors = self.request_interceptors.read().clone();
        let response_interceptors = self.response_interceptors.read().clone();
        
        tokio::spawn(async move {
            while let Some(command) = command_rx.recv().await {
                match command {
                    RpcProxyCommand::Execute { method, params, options, response_tx } => {
                        let result = Self::execute_request(
                            &client,
                            &config,
                            &request_interceptors,
                            &response_interceptors,
                            &method,
                            params,
                            options,
                        ).await;
                        
                        let _ = response_tx.send(result);
                    },
                    RpcProxyCommand::Shutdown { response_tx } => {
                        info!("Shutting down RPC proxy");
                        let _ = response_tx.send(());
                        break;
                    },
                }
            }
        });
        
        Ok(())
    }
    
    /// Execute a request
    async fn execute_request(
        client: &EnhancedRpcClient,
        config: &RpcProxyConfig,
        request_interceptors: &[Box<dyn Fn(&str, &Value) -> Option<Value> + Send + Sync>],
        response_interceptors: &[Box<dyn Fn(&str, &Value) -> Option<Value> + Send + Sync>],
        method: &str,
        params: Value,
        options: RequestOptions,
    ) -> Result<Value, RpcError> {
        let start = Instant::now();
        
        // Log the request
        if config.enable_request_logging {
            debug!("RPC request: {} with params: {}", method, params);
        }
        
        // Apply request interceptors
        let mut modified_params = params.clone();
        
        if config.enable_request_modification {
            for interceptor in request_interceptors {
                if let Some(new_params) = interceptor(method, &modified_params) {
                    modified_params = new_params;
                }
            }
        }
        
        // Get method-specific configuration
        let method_config = config.method_configs.get(method).cloned();
        
        // Determine if quorum is required
        let require_quorum = if let Some(ref method_config) = method_config {
            method_config.require_quorum
        } else {
            config.enable_quorum_verification
        };
        
        // Execute the request
        let result = if require_quorum && config.enable_quorum_verification {
            // Execute with quorum verification
            Self::execute_with_quorum(
                client,
                config,
                method,
                &modified_params,
                options,
                method_config,
            ).await
        } else {
            // Execute normally
            Self::execute_single(
                client,
                method,
                &modified_params,
                options,
            ).await
        };
        
        // Apply response interceptors
        let mut response = match result {
            Ok(value) => value,
            Err(err) => return Err(err),
        };
        
        if config.enable_response_modification {
            for interceptor in response_interceptors {
                if let Some(new_response) = interceptor(method, &response) {
                    response = new_response;
                }
            }
        }
        
        // Log the response
        if config.enable_response_logging {
            debug!("RPC response for {}: {} (took {:?})", method, response, start.elapsed());
        }
        
        Ok(response)
    }
    
    /// Execute a request with quorum verification
    async fn execute_with_quorum(
        client: &EnhancedRpcClient,
        config: &RpcProxyConfig,
        method: &str,
        params: &Value,
        options: RequestOptions,
        method_config: Option<RpcProxyMethodConfig>,
    ) -> Result<Value, RpcError> {
        // Get quorum parameters
        let quorum_size = method_config.as_ref()
            .map(|c| c.quorum_size)
            .unwrap_or(config.quorum_size);
        
        let quorum_threshold = method_config.as_ref()
            .map(|c| c.quorum_threshold)
            .unwrap_or(config.quorum_threshold);
        
        // Get all endpoints
        let endpoints = client.get_endpoint_status().await;
        
        // Filter enabled and healthy endpoints
        let active_endpoints: Vec<_> = endpoints.iter()
            .filter(|e| e.enabled && e.healthy)
            .collect();
        
        if active_endpoints.len() < quorum_size {
            return Err(RpcError::QuorumNotReached(format!(
                "Not enough active endpoints: {} (need {})",
                active_endpoints.len(),
                quorum_size
            )));
        }
        
        // Execute the request on multiple endpoints
        let mut futures = Vec::new();
        
        for endpoint in active_endpoints {
            let endpoint_url = endpoint.url.clone();
            let client = client.clone();
            let method = method.to_string();
            let params = params.clone();
            let mut options = options.clone();
            
            // Use specific endpoint
            options.specific_endpoint = Some(endpoint_url);
            
            let future = tokio::spawn(async move {
                let result = Self::execute_single(&client, &method, &params, options).await;
                (endpoint_url, result)
            });
            
            futures.push(future);
        }
        
        // Wait for all requests to complete
        let results = join_all(futures).await;
        
        // Process results
        let mut responses = Vec::new();
        let mut errors = Vec::new();
        
        for result in results {
            match result {
                Ok((endpoint, Ok(response))) => {
                    responses.push((endpoint, response));
                },
                Ok((endpoint, Err(err))) => {
                    errors.push((endpoint, err));
                },
                Err(err) => {
                    errors.push(("unknown".to_string(), RpcError::Call(err.to_string())));
                },
            }
        }
        
        // Check if we have enough responses
        if responses.len() < quorum_size {
            return Err(RpcError::QuorumNotReached(format!(
                "Not enough successful responses: {} (need {})",
                responses.len(),
                quorum_size
            )));
        }
        
        // Group responses by value
        let mut response_groups: HashMap<String, Vec<(String, Value)>> = HashMap::new();
        
        for (endpoint, response) in responses {
            let key = serde_json::to_string(&response).unwrap_or_else(|_| "error".to_string());
            response_groups.entry(key).or_default().push((endpoint, response));
        }
        
        // Find the largest group
        let largest_group = response_groups.values()
            .max_by_key(|group| group.len())
            .ok_or_else(|| RpcError::QuorumNotReached("No responses".to_string()))?;
        
        // Check if the largest group meets the quorum threshold
        let quorum_count = (active_endpoints.len() as f64 * quorum_threshold).ceil() as usize;
        
        if largest_group.len() < quorum_count {
            return Err(RpcError::QuorumNotReached(format!(
                "Quorum not reached: {} responses (need {})",
                largest_group.len(),
                quorum_count
            )));
        }
        
        // Return the response from the largest group
        Ok(largest_group[0].1.clone())
    }
    
    /// Execute a single request
    async fn execute_single(
        client: &EnhancedRpcClient,
        method: &str,
        params: &Value,
        options: RequestOptions,
    ) -> Result<Value, RpcError> {
        // TODO: Implement actual RPC method execution
        // For now, we'll just return a placeholder
        
        // In a real implementation, this would use the EnhancedRpcClient to execute the request
        // based on the method name and parameters
        
        match method {
            "getAccountInfo" => {
                // Extract pubkey from params
                let pubkey_str = params.as_array()
                    .and_then(|arr| arr.get(0))
                    .and_then(|val| val.as_str())
                    .ok_or_else(|| RpcError::Call("Invalid parameters".to_string()))?;
                
                // Parse pubkey
                let pubkey = solana_sdk::pubkey::Pubkey::try_from_str(pubkey_str)
                    .map_err(|e| RpcError::Call(format!("Invalid pubkey: {}", e)))?;
                
                // Get account
                let account = client.get_account_with_options(&pubkey, options).await?;
                
                // Convert to JSON
                let json_value = serde_json::to_value(account)
                    .map_err(|e| RpcError::Serialization(e.to_string()))?;
                
                Ok(json_value)
            },
            "getBalance" => {
                // Extract pubkey from params
                let pubkey_str = params.as_array()
                    .and_then(|arr| arr.get(0))
                    .and_then(|val| val.as_str())
                    .ok_or_else(|| RpcError::Call("Invalid parameters".to_string()))?;
                
                // Parse pubkey
                let pubkey = solana_sdk::pubkey::Pubkey::try_from_str(pubkey_str)
                    .map_err(|e| RpcError::Call(format!("Invalid pubkey: {}", e)))?;
                
                // Get account
                let account = client.get_account_with_options(&pubkey, options).await?;
                
                // Extract balance
                let balance = account.map(|a| a.lamports).unwrap_or(0);
                
                Ok(json!(balance))
            },
            "getLatestBlockhash" => {
                // Get latest blockhash
                let blockhash = client.get_latest_blockhash_with_options(options).await?;
                
                // Convert to JSON
                let json_value = serde_json::to_value(blockhash)
                    .map_err(|e| RpcError::Serialization(e.to_string()))?;
                
                Ok(json_value)
            },
            "sendTransaction" => {
                // Extract transaction from params
                let tx_str = params.as_array()
                    .and_then(|arr| arr.get(0))
                    .and_then(|val| val.as_str())
                    .ok_or_else(|| RpcError::Call("Invalid parameters".to_string()))?;
                
                // Parse transaction
                let tx_data = base64::decode(tx_str)
                    .map_err(|e| RpcError::Call(format!("Invalid transaction data: {}", e)))?;
                
                let tx = solana_sdk::transaction::Transaction::deserialize(&tx_data)
                    .map_err(|e| RpcError::Call(format!("Invalid transaction: {}", e)))?;
                
                // Send transaction
                let signature = client.send_transaction_with_options(&tx, options).await?;
                
                // Convert to JSON
                let json_value = serde_json::to_value(signature)
                    .map_err(|e| RpcError::Serialization(e.to_string()))?;
                
                Ok(json_value)
            },
            _ => {
                // Unsupported method
                Err(RpcError::Call(format!("Unsupported method: {}", method)))
            }
        }
    }
    
    /// Add a request interceptor
    pub fn add_request_interceptor<F>(&self, interceptor: F)
    where
        F: Fn(&str, &Value) -> Option<Value> + Send + Sync + 'static,
    {
        self.request_interceptors.write().push(Box::new(interceptor));
    }
    
    /// Add a response interceptor
    pub fn add_response_interceptor<F>(&self, interceptor: F)
    where
        F: Fn(&str, &Value) -> Option<Value> + Send + Sync + 'static,
    {
        self.response_interceptors.write().push(Box::new(interceptor));
    }
    
    /// Execute a request
    pub async fn execute(&self, method: &str, params: Value, options: RequestOptions) -> Result<Value, RpcError> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(RpcProxyCommand::Execute {
            method: method.to_string(),
            params,
            options,
            response_tx,
        }).await.map_err(|e| RpcError::Call(format!("Failed to send command: {}", e)))?;
        
        response_rx.await.map_err(|e| RpcError::Call(format!("Failed to receive response: {}", e)))?
    }
    
    /// Shutdown the proxy
    pub async fn shutdown(&self) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(RpcProxyCommand::Shutdown {
            response_tx,
        }).await.map_err(|e| anyhow!("Failed to send shutdown command: {}", e))?;
        
        response_rx.await.map_err(|e| anyhow!("Failed to receive shutdown response: {}", e))?;
        
        Ok(())
    }
}
