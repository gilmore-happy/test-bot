//! WebSocket protocol handler
//! 
//! This module provides optimizations for the WebSocket protocol.

use anyhow::{anyhow, Result};
use std::convert::TryFrom;
use tracing::{debug, error, info, warn};

use super::{ProtocolHandler, ProtocolType};

/// WebSocket opcode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSocketOpcode {
    /// Continuation frame
    Continuation = 0x0,
    
    /// Text frame
    Text = 0x1,
    
    /// Binary frame
    Binary = 0x2,
    
    /// Connection close
    Close = 0x8,
    
    /// Ping
    Ping = 0x9,
    
    /// Pong
    Pong = 0xA,
}

impl TryFrom<u8> for WebSocketOpcode {
    type Error = anyhow::Error;
    
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x0 => Ok(WebSocketOpcode::Continuation),
            0x1 => Ok(WebSocketOpcode::Text),
            0x2 => Ok(WebSocketOpcode::Binary),
            0x8 => Ok(WebSocketOpcode::Close),
            0x9 => Ok(WebSocketOpcode::Ping),
            0xA => Ok(WebSocketOpcode::Pong),
            _ => Err(anyhow!("Invalid WebSocket opcode: {}", value)),
        }
    }
}

/// WebSocket message
#[derive(Debug, Clone)]
pub struct WebSocketMessage {
    /// Opcode
    pub opcode: WebSocketOpcode,
    
    /// Payload
    pub payload: Vec<u8>,
    
    /// FIN flag
    pub fin: bool,
    
    /// Mask
    pub mask: Option<[u8; 4]>,
}

impl WebSocketMessage {
    /// Create a new WebSocket message
    pub fn new(opcode: WebSocketOpcode, payload: Vec<u8>, fin: bool, mask: Option<[u8; 4]>) -> Self {
        Self {
            opcode,
            payload,
            fin,
            mask,
        }
    }
    
    /// Create a new text message
    pub fn text(payload: Vec<u8>) -> Self {
        Self {
            opcode: WebSocketOpcode::Text,
            payload,
            fin: true,
            mask: None,
        }
    }
    
    /// Create a new binary message
    pub fn binary(payload: Vec<u8>) -> Self {
        Self {
            opcode: WebSocketOpcode::Binary,
            payload,
            fin: true,
            mask: None,
        }
    }
    
    /// Create a new close message
    pub fn close(code: u16, reason: &str) -> Self {
        let mut payload = Vec::with_capacity(2 + reason.len());
        payload.extend_from_slice(&code.to_be_bytes());
        payload.extend_from_slice(reason.as_bytes());
        
        Self {
            opcode: WebSocketOpcode::Close,
            payload,
            fin: true,
            mask: None,
        }
    }
    
    /// Create a new ping message
    pub fn ping(payload: Vec<u8>) -> Self {
        Self {
            opcode: WebSocketOpcode::Ping,
            payload,
            fin: true,
            mask: None,
        }
    }
    
    /// Create a new pong message
    pub fn pong(payload: Vec<u8>) -> Self {
        Self {
            opcode: WebSocketOpcode::Pong,
            payload,
            fin: true,
            mask: None,
        }
    }
    
    /// Encode the message to bytes
    pub fn encode(&self) -> Result<Vec<u8>> {
        let payload_len = self.payload.len();
        
        // Calculate the frame size
        let mut frame_size = 2; // First 2 bytes (fin, opcode, mask, 7-bit length)
        
        if payload_len > 125 && payload_len <= 65535 {
            frame_size += 2; // 16-bit length
        } else if payload_len > 65535 {
            frame_size += 8; // 64-bit length
        }
        
        if self.mask.is_some() {
            frame_size += 4; // Mask
        }
        
        frame_size += payload_len; // Payload
        
        // Create the frame
        let mut frame = Vec::with_capacity(frame_size);
        
        // First byte: FIN and opcode
        let first_byte = ((self.fin as u8) << 7) | (self.opcode as u8);
        frame.push(first_byte);
        
        // Second byte: MASK and 7-bit payload length
        let mut second_byte = 0;
        
        if self.mask.is_some() {
            second_byte |= 0x80;
        }
        
        if payload_len <= 125 {
            second_byte |= payload_len as u8;
            frame.push(second_byte);
        } else if payload_len <= 65535 {
            second_byte |= 126;
            frame.push(second_byte);
            
            // 16-bit length
            frame.extend_from_slice(&(payload_len as u16).to_be_bytes());
        } else {
            second_byte |= 127;
            frame.push(second_byte);
            
            // 64-bit length
            frame.extend_from_slice(&(payload_len as u64).to_be_bytes());
        }
        
        // Mask
        if let Some(mask) = self.mask {
            frame.extend_from_slice(&mask);
            
            // Masked payload
            let mut masked_payload = self.payload.clone();
            for i in 0..payload_len {
                masked_payload[i] ^= mask[i % 4];
            }
            
            frame.extend_from_slice(&masked_payload);
        } else {
            // Unmasked payload
            frame.extend_from_slice(&self.payload);
        }
        
        Ok(frame)
    }
    
    /// Decode a message from bytes
    pub fn decode(data: &[u8]) -> Result<(Self, usize)> {
        if data.len() < 2 {
            return Err(anyhow!("Incomplete WebSocket frame: too short"));
        }
        
        // First byte: FIN and opcode
        let first_byte = data[0];
        let fin = (first_byte & 0x80) != 0;
        let opcode = WebSocketOpcode::try_from(first_byte & 0x0F)?;
        
        // Second byte: MASK and 7-bit payload length
        let second_byte = data[1];
        let masked = (second_byte & 0x80) != 0;
        let mut payload_len = (second_byte & 0x7F) as usize;
        
        // Read extended payload length
        let mut header_len = 2;
        
        if payload_len == 126 {
            if data.len() < 4 {
                return Err(anyhow!("Incomplete WebSocket frame: too short for 16-bit length"));
            }
            
            payload_len = u16::from_be_bytes([data[2], data[3]]) as usize;
            header_len = 4;
        } else if payload_len == 127 {
            if data.len() < 10 {
                return Err(anyhow!("Incomplete WebSocket frame: too short for 64-bit length"));
            }
            
            payload_len = u64::from_be_bytes([
                data[2], data[3], data[4], data[5],
                data[6], data[7], data[8], data[9],
            ]) as usize;
            header_len = 10;
        }
        
        // Read mask
        let mask = if masked {
            if data.len() < header_len + 4 {
                return Err(anyhow!("Incomplete WebSocket frame: too short for mask"));
            }
            
            let mask = [data[header_len], data[header_len + 1], data[header_len + 2], data[header_len + 3]];
            header_len += 4;
            Some(mask)
        } else {
            None
        };
        
        // Check if we have enough data for the payload
        if data.len() < header_len + payload_len {
            return Err(anyhow!("Incomplete WebSocket frame: too short for payload"));
        }
        
        // Read payload
        let mut payload = Vec::with_capacity(payload_len);
        payload.extend_from_slice(&data[header_len..header_len + payload_len]);
        
        // Unmask payload if necessary
        if let Some(mask) = mask {
            for i in 0..payload_len {
                payload[i] ^= mask[i % 4];
            }
        }
        
        Ok((
            Self {
                opcode,
                payload,
                fin,
                mask,
            },
            header_len + payload_len,
        ))
    }
}

/// WebSocket handler
pub struct WebSocketHandler;

impl WebSocketHandler {
    /// Create a new WebSocket handler
    pub fn new() -> Self {
        Self
    }
}

impl ProtocolHandler for WebSocketHandler {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::WebSocket
    }
    
    fn encode(&self, message: &[u8]) -> Result<Vec<u8>> {
        // Create a binary message
        let ws_message = WebSocketMessage::binary(message.to_vec());
        
        // Encode the message
        ws_message.encode()
    }
    
    fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Decode the message
        let (ws_message, _) = WebSocketMessage::decode(data)?;
        
        // Return the payload
        Ok(ws_message.payload)
    }
    
    fn is_message_complete(&self, data: &[u8]) -> bool {
        // Check if we have enough data for the header
        if data.len() < 2 {
            return false;
        }
        
        // Get the payload length
        let second_byte = data[1];
        let payload_len_indicator = second_byte & 0x7F;
        let masked = (second_byte & 0x80) != 0;
        
        // Calculate the header length
        let mut header_len = 2;
        
        if payload_len_indicator == 126 {
            header_len = 4;
        } else if payload_len_indicator == 127 {
            header_len = 10;
        }
        
        if masked {
            header_len += 4;
        }
        
        // Check if we have enough data for the header
        if data.len() < header_len {
            return false;
        }
        
        // Get the payload length
        let payload_len = if payload_len_indicator <= 125 {
            payload_len_indicator as usize
        } else if payload_len_indicator == 126 {
            u16::from_be_bytes([data[2], data[3]]) as usize
        } else {
            u64::from_be_bytes([
                data[2], data[3], data[4], data[5],
                data[6], data[7], data[8], data[9],
            ]) as usize
        };
        
        // Check if we have enough data for the payload
        data.len() >= header_len + payload_len
    }
    
    fn get_message_size(&self, data: &[u8]) -> Result<usize> {
        // Check if we have enough data for the header
        if data.len() < 2 {
            return Err(anyhow!("Incomplete WebSocket frame: too short"));
        }
        
        // Get the payload length
        let second_byte = data[1];
        let payload_len_indicator = second_byte & 0x7F;
        let masked = (second_byte & 0x80) != 0;
        
        // Calculate the header length
        let mut header_len = 2;
        
        if payload_len_indicator == 126 {
            if data.len() < 4 {
                return Err(anyhow!("Incomplete WebSocket frame: too short for 16-bit length"));
            }
            
            header_len = 4;
        } else if payload_len_indicator == 127 {
            if data.len() < 10 {
                return Err(anyhow!("Incomplete WebSocket frame: too short for 64-bit length"));
            }
            
            header_len = 10;
        }
        
        if masked {
            header_len += 4;
        }
        
        // Get the payload length
        let payload_len = if payload_len_indicator <= 125 {
            payload_len_indicator as usize
        } else if payload_len_indicator == 126 {
            u16::from_be_bytes([data[2], data[3]]) as usize
        } else {
            u64::from_be_bytes([
                data[2], data[3], data[4], data[5],
                data[6], data[7], data[8], data[9],
            ]) as usize
        };
        
        Ok(header_len + payload_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_opcode() {
        assert_eq!(WebSocketOpcode::try_from(0x0).unwrap(), WebSocketOpcode::Continuation);
        assert_eq!(WebSocketOpcode::try_from(0x1).unwrap(), WebSocketOpcode::Text);
        assert_eq!(WebSocketOpcode::try_from(0x2).unwrap(), WebSocketOpcode::Binary);
        assert_eq!(WebSocketOpcode::try_from(0x8).unwrap(), WebSocketOpcode::Close);
        assert_eq!(WebSocketOpcode::try_from(0x9).unwrap(), WebSocketOpcode::Ping);
        assert_eq!(WebSocketOpcode::try_from(0xA).unwrap(), WebSocketOpcode::Pong);
        assert!(WebSocketOpcode::try_from(0x3).is_err());
    }

    #[test]
    fn test_websocket_message_encode_decode() {
        // Test small payload
        let message = WebSocketMessage::binary(vec![1, 2, 3, 4, 5]);
        let encoded = message.encode().unwrap();
        let (decoded, size) = WebSocketMessage::decode(&encoded).unwrap();
        
        assert_eq!(decoded.opcode, message.opcode);
        assert_eq!(decoded.payload, message.payload);
        assert_eq!(decoded.fin, message.fin);
        assert_eq!(decoded.mask, message.mask);
        assert_eq!(size, encoded.len());
        
        // Test medium payload
        let payload = vec![0; 1000];
        let message = WebSocketMessage::binary(payload);
        let encoded = message.encode().unwrap();
        let (decoded, size) = WebSocketMessage::decode(&encoded).unwrap();
        
        assert_eq!(decoded.opcode, message.opcode);
        assert_eq!(decoded.payload, message.payload);
        assert_eq!(decoded.fin, message.fin);
        assert_eq!(decoded.mask, message.mask);
        assert_eq!(size, encoded.len());
        
        // Test large payload
        let payload = vec![0; 70000];
        let message = WebSocketMessage::binary(payload);
        let encoded = message.encode().unwrap();
        let (decoded, size) = WebSocketMessage::decode(&encoded).unwrap();
        
        assert_eq!(decoded.opcode, message.opcode);
        assert_eq!(decoded.payload, message.payload);
        assert_eq!(decoded.fin, message.fin);
        assert_eq!(decoded.mask, message.mask);
        assert_eq!(size, encoded.len());
        
        // Test masked payload
        let message = WebSocketMessage::new(WebSocketOpcode::Text, vec![1, 2, 3, 4, 5], true, Some([0x11, 0x22, 0x33, 0x44]));
        let encoded = message.encode().unwrap();
        let (decoded, size) = WebSocketMessage::decode(&encoded).unwrap();
        
        assert_eq!(decoded.opcode, message.opcode);
        assert_eq!(decoded.payload, message.payload);
        assert_eq!(decoded.fin, message.fin);
        assert_eq!(size, encoded.len());
    }

    #[test]
    fn test_websocket_handler() {
        let handler = WebSocketHandler::new();
        
        // Test protocol type
        assert_eq!(handler.protocol_type(), ProtocolType::WebSocket);
        
        // Test encode/decode
        let message = b"Hello, world!";
        let encoded = handler.encode(message).unwrap();
        let decoded = handler.decode(&encoded).unwrap();
        
        assert_eq!(decoded, message);
        
        // Test is_message_complete
        assert!(handler.is_message_complete(&encoded));
        assert!(!handler.is_message_complete(&encoded[..encoded.len() - 1]));
        
        // Test get_message_size
        assert_eq!(handler.get_message_size(&encoded).unwrap(), encoded.len());
        assert!(handler.get_message_size(&encoded[..encoded.len() - 1]).is_err());
    }
}