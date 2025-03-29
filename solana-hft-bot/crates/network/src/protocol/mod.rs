//! Protocol handlers module
//! 
//! This module provides protocol-specific optimizations for high-performance networking.

mod solana;
mod websocket;

pub use solana::{SolanaProtocolHandler, SolanaMessage, SolanaMessageType};
pub use websocket::{WebSocketHandler, WebSocketMessage, WebSocketOpcode};

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    /// Solana protocol
    Solana,
    
    /// WebSocket protocol
    WebSocket,
    
    /// Raw TCP protocol
    RawTcp,
    
    /// Raw UDP protocol
    RawUdp,
}

/// Protocol handler trait
pub trait ProtocolHandler: Send + Sync {
    /// Get the protocol type
    fn protocol_type(&self) -> ProtocolType;
    
    /// Encode a message
    fn encode(&self, message: &[u8]) -> Result<Vec<u8>>;
    
    /// Decode a message
    fn decode(&self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Check if a message is complete
    fn is_message_complete(&self, data: &[u8]) -> bool;
    
    /// Get the message size
    fn get_message_size(&self, data: &[u8]) -> Result<usize>;
}

/// Protocol handler factory
pub struct ProtocolHandlerFactory;

impl ProtocolHandlerFactory {
    /// Create a new protocol handler
    pub fn create(protocol_type: ProtocolType) -> Result<Arc<dyn ProtocolHandler>> {
        match protocol_type {
            ProtocolType::Solana => Ok(Arc::new(SolanaProtocolHandler::new())),
            ProtocolType::WebSocket => Ok(Arc::new(WebSocketHandler::new())),
            ProtocolType::RawTcp => Err(anyhow!("Raw TCP protocol handler not implemented")),
            ProtocolType::RawUdp => Err(anyhow!("Raw UDP protocol handler not implemented")),
        }
    }
}

/// Protocol statistics
#[derive(Debug, Clone, Default)]
pub struct ProtocolStats {
    /// Number of messages sent
    pub messages_sent: u64,
    
    /// Number of messages received
    pub messages_received: u64,
    
    /// Number of bytes sent
    pub bytes_sent: u64,
    
    /// Number of bytes received
    pub bytes_received: u64,
    
    /// Number of encoding errors
    pub encoding_errors: u64,
    
    /// Number of decoding errors
    pub decoding_errors: u64,
}

impl ProtocolStats {
    /// Create new protocol statistics
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Reset statistics
    pub fn reset(&mut self) {
        self.messages_sent = 0;
        self.messages_received = 0;
        self.bytes_sent = 0;
        self.bytes_received = 0;
        self.encoding_errors = 0;
        self.decoding_errors = 0;
    }
    
    /// Record a sent message
    pub fn record_sent(&mut self, bytes: usize) {
        self.messages_sent += 1;
        self.bytes_sent += bytes as u64;
    }
    
    /// Record a received message
    pub fn record_received(&mut self, bytes: usize) {
        self.messages_received += 1;
        self.bytes_received += bytes as u64;
    }
    
    /// Record an encoding error
    pub fn record_encoding_error(&mut self) {
        self.encoding_errors += 1;
    }
    
    /// Record a decoding error
    pub fn record_decoding_error(&mut self) {
        self.decoding_errors += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_handler_factory() {
        let solana_handler = ProtocolHandlerFactory::create(ProtocolType::Solana);
        assert!(solana_handler.is_ok());
        assert_eq!(solana_handler.unwrap().protocol_type(), ProtocolType::Solana);
        
        let websocket_handler = ProtocolHandlerFactory::create(ProtocolType::WebSocket);
        assert!(websocket_handler.is_ok());
        assert_eq!(websocket_handler.unwrap().protocol_type(), ProtocolType::WebSocket);
        
        let raw_tcp_handler = ProtocolHandlerFactory::create(ProtocolType::RawTcp);
        assert!(raw_tcp_handler.is_err());
        
        let raw_udp_handler = ProtocolHandlerFactory::create(ProtocolType::RawUdp);
        assert!(raw_udp_handler.is_err());
    }

    #[test]
    fn test_protocol_stats() {
        let mut stats = ProtocolStats::new();
        
        // Test initial state
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.encoding_errors, 0);
        assert_eq!(stats.decoding_errors, 0);
        
        // Test recording
        stats.record_sent(100);
        stats.record_received(200);
        stats.record_encoding_error();
        stats.record_decoding_error();
        
        assert_eq!(stats.messages_sent, 1);
        assert_eq!(stats.messages_received, 1);
        assert_eq!(stats.bytes_sent, 100);
        assert_eq!(stats.bytes_received, 200);
        assert_eq!(stats.encoding_errors, 1);
        assert_eq!(stats.decoding_errors, 1);
        
        // Test reset
        stats.reset();
        
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.encoding_errors, 0);
        assert_eq!(stats.decoding_errors, 0);
    }
}