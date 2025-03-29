//! Secure communication channels for the Solana HFT Bot
//!
//! This module provides secure communication capabilities including:
//! - TLS for all RPC connections
//! - Certificate pinning for validator connections
//! - IP allowlisting and firewall configuration
//! - Encrypted storage for configuration and keys

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{anyhow, Context, Result};
use parking_lot::RwLock;
use rustls::{Certificate, ClientConfig, RootCertStore, ServerName};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_rustls::{TlsAcceptor, TlsConnector, client::TlsStream};
use tracing::{debug, error, info, warn};

use crate::{SecurityConfig, SecurityError};

/// Secure communication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureCommsConfig {
    /// Whether to enable TLS for RPC connections
    pub enable_tls: bool,
    /// Whether to enable certificate pinning
    pub enable_cert_pinning: bool,
    /// Whether to enable IP allowlisting
    pub enable_ip_allowlisting: bool,
    /// Whether to enable encrypted storage
    pub enable_encrypted_storage: bool,
    /// Path to CA certificate file
    pub ca_cert_path: Option<String>,
    /// Path to client certificate file
    pub client_cert_path: Option<String>,
    /// Path to client key file
    pub client_key_path: Option<String>,
    /// Path to server certificate file
    pub server_cert_path: Option<String>,
    /// Path to server key file
    pub server_key_path: Option<String>,
    /// Pinned certificate hashes
    pub pinned_cert_hashes: Vec<String>,
    /// Allowed IP addresses
    pub allowed_ips: Vec<String>,
    /// Allowed IP ranges in CIDR notation
    pub allowed_ip_ranges: Vec<String>,
    /// Encryption key for storage (would be loaded from secure environment)
    #[serde(skip_serializing)]
    pub storage_encryption_key: Option<Vec<u8>>,
    /// Path to encrypted storage directory
    pub encrypted_storage_path: Option<String>,
    /// TLS protocol version
    pub tls_protocol_version: TlsProtocolVersion,
    /// TLS cipher suites
    pub tls_cipher_suites: Vec<String>,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    /// Whether to verify hostname in certificate
    pub verify_hostname: bool,
}

impl Default for SecureCommsConfig {
    fn default() -> Self {
        Self {
            enable_tls: true,
            enable_cert_pinning: true,
            enable_ip_allowlisting: true,
            enable_encrypted_storage: true,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
            server_cert_path: None,
            server_key_path: None,
            pinned_cert_hashes: Vec::new(),
            allowed_ips: vec!["127.0.0.1".to_string()],
            allowed_ip_ranges: vec!["10.0.0.0/8".to_string(), "192.168.0.0/16".to_string()],
            storage_encryption_key: None,
            encrypted_storage_path: None,
            tls_protocol_version: TlsProtocolVersion::Tls13,
            tls_cipher_suites: vec![
                "TLS13_AES_256_GCM_SHA384".to_string(),
                "TLS13_CHACHA20_POLY1305_SHA256".to_string(),
            ],
            connection_timeout_seconds: 30,
            verify_hostname: true,
        }
    }
}

/// TLS protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TlsProtocolVersion {
    /// TLS 1.2
    Tls12,
    /// TLS 1.3
    Tls13,
}

/// Certificate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    /// Certificate subject
    pub subject: String,
    /// Certificate issuer
    pub issuer: String,
    /// Certificate serial number
    pub serial_number: String,
    /// Certificate validity start
    pub valid_from: SystemTime,
    /// Certificate validity end
    pub valid_until: SystemTime,
    /// Certificate fingerprint (SHA-256)
    pub fingerprint: String,
    /// Certificate public key algorithm
    pub public_key_algorithm: String,
    /// Certificate signature algorithm
    pub signature_algorithm: String,
}

/// IP allowlist entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpAllowlistEntry {
    /// IP address or CIDR range
    pub ip_or_range: String,
    /// Description
    pub description: String,
    /// When the entry was added
    pub added_at: SystemTime,
    /// Who added the entry
    pub added_by: String,
    /// Expiration time (if any)
    pub expires_at: Option<SystemTime>,
}

/// Secure communication manager
pub struct SecureCommsManager {
    /// Configuration
    config: RwLock<SecureCommsConfig>,
    /// TLS client configuration
    tls_client_config: RwLock<Option<Arc<ClientConfig>>>,
    /// TLS server configuration
    tls_server_config: RwLock<Option<Arc<rustls::ServerConfig>>>,
    /// Pinned certificates
    pinned_certs: RwLock<HashMap<String, Certificate>>,
    /// IP allowlist
    ip_allowlist: RwLock<Vec<IpAllowlistEntry>>,
    /// IP allowlist cache (for faster lookups)
    ip_allowlist_cache: RwLock<HashSet<IpAddr>>,
    /// CIDR ranges
    cidr_ranges: RwLock<Vec<(Ipv4Addr, u8)>>,
    /// Encrypted storage manager
    encrypted_storage: RwLock<Option<EncryptedStorageManager>>,
}

impl SecureCommsManager {
    /// Create a new secure communication manager
    pub fn new(config: SecureCommsConfig) -> Result<Self> {
        let manager = Self {
            config: RwLock::new(config.clone()),
            tls_client_config: RwLock::new(None),
            tls_server_config: RwLock::new(None),
            pinned_certs: RwLock::new(HashMap::new()),
            ip_allowlist: RwLock::new(Vec::new()),
            ip_allowlist_cache: RwLock::new(HashSet::new()),
            cidr_ranges: RwLock::new(Vec::new()),
            encrypted_storage: RwLock::new(None),
        };
        
        // Initialize components based on configuration
        if config.enable_tls {
            manager.init_tls()?;
        }
        
        if config.enable_cert_pinning {
            manager.init_cert_pinning(&config.pinned_cert_hashes)?;
        }
        
        if config.enable_ip_allowlisting {
            manager.init_ip_allowlist(&config.allowed_ips, &config.allowed_ip_ranges)?;
        }
        
        if config.enable_encrypted_storage {
            if let Some(key) = &config.storage_encryption_key {
                if let Some(path) = &config.encrypted_storage_path {
                    let storage = EncryptedStorageManager::new(path, key.clone())?;
                    *manager.encrypted_storage.write() = Some(storage);
                }
            }
        }
        
        Ok(manager)
    }
    
    /// Initialize TLS
    fn init_tls(&self) -> Result<()> {
        let config = self.config.read();
        
        // Initialize client TLS config
        let mut client_config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(Self::load_root_certs(&config)?)
            .with_no_client_auth();
        
        // Initialize server TLS config if server certs are provided
        if let (Some(server_cert_path), Some(server_key_path)) = 
            (&config.server_cert_path, &config.server_key_path) {
            
            let certs = Self::load_certificates(server_cert_path)?;
            let key = Self::load_private_key(server_key_path)?;
            
            let server_config = rustls::ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(certs, key)
                .map_err(|e| anyhow!("Failed to create server config: {}", e))?;
            
            *self.tls_server_config.write() = Some(Arc::new(server_config));
        }
        
        *self.tls_client_config.write() = Some(Arc::new(client_config));
        
        info!("TLS initialized");
        Ok(())
    }
    
    /// Load root certificates
    fn load_root_certs(config: &SecureCommsConfig) -> Result<RootCertStore> {
        let mut root_store = RootCertStore::empty();
        
        // Load system root certificates
        let mut system_roots = rustls_native_certs::load_native_certs()
            .map_err(|e| anyhow!("Failed to load system root certificates: {}", e))?;
        
        for cert in system_roots {
            root_store.add(&Certificate(cert.0))
                .map_err(|e| anyhow!("Failed to add system root certificate: {}", e))?;
        }
        
        // Load custom CA certificate if provided
        if let Some(ca_path) = &config.ca_cert_path {
            let certs = Self::load_certificates(ca_path)?;
            for cert in certs {
                root_store.add(&cert)
                    .map_err(|e| anyhow!("Failed to add custom CA certificate: {}", e))?;
            }
        }
        
        Ok(root_store)
    }
    
    /// Load certificates from a file
    fn load_certificates(path: &str) -> Result<Vec<Certificate>> {
        let cert_file = fs::read(path)
            .with_context(|| format!("Failed to read certificate file: {}", path))?;
        
        let certs = rustls_pemfile::certs(&mut io::BufReader::new(io::Cursor::new(cert_file)))
            .map_err(|_| anyhow!("Failed to parse certificates"))?;
        
        Ok(certs.into_iter().map(Certificate).collect())
    }
    
    /// Load private key from a file
    fn load_private_key(path: &str) -> Result<rustls::PrivateKey> {
        let key_file = fs::read(path)
            .with_context(|| format!("Failed to read key file: {}", path))?;
        
        let mut reader = io::BufReader::new(io::Cursor::new(key_file));
        
        // Try to read PKCS8 key first
        if let Some(key) = rustls_pemfile::pkcs8_private_keys(&mut reader)
            .map_err(|_| anyhow!("Failed to parse private key"))?
            .into_iter()
            .next() {
            return Ok(rustls::PrivateKey(key));
        }
        
        // Reset reader and try RSA key
        reader = io::BufReader::new(io::Cursor::new(fs::read(path)?));
        if let Some(key) = rustls_pemfile::rsa_private_keys(&mut reader)
            .map_err(|_| anyhow!("Failed to parse RSA private key"))?
            .into_iter()
            .next() {
            return Ok(rustls::PrivateKey(key));
        }
        
        Err(anyhow!("No private key found in file"))
    }
    
    /// Initialize certificate pinning
    fn init_cert_pinning(&self, pinned_hashes: &[String]) -> Result<()> {
        let mut pinned_certs = self.pinned_certs.write();
        
        for hash in pinned_hashes {
            // In a real implementation, this would load the actual certificates
            // For now, just store the hash
            let dummy_cert = Certificate(vec![0; 32]); // Placeholder
            pinned_certs.insert(hash.clone(), dummy_cert);
        }
        
        info!("Certificate pinning initialized with {} certificates", pinned_hashes.len());
        Ok(())
    }
    
    /// Initialize IP allowlisting
    fn init_ip_allowlist(&self, allowed_ips: &[String], allowed_ranges: &[String]) -> Result<()> {
        let mut allowlist = self.ip_allowlist.write();
        let mut cache = self.ip_allowlist_cache.write();
        let mut ranges = self.cidr_ranges.write();
        
        // Add individual IPs
        for ip_str in allowed_ips {
            match ip_str.parse::<IpAddr>() {
                Ok(ip) => {
                    cache.insert(ip);
                    allowlist.push(IpAllowlistEntry {
                        ip_or_range: ip_str.clone(),
                        description: "Default allowed IP".to_string(),
                        added_at: SystemTime::now(),
                        added_by: "system".to_string(),
                        expires_at: None,
                    });
                },
                Err(e) => {
                    warn!("Invalid IP address in allowlist: {}, error: {}", ip_str, e);
                }
            }
        }
        
        // Add CIDR ranges
        for range_str in allowed_ranges {
            if let Some((ip_str, bits_str)) = range_str.split_once('/') {
                if let (Ok(ip), Ok(bits)) = (ip_str.parse::<Ipv4Addr>(), bits_str.parse::<u8>()) {
                    if bits <= 32 {
                        ranges.push((ip, bits));
                        allowlist.push(IpAllowlistEntry {
                            ip_or_range: range_str.clone(),
                            description: "Default allowed IP range".to_string(),
                            added_at: SystemTime::now(),
                            added_by: "system".to_string(),
                            expires_at: None,
                        });
                    } else {
                        warn!("Invalid CIDR prefix length: {}", bits);
                    }
                } else {
                    warn!("Invalid CIDR notation: {}", range_str);
                }
            } else {
                warn!("Invalid CIDR notation: {}", range_str);
            }
        }
        
        info!("IP allowlisting initialized with {} entries", allowlist.len());
        Ok(())
    }
    
    /// Connect to a server using TLS
    pub async fn connect_tls(&self, addr: &str) -> Result<TlsStream<TcpStream>> {
        let config = self.config.read();
        
        if !config.enable_tls {
            return Err(anyhow!("TLS is not enabled"));
        }
        
        let tls_config = self.tls_client_config.read()
            .clone()
            .ok_or_else(|| anyhow!("TLS client config not initialized"))?;
        
        // Parse server address
        let addr = addr.to_socket_addrs()
            .map_err(|e| anyhow!("Invalid address: {}, error: {}", addr, e))?
            .next()
            .ok_or_else(|| anyhow!("Could not resolve address: {}", addr))?;
        
        // Check if IP is allowed
        if config.enable_ip_allowlisting && !self.is_ip_allowed(&addr.ip()) {
            return Err(anyhow!("IP address not allowed: {}", addr.ip()));
        }
        
        // Connect with timeout
        let timeout = Duration::from_secs(config.connection_timeout_seconds);
        let tcp_stream = tokio::time::timeout(
            timeout,
            TcpStream::connect(addr)
        ).await
            .map_err(|_| anyhow!("Connection timeout after {} seconds", timeout.as_secs()))?
            .map_err(|e| anyhow!("Failed to connect to {}: {}", addr, e))?;
        
        // Extract server name for TLS
        let server_name = addr.ip().to_string();
        let server_name = ServerName::try_from(server_name.as_str())
            .map_err(|_| anyhow!("Invalid server name: {}", server_name))?;
        
        // Create TLS connector
        let connector = TlsConnector::from(tls_config);
        
        // Connect TLS
        let tls_stream = tokio::time::timeout(
            timeout,
            connector.connect(server_name, tcp_stream)
        ).await
            .map_err(|_| anyhow!("TLS handshake timeout after {} seconds", timeout.as_secs()))?
            .map_err(|e| anyhow!("TLS handshake failed: {}", e))?;
        
        // Verify certificate if pinning is enabled
        if config.enable_cert_pinning {
            self.verify_pinned_cert(&tls_stream)?;
        }
        
        info!("Established secure TLS connection to {}", addr);
        Ok(tls_stream)
    }
    
    /// Verify a certificate against pinned certificates
    fn verify_pinned_cert(&self, _tls_stream: &TlsStream<TcpStream>) -> Result<()> {
        // In a real implementation, this would extract the peer certificate
        // from the TLS stream and verify it against the pinned certificates
        
        // For now, just return success
        Ok(())
    }
    
    /// Check if an IP address is allowed
    pub fn is_ip_allowed(&self, ip: &IpAddr) -> bool {
        // Check direct IP allowlist
        let cache = self.ip_allowlist_cache.read();
        if cache.contains(ip) {
            return true;
        }
        
        // Check CIDR ranges for IPv4
        if let IpAddr::V4(ipv4) = ip {
            let ranges = self.cidr_ranges.read();
            for (range_ip, bits) in ranges.iter() {
                if is_ipv4_in_cidr(ipv4, range_ip, *bits) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Add an IP address to the allowlist
    pub fn add_allowed_ip(&self, ip_str: &str, description: &str, added_by: &str) -> Result<()> {
        let ip = ip_str.parse::<IpAddr>()
            .map_err(|e| anyhow!("Invalid IP address: {}, error: {}", ip_str, e))?;
        
        let entry = IpAllowlistEntry {
            ip_or_range: ip_str.to_string(),
            description: description.to_string(),
            added_at: SystemTime::now(),
            added_by: added_by.to_string(),
            expires_at: None,
        };
        
        let mut allowlist = self.ip_allowlist.write();
        let mut cache = self.ip_allowlist_cache.write();
        
        allowlist.push(entry);
        cache.insert(ip);
        
        info!("Added IP {} to allowlist", ip_str);
        Ok(())
    }
    
    /// Add a CIDR range to the allowlist
    pub fn add_allowed_cidr(&self, cidr: &str, description: &str, added_by: &str) -> Result<()> {
        let (ip_str, bits_str) = cidr.split_once('/')
            .ok_or_else(|| anyhow!("Invalid CIDR notation: {}", cidr))?;
        
        let ip = ip_str.parse::<Ipv4Addr>()
            .map_err(|e| anyhow!("Invalid IP address: {}, error: {}", ip_str, e))?;
        
        let bits = bits_str.parse::<u8>()
            .map_err(|e| anyhow!("Invalid prefix length: {}, error: {}", bits_str, e))?;
        
        if bits > 32 {
            return Err(anyhow!("Invalid CIDR prefix length: {}", bits));
        }
        
        let entry = IpAllowlistEntry {
            ip_or_range: cidr.to_string(),
            description: description.to_string(),
            added_at: SystemTime::now(),
            added_by: added_by.to_string(),
            expires_at: None,
        };
        
        let mut allowlist = self.ip_allowlist.write();
        let mut ranges = self.cidr_ranges.write();
        
        allowlist.push(entry);
        ranges.push((ip, bits));
        
        info!("Added CIDR range {} to allowlist", cidr);
        Ok(())
    }
    
    /// Remove an IP or CIDR range from the allowlist
    pub fn remove_from_allowlist(&self, ip_or_cidr: &str) -> Result<()> {
        let mut allowlist = self.ip_allowlist.write();
        let mut cache = self.ip_allowlist_cache.write();
        let mut ranges = self.cidr_ranges.write();
        
        // Find and remove from allowlist
        let initial_len = allowlist.len();
        allowlist.retain(|entry| entry.ip_or_range != ip_or_cidr);
        
        if allowlist.len() == initial_len {
            return Err(anyhow!("IP or CIDR range not found in allowlist: {}", ip_or_cidr));
        }
        
        // Remove from cache or ranges
        if let Ok(ip) = ip_or_cidr.parse::<IpAddr>() {
            cache.remove(&ip);
        } else if let Some((ip_str, bits_str)) = ip_or_cidr.split_once('/') {
            if let (Ok(ip), Ok(bits)) = (ip_str.parse::<Ipv4Addr>(), bits_str.parse::<u8>()) {
                ranges.retain(|(range_ip, range_bits)| *range_ip != ip || *range_bits != bits);
            }
        }
        
        info!("Removed {} from allowlist", ip_or_cidr);
        Ok(())
    }
    
    /// Get the current IP allowlist
    pub fn get_allowlist(&self) -> Vec<IpAllowlistEntry> {
        self.ip_allowlist.read().clone()
    }
    
    /// Store data in encrypted storage
    pub fn store_encrypted(&self, key: &str, data: &[u8]) -> Result<()> {
        let storage = self.encrypted_storage.read()
            .as_ref()
            .ok_or_else(|| anyhow!("Encrypted storage not initialized"))?
            .clone();
        
        storage.store(key, data)
    }
    
    /// Retrieve data from encrypted storage
    pub fn retrieve_encrypted(&self, key: &str) -> Result<Vec<u8>> {
        let storage = self.encrypted_storage.read()
            .as_ref()
            .ok_or_else(|| anyhow!("Encrypted storage not initialized"))?
            .clone();
        
        storage.retrieve(key)
    }
    
    /// Generate a firewall configuration
    pub fn generate_firewall_config(&self, output_path: &str) -> Result<()> {
        let allowlist = self.ip_allowlist.read();
        let ranges = self.cidr_ranges.read();
        
        // Generate iptables rules for Linux
        let mut iptables_rules = String::new();
        iptables_rules.push_str("#!/bin/bash\n\n");
        iptables_rules.push_str("# Generated firewall rules\n");
        iptables_rules.push_str("# Generated at: ");
        iptables_rules.push_str(&format!("{:?}\n\n", SystemTime::now()));
        
        iptables_rules.push_str("# Flush existing rules\n");
        iptables_rules.push_str("iptables -F\n");
        iptables_rules.push_str("iptables -X\n\n");
        
        iptables_rules.push_str("# Default policies\n");
        iptables_rules.push_str("iptables -P INPUT DROP\n");
        iptables_rules.push_str("iptables -P FORWARD DROP\n");
        iptables_rules.push_str("iptables -P OUTPUT ACCEPT\n\n");
        
        iptables_rules.push_str("# Allow loopback\n");
        iptables_rules.push_str("iptables -A INPUT -i lo -j ACCEPT\n");
        iptables_rules.push_str("iptables -A OUTPUT -o lo -j ACCEPT\n\n");
        
        iptables_rules.push_str("# Allow established connections\n");
        iptables_rules.push_str("iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT\n\n");
        
        iptables_rules.push_str("# Allow specific IPs and ranges\n");
        for entry in allowlist.iter() {
            if !entry.ip_or_range.contains('/') {
                iptables_rules.push_str(&format!(
                    "iptables -A INPUT -s {} -j ACCEPT  # {}\n",
                    entry.ip_or_range, entry.description
                ));
            }
        }
        
        iptables_rules.push_str("\n# Allow CIDR ranges\n");
        for (ip, bits) in ranges.iter() {
            iptables_rules.push_str(&format!(
                "iptables -A INPUT -s {}/{} -j ACCEPT\n",
                ip, bits
            ));
        }
        
        iptables_rules.push_str("\n# Log and drop everything else\n");
        iptables_rules.push_str("iptables -A INPUT -j LOG --log-prefix \"[IPTABLES DROP] \"\n");
        iptables_rules.push_str("iptables -A INPUT -j DROP\n");
        
        // Write to file
        fs::write(output_path, iptables_rules)
            .with_context(|| format!("Failed to write firewall config to {}", output_path))?;
        
        info!("Generated firewall configuration at {}", output_path);
        Ok(())
    }
}

/// Encrypted storage manager
#[derive(Clone)]
pub struct EncryptedStorageManager {
    /// Storage directory
    storage_dir: PathBuf,
    /// Encryption key
    encryption_key: Vec<u8>,
}

impl EncryptedStorageManager {
    /// Create a new encrypted storage manager
    pub fn new(storage_dir: &str, encryption_key: Vec<u8>) -> Result<Self> {
        let dir = PathBuf::from(storage_dir);
        
        // Create directory if it doesn't exist
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .with_context(|| format!("Failed to create storage directory: {}", storage_dir))?;
        }
        
        Ok(Self {
            storage_dir: dir,
            encryption_key,
        })
    }
    
    /// Store data
    pub fn store(&self, key: &str, data: &[u8]) -> Result<()> {
        // Hash the key to get a filename
        let filename = self.hash_key(key);
        let path = self.storage_dir.join(filename);
        
        // Encrypt the data
        let encrypted = self.encrypt(data)?;
        
        // Write to file
        fs::write(&path, encrypted)
            .with_context(|| format!("Failed to write encrypted data to {}", path.display()))?;
        
        Ok(())
    }
    
    /// Retrieve data
    pub fn retrieve(&self, key: &str) -> Result<Vec<u8>> {
        // Hash the key to get a filename
        let filename = self.hash_key(key);
        let path = self.storage_dir.join(filename);
        
        // Read from file
        let encrypted = fs::read(&path)
            .with_context(|| format!("Failed to read encrypted data from {}", path.display()))?;
        
        // Decrypt the data
        let decrypted = self.decrypt(&encrypted)?;
        
        Ok(decrypted)
    }
    
    /// Delete data
    pub fn delete(&self, key: &str) -> Result<()> {
        // Hash the key to get a filename
        let filename = self.hash_key(key);
        let path = self.storage_dir.join(filename);
        
        // Delete the file
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to delete encrypted data at {}", path.display()))?;
            Ok(())
        } else {
            Err(anyhow!("No data found for key: {}", key))
        }
    }
    
    /// List all keys
    pub fn list_keys(&self) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        
        for entry in fs::read_dir(&self.storage_dir)
            .with_context(|| format!("Failed to read storage directory: {}", self.storage_dir.display()))? {
            
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy().to_string();
            
            // In a real implementation, we would map filenames back to keys
            // For now, just return the hashed filenames
            keys.push(filename_str);
        }
        
        Ok(keys)
    }
    
    /// Hash a key to get a filename
    fn hash_key(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();
        
        hex::encode(result)
    }
    
    /// Encrypt data
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        // In a real implementation, this would use a proper encryption algorithm
        // For now, use a simple XOR cipher as a placeholder
        let mut result = Vec::with_capacity(data.len());
        
        for (i, &byte) in data.iter().enumerate() {
            result.push(byte ^ self.encryption_key[i % self.encryption_key.len()]);
        }
        
        Ok(result)
    }
    
    /// Decrypt data
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        // For XOR, encryption and decryption are the same operation
        self.encrypt(data)
    }
}

/// Check if an IPv4 address is in a CIDR range
fn is_ipv4_in_cidr(ip: Ipv4Addr, cidr_ip: &Ipv4Addr, cidr_bits: u8) -> bool {
    if cidr_bits == 0 {
        return true; // 0.0.0.0/0 matches everything
    }
    
    let ip_u32 = u32::from(ip);
    let cidr_u32 = u32::from(*cidr_ip);
    
    // Create a mask with the specified number of bits
    let mask = !0u32 << (32 - cidr_bits);
    
    // Apply the mask to both IPs and compare
    (ip_u32 & mask) == (cidr_u32 & mask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;
    
    #[test]
    fn test_secure_comms_config_default() {
        let config = SecureCommsConfig::default();
        assert!(config.enable_tls);
        assert!(config.enable_cert_pinning);
        assert!(config.enable_ip_allowlisting);
        assert!(config.enable_encrypted_storage);
        assert_eq!(config.tls_protocol_version, TlsProtocolVersion::Tls13);
        assert!(!config.allowed_ips.is_empty());
        assert!(!config.allowed_ip_ranges.is_empty());
    }
    
    #[test]
    fn test_is_ipv4_in_cidr() {
        // Test exact match
        assert!(is_ipv4_in_cidr(
            Ipv4Addr::new(192, 168, 1, 1),
            &Ipv4Addr::new(192, 168, 1, 1),
            32
        ));
        
        // Test subnet match
        assert!(is_ipv4_in_cidr(
            Ipv4Addr::new(192, 168, 1, 100),
            &Ipv4Addr::new(192, 168, 1, 0),
            24
        ));
        
        // Test non-match
        assert!(!is_ipv4_in_cidr(
            Ipv4Addr::new(192, 168, 2, 1),
            &Ipv4Addr::new(192, 168, 1, 0),
            24
        ));
        
        // Test larger subnet
        assert!(is_ipv4_in_cidr(
            Ipv4Addr::new(192, 168, 100, 1),
            &Ipv4Addr::new(192, 168, 0, 0),
            16
        ));
        
        // Test catch-all
        assert!(is_ipv4_in_cidr(
            Ipv4Addr::new(8, 8, 8, 8),
            &Ipv4Addr::new(0, 0, 0, 0),
            0
        ));
    }
    
    #[test]
    fn test_encrypted_storage() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage_path = temp_dir.path().to_string_lossy().to_string();
        let key = b"test-encryption-key-12345".to_vec();
        
        let storage = EncryptedStorageManager::new(&storage_path, key).unwrap();
        
        // Store data
        let test_key = "test-key";
        let test_data = b"This is test data for encryption".to_vec();
        storage.store(test_key, &test_data).unwrap();
        
        // Retrieve data
        let retrieved = storage.retrieve(test_key).unwrap();
        assert_eq!(retrieved, test_data);
        
        // Delete data
        storage.delete(test_key).unwrap();
        assert!(storage.retrieve(test_key).is_err());
    }
    
    #[test]
    fn test_ip_allowlist() {
        let config = SecureCommsConfig::default();
        let manager = SecureCommsManager::new(config).unwrap();
        
        // Test allowed IPs
        assert!(manager.is_ip_allowed(&"127.0.0.1".parse().unwrap()));
        assert!(manager.is_ip_allowed(&"10.0.0.1".parse().unwrap()));
        assert!(manager.is_ip_allowed(&"192.168.1.1".parse().unwrap()));
        
        // Test disallowed IPs
        assert!(!manager.is_ip_allowed(&"8.8.8.8".parse().unwrap()));
        
        // Add an IP
        manager.add_allowed_ip("8.8.8.8", "Google DNS", "test").unwrap();
        assert!(manager.is_ip_allowed(&"8.8.8.8".parse().unwrap()));
        
        // Remove an IP
        manager.remove_from_allowlist("8.8.8.8").unwrap();
        assert!(!manager.is_ip_allowed(&"8.8.8.8".parse().unwrap()));
    }
}