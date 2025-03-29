// High-performance RPC client for Solana
// This file integrates with the solana-hft-bot/crates/rpc module

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use tracing::{debug, error, info, warn};

use solana_hft_bot::rpc::{
    EnhancedRpcClient,
    RpcConfig,
    EndpointConfig,
    BatchingConfig,
    RetryConfig,
    CacheConfig,
    RpcProxy,
    RpcProxyConfig,
    RequestOptions,
    RequestPriority,
};

/// High-performance RPC client for Solana
pub struct HighPerformanceRpcClient {
    /// Enhanced RPC client
    client: Arc<EnhancedRpcClient>,
    
    /// RPC proxy
    proxy: Option<RpcProxy>,
}

impl HighPerformanceRpcClient {
    /// Create a new high-performance RPC client
    pub async fn new(endpoints: Vec<String>, commitment: CommitmentConfig) -> Result<Self> {
        // Create endpoint configs
        let endpoint_configs = endpoints.into_iter()
            .map(|url| EndpointConfig {
                url,
                weight: 1,
                region: "global".to_string(),
                features: vec![],
                tier: "free".to_string(),
            })
            .collect();
        
        // Create RPC config
        let config = RpcConfig {
            endpoints: endpoint_configs,
            websocket_endpoints: vec![],
            commitment_config: commitment,
            connection_pool_size_per_endpoint: 5,
            request_timeout_ms: 15_000,
            max_concurrent_requests: 100,
            skip_preflight: true,
            send_transaction_retry_count: 3,
            rate_limit_requests_per_second: 100.0,
            rate_limit_burst_size: 200.0,
            endpoint_health_check_interval_ms: 30_000,
            connection_refresh_interval_ms: 60_000,
            cache_config: CacheConfig::default(),
            batching_config: BatchingConfig::default(),
            retry_config: RetryConfig::default(),
            endpoint_selection_strategy: solana_hft_bot::rpc::EndpointSelectionStrategy::WeightedRandom,
            use_websocket: true,
            method_configs: Default::default(),
        };
        
        // Create enhanced RPC client
        let client = Arc::new(EnhancedRpcClient::new(config).await?);
        
        // Create RPC proxy
        let proxy_config = RpcProxyConfig::default();
        let mut proxy = RpcProxy::new(client.clone(), proxy_config);
        proxy.start().await?;
        
        Ok(Self {
            client,
            proxy: Some(proxy),
        })
    }
    
    /// Send a transaction
    pub async fn send_transaction(&self, transaction: &Transaction) -> Result<Signature> {
        let options = RequestOptions::default()
            .with_priority(RequestPriority::Critical);
        
        if let Some(proxy) = &self.proxy {
            // Use proxy
            let params = serde_json::to_value(transaction)?;
            let result = proxy.execute("sendTransaction", params, options).await?;
            let signature = serde_json::from_value(result)?;
            Ok(signature)
        } else {
            // Use client directly
            let signature = self.client.send_transaction_with_options(transaction, options).await?;
            Ok(signature)
        }
    }
    
    /// Get account information
    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<solana_sdk::account::Account>> {
        let options = RequestOptions::default();
        
        if let Some(proxy) = &self.proxy {
            // Use proxy
            let params = serde_json::to_value(vec![pubkey.to_string()])?;
            let result = proxy.execute("getAccountInfo", params, options).await?;
            let account = serde_json::from_value(result)?;
            Ok(account)
        } else {
            // Use client directly
            let account = self.client.get_account_with_options(pubkey, options).await?;
            Ok(account)
        }
    }
    
    /// Get multiple accounts
    pub async fn get_multiple_accounts(&self, pubkeys: &[Pubkey]) -> Result<Vec<Option<solana_sdk::account::Account>>> {
        let options = RequestOptions::default();
        
        if let Some(proxy) = &self.proxy {
            // Use proxy
            let pubkey_strings: Vec<String> = pubkeys.iter().map(|p| p.to_string()).collect();
            let params = serde_json::to_value(pubkey_strings)?;
            let result = proxy.execute("getMultipleAccounts", params, options).await?;
            let accounts = serde_json::from_value(result)?;
            Ok(accounts)
        } else {
            // Use client directly
            let accounts = self.client.get_multiple_accounts_with_options(pubkeys, options).await?;
            Ok(accounts)
        }
    }
    
    /// Get the latest blockhash
    pub async fn get_latest_blockhash(&self) -> Result<solana_sdk::hash::Hash> {
        let options = RequestOptions::default()
            .with_priority(RequestPriority::High);
        
        if let Some(proxy) = &self.proxy {
            // Use proxy
            let params = serde_json::to_value([])?;
            let result = proxy.execute("getLatestBlockhash", params, options).await?;
            let blockhash = serde_json::from_value(result)?;
            Ok(blockhash)
        } else {
            // Use client directly
            let blockhash = self.client.get_latest_blockhash_with_options(options).await?;
            Ok(blockhash)
        }
    }
    
    /// Get transaction status
    pub async fn get_transaction_status(&self, signature: &Signature) -> Result<Option<solana_transaction_status::TransactionStatus>> {
        let options = RequestOptions::default();
        
        if let Some(proxy) = &self.proxy {
            // Use proxy
            let params = serde_json::to_value(vec![signature.to_string()])?;
            let result = proxy.execute("getSignatureStatuses", params, options).await?;
            let status = serde_json::from_value(result)?;
            Ok(status)
        } else {
            // Use client directly
            let status = self.client.get_transaction_status_with_options(signature, options).await?;
            Ok(status)
        }
    }
    
    /// Get the current slot
    pub async fn get_slot(&self) -> Result<u64> {
        let options = RequestOptions::default();
        
        if let Some(proxy) = &self.proxy {
            // Use proxy
            let params = serde_json::to_value([])?;
            let result = proxy.execute("getSlot", params, options).await?;
            let slot = serde_json::from_value(result)?;
            Ok(slot)
        } else {
            // Use client directly
            let slot = self.client.get_slot_with_options(options).await?;
            Ok(slot)
        }
    }
    
    /// Get endpoint status
    pub async fn get_endpoint_status(&self) -> Vec<solana_hft_bot::rpc::EndpointStatus> {
        self.client.get_endpoint_status().await
    }
    
    /// Get metrics
    pub fn get_metrics(&self) -> solana_hft_bot::rpc::RpcMetricsSnapshot {
        self.client.get_metrics()
    }
    
    /// Get cache stats
    pub async fn get_cache_stats(&self) -> solana_hft_bot::rpc::CacheStats {
        self.client.get_cache_stats().await
    }
    
    /// Get rate limiter stats
    pub fn get_rate_limiter_stats(&self) -> solana_hft_bot::rpc::RateLimiterStats {
        self.client.get_rate_limiter_stats()
    }
    
    /// Shutdown the client
    pub async fn shutdown(&self) -> Result<()> {
        // Shutdown proxy
        if let Some(proxy) = &self.proxy {
            proxy.shutdown().await?;
        }
        
        // Shutdown client
        self.client.shutdown().await;
        
        Ok(())
    }
}