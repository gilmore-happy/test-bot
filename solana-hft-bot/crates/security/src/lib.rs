//! Security module for the Solana HFT Bot
//!
//! This module provides comprehensive security functionality for the Solana HFT Bot,
//! including hardware security module (HSM) integration, key management, encryption,
//! secure communication channels, and certificate pinning.

use solana_hft_core::CoreError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use ed25519_dalek::{Keypair, Signer, Verifier, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};
use zeroize::Zeroize;

// Export submodules
pub mod hsm;
pub mod secure_comms;

// Re-export important types
pub use hsm::{
    HsmManager, HsmConfig, HsmProvider, KeyPurpose, KeyMetadata,
    ApprovalRequest, ApprovalStatus, AuditLogEntry, AuditLogProcessor,
    HsmError, KeyCompartmentConfig,
};

pub use secure_comms::{
    SecureCommsManager, SecureCommsConfig, TlsProtocolVersion,
    CertificateInfo, IpAllowlistEntry, EncryptedStorageManager,
};

/// Configuration for the security system
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Whether to enable encryption for sensitive data
    pub enable_encryption: bool,
    /// Whether to enable key rotation
    pub enable_key_rotation: bool,
    /// Key rotation interval in days
    pub key_rotation_days: u32,
    /// Whether to enable secure RPC communication
    pub secure_rpc: bool,
    /// HSM configuration
    pub hsm_config: Option<hsm::HsmConfig>,
    /// Secure communications configuration
    pub secure_comms_config: Option<secure_comms::SecureCommsConfig>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_encryption: true,
            enable_key_rotation: true,
            key_rotation_days: 30,
            secure_rpc: true,
            hsm_config: Some(hsm::HsmConfig::default()),
            secure_comms_config: Some(secure_comms::SecureCommsConfig::default()),
        }
    }
}

/// Security manager for handling cryptographic operations
pub struct SecurityManager {
    /// Configuration
    pub config: SecurityConfig,
    /// Whether the system is running
    pub running: bool,
    /// HSM manager
    pub hsm_manager: Option<Arc<hsm::HsmManager>>,
    /// Secure communications manager
    pub secure_comms_manager: Option<Arc<secure_comms::SecureCommsManager>>,
    /// Audit log processor
    pub audit_log_processor: Option<Arc<hsm::AuditLogProcessor>>,
}

impl SecurityManager {
    /// Create a new security manager with the given configuration
    pub fn new(config: SecurityConfig) -> Result<Self, SecurityError> {
        let mut manager = Self {
            config: config.clone(),
            running: false,
            hsm_manager: None,
            secure_comms_manager: None,
            audit_log_processor: None,
        };
        
        // Initialize HSM if configured
        if let Some(hsm_config) = &config.hsm_config {
            let (hsm_manager, audit_log_rx) = hsm::HsmManager::new(hsm_config.clone())
                .map_err(|e| SecurityError::HsmError(e.to_string()))?;
            
            let audit_log_processor = hsm::AuditLogProcessor::new(audit_log_rx, 10000);
            
            manager.hsm_manager = Some(Arc::new(hsm_manager));
            manager.audit_log_processor = Some(Arc::new(audit_log_processor));
        }
        
        // Initialize secure communications if configured
        if let Some(comms_config) = &config.secure_comms_config {
            let secure_comms = secure_comms::SecureCommsManager::new(comms_config.clone())
                .map_err(|e| SecurityError::SecureCommsError(e.to_string()))?;
            
            manager.secure_comms_manager = Some(Arc::new(secure_comms));
        }
        
        Ok(manager)
    }

    /// Start the security system
    pub async fn start(&mut self) -> Result<(), SecurityError> {
        if self.running {
            return Err(SecurityError::AlreadyRunning);
        }
        
        info!("Starting security system with config: {:?}", self.config);
        
        // Start HSM if available
        if let Some(hsm) = &self.hsm_manager {
            hsm.connect().await
                .map_err(|e| SecurityError::HsmError(e.to_string()))?;
        }
        
        // Start audit log processor if available
        if let Some(audit_processor) = &self.audit_log_processor {
            audit_processor.start().await;
        }
        
        self.running = true;
        info!("Security system started");
        Ok(())
    }
    
    /// Stop the security system
    pub async fn stop(&mut self) -> Result<(), SecurityError> {
        if !self.running {
            return Err(SecurityError::NotRunning);
        }
        
        info!("Stopping security system");
        
        // Stop HSM if available
        if let Some(hsm) = &self.hsm_manager {
            hsm.disconnect().await
                .map_err(|e| SecurityError::HsmError(e.to_string()))?;
        }
        
        self.running = false;
        info!("Security system stopped");
        Ok(())
    }
    
    /// Generate a new keypair
    pub fn generate_keypair() -> Result<Keypair, SecurityError> {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let keypair = Keypair::from(signing_key);
        Ok(keypair)
    }
    
    /// Sign a message with a keypair
    pub fn sign_message(keypair: &Keypair, message: &[u8]) -> Result<Vec<u8>, SecurityError> {
        let signature = keypair.sign(message);
        Ok(signature.to_bytes().to_vec())
    }
    
    /// Verify a signature
    pub fn verify_signature(
        public_key: &VerifyingKey,
        message: &[u8],
        signature: &[u8],
    ) -> Result<bool, SecurityError> {
        let sig = ed25519_dalek::Signature::from_bytes(signature)
            .map_err(|e| SecurityError::SignatureError(e.to_string()))?;
        
        match public_key.verify(message, &sig) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Hash data using SHA-256
    pub fn hash_data(data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }
    
    /// Encrypt data (placeholder - would use a proper encryption library in production)
    pub fn encrypt_data(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>, SecurityError> {
        if !self.config.enable_encryption {
            return Ok(data.to_vec());
        }
        
        if !self.running {
            return Err(SecurityError::NotRunning);
        }
        
        // This is a placeholder - in a real implementation, use a proper encryption library
        // such as ring, openssl, or sodiumoxide
        
        // XOR with key (NOT secure, just for demonstration)
        let mut result = Vec::with_capacity(data.len());
        for (i, byte) in data.iter().enumerate() {
            result.push(byte ^ key[i % key.len()]);
        }
        
        Ok(result)
    }
    
    /// Decrypt data (placeholder - would use a proper encryption library in production)
    pub fn decrypt_data(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>, SecurityError> {
        if !self.config.enable_encryption {
            return Ok(data.to_vec());
        }
        
        if !self.running {
            return Err(SecurityError::NotRunning);
        }
        
        // This is a placeholder - in a real implementation, use a proper encryption library
        // For XOR, encryption and decryption are the same operation
        self.encrypt_data(data, key)
    }
    
    /// Sign a transaction using HSM
    pub async fn sign_transaction_with_hsm(
        &self,
        key_id: &str,
        transaction: &solana_sdk::transaction::Transaction,
        requester: &str,
    ) -> Result<solana_sdk::transaction::Transaction, SecurityError> {
        let hsm = self.hsm_manager.as_ref()
            .ok_or_else(|| SecurityError::HsmNotConfigured)?;
        
        hsm.sign_transaction(key_id, transaction, requester)
            .await
            .map_err(|e| SecurityError::HsmError(e.to_string()))
    }
    
    /// Store data in encrypted storage
    pub fn store_encrypted(&self, key: &str, data: &[u8]) -> Result<(), SecurityError> {
        let secure_comms = self.secure_comms_manager.as_ref()
            .ok_or_else(|| SecurityError::SecureCommsNotConfigured)?;
        
        secure_comms.store_encrypted(key, data)
            .map_err(|e| SecurityError::SecureCommsError(e.to_string()))
    }
    
    /// Retrieve data from encrypted storage
    pub fn retrieve_encrypted(&self, key: &str) -> Result<Vec<u8>, SecurityError> {
        let secure_comms = self.secure_comms_manager.as_ref()
            .ok_or_else(|| SecurityError::SecureCommsNotConfigured)?;
        
        secure_comms.retrieve_encrypted(key)
            .map_err(|e| SecurityError::SecureCommsError(e.to_string()))
    }
    
    /// Connect to a server using TLS
    pub async fn connect_tls(&self, addr: &str) -> Result<tokio_rustls::client::TlsStream<tokio::net::TcpStream>, SecurityError> {
        let secure_comms = self.secure_comms_manager.as_ref()
            .ok_or_else(|| SecurityError::SecureCommsNotConfigured)?;
        
        secure_comms.connect_tls(addr)
            .await
            .map_err(|e| SecurityError::SecureCommsError(e.to_string()))
    }
    
    /// Generate a firewall configuration
    pub fn generate_firewall_config(&self, output_path: &str) -> Result<(), SecurityError> {
        let secure_comms = self.secure_comms_manager.as_ref()
            .ok_or_else(|| SecurityError::SecureCommsNotConfigured)?;
        
        secure_comms.generate_firewall_config(output_path)
            .map_err(|e| SecurityError::SecureCommsError(e.to_string()))
    }
}

/// Secure key storage (placeholder)
pub struct SecureKeyStorage {
    /// In-memory storage (would use secure storage in production)
    keys: parking_lot::RwLock<Vec<u8>>,
}

impl SecureKeyStorage {
    /// Create a new secure key storage
    pub fn new() -> Self {
        Self {
            keys: parking_lot::RwLock::new(Vec::new()),
        }
    }
    
    /// Store a key
    pub fn store_key(&self, key: &[u8]) {
        let mut keys = self.keys.write();
        keys.clear();
        keys.extend_from_slice(key);
    }
    
    /// Retrieve a key
    pub fn get_key(&self) -> Vec<u8> {
        let keys = self.keys.read();
        keys.clone()
    }
    
    /// Clear all keys
    pub fn clear(&self) {
        let mut keys = self.keys.write();
        keys.zeroize();
        keys.clear();
    }
}

impl Drop for SecureKeyStorage {
    fn drop(&mut self) {
        let mut keys = self.keys.write();
        keys.zeroize();
    }
}

/// Security error types
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Security system already running")]
    AlreadyRunning,
    
    #[error("Security system not running")]
    NotRunning,
    
    #[error("Key generation error: {0}")]
    KeyGenerationError(String),
    
    #[error("Signature error: {0}")]
    SignatureError(String),
    
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    
    #[error("Decryption error: {0}")]
    DecryptionError(String),
    
    #[error("HSM error: {0}")]
    HsmError(String),
    
    #[error("HSM not configured")]
    HsmNotConfigured,
    
    #[error("Secure communications error: {0}")]
    SecureCommsError(String),
    
    #[error("Secure communications not configured")]
    SecureCommsNotConfigured,
    
    #[error("Core error: {0}")]
    Core(#[from] CoreError),
}

/// Module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;
    
    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.enable_encryption);
        assert!(config.enable_key_rotation);
        assert_eq!(config.key_rotation_days, 30);
        assert!(config.secure_rpc);
        assert!(config.hsm_config.is_some());
        assert!(config.secure_comms_config.is_some());
    }
    
    #[test]
    fn test_security_manager_creation() {
        let config = SecurityConfig::default();
        let result = SecurityManager::new(config);
        assert!(result.is_ok());
        
        let manager = result.unwrap();
        assert!(!manager.running);
        assert!(manager.hsm_manager.is_some());
        assert!(manager.secure_comms_manager.is_some());
        assert!(manager.audit_log_processor.is_some());
    }
    
    #[test]
    fn test_security_manager_start_stop() {
        let config = SecurityConfig {
            hsm_config: None,
            secure_comms_config: None,
            ..SecurityConfig::default()
        };
        
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let mut manager = SecurityManager::new(config).unwrap();
            
            assert!(!manager.running);
            
            let result = manager.start().await;
            assert!(result.is_ok());
            assert!(manager.running);
            
            // Trying to start again should fail
            let result = manager.start().await;
            assert!(result.is_err());
            
            let result = manager.stop().await;
            assert!(result.is_ok());
            assert!(!manager.running);
            
            // Trying to stop again should fail
            let result = manager.stop().await;
            assert!(result.is_err());
        });
    }
    
    #[test]
    fn test_keypair_generation_and_signing() {
        let keypair = SecurityManager::generate_keypair().unwrap();
        let message = b"test message";
        
        let signature = SecurityManager::sign_message(&keypair, message).unwrap();
        let result = SecurityManager::verify_signature(
            keypair.verifying_key(),
            message,
            &signature
        ).unwrap();
        
        assert!(result);
        
        // Test with wrong message
        let wrong_message = b"wrong message";
        let result = SecurityManager::verify_signature(
            keypair.verifying_key(),
            wrong_message,
            &signature
        ).unwrap();
        
        assert!(!result);
    }
    
    #[test]
    fn test_secure_key_storage() {
        let storage = SecureKeyStorage::new();
        let key = b"test key".to_vec();
        
        storage.store_key(&key);
        let retrieved = storage.get_key();
        
        assert_eq!(retrieved, key);
        
        storage.clear();
        let empty = storage.get_key();
        
        assert!(empty.is_empty());
    }
}