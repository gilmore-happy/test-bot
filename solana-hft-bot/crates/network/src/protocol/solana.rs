//! Solana protocol handler
//! 
//! This module provides optimizations for the Solana RPC protocol.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use super::{ProtocolHandler, ProtocolType};

/// Solana message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolanaMessageType {
    /// JSON-RPC request
    Request,
    
    /// JSON-RPC response
    Response,
    
    /// JSON-RPC notification
    Notification,
    
    /// JSON-RPC error
    Error,
    
    /// Unknown message type
    Unknown,
}

/// Solana message
#[derive(Debug, Clone)]
pub struct SolanaMessage {
    /// Message type
    pub message_type: SolanaMessageType,
    
    /// Message ID
    pub id: Option<Value>,
    
    /// Method name (for requests and notifications)
    pub method: Option<String>,
    
    /// Parameters (for requests and notifications)
    pub params: Option<Value>,
    
    /// Result (for responses)
    pub result: Option<Value>,
    
    /// Error (for errors)
    pub error: Option<Value>,
}

impl SolanaMessage {
    /// Create a new request message
    pub fn new_request(id: Value, method: String, params: Value) -> Self {
        Self {
            message_type: SolanaMessageType::Request,
            id: Some(id),
            method: Some(method),
            params: Some(params),
            result: None,
            error: None,
        }
    }
    
    /// Create a new response message
    pub fn new_response(id: Value, result: Value) -> Self {
        Self {
            message_type: SolanaMessageType::Response,
            id: Some(id),
            method: None,
            params: None,
            result: Some(result),
            error: None,
        }
    }
    
    /// Create a new notification message
    pub fn new_notification(method: String, params: Value) -> Self {
        Self {
            message_type: SolanaMessageType::Notification,
            id: None,
            method: Some(method),
            params: Some(params),
            result: None,
            error: None,
        }
    }
    
    /// Create a new error message
    pub fn new_error(id: Value, error: Value) -> Self {
        Self {
            message_type: SolanaMessageType::Error,
            id: Some(id),
            method: None,
            params: None,
            result: None,
            error: Some(error),
        }
    }
    
    /// Parse a message from JSON
    pub fn from_json(json: &[u8]) -> Result<Self> {
        let value: Value = serde_json::from_slice(json)?;
        
        if !value.is_object() {
            return Err(anyhow!("Invalid JSON-RPC message: not an object"));
        }
        
        let obj = value.as_object().unwrap();
        
        // Check if it's a request or notification
        if obj.contains_key("method") {
            let method = obj.get("method").and_then(Value::as_str).ok_or_else(|| {
                anyhow!("Invalid JSON-RPC message: method is not a string")
            })?.to_string();
            
            let params = obj.get("params").cloned();
            
            if obj.contains_key("id") {
                // Request
                let id = obj.get("id").cloned();
                
                Ok(Self {
                    message_type: SolanaMessageType::Request,
                    id,
                    method: Some(method),
                    params,
                    result: None,
                    error: None,
                })
            } else {
                // Notification
                Ok(Self {
                    message_type: SolanaMessageType::Notification,
                    id: None,
                    method: Some(method),
                    params,
                    result: None,
                    error: None,
                })
            }
        } else if obj.contains_key("result") {
            // Response
            let id = obj.get("id").cloned();
            let result = obj.get("result").cloned();
            
            Ok(Self {
                message_type: SolanaMessageType::Response,
                id,
                method: None,
                params: None,
                result,
                error: None,
            })
        } else if obj.contains_key("error") {
            // Error
            let id = obj.get("id").cloned();
            let error = obj.get("error").cloned();
            
            Ok(Self {
                message_type: SolanaMessageType::Error,
                id,
                method: None,
                params: None,
                result: None,
                error,
            })
        } else {
            // Unknown
            Ok(Self {
                message_type: SolanaMessageType::Unknown,
                id: None,
                method: None,
                params: None,
                result: None,
                error: None,
            })
        }
    }
    
    /// Convert the message to JSON
    pub fn to_json(&self) -> Result<Vec<u8>> {
        let value = match self.message_type {
            SolanaMessageType::Request => {
                let mut obj = serde_json::Map::new();
                obj.insert("jsonrpc".to_string(), json!("2.0"));
                
                if let Some(id) = &self.id {
                    obj.insert("id".to_string(), id.clone());
                }
                
                if let Some(method) = &self.method {
                    obj.insert("method".to_string(), json!(method));
                }
                
                if let Some(params) = &self.params {
                    obj.insert("params".to_string(), params.clone());
                }
                
                Value::Object(obj)
            }
            SolanaMessageType::Response => {
                let mut obj = serde_json::Map::new();
                obj.insert("jsonrpc".to_string(), json!("2.0"));
                
                if let Some(id) = &self.id {
                    obj.insert("id".to_string(), id.clone());
                }
                
                if let Some(result) = &self.result {
                    obj.insert("result".to_string(), result.clone());
                }
                
                Value::Object(obj)
            }
            SolanaMessageType::Notification => {
                let mut obj = serde_json::Map::new();
                obj.insert("jsonrpc".to_string(), json!("2.0"));
                
                if let Some(method) = &self.method {
                    obj.insert("method".to_string(), json!(method));
                }
                
                if let Some(params) = &self.params {
                    obj.insert("params".to_string(), params.clone());
                }
                
                Value::Object(obj)
            }
            SolanaMessageType::Error => {
                let mut obj = serde_json::Map::new();
                obj.insert("jsonrpc".to_string(), json!("2.0"));
                
                if let Some(id) = &self.id {
                    obj.insert("id".to_string(), id.clone());
                }
                
                if let Some(error) = &self.error {
                    obj.insert("error".to_string(), error.clone());
                }
                
                Value::Object(obj)
            }
            SolanaMessageType::Unknown => {
                return Err(anyhow!("Cannot convert unknown message type to JSON"));
            }
        };
        
        Ok(serde_json::to_vec(&value)?)
    }
}

/// Solana protocol handler
pub struct SolanaProtocolHandler {
    /// Method cache for faster processing
    method_cache: HashMap<String, SolanaMessageType>,
}

impl SolanaProtocolHandler {
    /// Create a new Solana protocol handler
    pub fn new() -> Self {
        let mut method_cache = HashMap::new();
        
        // Pre-populate cache with common methods
        method_cache.insert("getAccountInfo".to_string(), SolanaMessageType::Request);
        method_cache.insert("getBalance".to_string(), SolanaMessageType::Request);
        method_cache.insert("getBlockHeight".to_string(), SolanaMessageType::Request);
        method_cache.insert("getBlockProduction".to_string(), SolanaMessageType::Request);
        method_cache.insert("getBlockCommitment".to_string(), SolanaMessageType::Request);
        method_cache.insert("getBlocks".to_string(), SolanaMessageType::Request);
        method_cache.insert("getBlocksWithLimit".to_string(), SolanaMessageType::Request);
        method_cache.insert("getBlockTime".to_string(), SolanaMessageType::Request);
        method_cache.insert("getClusterNodes".to_string(), SolanaMessageType::Request);
        method_cache.insert("getEpochInfo".to_string(), SolanaMessageType::Request);
        method_cache.insert("getEpochSchedule".to_string(), SolanaMessageType::Request);
        method_cache.insert("getFeeForMessage".to_string(), SolanaMessageType::Request);
        method_cache.insert("getFirstAvailableBlock".to_string(), SolanaMessageType::Request);
        method_cache.insert("getGenesisHash".to_string(), SolanaMessageType::Request);
        method_cache.insert("getHealth".to_string(), SolanaMessageType::Request);
        method_cache.insert("getHighestSnapshotSlot".to_string(), SolanaMessageType::Request);
        method_cache.insert("getIdentity".to_string(), SolanaMessageType::Request);
        method_cache.insert("getInflationGovernor".to_string(), SolanaMessageType::Request);
        method_cache.insert("getInflationRate".to_string(), SolanaMessageType::Request);
        method_cache.insert("getInflationReward".to_string(), SolanaMessageType::Request);
        method_cache.insert("getLargestAccounts".to_string(), SolanaMessageType::Request);
        method_cache.insert("getLatestBlockhash".to_string(), SolanaMessageType::Request);
        method_cache.insert("getLeaderSchedule".to_string(), SolanaMessageType::Request);
        method_cache.insert("getMinimumBalanceForRentExemption".to_string(), SolanaMessageType::Request);
        method_cache.insert("getMultipleAccounts".to_string(), SolanaMessageType::Request);
        method_cache.insert("getProgramAccounts".to_string(), SolanaMessageType::Request);
        method_cache.insert("getRecentBlockhash".to_string(), SolanaMessageType::Request);
        method_cache.insert("getRecentPerformanceSamples".to_string(), SolanaMessageType::Request);
        method_cache.insert("getSignaturesForAddress".to_string(), SolanaMessageType::Request);
        method_cache.insert("getSignatureStatuses".to_string(), SolanaMessageType::Request);
        method_cache.insert("getSlot".to_string(), SolanaMessageType::Request);
        method_cache.insert("getSlotLeader".to_string(), SolanaMessageType::Request);
        method_cache.insert("getSlotLeaders".to_string(), SolanaMessageType::Request);
        method_cache.insert("getStakeActivation".to_string(), SolanaMessageType::Request);
        method_cache.insert("getSupply".to_string(), SolanaMessageType::Request);
        method_cache.insert("getTokenAccountBalance".to_string(), SolanaMessageType::Request);
        method_cache.insert("getTokenAccountsByDelegate".to_string(), SolanaMessageType::Request);
        method_cache.insert("getTokenAccountsByOwner".to_string(), SolanaMessageType::Request);
        method_cache.insert("getTokenLargestAccounts".to_string(), SolanaMessageType::Request);
        method_cache.insert("getTokenSupply".to_string(), SolanaMessageType::Request);
        method_cache.insert("getTransaction".to_string(), SolanaMessageType::Request);
        method_cache.insert("getTransactionCount".to_string(), SolanaMessageType::Request);
        method_cache.insert("getVersion".to_string(), SolanaMessageType::Request);
        method_cache.insert("getVoteAccounts".to_string(), SolanaMessageType::Request);
        method_cache.insert("minimumLedgerSlot".to_string(), SolanaMessageType::Request);
        method_cache.insert("requestAirdrop".to_string(), SolanaMessageType::Request);
        method_cache.insert("sendTransaction".to_string(), SolanaMessageType::Request);
        method_cache.insert("simulateTransaction".to_string(), SolanaMessageType::Request);
        method_cache.insert("accountSubscribe".to_string(), SolanaMessageType::Request);
        method_cache.insert("accountUnsubscribe".to_string(), SolanaMessageType::Request);
        method_cache.insert("logsSubscribe".to_string(), SolanaMessageType::Request);
        method_cache.insert("logsUnsubscribe".to_string(), SolanaMessageType::Request);
        method_cache.insert("programSubscribe".to_string(), SolanaMessageType::Request);
        method_cache.insert("programUnsubscribe".to_string(), SolanaMessageType::Request);
        method_cache.insert("signatureSubscribe".to_string(), SolanaMessageType::Request);
        method_cache.insert("signatureUnsubscribe".to_string(), SolanaMessageType::Request);
        method_cache.insert("slotSubscribe".to_string(), SolanaMessageType::Request);
        method_cache.insert("slotUnsubscribe".to_string(), SolanaMessageType::Request);
        
        Self { method_cache }
    }
    
    /// Get the message type for a method
    pub fn get_message_type_for_method(&self, method: &str) -> SolanaMessageType {
        self.method_cache.get(method).copied().unwrap_or(SolanaMessageType::Unknown)
    }
}

impl ProtocolHandler for SolanaProtocolHandler {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::Solana
    }
    
    fn encode(&self, message: &[u8]) -> Result<Vec<u8>> {
        // Parse the message
        let solana_message = SolanaMessage::from_json(message)?;
        
        // Encode the message
        solana_message.to_json()
    }
    
    fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        // For Solana, we just validate that it's valid JSON
        let _: Value = serde_json::from_slice(data)?;
        
        // Return the original data
        Ok(data.to_vec())
    }
    
    fn is_message_complete(&self, data: &[u8]) -> bool {
        // For JSON-RPC, we need to parse the message to check if it's complete
        // This is a simple check that the JSON is valid and balanced
        let result: Result<Value, _> = serde_json::from_slice(data);
        result.is_ok()
    }
    
    fn get_message_size(&self, data: &[u8]) -> Result<usize> {
        // For JSON-RPC, we need to parse the message to get its size
        // This is a simple check that returns the length of the data
        if self.is_message_complete(data) {
            Ok(data.len())
        } else {
            Err(anyhow!("Incomplete message"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solana_message_request() {
        let request = SolanaMessage::new_request(
            json!(1),
            "getAccountInfo".to_string(),
            json!(["9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"]),
        );
        
        assert_eq!(request.message_type, SolanaMessageType::Request);
        assert_eq!(request.id, Some(json!(1)));
        assert_eq!(request.method, Some("getAccountInfo".to_string()));
        assert_eq!(request.params, Some(json!(["9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"])));
        assert_eq!(request.result, None);
        assert_eq!(request.error, None);
        
        let json = request.to_json().unwrap();
        let parsed = SolanaMessage::from_json(&json).unwrap();
        
        assert_eq!(parsed.message_type, request.message_type);
        assert_eq!(parsed.id, request.id);
        assert_eq!(parsed.method, request.method);
        assert_eq!(parsed.params, request.params);
        assert_eq!(parsed.result, request.result);
        assert_eq!(parsed.error, request.error);
    }

    #[test]
    fn test_solana_message_response() {
        let response = SolanaMessage::new_response(
            json!(1),
            json!({
                "context": {
                    "slot": 1
                },
                "value": {
                    "data": "",
                    "executable": false,
                    "lamports": 1000000000,
                    "owner": "11111111111111111111111111111111",
                    "rentEpoch": 0
                }
            }),
        );
        
        assert_eq!(response.message_type, SolanaMessageType::Response);
        assert_eq!(response.id, Some(json!(1)));
        assert_eq!(response.method, None);
        assert_eq!(response.params, None);
        assert!(response.result.is_some());
        assert_eq!(response.error, None);
        
        let json = response.to_json().unwrap();
        let parsed = SolanaMessage::from_json(&json).unwrap();
        
        assert_eq!(parsed.message_type, response.message_type);
        assert_eq!(parsed.id, response.id);
        assert_eq!(parsed.method, response.method);
        assert_eq!(parsed.params, response.params);
        assert_eq!(parsed.result, response.result);
        assert_eq!(parsed.error, response.error);
    }

    #[test]
    fn test_solana_protocol_handler() {
        let handler = SolanaProtocolHandler::new();
        
        // Test protocol type
        assert_eq!(handler.protocol_type(), ProtocolType::Solana);
        
        // Test message type for method
        assert_eq!(handler.get_message_type_for_method("getAccountInfo"), SolanaMessageType::Request);
        assert_eq!(handler.get_message_type_for_method("unknown"), SolanaMessageType::Unknown);
        
        // Test encode/decode
        let request = SolanaMessage::new_request(
            json!(1),
            "getAccountInfo".to_string(),
            json!(["9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"]),
        );
        
        let json = request.to_json().unwrap();
        let encoded = handler.encode(&json).unwrap();
        let decoded = handler.decode(&encoded).unwrap();
        
        assert_eq!(decoded, encoded);
        
        // Test is_message_complete
        assert!(handler.is_message_complete(&json));
        assert!(!handler.is_message_complete(&json[..json.len() - 1]));
        
        // Test get_message_size
        assert_eq!(handler.get_message_size(&json).unwrap(), json.len());
        assert!(handler.get_message_size(&json[..json.len() - 1]).is_err());
    }
}