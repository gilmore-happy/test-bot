//! Transaction simulation
//!
//! This module provides transaction simulation capabilities to validate transactions
//! before submitting them to the network.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use solana_client::rpc_config::RpcSimulateTransactionConfig;
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    transaction::Transaction,
};
use solana_transaction_status::{
    TransactionStatus, UiTransactionStatusMeta, UiTransactionEncoding,
};
use tracing::{debug, error, info, trace, warn};

use crate::ExecutionError;

/// Simulation result
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Whether the simulation was successful
    pub success: bool,
    
    /// Error message if the simulation failed
    pub error: Option<String>,
    
    /// Logs from the simulation
    pub logs: Vec<String>,
    
    /// Accounts after the simulation
    pub accounts: HashMap<Pubkey, Account>,
    
    /// Units consumed
    pub units_consumed: u64,
    
    /// Fee paid
    pub fee: u64,
    
    /// Accounts written during simulation
    pub accounts_written: Vec<Pubkey>,
    
    /// Accounts read during simulation
    pub accounts_read: Vec<Pubkey>,
    
    /// Raw simulation response for advanced analysis
    pub raw_response: Option<serde_json::Value>,
}

/// Transaction simulator
pub struct TransactionSimulator {
    /// RPC client
    rpc_client: Arc<solana_hft_rpc::EnhancedRpcClient>,
    
    /// Commitment level
    commitment: CommitmentConfig,
    
    /// Whether to include accounts in the simulation result
    include_accounts: bool,
    
    /// Whether to replace the blockhash in the transaction
    replace_blockhash: bool,
    
    /// Whether to verify signatures during simulation
    verify_signatures: bool,
    
    /// Cache of recent simulation results
    simulation_cache: parking_lot::RwLock<lru::LruCache<[u8; 32], SimulationResult>>,
}

impl TransactionSimulator {
    /// Create a new transaction simulator
    pub fn new(
        rpc_client: Arc<solana_hft_rpc::EnhancedRpcClient>,
        commitment: CommitmentConfig,
        include_accounts: bool,
    ) -> Self {
        Self {
            rpc_client,
            commitment,
            include_accounts,
            replace_blockhash: true,
            verify_signatures: false,
            simulation_cache: parking_lot::RwLock::new(lru::LruCache::new(100)),
        }
    }
    
    /// Simulate a transaction
    pub async fn simulate_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<SimulationResult, ExecutionError> {
        debug!("Simulating transaction: {:?}", transaction);
        
        // Check cache first
        let tx_hash = {
            let mut hasher = blake3::Hasher::new();
            hasher.update(&bincode::serialize(transaction).map_err(|e| {
                ExecutionError::Simulation(format!("Failed to serialize transaction: {}", e))
            })?);
            hasher.finalize().into()
        };
        
        {
            let cache = self.simulation_cache.read();
            if let Some(result) = cache.get(&tx_hash) {
                debug!("Using cached simulation result");
                return Ok(result.clone());
            }
        }
        
        // Set up simulation config
        let config = RpcSimulateTransactionConfig {
            sig_verify: self.verify_signatures,
            replace_recent_blockhash: self.replace_blockhash,
            commitment: Some(self.commitment),
            encoding: Some(UiTransactionEncoding::Base64),
            accounts: if self.include_accounts {
                Some(solana_client::rpc_config::RpcAccountInfoConfig {
                    encoding: Some(UiTransactionEncoding::Base64),
                    commitment: Some(self.commitment),
                    data_slice: None,
                    min_context_slot: None,
                })
            } else {
                None
            },
            min_context_slot: None,
        };
        
        // Perform simulation
        let simulation_result = self.rpc_client.simulate_transaction_with_config(
            transaction,
            config,
        ).await.map_err(|e| {
            ExecutionError::Simulation(format!("RPC simulation error: {}", e))
        })?;
        
        // Extract simulation data
        let success = simulation_result.value.err.is_none();
        let error = simulation_result.value.err.map(|e| format!("{:?}", e));
        let logs = simulation_result.value.logs.unwrap_or_default();
        
        // Extract compute units consumed
        let units_consumed = extract_compute_units_from_logs(&logs).unwrap_or(0);
        
        // Extract fee
        let fee = simulation_result.value.units_consumed
            .map(|units| units as u64)
            .unwrap_or(0);
        
        // Extract accounts
        let mut accounts = HashMap::new();
        let mut accounts_written = Vec::new();
        let mut accounts_read = Vec::new();
        
        if let Some(accounts_data) = simulation_result.value.accounts {
            for (i, account_option) in accounts_data.iter().enumerate() {
                if let Some(account_data) = account_option {
                    if i < transaction.message.account_keys.len() {
                        let pubkey = transaction.message.account_keys[i];
                        
                        // Convert UI account to Account
                        if let Ok(account) = account_data.decode() {
                            accounts.insert(pubkey, account);
                            
                            // Determine if account was written or read
                            if account_data.executable {
                                accounts_read.push(pubkey);
                            } else {
                                accounts_written.push(pubkey);
                            }
                        }
                    }
                }
            }
        }
        
        // Create simulation result
        let result = SimulationResult {
            success,
            error,
            logs,
            accounts,
            units_consumed,
            fee,
            accounts_written,
            accounts_read,
            raw_response: Some(serde_json::to_value(&simulation_result).unwrap_or_default()),
        };
        
        // Cache the result
        {
            let mut cache = self.simulation_cache.write();
            cache.put(tx_hash, result.clone());
        }
        
        if success {
            debug!("Simulation successful: consumed {} units, fee: {}", units_consumed, fee);
            Ok(result)
        } else {
            let error_msg = error.clone().unwrap_or_else(|| "Unknown simulation error".to_string());
            warn!("Simulation failed: {}", error_msg);
            
            if result.logs.iter().any(|log| log.contains("InstructionError")) {
                debug!("Simulation logs: {:?}", result.logs);
            }
            
            Ok(result) // Return the result even if simulation failed
        }
    }
    
    /// Check if a transaction is valid
    pub async fn is_valid_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<bool, ExecutionError> {
        let result = self.simulate_transaction(transaction).await?;
        Ok(result.success)
    }
    
    /// Estimate the fee for a transaction
    pub async fn estimate_fee(
        &self,
        transaction: &Transaction,
    ) -> Result<u64, ExecutionError> {
        let result = self.simulate_transaction(transaction).await?;
        Ok(result.fee)
    }
    
    /// Estimate the compute units for a transaction
    pub async fn estimate_compute_units(
        &self,
        transaction: &Transaction,
    ) -> Result<u64, ExecutionError> {
        let result = self.simulate_transaction(transaction).await?;
        Ok(result.units_consumed)
    }
    
    /// Get the logs for a transaction
    pub async fn get_logs(
        &self,
        transaction: &Transaction,
    ) -> Result<Vec<String>, ExecutionError> {
        let result = self.simulate_transaction(transaction).await?;
        Ok(result.logs)
    }
    
    /// Analyze transaction logs to extract useful information
    pub fn analyze_logs(&self, logs: &[String]) -> HashMap<String, String> {
        analyze_logs(logs)
    }
    
    /// Clear the simulation cache
    pub fn clear_cache(&self) {
        let mut cache = self.simulation_cache.write();
        cache.clear();
    }
}

/// Simulate a transaction with a timeout
pub async fn simulate_transaction_with_timeout(
    simulator: &TransactionSimulator,
    transaction: &Transaction,
    timeout: Duration,
) -> Result<SimulationResult, ExecutionError> {
    match tokio::time::timeout(timeout, simulator.simulate_transaction(transaction)).await {
        Ok(result) => result,
        Err(_) => Err(ExecutionError::Timeout),
    }
}

/// Extract compute units consumed from logs
fn extract_compute_units_from_logs(logs: &[String]) -> Option<u64> {
    for log in logs {
        if log.contains("consumed") && log.contains("compute units") {
            let parts: Vec<&str> = log.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if *part == "consumed" && i + 1 < parts.len() {
                    if let Ok(units) = parts[i + 1].parse::<u64>() {
                        return Some(units);
                    }
                }
            }
        }
    }
    None
}

/// Analyze transaction logs to extract useful information
pub fn analyze_logs(logs: &[String]) -> HashMap<String, String> {
    let mut info = HashMap::new();
    
    for log in logs {
        // Extract program invocations
        if log.contains("Program ") && log.contains(" invoke [") {
            let parts: Vec<&str> = log.split(" invoke [").collect();
            if parts.len() >= 2 {
                let program = parts[0].trim_start_matches("Program ");
                info.insert(format!("program_{}", info.len()), program.to_string());
            }
        }
        
        // Extract specific information based on program
        if log.contains("Program log: ") {
            let msg = log.trim_start_matches("Program log: ");
            info.insert(format!("log_{}", info.len()), msg.to_string());
            
            // Extract token transfers
            if msg.contains("Transfer") {
                info.insert("has_transfer".to_string(), "true".to_string());
            }
            
            // Extract swap information
            if msg.contains("Swap") {
                info.insert("has_swap".to_string(), "true".to_string());
            }
            
            // Extract error information
            if msg.contains("Error") || msg.contains("error") || msg.contains("failed") || msg.contains("Failed") {
                info.insert("has_error".to_string(), "true".to_string());
                info.insert("error_message".to_string(), msg.to_string());
            }
        }
    }
    
    info
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::system_instruction;
    use solana_sdk::signature::Keypair;
    
    #[test]
    fn test_extract_compute_units() {
        let logs = vec![
            "Program 11111111111111111111111111111111 invoke [1]".to_string(),
            "Program log: Instruction: Transfer".to_string(),
            "Program 11111111111111111111111111111111 consumed 1234 of 200000 compute units".to_string(),
            "Program 11111111111111111111111111111111 success".to_string(),
        ];
        
        let units = extract_compute_units_from_logs(&logs);
        assert_eq!(units, Some(1234));
    }
    
    #[test]
    fn test_analyze_logs() {
        let logs = vec![
            "Program 11111111111111111111111111111111 invoke [1]".to_string(),
            "Program log: Instruction: Transfer".to_string(),
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]".to_string(),
            "Program log: Transfer 1000 tokens".to_string(),
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 3000 of 200000 compute units".to_string(),
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success".to_string(),
            "Program 11111111111111111111111111111111 consumed 1234 of 200000 compute units".to_string(),
            "Program 11111111111111111111111111111111 success".to_string(),
        ];
        
        let info = analyze_logs(&logs);
        
        assert_eq!(info.get("program_0"), Some(&"11111111111111111111111111111111".to_string()));
        assert_eq!(info.get("program_1"), Some(&"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string()));
        assert_eq!(info.get("has_transfer"), Some(&"true".to_string()));
    }
}
