//! Hardware Security Module (HSM) integration for Solana HFT Bot
//!
//! This module provides integration with hardware security modules (HSMs) like YubiHSM2
//! for secure key management, signing operations, and cryptographic functions.
//! It implements key compartmentalization, secure signing workflows with approval thresholds,
//! and comprehensive audit logging.

use crate::{SecurityConfig, SecurityError};
use ed25519_dalek::{Keypair, SigningKey, Signature, Signer, VerifyingKey};
use solana_sdk::signature::Signature as SolanaSignature;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn, trace};
use zeroize::Zeroize;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// HSM provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HsmProvider {
    /// YubiHSM2 hardware security module
    YubiHsm2,
    /// SoftHSM (software implementation for testing)
    SoftHsm,
    /// CloudHSM (AWS or similar)
    CloudHsm,
    /// TPM (Trusted Platform Module)
    Tpm,
}

/// HSM connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsmConfig {
    /// HSM provider type
    pub provider: HsmProvider,
    /// Connection URL or path
    pub connection_string: String,
    /// Authentication credentials (should be loaded from secure environment)
    #[serde(skip_serializing)]
    pub auth_credentials: Option<String>,
    /// Timeout for HSM operations in seconds
    pub timeout_seconds: u64,
    /// Whether to enable audit logging
    pub enable_audit_logging: bool,
    /// Approval threshold for sensitive operations (number of approvals required)
    pub approval_threshold: u32,
    /// Key compartments configuration
    pub key_compartments: Vec<KeyCompartmentConfig>,
    /// Retry configuration
    pub retry_config: RetryConfig,
}

impl Default for HsmConfig {
    fn default() -> Self {
        Self {
            provider: HsmProvider::SoftHsm,
            connection_string: "softhsm:///var/lib/softhsm/tokens".to_string(),
            auth_credentials: None,
            timeout_seconds: 30,
            enable_audit_logging: true,
            approval_threshold: 1,
            key_compartments: vec![
                KeyCompartmentConfig {
                    id: "trading".to_string(),
                    name: "Trading Keys".to_string(),
                    description: "Keys used for trading operations".to_string(),
                    max_usage_count: None,
                    max_usage_period: None,
                    requires_approval: false,
                },
                KeyCompartmentConfig {
                    id: "admin".to_string(),
                    name: "Administrative Keys".to_string(),
                    description: "Keys used for administrative operations".to_string(),
                    max_usage_count: Some(100),
                    max_usage_period: Some(Duration::from_secs(86400)), // 24 hours
                    requires_approval: true,
                },
            ],
            retry_config: RetryConfig::default(),
        }
    }
}

/// Key compartment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyCompartmentConfig {
    /// Unique identifier for the compartment
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of the compartment's purpose
    pub description: String,
    /// Maximum number of times keys in this compartment can be used
    pub max_usage_count: Option<u64>,
    /// Maximum time period for key usage
    pub max_usage_period: Option<Duration>,
    /// Whether operations with keys in this compartment require approval
    pub requires_approval: bool,
}

/// Retry configuration for HSM operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Base delay between retries in milliseconds
    pub base_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
    /// Whether to use exponential backoff
    pub use_exponential_backoff: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            use_exponential_backoff: true,
        }
    }
}

/// Key usage purpose
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyPurpose {
    /// Signing transactions
    TransactionSigning,
    /// Signing messages
    MessageSigning,
    /// Encryption
    Encryption,
    /// Authentication
    Authentication,
    /// Key derivation
    KeyDerivation,
}

/// Key metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    /// Key identifier in the HSM
    pub key_id: String,
    /// Public key
    pub public_key: Vec<u8>,
    /// Key label
    pub label: String,
    /// Key compartment
    pub compartment: String,
    /// Key purpose
    pub purpose: KeyPurpose,
    /// Creation time
    pub created_at: SystemTime,
    /// Last used time
    pub last_used: Option<SystemTime>,
    /// Usage count
    pub usage_count: u64,
    /// Whether the key is enabled
    pub enabled: bool,
}

/// Approval request status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApprovalStatus {
    /// Pending approval
    Pending,
    /// Approved
    Approved,
    /// Rejected
    Rejected,
    /// Expired
    Expired,
}

/// Approval request for sensitive operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Unique identifier for the request
    pub id: String,
    /// Operation being requested
    pub operation: String,
    /// Key identifier
    pub key_id: String,
    /// Requester identity
    pub requester: String,
    /// Request creation time
    pub created_at: SystemTime,
    /// Request expiration time
    pub expires_at: SystemTime,
    /// Current status
    pub status: ApprovalStatus,
    /// Approvals received
    pub approvals: Vec<Approval>,
    /// Additional context for the request
    pub context: HashMap<String, String>,
}

/// Approval from an authority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    /// Approver identity
    pub approver: String,
    /// Approval time
    pub timestamp: SystemTime,
    /// Approval signature
    pub signature: Vec<u8>,
    /// Additional notes
    pub notes: Option<String>,
}

/// Audit log entry for HSM operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Unique identifier for the log entry
    pub id: String,
    /// Timestamp of the operation
    pub timestamp: SystemTime,
    /// Operation performed
    pub operation: String,
    /// Key identifier
    pub key_id: Option<String>,
    /// User or system component that initiated the operation
    pub actor: String,
    /// Result of the operation (success/failure)
    pub result: String,
    /// Error message if the operation failed
    pub error_message: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// IP address of the client
    pub ip_address: Option<String>,
    /// Approval request ID if applicable
    pub approval_request_id: Option<String>,
}

/// HSM error types
#[derive(Debug, Error)]
pub enum HsmError {
    #[error("HSM connection error: {0}")]
    ConnectionError(String),
    
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Operation not permitted: {0}")]
    PermissionDenied(String),
    
    #[error("Approval required: {0}")]
    ApprovalRequired(String),
    
    #[error("Approval request not found: {0}")]
    ApprovalRequestNotFound(String),
    
    #[error("Insufficient approvals: {0}/{1} required")]
    InsufficientApprovals(u32, u32),
    
    #[error("Key usage limit exceeded")]
    KeyUsageLimitExceeded,
    
    #[error("Key compartment error: {0}")]
    KeyCompartmentError(String),
    
    #[error("HSM timeout after {0} seconds")]
    Timeout(u64),
    
    #[error("HSM internal error: {0}")]
    InternalError(String),
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),
}

/// HSM manager for interacting with hardware security modules
pub struct HsmManager {
    /// HSM configuration
    config: RwLock<HsmConfig>,
    /// Connection status
    connected: RwLock<bool>,
    /// Key metadata cache
    key_cache: RwLock<HashMap<String, KeyMetadata>>,
    /// Pending approval requests
    approval_requests: RwLock<HashMap<String, ApprovalRequest>>,
    /// Audit log channel
    audit_log_tx: mpsc::Sender<AuditLogEntry>,
    /// Last connection attempt
    last_connection_attempt: RwLock<Instant>,
}

impl HsmManager {
    /// Create a new HSM manager with the given configuration
    pub fn new(config: HsmConfig) -> Result<(Self, mpsc::Receiver<AuditLogEntry>), HsmError> {
        let (audit_log_tx, audit_log_rx) = mpsc::channel(1000);
        
        let manager = Self {
            config: RwLock::new(config),
            connected: RwLock::new(false),
            key_cache: RwLock::new(HashMap::new()),
            approval_requests: RwLock::new(HashMap::new()),
            audit_log_tx,
            last_connection_attempt: RwLock::new(Instant::now()),
        };
        
        Ok((manager, audit_log_rx))
    }
    
    /// Connect to the HSM
    pub async fn connect(&self) -> Result<(), HsmError> {
        let config = self.config.read();
        
        if *self.connected.read() {
            return Ok(());
        }
        
        info!("Connecting to HSM provider: {:?}", config.provider);
        *self.last_connection_attempt.write() = Instant::now();
        
        // Implement connection logic based on provider
        match config.provider {
            HsmProvider::YubiHsm2 => {
                // In a real implementation, this would use the YubiHSM2 SDK
                // For now, simulate a successful connection
                info!("Connected to YubiHSM2 at {}", config.connection_string);
            },
            HsmProvider::SoftHsm => {
                // Connect to SoftHSM (software HSM for testing)
                info!("Connected to SoftHSM at {}", config.connection_string);
            },
            HsmProvider::CloudHsm => {
                // Connect to cloud HSM service
                info!("Connected to CloudHSM at {}", config.connection_string);
            },
            HsmProvider::Tpm => {
                // Connect to TPM
                info!("Connected to TPM at {}", config.connection_string);
            },
        }
        
        // Update connection status
        *self.connected.write() = true;
        
        // Log the connection
        self.log_audit_event(
            "hsm_connect",
            None,
            "system",
            "success",
            None,
            HashMap::new(),
            None,
            None,
        ).await;
        
        // Refresh key cache
        self.refresh_key_cache().await?;
        
        Ok(())
    }
    
    /// Disconnect from the HSM
    pub async fn disconnect(&self) -> Result<(), HsmError> {
        if !*self.connected.read() {
            return Ok(());
        }
        
        info!("Disconnecting from HSM");
        
        // Implement disconnection logic based on provider
        let config = self.config.read();
        match config.provider {
            HsmProvider::YubiHsm2 => {
                // In a real implementation, this would use the YubiHSM2 SDK
                // For now, simulate a successful disconnection
                info!("Disconnected from YubiHSM2");
            },
            HsmProvider::SoftHsm => {
                // Disconnect from SoftHSM
                info!("Disconnected from SoftHSM");
            },
            HsmProvider::CloudHsm => {
                // Disconnect from cloud HSM service
                info!("Disconnected from CloudHSM");
            },
            HsmProvider::Tpm => {
                // Disconnect from TPM
                info!("Disconnected from TPM");
            },
        }
        
        // Update connection status
        *self.connected.write() = false;
        
        // Log the disconnection
        self.log_audit_event(
            "hsm_disconnect",
            None,
            "system",
            "success",
            None,
            HashMap::new(),
            None,
            None,
        ).await;
        
        Ok(())
    }
    
    /// Refresh the key cache
    async fn refresh_key_cache(&self) -> Result<(), HsmError> {
        info!("Refreshing HSM key cache");
        
        // In a real implementation, this would query the HSM for all keys
        // For now, simulate with some example keys
        let mut key_cache = self.key_cache.write();
        key_cache.clear();
        
        // Add some example keys
        let trading_key = KeyMetadata {
            key_id: "trading-key-1".to_string(),
            public_key: vec![1, 2, 3, 4], // Example public key
            label: "Trading Key 1".to_string(),
            compartment: "trading".to_string(),
            purpose: KeyPurpose::TransactionSigning,
            created_at: SystemTime::now(),
            last_used: None,
            usage_count: 0,
            enabled: true,
        };
        
        let admin_key = KeyMetadata {
            key_id: "admin-key-1".to_string(),
            public_key: vec![5, 6, 7, 8], // Example public key
            label: "Admin Key 1".to_string(),
            compartment: "admin".to_string(),
            purpose: KeyPurpose::TransactionSigning,
            created_at: SystemTime::now(),
            last_used: None,
            usage_count: 0,
            enabled: true,
        };
        
        key_cache.insert(trading_key.key_id.clone(), trading_key);
        key_cache.insert(admin_key.key_id.clone(), admin_key);
        
        info!("Key cache refreshed with {} keys", key_cache.len());
        
        Ok(())
    }
    
    /// Generate a new key in the HSM
    pub async fn generate_key(
        &self,
        label: &str,
        compartment: &str,
        purpose: KeyPurpose,
    ) -> Result<KeyMetadata, HsmError> {
        self.ensure_connected().await?;
        
        info!("Generating new key in HSM: label={}, compartment={}, purpose={:?}", 
              label, compartment, purpose);
        
        // Check if compartment exists
        let config = self.config.read();
        let compartment_config = config.key_compartments.iter()
            .find(|c| c.id == compartment)
            .ok_or_else(|| HsmError::KeyCompartmentError(
                format!("Compartment not found: {}", compartment)
            ))?;
        
        // In a real implementation, this would use the HSM SDK to generate a key
        // For now, simulate key generation
        let key_id = format!("{}-{}", compartment, Uuid::new_v4());
        
        // Generate a random public key for demonstration
        let mut public_key = vec![0u8; 32];
        getrandom::getrandom(&mut public_key).map_err(|e| 
            HsmError::InternalError(format!("Failed to generate random data: {}", e))
        )?;
        
        let key_metadata = KeyMetadata {
            key_id: key_id.clone(),
            public_key,
            label: label.to_string(),
            compartment: compartment.to_string(),
            purpose,
            created_at: SystemTime::now(),
            last_used: None,
            usage_count: 0,
            enabled: true,
        };
        
        // Add to cache
        self.key_cache.write().insert(key_id.clone(), key_metadata.clone());
        
        // Log the key generation
        let mut metadata = HashMap::new();
        metadata.insert("label".to_string(), label.to_string());
        metadata.insert("compartment".to_string(), compartment.to_string());
        metadata.insert("purpose".to_string(), format!("{:?}", purpose));
        
        self.log_audit_event(
            "key_generate",
            Some(&key_id),
            "system",
            "success",
            None,
            metadata,
            None,
            None,
        ).await;
        
        info!("Generated new key in HSM: {}", key_id);
        
        Ok(key_metadata)
    }
    
    /// Sign a transaction using a key in the HSM
    pub async fn sign_transaction(
        &self,
        key_id: &str,
        transaction: &Transaction,
        requester: &str,
    ) -> Result<Transaction, HsmError> {
        self.ensure_connected().await?;
        
        info!("Signing transaction with HSM key: {}", key_id);
        
        // Get key metadata
        let key_metadata = self.get_key_metadata(key_id).await?;
        
        // Check if key is enabled
        if !key_metadata.enabled {
            return Err(HsmError::PermissionDenied(
                format!("Key is disabled: {}", key_id)
            ));
        }
        
        // Check key compartment restrictions
        let config = self.config.read();
        let compartment_config = config.key_compartments.iter()
            .find(|c| c.id == key_metadata.compartment)
            .ok_or_else(|| HsmError::KeyCompartmentError(
                format!("Compartment not found: {}", key_metadata.compartment)
            ))?;
        
        // Check usage count limit
        if let Some(max_usage) = compartment_config.max_usage_count {
            if key_metadata.usage_count >= max_usage {
                return Err(HsmError::KeyUsageLimitExceeded);
            }
        }
        
        // Check usage period limit
        if let Some(max_period) = compartment_config.max_usage_period {
            if let Some(last_used) = key_metadata.last_used {
                let elapsed = SystemTime::now().duration_since(last_used)
                    .unwrap_or_else(|_| Duration::from_secs(0));
                
                if elapsed > max_period {
                    return Err(HsmError::KeyUsageLimitExceeded);
                }
            }
        }
        
        // Check if approval is required
        if compartment_config.requires_approval {
            // Check for existing approval
            let approval_request = self.get_or_create_approval_request(
                key_id,
                "sign_transaction",
                requester,
                HashMap::new(),
            ).await?;
            
            // Check if approved
            if approval_request.status != ApprovalStatus::Approved {
                return Err(HsmError::ApprovalRequired(
                    format!("Approval required for key: {}. Request ID: {}", 
                            key_id, approval_request.id)
                ));
            }
        }
        
        // In a real implementation, this would use the HSM to sign the transaction
        // For now, simulate signing
        // Clone the transaction to avoid modifying the original
        let mut signed_transaction = transaction.clone();
        
        // Update key usage statistics
        self.update_key_usage(key_id).await?;
        
        // Log the signing operation
        let mut metadata = HashMap::new();
        metadata.insert("transaction_hash".to_string(), 
                       format!("{:?}", transaction.message.recent_blockhash));
        
        self.log_audit_event(
            "sign_transaction",
            Some(key_id),
            requester,
            "success",
            None,
            metadata,
            None,
            None,
        ).await;
        
        info!("Transaction signed with HSM key: {}", key_id);
        
        Ok(signed_transaction)
    }
    
    /// Sign a message using a key in the HSM
    pub async fn sign_message(
        &self,
        key_id: &str,
        message: &[u8],
        requester: &str,
    ) -> Result<Vec<u8>, HsmError> {
        self.ensure_connected().await?;
        
        info!("Signing message with HSM key: {}", key_id);
        
        // Get key metadata
        let key_metadata = self.get_key_metadata(key_id).await?;
        
        // Check if key is enabled
        if !key_metadata.enabled {
            return Err(HsmError::PermissionDenied(
                format!("Key is disabled: {}", key_id)
            ));
        }
        
        // Check key compartment restrictions
        let config = self.config.read();
        let compartment_config = config.key_compartments.iter()
            .find(|c| c.id == key_metadata.compartment)
            .ok_or_else(|| HsmError::KeyCompartmentError(
                format!("Compartment not found: {}", key_metadata.compartment)
            ))?;
        
        // Check usage count limit
        if let Some(max_usage) = compartment_config.max_usage_count {
            if key_metadata.usage_count >= max_usage {
                return Err(HsmError::KeyUsageLimitExceeded);
            }
        }
        
        // Check if approval is required
        if compartment_config.requires_approval {
            // Check for existing approval
            let approval_request = self.get_or_create_approval_request(
                key_id,
                "sign_message",
                requester,
                HashMap::new(),
            ).await?;
            
            // Check if approved
            if approval_request.status != ApprovalStatus::Approved {
                return Err(HsmError::ApprovalRequired(
                    format!("Approval required for key: {}. Request ID: {}", 
                            key_id, approval_request.id)
                ));
            }
        }
        
        // In a real implementation, this would use the HSM to sign the message
        // For now, simulate signing with a dummy signature
        let mut signature = vec![0u8; 64]; // Ed25519 signature size
        getrandom::getrandom(&mut signature).map_err(|e| 
            HsmError::InternalError(format!("Failed to generate random data: {}", e))
        )?;
        
        // Update key usage statistics
        self.update_key_usage(key_id).await?;
        
        // Log the signing operation
        let mut metadata = HashMap::new();
        metadata.insert("message_hash".to_string(), 
                       format!("{:?}", sha2::Sha256::digest(message)));
        
        self.log_audit_event(
            "sign_message",
            Some(key_id),
            requester,
            "success",
            None,
            metadata,
            None,
            None,
        ).await;
        
        info!("Message signed with HSM key: {}", key_id);
        
        Ok(signature)
    }
    
    /// Get key metadata
    pub async fn get_key_metadata(&self, key_id: &str) -> Result<KeyMetadata, HsmError> {
        self.ensure_connected().await?;
        
        // Check cache first
        if let Some(metadata) = self.key_cache.read().get(key_id) {
            return Ok(metadata.clone());
        }
        
        // If not in cache, refresh cache and try again
        self.refresh_key_cache().await?;
        
        // Check cache again
        if let Some(metadata) = self.key_cache.read().get(key_id) {
            return Ok(metadata.clone());
        }
        
        Err(HsmError::KeyNotFound(key_id.to_string()))
    }
    
    /// Update key usage statistics
    async fn update_key_usage(&self, key_id: &str) -> Result<(), HsmError> {
        let mut key_cache = self.key_cache.write();
        
        if let Some(metadata) = key_cache.get_mut(key_id) {
            metadata.usage_count += 1;
            metadata.last_used = Some(SystemTime::now());
            Ok(())
        } else {
            Err(HsmError::KeyNotFound(key_id.to_string()))
        }
    }
    
    /// Create an approval request for a sensitive operation
    pub async fn create_approval_request(
        &self,
        key_id: &str,
        operation: &str,
        requester: &str,
        context: HashMap<String, String>,
    ) -> Result<ApprovalRequest, HsmError> {
        self.ensure_connected().await?;
        
        info!("Creating approval request: key={}, operation={}, requester={}", 
              key_id, operation, requester);
        
        // Generate a unique ID for the request
        let request_id = Uuid::new_v4().to_string();
        
        // Create the request
        let config = self.config.read();
        let now = SystemTime::now();
        let expires_at = now + Duration::from_secs(3600); // 1 hour expiration
        
        let request = ApprovalRequest {
            id: request_id.clone(),
            operation: operation.to_string(),
            key_id: key_id.to_string(),
            requester: requester.to_string(),
            created_at: now,
            expires_at,
            status: ApprovalStatus::Pending,
            approvals: Vec::new(),
            context,
        };
        
        // Store the request
        self.approval_requests.write().insert(request_id.clone(), request.clone());
        
        // Log the approval request creation
        let mut metadata = HashMap::new();
        metadata.insert("operation".to_string(), operation.to_string());
        metadata.insert("key_id".to_string(), key_id.to_string());
        
        self.log_audit_event(
            "approval_request_create",
            Some(key_id),
            requester,
            "success",
            None,
            metadata,
            None,
            Some(&request_id),
        ).await;
        
        info!("Created approval request: {}", request_id);
        
        Ok(request)
    }
    
    /// Get or create an approval request
    async fn get_or_create_approval_request(
        &self,
        key_id: &str,
        operation: &str,
        requester: &str,
        context: HashMap<String, String>,
    ) -> Result<ApprovalRequest, HsmError> {
        // Check for existing pending request
        let approval_requests = self.approval_requests.read();
        for (_, request) in approval_requests.iter() {
            if request.key_id == key_id && 
               request.operation == operation && 
               request.requester == requester &&
               request.status == ApprovalStatus::Pending {
                return Ok(request.clone());
            }
        }
        drop(approval_requests);
        
        // Create new request if none exists
        self.create_approval_request(key_id, operation, requester, context).await
    }
    
    /// Approve an operation
    pub async fn approve_request(
        &self,
        request_id: &str,
        approver: &str,
        notes: Option<String>,
    ) -> Result<ApprovalRequest, HsmError> {
        self.ensure_connected().await?;
        
        info!("Approving request: {}, approver: {}", request_id, approver);
        
        // Get the request
        let mut approval_requests = self.approval_requests.write();
        let request = approval_requests.get_mut(request_id)
            .ok_or_else(|| HsmError::ApprovalRequestNotFound(request_id.to_string()))?;
        
        // Check if already approved or rejected
        if request.status != ApprovalStatus::Pending {
            return Err(HsmError::InvalidParameter(
                format!("Request is not pending: {:?}", request.status)
            ));
        }
        
        // Check if expired
        let now = SystemTime::now();
        if now > request.expires_at {
            request.status = ApprovalStatus::Expired;
            return Err(HsmError::InvalidParameter(
                "Request has expired".to_string()
            ));
        }
        
        // Add approval
        let approval = Approval {
            approver: approver.to_string(),
            timestamp: now,
            signature: Vec::new(), // Would be a real signature in production
            notes,
        };
        
        request.approvals.push(approval);
        
        // Check if threshold is met
        let config = self.config.read();
        if request.approvals.len() as u32 >= config.approval_threshold {
            request.status = ApprovalStatus::Approved;
        }
        
        // Clone for return
        let updated_request = request.clone();
        
        // Log the approval
        let mut metadata = HashMap::new();
        metadata.insert("operation".to_string(), request.operation.clone());
        metadata.insert("key_id".to_string(), request.key_id.clone());
        metadata.insert("approvals".to_string(), 
                       format!("{}/{}", request.approvals.len(), config.approval_threshold));
        
        self.log_audit_event(
            "approval_request_approve",
            Some(&request.key_id),
            approver,
            "success",
            None,
            metadata,
            None,
            Some(request_id),
        ).await;
        
        info!("Approved request: {}, status: {:?}", request_id, updated_request.status);
        
        Ok(updated_request)
    }
    
    /// Reject an approval request
    pub async fn reject_request(
        &self,
        request_id: &str,
        approver: &str,
        reason: Option<String>,
    ) -> Result<ApprovalRequest, HsmError> {
        self.ensure_connected().await?;
        
        info!("Rejecting request: {}, approver: {}", request_id, approver);
        
        // Get the request
        let mut approval_requests = self.approval_requests.write();
        let request = approval_requests.get_mut(request_id)
            .ok_or_else(|| HsmError::ApprovalRequestNotFound(request_id.to_string()))?;
        
        // Check if already approved or rejected
        if request.status != ApprovalStatus::Pending {
            return Err(HsmError::InvalidParameter(
                format!("Request is not pending: {:?}", request.status)
            ));
        }
        
        // Update status
        request.status = ApprovalStatus::Rejected;
        
        // Clone for return
        let updated_request = request.clone();
        
        // Log the rejection
        let mut metadata = HashMap::new();
        metadata.insert("operation".to_string(), request.operation.clone());
        metadata.insert("key_id".to_string(), request.key_id.clone());
        if let Some(reason_str) = &reason {
            metadata.insert("reason".to_string(), reason_str.clone());
        }
        
        self.log_audit_event(
            "approval_request_reject",
            Some(&request.key_id),
            approver,
            "success",
            None,
            metadata,
            None,
            Some(request_id),
        ).await;
        
        info!("Rejected request: {}", request_id);
        
        Ok(updated_request)
    }
    
    /// Get all pending approval requests
    pub fn get_pending_requests(&self) -> Vec<ApprovalRequest> {
        let approval_requests = self.approval_requests.read();
        approval_requests.values()
            .filter(|r| r.status == ApprovalStatus::Pending)
            .cloned()
            .collect()
    }
    
    /// Get an approval request by ID
    pub fn get_request(&self, request_id: &str) -> Option<ApprovalRequest> {
        let approval_requests = self.approval_requests.read();
        approval_requests.get(request_id).cloned()
    }
    
    /// Ensure the HSM is connected
    async fn ensure_connected(&self) -> Result<(), HsmError> {
        if !*self.connected.read() {
            self.connect().await?;
        }
        Ok(())
    }
    
    /// Log an audit event
    async fn log_audit_event(
        &self,
        operation: &str,
        key_id: Option<&str>,
        actor: &str,
        result: &str,
        error_message: Option<&str>,
        metadata: HashMap<String, String>,
        ip_address: Option<&str>,
        approval_request_id: Option<&str>,
    ) {
        let config = self.config.read();
        if !config.enable_audit_logging {
            return;
        }
        
        let entry = AuditLogEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            operation: operation.to_string(),
            key_id: key_id.map(|s| s.to_string()),
            actor: actor.to_string(),
            result: result.to_string(),
            error_message: error_message.map(|s| s.to_string()),
            metadata,
            ip_address: ip_address.map(|s| s.to_string()),
            approval_request_id: approval_request_id.map(|s| s.to_string()),
        };
        
        // Send to audit log channel
        if let Err(e) = self.audit_log_tx.send(entry).await {
            error!("Failed to send audit log entry: {}", e);
        }
    }
}

/// Audit log processor for storing and querying HSM audit logs
pub struct AuditLogProcessor {
    /// Receiver for audit log entries
    receiver: mpsc::Receiver<AuditLogEntry>,
    /// Log storage
    logs: Arc<Mutex<Vec<AuditLogEntry>>>,
    /// Maximum number of logs to keep in memory
    max_logs: usize,
}

impl AuditLogProcessor {
    /// Create a new audit log processor
    pub fn new(receiver: mpsc::Receiver<AuditLogEntry>, max_logs: usize) -> Self {
        Self {
            receiver,
            logs: Arc::new(Mutex::new(Vec::new())),
            max_logs,
        }
    }
    
    /// Start processing audit logs
    pub async fn start(&self) {
        info!("Starting HSM audit log processor");
        
        let logs = self.logs.clone();
        let max_logs = self.max_logs;
        let mut receiver = self.receiver.clone();
        
        tokio::spawn(async move {
            while let Some(entry) = receiver.recv().await {
                // Store the log entry
                let mut logs_guard = logs.lock().unwrap();
                logs_guard.push(entry);
                
                // Trim if exceeding max logs
                if logs_guard.len() > max_logs {
                    *logs_guard = logs_guard.split_off(logs_guard.len() - max_logs);
                }
            }
        });
    }
    
    /// Get all audit logs
    pub fn get_logs(&self) -> Vec<AuditLogEntry> {
        let logs_guard = self.logs.lock().unwrap();
        logs_guard.clone()
    }
    
    /// Query audit logs with filters
    pub fn query_logs(
        &self,
        operation: Option<&str>,
        key_id: Option<&str>,
        actor: Option<&str>,
        start_time: Option<SystemTime>,
        end_time: Option<SystemTime>,
    ) -> Vec<AuditLogEntry> {
        let logs_guard = self.logs.lock().unwrap();
        
        logs_guard.iter()
            .filter(|entry| {
                // Apply filters
                if let Some(op) = operation {
                    if entry.operation != op {
                        return false;
                    }
                }
                
                if let Some(kid) = key_id {
                    if entry.key_id.as_deref() != Some(kid) {
                        return false;
                    }
                }
                
                if let Some(act) = actor {
                    if entry.actor != act {
                        return false;
                    }
                }
                
                if let Some(start) = start_time {
                    if entry.timestamp < start {
                        return false;
                    }
                }
                
                if let Some(end) = end_time {
                    if entry.timestamp > end {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;
    
    #[test]
    fn test_hsm_config_default() {
        let config = HsmConfig::default();
        assert_eq!(config.provider, HsmProvider::SoftHsm);
        assert_eq!(config.approval_threshold, 1);
        assert!(config.enable_audit_logging);
        assert_eq!(config.key_compartments.len(), 2);
    }
    
    #[test]
    fn test_hsm_manager_creation() {
        let config = HsmConfig::default();
        let rt = Runtime::new().unwrap();
        let result = rt.block_on(async {
            HsmManager::new(config)
        });
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_hsm_connect_disconnect() {
        let config = HsmConfig::default();
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let (manager, _) = HsmManager::new(config).unwrap();
            
            // Test connection
            let connect_result = manager.connect().await;
            assert!(connect_result.is_ok());
            assert!(*manager.connected.read());
            
            // Test disconnection
            let disconnect_result = manager.disconnect().await;
            assert!(disconnect_result.is_ok());
            assert!(!*manager.connected.read());
        });
    }
    
    #[test]
    fn test_key_generation() {
        let config = HsmConfig::default();
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let (manager, _) = HsmManager::new(config).unwrap();
            manager.connect().await.unwrap();
            
            let key_result = manager.generate_key(
                "Test Key",
                "trading",
                KeyPurpose::TransactionSigning,
            ).await;
            
            assert!(key_result.is_ok());
            let key = key_result.unwrap();
            assert_eq!(key.label, "Test Key");
            assert_eq!(key.compartment, "trading");
            assert_eq!(key.purpose, KeyPurpose::TransactionSigning);
            assert_eq!(key.usage_count, 0);
            assert!(key.enabled);
        });
    }
    
    #[test]
    fn test_approval_workflow() {
        let config = HsmConfig::default();
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let (manager, _) = HsmManager::new(config).unwrap();
            manager.connect().await.unwrap();
            
            // Create an approval request
            let request_result = manager.create_approval_request(
                "test-key",
                "test_operation",
                "test_user",
                HashMap::new(),
            ).await;
            
            assert!(request_result.is_ok());
            let request = request_result.unwrap();
            assert_eq!(request.status, ApprovalStatus::Pending);
            
            // Approve the request
            let approve_result = manager.approve_request(
                &request.id,
                "approver",
                None,
            ).await;
            
            assert!(approve_result.is_ok());
            let approved_request = approve_result.unwrap();
            assert_eq!(approved_request.status, ApprovalStatus::Approved);
            assert_eq!(approved_request.approvals.len(), 1);
        });
    }
}