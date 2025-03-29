//! WebSocket client for Solana
//! 
//! This module provides WebSocket functionality for subscribing to Solana events
//! such as account updates, slot updates, and transaction confirmations.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::{SinkExt, StreamExt};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_client::pubsub_client::{PubsubClient, PubsubClientSubscription};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::interval;
use tracing::{debug, error, info, trace, warn};
use anyhow::{anyhow, Result};

use crate::metrics::RpcMetrics;

/// WebSocket subscription type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubscriptionType {
    /// Account subscription
    Account(Pubkey),
    
    /// Program subscription
    Program(Pubkey),
    
    /// Signature subscription
    Signature(Signature),
    
    /// Slot subscription
    Slot,
    
    /// Root subscription
    Root,
    
    /// Vote subscription
    Vote,
}

/// WebSocket subscription
#[derive(Debug, Clone)]
pub struct WebsocketSubscription {
    /// Subscription ID
    pub id: u64,
    
    /// Subscription type
    pub subscription_type: SubscriptionType,
    
    /// Commitment level
    pub commitment: CommitmentConfig,
    
    /// Subscription time
    pub created_at: Instant,
    
    /// Last notification time
    pub last_notification: Option<Instant>,
    
    /// Notification channel
    pub notification_tx: broadcast::Sender<Value>,
}

/// WebSocket connection
struct WebsocketConnection {
    /// Connection URL
    url: String,
    
    /// PubSub client
    client: Option<PubsubClient>,
    
    /// Active subscriptions
    subscriptions: HashMap<u64, PubsubClientSubscription>,
    
    /// Last connection attempt
    last_connection_attempt: Instant,
    
    /// Connection status
    is_connected: bool,
    
    /// Connection errors
    connection_errors: u32,
}

impl WebsocketConnection {
    /// Create a new WebSocket connection
    fn new(url: String) -> Self {
        Self {
            url,
            client: None,
            subscriptions: HashMap::new(),
            last_connection_attempt: Instant::now(),
            is_connected: false,
            connection_errors: 0,
        }
    }
    
    /// Connect to the WebSocket server
    async fn connect(&mut self) -> Result<()> {
        if self.is_connected {
            return Ok(());
        }
        
        self.last_connection_attempt = Instant::now();
        
        match PubsubClient::new(&self.url).await {
            Ok(client) => {
                self.client = Some(client);
                self.is_connected = true;
                self.connection_errors = 0;
                info!("Connected to WebSocket server: {}", self.url);
                Ok(())
            },
            Err(err) => {
                self.is_connected = false;
                self.connection_errors += 1;
                error!("Failed to connect to WebSocket server {}: {}", self.url, err);
                Err(anyhow!("WebSocket connection failed: {}", err))
            }
        }
    }
    
    /// Disconnect from the WebSocket server
    async fn disconnect(&mut self) {
        self.is_connected = false;
        self.client = None;
        self.subscriptions.clear();
        debug!("Disconnected from WebSocket server: {}", self.url);
    }
    
    /// Subscribe to an account
    async fn subscribe_account(
        &mut self,
        pubkey: &Pubkey,
        commitment: CommitmentConfig,
        notification_handler: impl Fn(Value) + Send + 'static,
    ) -> Result<PubsubClientSubscription> {
        if !self.is_connected {
            return Err(anyhow!("Not connected to WebSocket server"));
        }
        
        let client = self.client.as_ref().unwrap();
        
        match client.account_subscribe(pubkey, commitment, notification_handler).await {
            Ok(subscription) => {
                debug!("Subscribed to account {} on {}", pubkey, self.url);
                Ok(subscription)
            },
            Err(err) => {
                error!("Failed to subscribe to account {} on {}: {}", pubkey, self.url, err);
                Err(anyhow!("Account subscription failed: {}", err))
            }
        }
    }
    
    /// Subscribe to a program
    async fn subscribe_program(
        &mut self,
        program_id: &Pubkey,
        commitment: CommitmentConfig,
        notification_handler: impl Fn(Value) + Send + 'static,
    ) -> Result<PubsubClientSubscription> {
        if !self.is_connected {
            return Err(anyhow!("Not connected to WebSocket server"));
        }
        
        let client = self.client.as_ref().unwrap();
        
        match client.program_subscribe(program_id, commitment, notification_handler).await {
            Ok(subscription) => {
                debug!("Subscribed to program {} on {}", program_id, self.url);
                Ok(subscription)
            },
            Err(err) => {
                error!("Failed to subscribe to program {} on {}: {}", program_id, self.url, err);
                Err(anyhow!("Program subscription failed: {}", err))
            }
        }
    }
    
    /// Subscribe to a signature
    async fn subscribe_signature(
        &mut self,
        signature: &Signature,
        commitment: CommitmentConfig,
        notification_handler: impl Fn(Value) + Send + 'static,
    ) -> Result<PubsubClientSubscription> {
        if !self.is_connected {
            return Err(anyhow!("Not connected to WebSocket server"));
        }
        
        let client = self.client.as_ref().unwrap();
        
        match client.signature_subscribe(signature, commitment, notification_handler).await {
            Ok(subscription) => {
                debug!("Subscribed to signature {} on {}", signature, self.url);
                Ok(subscription)
            },
            Err(err) => {
                error!("Failed to subscribe to signature {} on {}: {}", signature, self.url, err);
                Err(anyhow!("Signature subscription failed: {}", err))
            }
        }
    }
    
    /// Subscribe to slot updates
    async fn subscribe_slot(
        &mut self,
        notification_handler: impl Fn(Value) + Send + 'static,
    ) -> Result<PubsubClientSubscription> {
        if !self.is_connected {
            return Err(anyhow!("Not connected to WebSocket server"));
        }
        
        let client = self.client.as_ref().unwrap();
        
        match client.slot_subscribe(notification_handler).await {
            Ok(subscription) => {
                debug!("Subscribed to slot updates on {}", self.url);
                Ok(subscription)
            },
            Err(err) => {
                error!("Failed to subscribe to slot updates on {}: {}", self.url, err);
                Err(anyhow!("Slot subscription failed: {}", err))
            }
        }
    }
    
    /// Subscribe to root updates
    async fn subscribe_root(
        &mut self,
        notification_handler: impl Fn(Value) + Send + 'static,
    ) -> Result<PubsubClientSubscription> {
        if !self.is_connected {
            return Err(anyhow!("Not connected to WebSocket server"));
        }
        
        let client = self.client.as_ref().unwrap();
        
        match client.root_subscribe(notification_handler).await {
            Ok(subscription) => {
                debug!("Subscribed to root updates on {}", self.url);
                Ok(subscription)
            },
            Err(err) => {
                error!("Failed to subscribe to root updates on {}: {}", self.url, err);
                Err(anyhow!("Root subscription failed: {}", err))
            }
        }
    }
    
    /// Subscribe to vote updates
    async fn subscribe_vote(
        &mut self,
        notification_handler: impl Fn(Value) + Send + 'static,
    ) -> Result<PubsubClientSubscription> {
        if !self.is_connected {
            return Err(anyhow!("Not connected to WebSocket server"));
        }
        
        let client = self.client.as_ref().unwrap();
        
        match client.vote_subscribe(notification_handler).await {
            Ok(subscription) => {
                debug!("Subscribed to vote updates on {}", self.url);
                Ok(subscription)
            },
            Err(err) => {
                error!("Failed to subscribe to vote updates on {}: {}", self.url, err);
                Err(anyhow!("Vote subscription failed: {}", err))
            }
        }
    }
}

/// WebSocket manager
pub struct WebsocketManager {
    /// WebSocket connections
    connections: RwLock<HashMap<String, WebsocketConnection>>,
    
    /// Active subscriptions
    subscriptions: RwLock<HashMap<u64, WebsocketSubscription>>,
    
    /// Next subscription ID
    next_subscription_id: Mutex<u64>,
    
    /// Default commitment level
    commitment: CommitmentConfig,
    
    /// Metrics collector
    metrics: Arc<RpcMetrics>,
    
    /// Command channel
    command_tx: mpsc::Sender<WebsocketCommand>,
    
    /// Command receiver
    command_rx: Mutex<Option<mpsc::Receiver<WebsocketCommand>>>,
    
    /// Shutdown signal sender
    shutdown_tx: Mutex<Option<broadcast::Sender<()>>>,
}

/// WebSocket command
enum WebsocketCommand {
    /// Subscribe to an event
    Subscribe {
        /// Subscription type
        subscription_type: SubscriptionType,
        
        /// Commitment level
        commitment: CommitmentConfig,
        
        /// Response channel
        response_tx: oneshot::Sender<Result<WebsocketSubscription>>,
    },
    
    /// Unsubscribe from an event
    Unsubscribe {
        /// Subscription ID
        subscription_id: u64,
        
        /// Response channel
        response_tx: oneshot::Sender<Result<()>>,
    },
}

impl WebsocketManager {
    /// Create a new WebSocket manager
    pub async fn new(
        endpoints: Vec<String>,
        commitment: CommitmentConfig,
        metrics: Arc<RpcMetrics>,
    ) -> Result<Self> {
        let (command_tx, command_rx) = mpsc::channel(100);
        let (shutdown_tx, _) = broadcast::channel(1);
        
        let manager = Self {
            connections: RwLock::new(HashMap::new()),
            subscriptions: RwLock::new(HashMap::new()),
            next_subscription_id: Mutex::new(1),
            commitment,
            metrics,
            command_tx,
            command_rx: Mutex::new(Some(command_rx)),
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
        };
        
        // Initialize connections
        for url in endpoints {
            manager.connections.write().insert(url.clone(), WebsocketConnection::new(url));
        }
        
        // Start the command processor
        manager.start_command_processor();
        
        // Start the connection monitor
        manager.start_connection_monitor();
        
        Ok(manager)
    }
    
    /// Start the command processor
    fn start_command_processor(&self) {
        let manager = self.clone();
        let mut command_rx = self.command_rx.lock().take().unwrap();
        let mut shutdown_rx = self.shutdown_tx.lock().as_ref().unwrap().subscribe();
        
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(command) = command_rx.recv() => {
                        match command {
                            WebsocketCommand::Subscribe { subscription_type, commitment, response_tx } => {
                                let result = manager.handle_subscribe(subscription_type, commitment).await;
                                let _ = response_tx.send(result);
                            },
                            WebsocketCommand::Unsubscribe { subscription_id, response_tx } => {
                                let result = manager.handle_unsubscribe(subscription_id).await;
                                let _ = response_tx.send(result);
                            },
                        }
                    },
                    _ = shutdown_rx.recv() => {
                        info!("WebSocket command processor shutting down");
                        break;
                    },
                    else => break,
                }
            }
        });
    }
    
    /// Start the connection monitor
    fn start_connection_monitor(&self) {
        let manager = self.clone();
        let mut shutdown_rx = self.shutdown_tx.lock().as_ref().unwrap().subscribe();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = manager.check_connections().await {
                            error!("Failed to check WebSocket connections: {}", e);
                        }
                    },
                    _ = shutdown_rx.recv() => {
                        info!("WebSocket connection monitor shutting down");
                        break;
                    },
                }
            }
        });
    }
    
    /// Check and maintain WebSocket connections
    async fn check_connections(&self) -> Result<()> {
        let mut connections = self.connections.write();
        
        for (url, connection) in connections.iter_mut() {
            // Check if connection is active
            if !connection.is_connected {
                // Try to reconnect if enough time has passed
                if connection.last_connection_attempt.elapsed() > Duration::from_secs(30) {
                    debug!("Attempting to reconnect to WebSocket server: {}", url);
                    if let Err(e) = connection.connect().await {
                        warn!("Failed to reconnect to WebSocket server {}: {}", url, e);
                    } else {
                        // Resubscribe to active subscriptions
                        self.resubscribe(url, connection).await?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Resubscribe to active subscriptions for a connection
    async fn resubscribe(&self, url: &str, connection: &mut WebsocketConnection) -> Result<()> {
        let subscriptions = self.subscriptions.read();
        
        for (id, subscription) in subscriptions.iter() {
            match &subscription.subscription_type {
                SubscriptionType::Account(pubkey) => {
                    let notification_tx = subscription.notification_tx.clone();
                    let sub = connection.subscribe_account(
                        pubkey,
                        subscription.commitment,
                        move |notification| {
                            let _ = notification_tx.send(notification);
                        },
                    ).await;
                    
                    if let Ok(sub) = sub {
                        connection.subscriptions.insert(*id, sub);
                    }
                },
                SubscriptionType::Program(program_id) => {
                    let notification_tx = subscription.notification_tx.clone();
                    let sub = connection.subscribe_program(
                        program_id,
                        subscription.commitment,
                        move |notification| {
                            let _ = notification_tx.send(notification);
                        },
                    ).await;
                    
                    if let Ok(sub) = sub {
                        connection.subscriptions.insert(*id, sub);
                    }
                },
                SubscriptionType::Signature(signature) => {
                    let notification_tx = subscription.notification_tx.clone();
                    let sub = connection.subscribe_signature(
                        signature,
                        subscription.commitment,
                        move |notification| {
                            let _ = notification_tx.send(notification);
                        },
                    ).await;
                    
                    if let Ok(sub) = sub {
                        connection.subscriptions.insert(*id, sub);
                    }
                },
                SubscriptionType::Slot => {
                    let notification_tx = subscription.notification_tx.clone();
                    let sub = connection.subscribe_slot(
                        move |notification| {
                            let _ = notification_tx.send(notification);
                        },
                    ).await;
                    
                    if let Ok(sub) = sub {
                        connection.subscriptions.insert(*id, sub);
                    }
                },
                SubscriptionType::Root => {
                    let notification_tx = subscription.notification_tx.clone();
                    let sub = connection.subscribe_root(
                        move |notification| {
                            let _ = notification_tx.send(notification);
                        },
                    ).await;
                    
                    if let Ok(sub) = sub {
                        connection.subscriptions.insert(*id, sub);
                    }
                },
                SubscriptionType::Vote => {
                    let notification_tx = subscription.notification_tx.clone();
                    let sub = connection.subscribe_vote(
                        move |notification| {
                            let _ = notification_tx.send(notification);
                        },
                    ).await;
                    
                    if let Ok(sub) = sub {
                        connection.subscriptions.insert(*id, sub);
                    }
                },
            }
        }
        
        Ok(())
    }
    
    /// Handle a subscribe command
    async fn handle_subscribe(
        &self,
        subscription_type: SubscriptionType,
        commitment: CommitmentConfig,
    ) -> Result<WebsocketSubscription> {
        // Generate a new subscription ID
        let id = {
            let mut next_id = self.next_subscription_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };
        
        // Create notification channel
        let (notification_tx, _) = broadcast::channel(100);
        
        // Create subscription
        let subscription = WebsocketSubscription {
            id,
            subscription_type: subscription_type.clone(),
            commitment,
            created_at: Instant::now(),
            last_notification: None,
            notification_tx: notification_tx.clone(),
        };
        
        // Store subscription
        self.subscriptions.write().insert(id, subscription.clone());
        
        // Subscribe to all connections
        let mut connections = self.connections.write();
        
        for (url, connection) in connections.iter_mut() {
            if !connection.is_connected {
                // Try to connect
                if let Err(e) = connection.connect().await {
                    warn!("Failed to connect to WebSocket server {}: {}", url, e);
                    continue;
                }
            }
            
            // Subscribe based on type
            let result = match &subscription_type {
                SubscriptionType::Account(pubkey) => {
                    let notification_tx = notification_tx.clone();
                    connection.subscribe_account(
                        pubkey,
                        commitment,
                        move |notification| {
                            let _ = notification_tx.send(notification);
                        },
                    ).await
                },
                SubscriptionType::Program(program_id) => {
                    let notification_tx = notification_tx.clone();
                    connection.subscribe_program(
                        program_id,
                        commitment,
                        move |notification| {
                            let _ = notification_tx.send(notification);
                        },
                    ).await
                },
                SubscriptionType::Signature(signature) => {
                    let notification_tx = notification_tx.clone();
                    connection.subscribe_signature(
                        signature,
                        commitment,
                        move |notification| {
                            let _ = notification_tx.send(notification);
                        },
                    ).await
                },
                SubscriptionType::Slot => {
                    let notification_tx = notification_tx.clone();
                    connection.subscribe_slot(
                        move |notification| {
                            let _ = notification_tx.send(notification);
                        },
                    ).await
                },
                SubscriptionType::Root => {
                    let notification_tx = notification_tx.clone();
                    connection.subscribe_root(
                        move |notification| {
                            let _ = notification_tx.send(notification);
                        },
                    ).await
                },
                SubscriptionType::Vote => {
                    let notification_tx = notification_tx.clone();
                    connection.subscribe_vote(
                        move |notification| {
                            let _ = notification_tx.send(notification);
                        },
                    ).await
                },
            };
            
            if let Ok(sub) = result {
                connection.subscriptions.insert(id, sub);
            }
        }
        
        Ok(subscription)
    }
    
    /// Handle an unsubscribe command
    async fn handle_unsubscribe(&self, subscription_id: u64) -> Result<()> {
        // Remove subscription
        let subscription = self.subscriptions.write().remove(&subscription_id);
        
        if subscription.is_none() {
            return Err(anyhow!("Subscription not found"));
        }
        
        // Unsubscribe from all connections
        let mut connections = self.connections.write();
        
        for (_, connection) in connections.iter_mut() {
            connection.subscriptions.remove(&subscription_id);
        }
        
        Ok(())
    }
    
    /// Subscribe to an account
    pub async fn subscribe_account(
        &self,
        pubkey: &Pubkey,
        commitment: Option<CommitmentConfig>,
    ) -> Result<WebsocketSubscription> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(WebsocketCommand::Subscribe {
            subscription_type: SubscriptionType::Account(*pubkey),
            commitment: commitment.unwrap_or(self.commitment),
            response_tx,
        }).await.map_err(|e| anyhow!("Failed to send subscribe command: {}", e))?;
        
        response_rx.await.map_err(|e| anyhow!("Failed to receive subscribe response: {}", e))?
    }
    
    /// Subscribe to a program
    pub async fn subscribe_program(
        &self,
        program_id: &Pubkey,
        commitment: Option<CommitmentConfig>,
    ) -> Result<WebsocketSubscription> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(WebsocketCommand::Subscribe {
            subscription_type: SubscriptionType::Program(*program_id),
            commitment: commitment.unwrap_or(self.commitment),
            response_tx,
        }).await.map_err(|e| anyhow!("Failed to send subscribe command: {}", e))?;
        
        response_rx.await.map_err(|e| anyhow!("Failed to receive subscribe response: {}", e))?
    }
    
    /// Subscribe to a signature
    pub async fn subscribe_signature(
        &self,
        signature: &Signature,
        commitment: Option<CommitmentConfig>,
    ) -> Result<WebsocketSubscription> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(WebsocketCommand::Subscribe {
            subscription_type: SubscriptionType::Signature(*signature),
            commitment: commitment.unwrap_or(self.commitment),
            response_tx,
        }).await.map_err(|e| anyhow!("Failed to send subscribe command: {}", e))?;
        
        response_rx.await.map_err(|e| anyhow!("Failed to receive subscribe response: {}", e))?
    }
    
    /// Subscribe to slot updates
    pub async fn subscribe_slot(&self) -> Result<WebsocketSubscription> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(WebsocketCommand::Subscribe {
            subscription_type: SubscriptionType::Slot,
            commitment: self.commitment,
            response_tx,
        }).await.map_err(|e| anyhow!("Failed to send subscribe command: {}", e))?;
        
        response_rx.await.map_err(|e| anyhow!("Failed to receive subscribe response: {}", e))?
    }
    
    /// Subscribe to root updates
    pub async fn subscribe_root(&self) -> Result<WebsocketSubscription> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(WebsocketCommand::Subscribe {
            subscription_type: SubscriptionType::Root,
            commitment: self.commitment,
            response_tx,
        }).await.map_err(|e| anyhow!("Failed to send subscribe command: {}", e))?;
        
        response_rx.await.map_err(|e| anyhow!("Failed to receive subscribe response: {}", e))?
    }
    
    /// Subscribe to vote updates
    pub async fn subscribe_vote(&self) -> Result<WebsocketSubscription> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(WebsocketCommand::Subscribe {
            subscription_type: SubscriptionType::Vote,
            commitment: self.commitment,
            response_tx,
        }).await.map_err(|e| anyhow!("Failed to send subscribe command: {}", e))?;
        
        response_rx.await.map_err(|e| anyhow!("Failed to receive subscribe response: {}", e))?
    }
    
    /// Unsubscribe from a subscription
    pub async fn unsubscribe(&self, subscription_id: u64) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(WebsocketCommand::Unsubscribe {
            subscription_id,
            response_tx,
        }).await.map_err(|e| anyhow!("Failed to send unsubscribe command: {}", e))?;
        
        response_rx.await.map_err(|e| anyhow!("Failed to receive unsubscribe response: {}", e))?
    }
    
    /// Shutdown the WebSocket manager
    pub async fn shutdown(&self) {
        info!("Shutting down WebSocket manager");
        
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.lock().take() {
            let _ = tx.send(());
        }
        
        // Close all connections
        let mut connections = self.connections.write();
        
        for (_, connection) in connections.iter_mut() {
            connection.disconnect().await;
        }
    }
}

impl Clone for WebsocketManager {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            subscriptions: self.subscriptions.clone(),
            next_subscription_id: Mutex::new(*self.next_subscription_id.lock()),
            commitment: self.commitment,
            metrics: self.metrics.clone(),
            command_tx: self.command_tx.clone(),
            command_rx: Mutex::new(None),
            shutdown_tx: Mutex::new(None),
        }
    }
}