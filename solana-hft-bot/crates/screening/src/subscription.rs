use std::sync::Arc;
use anyhow::{anyhow, Result};
use futures::{stream::StreamExt, SinkExt};
use parking_lot::Mutex;
use solana_client::{
    nonblocking::pubsub_client::{PubsubClient, PubsubClientSubscription},
    rpc_config::{RpcAccountSubscribeConfig, RpcProgramAccountsConfig},
    rpc_filter::RpcFilterType,
    rpc_response::{Response, RpcKeyedAccount, SlotInfo},
};
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    signature::Signature,
};
use solana_transaction_status::TransactionStatus;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Subscription manager for websocket connections
pub struct SubscriptionManager {
    /// Websocket URL
    ws_url: String,
    
    /// Active subscriptions
    subscriptions: Mutex<Vec<PubsubClientSubscription<(), ()>>>,
    
    /// Commitment config
    commitment: CommitmentConfig,
}

/// Account update callback type
pub type AccountUpdateCallback = Box<dyn Fn(Pubkey, Account) + Send + 'static>;

/// Signature update callback type
pub type SignatureUpdateCallback = Box<dyn Fn(Signature, Option<TransactionStatus>) + Send + 'static>;

/// Slot update callback type
pub type SlotUpdateCallback = Box<dyn Fn(SlotInfo) + Send + 'static>;

impl SubscriptionManager {
    /// Create a new subscription manager
    pub async fn new(ws_url: String, commitment: CommitmentConfig) -> Result<Self> {
        Ok(Self {
            ws_url,
            subscriptions: Mutex::new(Vec::new()),
            commitment,
        })
    }
    
    /// Subscribe to program account updates
    pub async fn subscribe_program<F>(
        &self,
        program_id: Pubkey,
        filters: Option<Vec<RpcFilterType>>,
        callback: F,
    ) -> Result<()>
    where
        F: Fn(Pubkey, Account) + Send + 'static,
    {
        let client = PubsubClient::new(&self.ws_url).await?;
        
        let subscription = client.program_subscribe(
            &program_id,
            filters.as_ref(),
            Some(self.commitment),
        ).await?;
        
        let mut subscriptions = self.subscriptions.lock();
        
        // Start a task to process account updates
        tokio::spawn(async move {
            let mut subscription_stream = subscription.0;
            
            while let Some(result) = subscription_stream.next().await {
                match result {
                    Ok(Response { context: _, value: RpcKeyedAccount { pubkey, account } }) => {
                        let pubkey = match Pubkey::from_str(&pubkey) {
                            Ok(pubkey) => pubkey,
                            Err(err) => {
                                error!("Failed to parse pubkey: {}", err);
                                continue;
                            }
                        };
                        
                        callback(pubkey, account);
                    },
                    Err(err) => {
                        error!("Program subscription error: {}", err);
                    }
                }
            }
        });
        
        subscriptions.push(subscription.1);
        
        Ok(())
    }
    
    /// Subscribe to signature updates for a program
    pub async fn subscribe_signatures<F>(
        &self,
        program_id: Pubkey,
        callback: F,
    ) -> Result<()>
    where
        F: Fn(Signature, Option<TransactionStatus>) + Send + 'static,
    {
        let client = PubsubClient::new(&self.ws_url).await?;
        
        let subscription = client.signature_subscribe_with_options(
            &program_id.to_string(),
            Some(solana_client::rpc_config::RpcSignatureSubscribeConfig {
                commitment: Some(self.commitment),
                enable_received_notification: Some(false),
            }),
        ).await?;
        
        let mut subscriptions = self.subscriptions.lock();
        
        // Start a task to process signature updates
        tokio::spawn(async move {
            let mut subscription_stream = subscription.0;
            
            while let Some(result) = subscription_stream.next().await {
                match result {
                    Ok(Response { context: _, value }) => {
                        let signature = match Signature::from_str(&value.signature) {
                            Ok(signature) => signature,
                            Err(err) => {
                                error!("Failed to parse signature: {}", err);
                                continue;
                            }
                        };
                        
                        callback(signature, value.err);
                    },
                    Err(err) => {
                        error!("Signature subscription error: {}", err);
                    }
                }
            }
        });
        
        subscriptions.push(subscription.1);
        
        Ok(())
    }
    
    /// Subscribe to slot updates
    pub async fn subscribe_slots<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(SlotInfo) + Send + 'static,
    {
        let client = PubsubClient::new(&self.ws_url).await?;
        
        let subscription = client.slots_subscribe().await?;
        
        let mut subscriptions = self.subscriptions.lock();
        
        // Start a task to process slot updates
        tokio::spawn(async move {
            let mut subscription_stream = subscription.0;
            
            while let Some(result) = subscription_stream.next().await {
                match result {
                    Ok(slot_info) => {
                        callback(slot_info);
                    },
                    Err(err) => {
                        error!("Slot subscription error: {}", err);
                    }
                }
            }
        });
        
        subscriptions.push(subscription.1);
        
        Ok(())
    }
    
    /// Subscribe to account updates
    pub async fn subscribe_account<F>(
        &self,
        account: Pubkey,
        callback: F,
    ) -> Result<()>
    where
        F: Fn(Account) + Send + 'static,
    {
        let client = PubsubClient::new(&self.ws_url).await?;
        
        let subscription = client.account_subscribe(
            &account,
            Some(RpcAccountSubscribeConfig {
                commitment: Some(self.commitment),
                encoding: None,
                data_slice: None,
            }),
        ).await?;
        
        let mut subscriptions = self.subscriptions.lock();
        
        // Start a task to process account updates
        tokio::spawn(async move {
            let mut subscription_stream = subscription.0;
            
            while let Some(result) = subscription_stream.next().await {
                match result {
                    Ok(Response { context: _, value }) => {
                        callback(value);
                    },
                    Err(err) => {
                        error!("Account subscription error: {}", err);
                    }
                }
            }
        });
        
        subscriptions.push(subscription.1);
        
        Ok(())
    }
    
    /// Subscribe to multiple accounts
    pub async fn subscribe_multiple_accounts<F>(
        &self,
        accounts: Vec<Pubkey>,
        callback: F,
    ) -> Result<()>
    where
        F: Fn(Pubkey, Account) + Send + 'static,
    {
        for account in accounts {
            let callback_clone = callback.clone();
            self.subscribe_account(account, move |acc| {
                callback_clone(account, acc);
            }).await?;
        }
        
        Ok(())
    }
    
    /// Get the number of active subscriptions
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.lock().len()
    }
}

impl Drop for SubscriptionManager {
    fn drop(&mut self) {
        let subscriptions = self.subscriptions.lock();
        
        for subscription in subscriptions.iter() {
            // No need to explicitly unsubscribe, as dropping the subscription
            // will automatically unsubscribe
            // This is handled by Drop implementation in PubsubClientSubscription
        }
    }
}

/// Helper function to convert string to Pubkey
fn from_str(s: &str) -> Result<Pubkey> {
    Pubkey::from_str(s).map_err(|e| anyhow!("Failed to parse pubkey: {}", e))
}

/// Helper trait to convert from string to Pubkey
trait FromStr {
    fn from_str(s: &str) -> Result<Self> where Self: Sized;
}

impl FromStr for Pubkey {
    fn from_str(s: &str) -> Result<Self> {
        Pubkey::from_str(s).map_err(|e| anyhow!("Failed to parse pubkey: {}", e))
    }
}

impl FromStr for Signature {
    fn from_str(s: &str) -> Result<Self> {
        Signature::from_str(s).map_err(|e| anyhow!("Failed to parse signature: {}", e))
    }
}