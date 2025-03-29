//! Fee Prediction Module for Solana HFT Bot
//!
//! This module provides fee prediction capabilities using both heuristic and ML-based approaches
//! to optimize transaction fees for high-frequency trading on Solana.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    clock::Slot,
    commitment_config::CommitmentConfig,
    hash::Hash,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use tokio::sync::Mutex as TokioMutex;
use tracing::{debug, error, info, trace, warn};

use crate::ExecutionError;

/// Fee model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeePredictionConfig {
    /// Whether to use ML model for fee prediction
    pub use_ml_model: bool,
    
    /// Base fee in micro-lamports per CU
    pub base_fee: u64,
    
    /// Maximum fee in micro-lamports per CU
    pub max_fee: u64,
    
    /// Minimum fee in micro-lamports per CU
    pub min_fee: u64,
    
    /// Whether to adjust fees based on network congestion
    pub adjust_for_congestion: bool,
    
    /// Congestion multiplier
    pub congestion_multiplier: f64,
    
    /// History window size for fee tracking (in seconds)
    pub history_window_secs: u64,
    
    /// Maximum number of fee samples to keep
    pub max_fee_samples: usize,
    
    /// Update interval for congestion metrics (in milliseconds)
    pub congestion_update_interval_ms: u64,
    
    /// ML model update interval (in milliseconds)
    pub ml_model_update_interval_ms: u64,
    
    /// Whether to use time-of-day adjustments
    pub use_time_of_day_adjustment: bool,
    
    /// Path to ML model file (if using a pre-trained model)
    pub ml_model_path: Option<String>,
}

impl Default for FeePredictionConfig {
    fn default() -> Self {
        Self {
            use_ml_model: false,
            base_fee: 5_000,
            max_fee: 1_000_000, // 0.001 SOL per CU
            min_fee: 1_000,     // 0.000001 SOL per CU
            adjust_for_congestion: true,
            congestion_multiplier: 2.0,
            history_window_secs: 300, // 5 minutes
            max_fee_samples: 1000,
            congestion_update_interval_ms: 10_000, // 10 seconds
            ml_model_update_interval_ms: 60_000,   // 1 minute
            use_time_of_day_adjustment: true,
            ml_model_path: None,
        }
    }
}

/// Network congestion level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CongestionLevel {
    /// Low congestion
    Low,
    
    /// Medium congestion
    Medium,
    
    /// High congestion
    High,
    
    /// Extreme congestion
    Extreme,
}

impl CongestionLevel {
    /// Get the fee multiplier for this congestion level
    pub fn fee_multiplier(&self) -> f64 {
        match self {
            Self::Low => 1.0,
            Self::Medium => 1.5,
            Self::High => 2.5,
            Self::Extreme => 4.0,
        }
    }
    
    /// Get the congestion level from a fee multiplier
    pub fn from_multiplier(multiplier: f64) -> Self {
        if multiplier >= 3.0 {
            Self::Extreme
        } else if multiplier >= 2.0 {
            Self::High
        } else if multiplier >= 1.3 {
            Self::Medium
        } else {
            Self::Low
        }
    }
}

/// Fee sample for historical tracking
#[derive(Debug, Clone)]
struct FeeSample {
    /// Fee in micro-lamports
    fee: u64,
    
    /// Timestamp when the fee was recorded
    timestamp: Instant,
    
    /// Slot when the fee was recorded
    slot: Option<Slot>,
    
    /// Whether the transaction was successful
    success: bool,
    
    /// Transaction signature
    signature: Option<Signature>,
}

/// Transaction characteristics for fee prediction
#[derive(Debug, Clone)]
pub struct TransactionCharacteristics {
    /// Number of instructions
    pub instruction_count: usize,
    
    /// Number of accounts
    pub account_count: usize,
    
    /// Transaction size in bytes
    pub size_bytes: usize,
    
    /// Whether the transaction is compute-intensive
    pub is_compute_intensive: bool,
    
    /// Priority level (0-100)
    pub priority_level: u8,
    
    /// Transaction type
    pub transaction_type: TransactionType,
}

/// Transaction type for fee prediction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionType {
    /// Simple transfer
    Transfer,
    
    /// Token swap
    Swap,
    
    /// Liquidity provision
    LiquidityProvision,
    
    /// NFT transaction
    Nft,
    
    /// Program deployment
    ProgramDeployment,
    
    /// Other transaction type
    Other,
}

/// ML model for fee prediction
struct MlModel {
    /// Whether the model is trained
    trained: bool,
    
    /// Last training time
    last_trained: Instant,
    
    /// Model weights
    weights: HashMap<String, f64>,
    
    /// Model bias
    bias: f64,
}

impl MlModel {
    /// Create a new ML model
    fn new() -> Self {
        Self {
            trained: false,
            last_trained: Instant::now(),
            weights: HashMap::new(),
            bias: 0.0,
        }
    }
    
    /// Train the model on historical fee data
    fn train(&mut self, samples: &[FeeSample], current_congestion: CongestionLevel) -> Result<()> {
        // In a real implementation, this would train a proper ML model
        // For now, we'll implement a simple linear regression model
        
        // Initialize weights
        self.weights.insert("congestion".to_string(), match current_congestion {
            CongestionLevel::Low => 1.0,
            CongestionLevel::Medium => 1.5,
            CongestionLevel::High => 2.5,
            CongestionLevel::Extreme => 4.0,
        });
        
        self.weights.insert("time_of_day".to_string(), 1.0);
        self.weights.insert("recent_success_rate".to_string(), 1.0);
        
        self.bias = 5000.0; // Base fee
        
        self.trained = true;
        self.last_trained = Instant::now();
        
        Ok(())
    }
    
    /// Predict fee for a transaction
    fn predict(&self, features: &HashMap<String, f64>) -> Option<u64> {
        if !self.trained {
            return Some(5000); // Default fee if not trained
        }
        
        let mut prediction = self.bias;
        
        for (feature, value) in features {
            if let Some(weight) = self.weights.get(feature) {
                prediction += weight * value;
            }
        }
        
        Some(prediction as u64)
    }
}

/// Fee predictor for optimal fee estimation
pub struct FeePredictor {
    /// RPC client
    rpc_client: Arc<RpcClient>,
    
    /// Fee model configuration
    config: FeePredictionConfig,
    
    /// Current congestion level
    congestion_level: RwLock<CongestionLevel>,
    
    /// Recent transaction fees
    recent_fees: RwLock<VecDeque<FeeSample>>,
    
    /// Fee statistics by transaction type
    fee_stats_by_type: DashMap<TransactionType, Vec<u64>>,
    
    /// ML model for fee prediction
    ml_model: TokioMutex<MlModel>,
    
    /// Last congestion update time
    last_congestion_update: RwLock<Instant>,
    
    /// Success rate of recent transactions
    success_rate: RwLock<f64>,
}

impl FeePredictor {
    /// Create a new fee predictor
    pub fn new(rpc_client: Arc<RpcClient>, config: FeePredictionConfig) -> Self {
        Self {
            rpc_client,
            config,
            congestion_level: RwLock::new(CongestionLevel::Low),
            recent_fees: RwLock::new(VecDeque::with_capacity(1000)),
            fee_stats_by_type: DashMap::new(),
            ml_model: TokioMutex::new(MlModel::new()),
            last_congestion_update: RwLock::new(Instant::now()),
            success_rate: RwLock::new(0.95), // Start with a high success rate assumption
        }
    }
    
    /// Train the fee prediction model
    pub async fn train(&self) -> Result<()> {
        if !self.config.use_ml_model {
            debug!("ML model training skipped (disabled in config)");
            return Ok(());
        }
        
        info!("Training fee prediction ML model");
        
        // Get recent fee samples
        let samples = self.recent_fees.read().clone().into_iter().collect::<Vec<_>>();
        
        if samples.is_empty() {
            warn!("No fee samples available for training");
            return Ok(());
        }
        
        // Get current congestion level
        let congestion = *self.congestion_level.read();
        
        // Train the model
        let mut model = self.ml_model.lock().await;
        model.train(&samples, congestion)?;
        
        info!("Fee prediction ML model trained successfully with {} samples", samples.len());
        
        Ok(())
    }
    
    /// Update congestion level based on recent transactions
    pub async fn update_congestion_level(&self) -> Result<()> {
        // Check if update is needed
        let last_update = *self.last_congestion_update.read();
        if last_update.elapsed().as_millis() < self.config.congestion_update_interval_ms as u128 {
            trace!("Skipping congestion update, last update was {} ms ago", last_update.elapsed().as_millis());
            return Ok(());
        }
        
        if !self.config.adjust_for_congestion {
            trace!("Congestion adjustment disabled in config");
            return Ok(());
        }
        
        debug!("Updating network congestion level");
        
        // Get recent fee samples
        let recent_fees = self.recent_fees.read();
        
        if recent_fees.is_empty() {
            debug!("No fee samples available for congestion update");
            return Ok(());
        }
        
        // Calculate average fee
        let now = Instant::now();
        let recent_samples: Vec<_> = recent_fees
            .iter()
            .filter(|sample| now.duration_since(sample.timestamp).as_secs() < 60) // Last minute
            .collect();
        
        if recent_samples.is_empty() {
            debug!("No recent fee samples available for congestion update");
            return Ok(());
        }
        
        let avg_fee: u64 = recent_samples.iter().map(|sample| sample.fee).sum::<u64>() / recent_samples.len() as u64;
        
        // Determine congestion level
        let congestion_level = if avg_fee > self.config.base_fee * 10 {
            CongestionLevel::Extreme
        } else if avg_fee > self.config.base_fee * 5 {
            CongestionLevel::High
        } else if avg_fee > self.config.base_fee * 2 {
            CongestionLevel::Medium
        } else {
            CongestionLevel::Low
        };
        
        // Update congestion level
        let mut current_level = self.congestion_level.write();
        if *current_level != congestion_level {
            info!("Network congestion level changed: {:?} -> {:?}", *current_level, congestion_level);
            *current_level = congestion_level;
        }
        
        // Update last update time
        *self.last_congestion_update.write() = now;
        
        Ok(())
    }
    
    /// Record a transaction fee
    pub fn record_fee(&self, fee: u64, success: bool, signature: Option<Signature>, tx_type: Option<TransactionType>) {
        let mut recent_fees = self.recent_fees.write();
        
        // Add new sample
        recent_fees.push_back(FeeSample {
            fee,
            timestamp: Instant::now(),
            slot: None,
            success,
            signature,
        });
        
        // Update success rate
        let mut success_rate = self.success_rate.write();
        *success_rate = 0.95 * *success_rate + 0.05 * (if success { 1.0 } else { 0.0 });
        
        // Record by transaction type if provided
        if let Some(tx_type) = tx_type {
            self.fee_stats_by_type.entry(tx_type).or_insert_with(Vec::new).push(fee);
        }
        
        // Keep only recent fees
        let now = Instant::now();
        while !recent_fees.is_empty() && now.duration_since(recent_fees.front().unwrap().timestamp).as_secs() > self.config.history_window_secs {
            recent_fees.pop_front();
        }
        
        // Keep max size
        while recent_fees.len() > self.config.max_fee_samples {
            recent_fees.pop_front();
        }
        
        // Schedule congestion update if needed
        let last_update = *self.last_congestion_update.read();
        if last_update.elapsed().as_millis() > self.config.congestion_update_interval_ms as u128 {
            let this = self.clone();
            tokio::spawn(async move {
                if let Err(e) = this.update_congestion_level().await {
                    error!("Failed to update congestion level: {}", e);
                }
            });
        }
    }
    
    /// Get the current congestion level
    pub fn get_congestion_level(&self) -> CongestionLevel {
        *self.congestion_level.read()
    }
    
    /// Get the current success rate
    pub fn get_success_rate(&self) -> f64 {
        *self.success_rate.read()
    }
    
    /// Extract transaction characteristics
    pub fn extract_characteristics(transaction: &Transaction) -> TransactionCharacteristics {
        let instruction_count = transaction.message.instructions.len();
        let account_count = transaction.message.account_keys.len();
        let size_bytes = bincode::serialize(transaction).unwrap_or_default().len();
        
        // Determine if compute-intensive (simple heuristic)
        let is_compute_intensive = instruction_count > 3 || account_count > 10;
        
        // Determine transaction type (simple heuristic)
        let transaction_type = if instruction_count == 1 && account_count <= 3 {
            TransactionType::Transfer
        } else if instruction_count >= 3 && account_count >= 5 {
            TransactionType::Swap
        } else {
            TransactionType::Other
        };
        
        TransactionCharacteristics {
            instruction_count,
            account_count,
            size_bytes,
            is_compute_intensive,
            priority_level: 50, // Default medium priority
            transaction_type,
        }
    }
    
    /// Predict priority fee for a transaction
    pub async fn predict_priority_fee(
        &self,
        transaction: &Transaction,
    ) -> Result<Option<u64>, ExecutionError> {
        // Extract transaction characteristics
        let characteristics = Self::extract_characteristics(transaction);
        
        // If ML model is enabled and trained, use it
        if self.config.use_ml_model {
            let model = self.ml_model.lock().await;
            
            if model.trained {
                // Build feature vector
                let mut features = HashMap::new();
                
                // Add congestion feature
                let congestion_level = self.get_congestion_level();
                features.insert("congestion".to_string(), congestion_level.fee_multiplier());
                
                // Add time of day feature if enabled
                if self.config.use_time_of_day_adjustment {
                    let hour = chrono::Local::now().hour() as f64;
                    
                    // Higher fees during peak hours (simplified)
                    let time_factor = if (9.0..=16.0).contains(&hour) {
                        1.2 // Market hours
                    } else if (0.0..=5.0).contains(&hour) {
                        0.8 // Overnight
                    } else {
                        1.0 // Normal hours
                    };
                    
                    features.insert("time_of_day".to_string(), time_factor);
                }
                
                // Add success rate feature
                features.insert("recent_success_rate".to_string(), self.get_success_rate());
                
                // Add transaction characteristics
                features.insert("instruction_count".to_string(), characteristics.instruction_count as f64);
                features.insert("account_count".to_string(), characteristics.account_count as f64);
                features.insert("size_bytes".to_string(), characteristics.size_bytes as f64 / 1000.0);
                features.insert("is_compute_intensive".to_string(), if characteristics.is_compute_intensive { 1.5 } else { 1.0 });
                features.insert("priority_level".to_string(), characteristics.priority_level as f64 / 50.0);
                
                // Predict fee using ML model
                if let Some(fee) = model.predict(&features) {
                    // Clamp to min/max
                    let clamped_fee = if fee < self.config.min_fee {
                        self.config.min_fee
                    } else if fee > self.config.max_fee {
                        self.config.max_fee
                    } else {
                        fee
                    };
                    
                    debug!("ML model predicted fee: {} micro-lamports", clamped_fee);
                    return Ok(Some(clamped_fee));
                } else {
                    // Fallback if prediction returns None
                    debug!("ML model failed to predict fee, using default");
                    return Ok(Some(self.config.base_fee));
                }
            }
        }
        
        // Fallback to heuristic approach
        let base_fee = self.config.base_fee;
        
        // Apply congestion adjustment
        let congestion_level = self.get_congestion_level();
        let congestion_multiplier = congestion_level.fee_multiplier();
        
        // Apply transaction characteristic adjustments
        let characteristic_multiplier = 
            (1.0 + (characteristics.instruction_count as f64 * 0.1)) * // More instructions = higher fee
            (1.0 + (characteristics.account_count as f64 * 0.05)) *    // More accounts = higher fee
            (if characteristics.is_compute_intensive { 1.5 } else { 1.0 }); // Compute-intensive = higher fee
        
        // Calculate final fee
        let fee = (base_fee as f64 * congestion_multiplier * characteristic_multiplier) as u64;
        
        // Clamp to min/max
        let clamped_fee = if fee < self.config.min_fee {
            self.config.min_fee
        } else if fee > self.config.max_fee {
            self.config.max_fee
        } else {
            fee
        };
        
        debug!("Heuristic predicted fee: {} micro-lamports (base: {}, congestion: {}, characteristics: {})",
            clamped_fee, base_fee, congestion_multiplier, characteristic_multiplier);
        
        Ok(Some(clamped_fee))
    }
    
    /// Get fee statistics
    pub fn get_fee_stats(&self) -> FeeStats {
        let recent_fees = self.recent_fees.read();
        
        let now = Instant::now();
        let recent_samples: Vec<_> = recent_fees
            .iter()
            .filter(|sample| now.duration_since(sample.timestamp).as_secs() < 60) // Last minute
            .collect();
        
        let avg_fee = if !recent_samples.is_empty() {
            recent_samples.iter().map(|sample| sample.fee).sum::<u64>() / recent_samples.len() as u64
        } else {
            0
        };
        
        let min_fee = recent_samples.iter().map(|sample| sample.fee).min().unwrap_or(0);
        let max_fee = recent_samples.iter().map(|sample| sample.fee).max().unwrap_or(0);
        
        let success_count = recent_samples.iter().filter(|sample| sample.success).count();
        let success_rate = if !recent_samples.is_empty() {
            success_count as f64 / recent_samples.len() as f64
        } else {
            0.0
        };
        
        FeeStats {
            sample_count: recent_fees.len(),
            recent_sample_count: recent_samples.len(),
            avg_fee,
            min_fee,
            max_fee,
            congestion_level: self.get_congestion_level(),
            success_rate,
        }
    }
}

impl Clone for FeePredictor {
    fn clone(&self) -> Self {
        Self {
            rpc_client: self.rpc_client.clone(),
            config: self.config.clone(),
            congestion_level: RwLock::new(*self.congestion_level.read()),
            recent_fees: RwLock::new(self.recent_fees.read().clone()),
            fee_stats_by_type: self.fee_stats_by_type.clone(),
            ml_model: TokioMutex::new(MlModel::new()), // Can't clone the mutex, create a new one
            last_congestion_update: RwLock::new(*self.last_congestion_update.read()),
            success_rate: RwLock::new(*self.success_rate.read()),
        }
    }
}

/// Fee statistics
#[derive(Debug, Clone)]
pub struct FeeStats {
    /// Total number of fee samples
    pub sample_count: usize,
    
    /// Number of recent fee samples (last minute)
    pub recent_sample_count: usize,
    
    /// Average fee
    pub avg_fee: u64,
    
    /// Minimum fee
    pub min_fee: u64,
    
    /// Maximum fee
    pub max_fee: u64,
    
    /// Current congestion level
    pub congestion_level: CongestionLevel,
    
    /// Success rate of recent transactions
    pub success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::system_instruction;
    use solana_sdk::signature::Keypair;
    
    #[tokio::test]
    async fn test_fee_predictor_heuristic() {
        let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
        let config = FeePredictionConfig {
            use_ml_model: false,
            ..Default::default()
        };
        
        let fee_predictor = FeePredictor::new(rpc_client, config);
        
        // Create a simple transaction
        let keypair = Keypair::new();
        let instruction = system_instruction::transfer(
            &keypair.pubkey(),
            &Pubkey::new_unique(),
            1000,
        );
        
        let transaction = Transaction::new_with_payer(
            &[instruction],
            Some(&keypair.pubkey()),
        );
        
        // Predict fee
        let fee = fee_predictor.predict_priority_fee(&transaction).await.unwrap().unwrap();
        
        // Fee should be within expected range
        assert!(fee >= fee_predictor.config.min_fee);
        assert!(fee <= fee_predictor.config.max_fee);
    }
    
    #[tokio::test]
    async fn test_congestion_level() {
        let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
        let config = FeePredictionConfig::default();
        
        let fee_predictor = FeePredictor::new(rpc_client, config);
        
        // Record some fees
        for i in 0..100 {
            let fee = 5000 + (i * 1000); // Increasing fees
            fee_predictor.record_fee(fee, true, None, None);
        }
        
        // Update congestion level
        fee_predictor.update_congestion_level().await.unwrap();
        
        // Congestion level should be higher than Low due to increasing fees
        assert!(matches!(fee_predictor.get_congestion_level(), CongestionLevel::Medium | CongestionLevel::High));
    }
}