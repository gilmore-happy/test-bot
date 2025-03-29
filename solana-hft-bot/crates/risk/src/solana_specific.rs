//! Solana-specific risk factors for the Solana HFT Bot
//!
//! This module provides risk assessment specific to Solana's architecture and ecosystem.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use tracing::{debug, info, warn};

/// Solana-specific risk factor types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SolanaRiskFactorType {
    /// Transaction prioritization fee risk
    PrioritizationFee,
    
    /// Validator stake concentration risk
    ValidatorConcentration,
    
    /// Program upgrade risk
    ProgramUpgrade,
    
    /// Token account ownership risk
    TokenAccountOwnership,
    
    /// Instruction compute limit risk
    ComputeLimit,
    
    /// Account data size limit risk
    AccountDataSize,
    
    /// Transaction size limit risk
    TransactionSize,
    
    /// Slot processing time risk
    SlotProcessingTime,
    
    /// Vote account risk
    VoteAccount,
    
    /// Rent exemption risk
    RentExemption,
    
    /// Token program version risk
    TokenProgramVersion,
}

/// Solana-specific risk factor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaRiskFactor {
    /// Risk factor type
    pub factor_type: SolanaRiskFactorType,
    
    /// Risk factor name
    pub name: String,
    
    /// Risk factor description
    pub description: String,
    
    /// Severity (0-100)
    pub severity: u8,
    
    /// Mitigation strategy
    pub mitigation: String,
    
    /// Additional data
    pub additional_data: Option<serde_json::Value>,
}

/// Solana network status for risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaNetworkStatus {
    /// Current slot
    pub current_slot: u64,
    
    /// Average slot processing time (ms)
    pub avg_slot_time_ms: u64,
    
    /// Average transaction confirmation time (ms)
    pub avg_confirmation_time_ms: u64,
    
    /// Current transaction count per second
    pub transactions_per_second: f64,
    
    /// Average prioritization fee (micro-lamports)
    pub avg_prioritization_fee: u64,
    
    /// Network congestion level (0-100)
    pub congestion_level: u8,
    
    /// Validator stake concentration (Gini coefficient)
    pub validator_concentration: f64,
    
    /// Recent program upgrades
    pub recent_program_upgrades: Vec<ProgramUpgradeInfo>,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Program upgrade information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramUpgradeInfo {
    /// Program ID
    pub program_id: String,
    
    /// Upgrade slot
    pub upgrade_slot: u64,
    
    /// Upgrade timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Solana-specific risk assessor
pub struct SolanaRiskAssessor {
    /// RPC client
    rpc_client: Arc<RpcClient>,
    
    /// Network status cache
    network_status: parking_lot::RwLock<Option<SolanaNetworkStatus>>,
    
    /// Last update time
    last_update: parking_lot::RwLock<Instant>,
    
    /// Update interval
    update_interval: Duration,
    
    /// Known program IDs with their names
    known_programs: HashMap<String, String>,
}

impl SolanaRiskAssessor {
    /// Create a new Solana risk assessor
    pub fn new(rpc_client: Arc<RpcClient>, update_interval: Duration) -> Self {
        let mut known_programs = HashMap::new();
        
        // Add well-known Solana program IDs
        known_programs.insert("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(), "Token Program".to_string());
        known_programs.insert("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL".to_string(), "Associated Token Account Program".to_string());
        known_programs.insert("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s".to_string(), "Metaplex Token Metadata Program".to_string());
        known_programs.insert("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(), "Jupiter Aggregator v6".to_string());
        known_programs.insert("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin".to_string(), "Serum DEX v3".to_string());
        known_programs.insert("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX".to_string(), "OpenBook DEX".to_string());
        known_programs.insert("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string(), "Orca Whirlpools".to_string());
        
        Self {
            rpc_client,
            network_status: parking_lot::RwLock::new(None),
            last_update: parking_lot::RwLock::new(Instant::now() - Duration::from_secs(3600)), // Force initial update
            update_interval,
            known_programs,
        }
    }
    
    /// Update network status
    pub async fn update_network_status(&self) -> Result<()> {
        let now = Instant::now();
        let last_update = *self.last_update.read();
        
        if now.duration_since(last_update) < self.update_interval {
            return Ok(());
        }
        
        info!("Updating Solana network status");
        
        // Get current slot
        let current_slot = self.rpc_client.get_slot().await?;
        
        // Get recent performance samples
        let performance_samples = self.rpc_client.get_recent_performance_samples(Some(10)).await?;
        
        // Calculate average slot time
        let avg_slot_time_ms = if !performance_samples.is_empty() {
            let total_time: u64 = performance_samples.iter()
                .map(|sample| sample.num_slots * sample.sample_period_secs * 1000 / sample.num_slots.max(1))
                .sum();
            total_time / performance_samples.len() as u64
        } else {
            500 // Default 500ms if no data
        };
        
        // Calculate TPS
        let transactions_per_second = if !performance_samples.is_empty() {
            let total_transactions: u64 = performance_samples.iter()
                .map(|sample| sample.num_transactions)
                .sum();
            let total_time: f64 = performance_samples.iter()
                .map(|sample| sample.sample_period_secs as f64)
                .sum();
            
            if total_time > 0.0 {
                total_transactions as f64 / total_time
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        // Estimate confirmation time based on slot time and congestion
        let avg_confirmation_time_ms = avg_slot_time_ms * 32 / 10; // Assuming confirmation after ~3.2 slots
        
        // Get recent prioritization fees
        // This is a simplified approach - in a real implementation, you would analyze recent blocks
        let avg_prioritization_fee = 1000; // Placeholder value in micro-lamports
        
        // Calculate congestion level based on TPS
        // Solana can handle ~50k TPS theoretically, but practical limits are lower
        let congestion_level = (transactions_per_second / 5000.0 * 100.0).min(100.0) as u8;
        
        // Get validator list to calculate stake concentration
        let validator_list = self.rpc_client.get_vote_accounts().await?;
        let validator_concentration = Self::calculate_stake_concentration(&validator_list.current);
        
        // Check for recent program upgrades (simplified)
        let recent_program_upgrades = Vec::new(); // In a real implementation, you would track program upgrades
        
        let network_status = SolanaNetworkStatus {
            current_slot,
            avg_slot_time_ms,
            avg_confirmation_time_ms,
            transactions_per_second,
            avg_prioritization_fee,
            congestion_level,
            validator_concentration,
            recent_program_upgrades,
            timestamp: chrono::Utc::now(),
        };
        
        // Update cache
        *self.network_status.write() = Some(network_status);
        *self.last_update.write() = now;
        
        Ok(())
    }
    
    /// Calculate stake concentration using Gini coefficient
    fn calculate_stake_concentration(validators: &[solana_client::rpc_response::RpcVoteAccountInfo]) -> f64 {
        if validators.is_empty() {
            return 0.0;
        }
        
        // Extract stake amounts
        let mut stakes: Vec<f64> = validators.iter()
            .map(|v| v.activated_stake as f64)
            .collect();
        
        // Sort stakes
        stakes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        // Calculate Gini coefficient
        let n = stakes.len() as f64;
        let sum_stakes: f64 = stakes.iter().sum();
        
        if sum_stakes == 0.0 {
            return 0.0;
        }
        
        let mut sum = 0.0;
        for (i, &stake) in stakes.iter().enumerate() {
            sum += (2.0 * (i as f64) - n + 1.0) * stake;
        }
        
        sum / (n * n * sum_stakes / 2.0)
    }
    
    /// Assess transaction prioritization fee risk
    pub async fn assess_prioritization_fee_risk(&self, proposed_fee: Option<u64>) -> Result<SolanaRiskFactor> {
        self.update_network_status().await?;
        
        let network_status = self.network_status.read().clone()
            .ok_or_else(|| anyhow!("Network status not available"))?;
        
        let avg_fee = network_status.avg_prioritization_fee;
        let proposed_fee = proposed_fee.unwrap_or(0);
        
        let (severity, description, mitigation) = if proposed_fee == 0 {
            (
                80,
                format!("No prioritization fee specified while network average is {} micro-lamports", avg_fee),
                "Add a prioritization fee to increase transaction priority".to_string()
            )
        } else if proposed_fee < avg_fee / 2 {
            (
                60,
                format!("Prioritization fee ({} micro-lamports) is less than half the network average ({} micro-lamports)", proposed_fee, avg_fee),
                "Consider increasing the prioritization fee".to_string()
            )
        } else if proposed_fee < avg_fee {
            (
                30,
                format!("Prioritization fee ({} micro-lamports) is below the network average ({} micro-lamports)", proposed_fee, avg_fee),
                "Consider slightly increasing the prioritization fee".to_string()
            )
        } else {
            (
                10,
                format!("Prioritization fee ({} micro-lamports) is at or above the network average ({} micro-lamports)", proposed_fee, avg_fee),
                "Current fee is adequate".to_string()
            )
        };
        
        // Adjust severity based on network congestion
        let adjusted_severity = (severity as f64 * (0.5 + network_status.congestion_level as f64 / 200.0)) as u8;
        
        Ok(SolanaRiskFactor {
            factor_type: SolanaRiskFactorType::PrioritizationFee,
            name: "Transaction Prioritization Fee Risk".to_string(),
            description,
            severity: adjusted_severity,
            mitigation,
            additional_data: Some(serde_json::json!({
                "network_avg_fee": avg_fee,
                "proposed_fee": proposed_fee,
                "congestion_level": network_status.congestion_level
            })),
        })
    }
    
    /// Assess compute limit risk
    pub fn assess_compute_limit_risk(&self, estimated_cu: u32) -> SolanaRiskFactor {
        const MAX_COMPUTE_UNITS: u32 = 1_400_000;
        
        let (severity, description, mitigation) = if estimated_cu > MAX_COMPUTE_UNITS {
            (
                100,
                format!("Estimated compute units ({}) exceed maximum allowed ({})", estimated_cu, MAX_COMPUTE_UNITS),
                "Split transaction into multiple smaller transactions".to_string()
            )
        } else if estimated_cu > MAX_COMPUTE_UNITS * 9 / 10 {
            (
                80,
                format!("Estimated compute units ({}) are very close to maximum allowed ({})", estimated_cu, MAX_COMPUTE_UNITS),
                "Consider optimizing transaction or splitting it".to_string()
            )
        } else if estimated_cu > MAX_COMPUTE_UNITS * 8 / 10 {
            (
                60,
                format!("Estimated compute units ({}) are approaching maximum allowed ({})", estimated_cu, MAX_COMPUTE_UNITS),
                "Monitor compute usage and consider optimizations".to_string()
            )
        } else if estimated_cu > MAX_COMPUTE_UNITS * 6 / 10 {
            (
                30,
                format!("Estimated compute units ({}) are moderately high", estimated_cu),
                "No immediate action required, but monitor usage".to_string()
            )
        } else {
            (
                10,
                format!("Estimated compute units ({}) are within safe limits", estimated_cu),
                "No action required".to_string()
            )
        };
        
        SolanaRiskFactor {
            factor_type: SolanaRiskFactorType::ComputeLimit,
            name: "Compute Unit Limit Risk".to_string(),
            description,
            severity,
            mitigation,
            additional_data: Some(serde_json::json!({
                "estimated_cu": estimated_cu,
                "max_cu": MAX_COMPUTE_UNITS,
                "percentage": (estimated_cu as f64 / MAX_COMPUTE_UNITS as f64 * 100.0)
            })),
        }
    }
    
    /// Assess transaction size limit risk
    pub fn assess_transaction_size_risk(&self, estimated_size: usize) -> SolanaRiskFactor {
        const MAX_TRANSACTION_SIZE: usize = 1232;
        
        let (severity, description, mitigation) = if estimated_size > MAX_TRANSACTION_SIZE {
            (
                100,
                format!("Estimated transaction size ({} bytes) exceeds maximum allowed ({} bytes)", estimated_size, MAX_TRANSACTION_SIZE),
                "Split transaction into multiple smaller transactions".to_string()
            )
        } else if estimated_size > MAX_TRANSACTION_SIZE * 9 / 10 {
            (
                80,
                format!("Estimated transaction size ({} bytes) is very close to maximum allowed ({} bytes)", estimated_size, MAX_TRANSACTION_SIZE),
                "Consider reducing transaction size".to_string()
            )
        } else if estimated_size > MAX_TRANSACTION_SIZE * 8 / 10 {
            (
                50,
                format!("Estimated transaction size ({} bytes) is approaching maximum allowed ({} bytes)", estimated_size, MAX_TRANSACTION_SIZE),
                "Monitor transaction size".to_string()
            )
        } else {
            (
                10,
                format!("Estimated transaction size ({} bytes) is within safe limits", estimated_size),
                "No action required".to_string()
            )
        };
        
        SolanaRiskFactor {
            factor_type: SolanaRiskFactorType::TransactionSize,
            name: "Transaction Size Limit Risk".to_string(),
            description,
            severity,
            mitigation,
            additional_data: Some(serde_json::json!({
                "estimated_size": estimated_size,
                "max_size": MAX_TRANSACTION_SIZE,
                "percentage": (estimated_size as f64 / MAX_TRANSACTION_SIZE as f64 * 100.0)
            })),
        }
    }
    
    /// Assess validator concentration risk
    pub async fn assess_validator_concentration_risk(&self) -> Result<SolanaRiskFactor> {
        self.update_network_status().await?;
        
        let network_status = self.network_status.read().clone()
            .ok_or_else(|| anyhow!("Network status not available"))?;
        
        let concentration = network_status.validator_concentration;
        
        let (severity, description, mitigation) = if concentration > 0.7 {
            (
                90,
                format!("Extremely high validator stake concentration (Gini: {:.2})", concentration),
                "Extreme caution advised due to centralization risk".to_string()
            )
        } else if concentration > 0.6 {
            (
                70,
                format!("Very high validator stake concentration (Gini: {:.2})", concentration),
                "Be cautious of potential centralization issues".to_string()
            )
        } else if concentration > 0.5 {
            (
                50,
                format!("High validator stake concentration (Gini: {:.2})", concentration),
                "Monitor network decentralization".to_string()
            )
        } else if concentration > 0.4 {
            (
                30,
                format!("Moderate validator stake concentration (Gini: {:.2})", concentration),
                "No immediate action required".to_string()
            )
        } else {
            (
                10,
                format!("Low validator stake concentration (Gini: {:.2})", concentration),
                "Network appears well decentralized".to_string()
            )
        };
        
        Ok(SolanaRiskFactor {
            factor_type: SolanaRiskFactorType::ValidatorConcentration,
            name: "Validator Stake Concentration Risk".to_string(),
            description,
            severity,
            mitigation,
            additional_data: Some(serde_json::json!({
                "gini_coefficient": concentration
            })),
        })
    }
    
    /// Assess program upgrade risk
    pub async fn assess_program_upgrade_risk(&self, program_ids: &[Pubkey]) -> Result<Vec<SolanaRiskFactor>> {
        self.update_network_status().await?;
        
        let network_status = self.network_status.read().clone()
            .ok_or_else(|| anyhow!("Network status not available"))?;
        
        let mut risk_factors = Vec::new();
        
        for program_id in program_ids {
            let program_id_str = program_id.to_string();
            let program_name = self.known_programs.get(&program_id_str)
                .cloned()
                .unwrap_or_else(|| "Unknown Program".to_string());
            
            // Check if program has been recently upgraded
            let recent_upgrade = network_status.recent_program_upgrades.iter()
                .find(|upgrade| upgrade.program_id == program_id_str);
            
            if let Some(upgrade) = recent_upgrade {
                let elapsed = chrono::Utc::now().signed_duration_since(upgrade.timestamp);
                
                if elapsed < chrono::Duration::hours(24) {
                    // Program upgraded in the last 24 hours
                    risk_factors.push(SolanaRiskFactor {
                        factor_type: SolanaRiskFactorType::ProgramUpgrade,
                        name: format!("{} Upgrade Risk", program_name),
                        description: format!("{} was upgraded very recently ({})", program_name, upgrade.timestamp),
                        severity: 80,
                        mitigation: "Consider delaying transactions using this program".to_string(),
                        additional_data: Some(serde_json::json!({
                            "program_id": program_id_str,
                            "program_name": program_name,
                            "upgrade_time": upgrade.timestamp,
                            "hours_since_upgrade": elapsed.num_hours()
                        })),
                    });
                } else if elapsed < chrono::Duration::hours(72) {
                    // Program upgraded in the last 72 hours
                    risk_factors.push(SolanaRiskFactor {
                        factor_type: SolanaRiskFactorType::ProgramUpgrade,
                        name: format!("{} Upgrade Risk", program_name),
                        description: format!("{} was upgraded recently ({})", program_name, upgrade.timestamp),
                        severity: 50,
                        mitigation: "Monitor transactions using this program carefully".to_string(),
                        additional_data: Some(serde_json::json!({
                            "program_id": program_id_str,
                            "program_name": program_name,
                            "upgrade_time": upgrade.timestamp,
                            "hours_since_upgrade": elapsed.num_hours()
                        })),
                    });
                }
            }
            
            // For unknown programs, add a general risk factor
            if !self.known_programs.contains_key(&program_id_str) {
                risk_factors.push(SolanaRiskFactor {
                    factor_type: SolanaRiskFactorType::ProgramUpgrade,
                    name: "Unknown Program Risk".to_string(),
                    description: format!("Transaction uses unknown program {}", program_id_str),
                    severity: 40,
                    mitigation: "Verify the program's legitimacy before proceeding".to_string(),
                    additional_data: Some(serde_json::json!({
                        "program_id": program_id_str
                    })),
                });
            }
        }
        
        Ok(risk_factors)
    }
    
    /// Assess network congestion risk
    pub async fn assess_network_congestion_risk(&self) -> Result<SolanaRiskFactor> {
        self.update_network_status().await?;
        
        let network_status = self.network_status.read().clone()
            .ok_or_else(|| anyhow!("Network status not available"))?;
        
        let congestion = network_status.congestion_level;
        
        let (severity, description, mitigation) = if congestion > 90 {
            (
                90,
                format!("Extreme network congestion ({}%)", congestion),
                "Consider delaying non-critical transactions".to_string()
            )
        } else if congestion > 75 {
            (
                70,
                format!("High network congestion ({}%)", congestion),
                "Use higher prioritization fees and monitor confirmations".to_string()
            )
        } else if congestion > 50 {
            (
                50,
                format!("Moderate network congestion ({}%)", congestion),
                "Consider using prioritization fees".to_string()
            )
        } else if congestion > 25 {
            (
                30,
                format!("Light network congestion ({}%)", congestion),
                "No immediate action required".to_string()
            )
        } else {
            (
                10,
                format!("Low network congestion ({}%)", congestion),
                "Network is operating normally".to_string()
            )
        };
        
        Ok(SolanaRiskFactor {
            factor_type: SolanaRiskFactorType::SlotProcessingTime,
            name: "Network Congestion Risk".to_string(),
            description,
            severity,
            mitigation,
            additional_data: Some(serde_json::json!({
                "congestion_level": congestion,
                "tps": network_status.transactions_per_second,
                "avg_confirmation_time_ms": network_status.avg_confirmation_time_ms
            })),
        })
    }
    
    /// Assess all Solana-specific risks for a transaction
    pub async fn assess_transaction_risks(
        &self,
        program_ids: &[Pubkey],
        estimated_cu: u32,
        estimated_size: usize,
        prioritization_fee: Option<u64>,
    ) -> Result<Vec<SolanaRiskFactor>> {
        let mut risk_factors = Vec::new();
        
        // Assess compute limit risk
        risk_factors.push(self.assess_compute_limit_risk(estimated_cu));
        
        // Assess transaction size risk
        risk_factors.push(self.assess_transaction_size_risk(estimated_size));
        
        // Assess prioritization fee risk
        risk_factors.push(self.assess_prioritization_fee_risk(prioritization_fee).await?);
        
        // Assess network congestion risk
        risk_factors.push(self.assess_network_congestion_risk().await?);
        
        // Assess program upgrade risks
        let program_risks = self.assess_program_upgrade_risk(program_ids).await?;
        risk_factors.extend(program_risks);
        
        // Sort by severity (highest first)
        risk_factors.sort_by(|a, b| b.severity.cmp(&a.severity));
        
        Ok(risk_factors)
    }
    
    /// Get current network status
    pub async fn get_network_status(&self) -> Result<SolanaNetworkStatus> {
        self.update_network_status().await?;
        
        self.network_status.read().clone()
            .ok_or_else(|| anyhow!("Network status not available"))
    }
    
    /// Register a known program
    pub fn register_known_program(&mut self, program_id: &str, name: &str) {
        self.known_programs.insert(program_id.to_string(), name.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_client::rpc_client::RpcClient;
    
    #[test]
    fn test_compute_limit_risk() {
        let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
        let assessor = SolanaRiskAssessor::new(rpc_client, Duration::from_secs(60));
        
        // Test with different compute unit values
        let low_risk = assessor.assess_compute_limit_risk(500_000);
        let medium_risk = assessor.assess_compute_limit_risk(1_000_000);
        let high_risk = assessor.assess_compute_limit_risk(1_300_000);
        let extreme_risk = assessor.assess_compute_limit_risk(1_500_000);
        
        assert!(low_risk.severity < medium_risk.severity);
        assert!(medium_risk.severity < high_risk.severity);
        assert!(high_risk.severity < extreme_risk.severity);
        assert_eq!(extreme_risk.severity, 100);
    }
    
    #[test]
    fn test_transaction_size_risk() {
        let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
        let assessor = SolanaRiskAssessor::new(rpc_client, Duration::from_secs(60));
        
        // Test with different transaction sizes
        let low_risk = assessor.assess_transaction_size_risk(500);
        let medium_risk = assessor.assess_transaction_size_risk(1000);
        let high_risk = assessor.assess_transaction_size_risk(1150);
        let extreme_risk = assessor.assess_transaction_size_risk(1300);
        
        assert!(low_risk.severity < medium_risk.severity);
        assert!(medium_risk.severity < high_risk.severity);
        assert!(high_risk.severity < extreme_risk.severity);
        assert_eq!(extreme_risk.severity, 100);
    }
    
    #[test]
    fn test_stake_concentration() {
        use solana_client::rpc_response::RpcVoteAccountInfo;
        
        // Create mock validators with different stake distributions
        
        // Equal distribution
        let equal_validators: Vec<RpcVoteAccountInfo> = (0..10)
            .map(|i| RpcVoteAccountInfo {
                vote_pubkey: format!("validator{}", i),
                node_pubkey: format!("node{}", i),
                activated_stake: 100,
                commission: 10,
                epoch_vote_account: true,
                epoch_credits: vec![],
                last_vote: 0,
                root_slot: None,
            })
            .collect();
        
        // Highly concentrated distribution
        let mut concentrated_validators = Vec::new();
        concentrated_validators.push(RpcVoteAccountInfo {
            vote_pubkey: "validator0".to_string(),
            node_pubkey: "node0".to_string(),
            activated_stake: 900,
            commission: 10,
            epoch_vote_account: true,
            epoch_credits: vec![],
            last_vote: 0,
            root_slot: None,
        });
        
        for i in 1..10 {
            concentrated_validators.push(RpcVoteAccountInfo {
                vote_pubkey: format!("validator{}", i),
                node_pubkey: format!("node{}", i),
                activated_stake: 10,
                commission: 10,
                epoch_vote_account: true,
                epoch_credits: vec![],
                last_vote: 0,
                root_slot: None,
            });
        }
        
        let equal_gini = SolanaRiskAssessor::calculate_stake_concentration(equal_validators.as_slice());
        let concentrated_gini = SolanaRiskAssessor::calculate_stake_concentration(concentrated_validators.as_slice());
        
        // Equal distribution should have low Gini coefficient
        assert!(equal_gini < 0.1);
        
        // Concentrated distribution should have high Gini coefficient
        assert!(concentrated_gini > 0.7);
    }
}