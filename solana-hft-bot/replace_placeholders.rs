//! Script to identify and replace placeholder code in the Solana HFT Bot codebase
//!
//! This script scans the codebase for placeholder comments and provides
//! suggestions for replacing them with actual implementations.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// Command line arguments
#[derive(Parser, Debug)]
#[clap(name = "replace-placeholders", about = "Replace placeholder code in the Solana HFT Bot codebase")]
struct Args {
    /// Path to the project root
    #[clap(short, long, default_value = ".")]
    path: PathBuf,
    
    /// Whether to actually replace the placeholders or just identify them
    #[clap(short, long)]
    dry_run: bool,
    
    /// Path to the replacement configuration file
    #[clap(short, long)]
    config: Option<PathBuf>,
    
    /// Whether to generate a replacement configuration file
    #[clap(short, long)]
    generate_config: bool,
    
    /// Path to write the generated configuration file
    #[clap(long, default_value = "placeholder-replacements.json")]
    output_config: PathBuf,
}

/// Placeholder information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Placeholder {
    /// File path
    file: String,
    
    /// Line number
    line: usize,
    
    /// The placeholder text
    text: String,
    
    /// The suggested replacement
    replacement: Option<String>,
    
    /// The priority of the replacement (1-5, with 1 being highest)
    priority: u8,
    
    /// The category of the placeholder
    category: String,
}

/// Replacement configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReplacementConfig {
    /// Placeholders to replace
    placeholders: Vec<Placeholder>,
    
    /// Patterns to look for
    patterns: Vec<String>,
    
    /// Categories and their priorities
    categories: HashMap<String, u8>,
}

impl Default for ReplacementConfig {
    fn default() -> Self {
        let mut categories = HashMap::new();
        categories.insert("network".to_string(), 1);
        categories.insert("rpc".to_string(), 2);
        categories.insert("execution".to_string(), 1);
        categories.insert("arbitrage".to_string(), 3);
        categories.insert("screening".to_string(), 4);
        categories.insert("risk".to_string(), 5);
        
        Self {
            placeholders: Vec::new(),
            patterns: vec![
                r"// In a real implementation".to_string(),
                r"// This is a placeholder".to_string(),
                r"// Placeholder for".to_string(),
                r"// TODO: Implement".to_string(),
                r"// FIXME: Implement".to_string(),
            ],
            categories,
        }
    }
}

/// Find placeholders in the codebase
async fn find_placeholders(args: &Args) -> Result<Vec<Placeholder>> {
    let config = if let Some(config_path) = &args.config {
        // Load configuration from file
        let config_str = fs::read_to_string(config_path)
            .context(format!("Failed to read configuration file: {:?}", config_path))?;
        serde_json::from_str(&config_str)
            .context(format!("Failed to parse configuration file: {:?}", config_path))?
    } else {
        // Use default configuration
        ReplacementConfig::default()
    };
    
    // Compile regex patterns
    let patterns: Vec<Regex> = config.patterns.iter()
        .map(|p| Regex::new(p).unwrap())
        .collect();
    
    // Find all Rust files in the project
    let mut rust_files = Vec::new();
    find_rust_files(&args.path, &mut rust_files)?;
    
    println!("Found {} Rust files to scan", rust_files.len());
    
    // Scan files for placeholders
    let placeholders = Arc::new(Mutex::new(Vec::new()));
    
    let tasks: Vec<_> = rust_files.into_par_iter()
        .map(|file| {
            let patterns = patterns.clone();
            let placeholders = placeholders.clone();
            let categories = config.categories.clone();
            
            async move {
                scan_file_for_placeholders(&file, &patterns, &categories, placeholders).await
            }
        })
        .collect();
    
    // Wait for all tasks to complete
    for task in tasks {
        task.await?;
    }
    
    let result = placeholders.lock().await.clone();
    println!("Found {} placeholders", result.len());
    
    Ok(result)
}

/// Find all Rust files in a directory recursively
fn find_rust_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Skip target directory
                if path.file_name().map_or(false, |name| name == "target") {
                    continue;
                }
                
                find_rust_files(&path, files)?;
            } else if path.extension().map_or(false, |ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    
    Ok(())
}

/// Scan a file for placeholders
async fn scan_file_for_placeholders(
    file: &Path,
    patterns: &[Regex],
    categories: &HashMap<String, u8>,
    placeholders: Arc<Mutex<Vec<Placeholder>>>,
) -> Result<()> {
    let file_str = file.to_string_lossy().to_string();
    let reader = BufReader::new(File::open(file)?);
    
    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result?;
        
        for pattern in patterns {
            if pattern.is_match(&line) {
                // Determine category based on file path
                let category = determine_category(&file_str);
                
                // Determine priority based on category
                let priority = categories.get(&category).copied().unwrap_or(5);
                
                // Create placeholder
                let placeholder = Placeholder {
                    file: file_str.clone(),
                    line: line_num + 1,
                    text: line.clone(),
                    replacement: None,
                    priority,
                    category,
                };
                
                // Add to list
                placeholders.lock().await.push(placeholder);
                
                // Only match one pattern per line
                break;
            }
        }
    }
    
    Ok(())
}

/// Determine the category of a file based on its path
fn determine_category(file_path: &str) -> String {
    if file_path.contains("network") {
        "network".to_string()
    } else if file_path.contains("rpc") {
        "rpc".to_string()
    } else if file_path.contains("execution") {
        "execution".to_string()
    } else if file_path.contains("arbitrage") {
        "arbitrage".to_string()
    } else if file_path.contains("screening") {
        "screening".to_string()
    } else if file_path.contains("risk") {
        "risk".to_string()
    } else {
        "other".to_string()
    }
}

/// Generate suggested replacements for placeholders
fn generate_replacements(placeholders: &mut [Placeholder]) -> Result<()> {
    for placeholder in placeholders.iter_mut() {
        // Generate replacement based on category and text
        let replacement = match placeholder.category.as_str() {
            "network" => {
                if placeholder.text.contains("DPDK") {
                    Some(r#"// Initialize DPDK
let mut dpdk_args = vec!["dpdk".to_string()];
if let Some(cores) = &self.config.dpdk_cores {
    dpdk_args.push("-l".to_string());
    dpdk_args.push(cores.clone());
}
if let Some(pci) = &self.config.dpdk_pci_addr {
    dpdk_args.push("-a".to_string());
    dpdk_args.push(pci.clone());
}
dpdk_sys::rte_eal_init(dpdk_args)?;

// Configure memory pools
let mbuf_pool = dpdk_sys::rte_pktmbuf_pool_create(
    "mbuf_pool\0".as_ptr() as *const i8,
    8192, // Number of elements
    256,  // Cache size
    0,    // Private data size
    dpdk_sys::RTE_MBUF_DEFAULT_BUF_SIZE as u16,
    dpdk_sys::rte_socket_id() as i32,
)?;

// Configure network ports
let port_id = 0; // Use first port
dpdk_sys::rte_eth_dev_configure(
    port_id,
    1, // RX queues
    1, // TX queues
    &mut dpdk_sys::rte_eth_conf::default(),
)?;

// Start the port
dpdk_sys::rte_eth_dev_start(port_id)?;"#.to_string())
                } else if placeholder.text.contains("io_uring") {
                    Some(r#"// Initialize io_uring
let ring = io_uring::IoUring::new(1024)?;
let (submitter, mut completion) = ring.split();

// Set up submission queue
let mut sq = submitter.sq();
let mut sqe = sq.prepare_sqe();
sqe.prep_timeout(&libc::__kernel_timespec { tv_sec: 0, tv_nsec: 0 }, 0, 0);
unsafe { sq.submit()? };

// Process completions
for cqe in &mut completion {
    // Process completion queue entry
    let _result = cqe.result();
}"#.to_string())
                } else if placeholder.text.contains("zero-copy") {
                    Some(r#"// Use zero-copy buffer for data transfer
let buffer = zero_copy::ZeroCopyBuffer::new(65536)?;
let data_len = socket.recv(buffer.as_mut_ptr(), buffer.capacity())?;
unsafe { buffer.set_len(data_len) };

// Process the data without copying
process_packet_data(&buffer)?;

// Send response without copying
socket.send(buffer.as_ptr(), buffer.len())?;"#.to_string())
                } else {
                    None
                }
            },
            "execution" => {
                if placeholder.text.contains("transaction") {
                    Some(r#"// Create a transaction with optimized parameters
let mut instructions = Vec::new();

// Add compute budget instruction for priority fee
instructions.push(
    solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(
        priority_fee
    )
);

// Add compute budget instruction for compute units
instructions.push(
    solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
        compute_units
    )
);

// Add the main instruction
instructions.push(instruction);

// Create and sign the transaction
let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
let transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
    &instructions,
    Some(&self.keypair.pubkey()),
    &[&self.keypair],
    recent_blockhash,
);"#.to_string())
                } else if placeholder.text.contains("Jito") {
                    Some(r#"// Create a Jito MEV bundle
let mut bundle_builder = jito_bundle::BundleBuilder::new();

// Add transactions to the bundle
bundle_builder.add_transaction(transaction.clone());

// Set tip options
let tip_account = self.config.jito_tip_account.parse::<Pubkey>()?;
bundle_builder.set_tip_options(
    jito_bundle::TipOptions {
        tip_account,
        tip_amount: self.config.jito_tip_amount,
    }
);

// Build the bundle
let bundle = bundle_builder.build()?;

// Submit the bundle to Jito
let jito_client = jito_searcher_client::Client::new(
    self.config.jito_endpoint.clone(),
    self.config.jito_auth_token.clone(),
)?;

let result = jito_client.send_bundle(bundle).await?;"#.to_string())
                } else {
                    None
                }
            },
            "rpc" => {
                if placeholder.text.contains("cache") {
                    Some(r#"// Check cache first
let cache_key = format!("account:{}", pubkey);
if let Some(cached_data) = self.cache.get(&cache_key).await {
    // Update cache hit metrics
    self.metrics.record_cache_hit();
    
    // Deserialize and return cached data
    return Ok(serde_json::from_value(cached_data)?);
}

// Cache miss, fetch from RPC
let result = self.rpc_client.get_account(&pubkey).await?;

// Store in cache with TTL
if let Ok(json_value) = serde_json::to_value(&result) {
    self.cache.set(&cache_key, json_value, self.config.cache_ttl_ms).await;
}

// Update cache miss metrics
self.metrics.record_cache_miss();"#.to_string())
                } else if placeholder.text.contains("rate limit") {
                    Some(r#"// Apply rate limiting
let cost = match method {
    "getAccountInfo" => 1.0,
    "getBalance" => 0.5,
    "getRecentBlockhash" => 0.5,
    "getSignatureStatuses" => 1.0,
    "getTransaction" => 2.0,
    "getMultipleAccounts" => 2.0 * (params.len() as f64),
    "getProgramAccounts" => 5.0,
    "sendTransaction" => 3.0,
    _ => 1.0,
};

if !self.rate_limiter.try_acquire_with_cost(endpoint, method, cost).await {
    // Rate limit exceeded
    self.metrics.record_rate_limited();
    return Err(RpcError::RateLimited);
}"#.to_string())
                } else {
                    None
                }
            },
            "arbitrage" => {
                if placeholder.text.contains("arbitrage") {
                    Some(r#"// Find arbitrage opportunities across DEXes
let pools = self.pool_registry.get_pools_for_token(token_a, token_b);
let mut opportunities = Vec::new();

// Check for price differences between pools
for (i, pool1) in pools.iter().enumerate() {
    for pool2 in pools.iter().skip(i + 1) {
        // Skip if pools are on the same DEX
        if pool1.dex == pool2.dex {
            continue;
        }
        
        // Calculate prices in both directions
        let price1_a_to_b = pool1.get_price(token_a, token_b)?;
        let price1_b_to_a = pool1.get_price(token_b, token_a)?;
        let price2_a_to_b = pool2.get_price(token_a, token_b)?;
        let price2_b_to_a = pool2.get_price(token_b, token_a)?;
        
        // Check for arbitrage in A->B->A direction
        let profit_bps_a = calculate_profit_bps(price1_a_to_b, price2_b_to_a);
        if profit_bps_a > self.config.min_profit_threshold_bps {
            opportunities.push(ArbitrageOpportunity {
                id: format!("arb-{}-{}-a", pool1.address, pool2.address),
                path: ArbitragePath {
                    tokens: vec![token_a, token_b, token_a],
                    pools: vec![pool1.address, pool2.address],
                },
                expected_profit_bps: profit_bps_a,
                expected_profit_usd: calculate_profit_usd(amount, profit_bps_a),
                input_amount: amount,
                timestamp: chrono::Utc::now(),
                strategy: "cross_dex".to_string(),
                priority: ExecutionPriority::High,
                ttl_ms: 10000,
            });
        }
        
        // Check for arbitrage in B->A->B direction
        let profit_bps_b = calculate_profit_bps(price1_b_to_a, price2_a_to_b);
        if profit_bps_b > self.config.min_profit_threshold_bps {
            opportunities.push(ArbitrageOpportunity {
                id: format!("arb-{}-{}-b", pool1.address, pool2.address),
                path: ArbitragePath {
                    tokens: vec![token_b, token_a, token_b],
                    pools: vec![pool1.address, pool2.address],
                },
                expected_profit_bps: profit_bps_b,
                expected_profit_usd: calculate_profit_usd(amount, profit_bps_b),
                input_amount: amount,
                timestamp: chrono::Utc::now(),
                strategy: "cross_dex".to_string(),
                priority: ExecutionPriority::High,
                ttl_ms: 10000,
            });
        }
    }
}

return Ok(opportunities);"#.to_string())
                } else if placeholder.text.contains("flash loan") {
                    Some(r#"// Create flash loan instructions
let flash_loan_program_id = solana_sdk::pubkey::Pubkey::from_str("So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo")?;
let token_program_id = solana_sdk::pubkey::Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?;

// Create flash loan accounts
let (flash_loan_state, _) = Pubkey::find_program_address(
    &[b"flash_loan", token_mint.as_ref()],
    &flash_loan_program_id,
);

let (vault_account, _) = Pubkey::find_program_address(
    &[b"vault", token_mint.as_ref()],
    &flash_loan_program_id,
);

// Create flash loan instruction
let borrow_ix = solana_sdk::instruction::Instruction {
    program_id: flash_loan_program_id,
    accounts: vec![
        AccountMeta::new(flash_loan_state, false),
        AccountMeta::new(vault_account, false),
        AccountMeta::new(user_token_account, false),
        AccountMeta::new_readonly(token_program_id, false),
        AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
    ],
    data: borsh::to_vec(&FlashLoanInstruction::Borrow { amount })?,
};

// Create repay instruction
let repay_ix = solana_sdk::instruction::Instruction {
    program_id: flash_loan_program_id,
    accounts: vec![
        AccountMeta::new(flash_loan_state, false),
        AccountMeta::new(vault_account, false),
        AccountMeta::new(user_token_account, false),
        AccountMeta::new_readonly(token_program_id, false),
    ],
    data: borsh::to_vec(&FlashLoanInstruction::Repay { amount })?,
};

// Return instructions
Ok((borrow_ix, repay_ix))"#.to_string())
                } else {
                    None
                }
            },
            "screening" => {
                if placeholder.text.contains("token") {
                    Some(r#"// Process token account data
if account.owner == solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") {
    // Check if this is a mint account (token creation)
    if account.data.len() == spl_token::state::Mint::LEN {
        if let Ok(mint) = spl_token::state::Mint::unpack(&account.data) {
            // This is a mint account
            let token = Token {
                mint: pubkey,
                name: "Unknown".to_string(),
                symbol: "UNKNOWN".to_string(),
                decimals: mint.decimals,
                total_supply: mint.supply,
                circulating_supply: Some(mint.supply),
                metadata: TokenMetadata {
                    logo_uri: None,
                    website: None,
                    twitter: None,
                    is_verified: false,
                    launch_timestamp: Some(chrono::Utc::now()),
                    tags: vec![],
                    description: None,
                },
                price_usd: None,
                market_cap_usd: None,
                volume_24h_usd: None,
                score: TokenScore::default(),
                liquidity: HashMap::new(),
                holders: Vec::new(),
                tracked_since: chrono::Utc::now(),
                last_updated: chrono::Utc::now(),
                tags: vec!["new".to_string()],
            };
            
            // Add token to database
            let mut token_database = token_database.write().await;
            token_database.insert(pubkey, token);
            
            // Trigger token launch detection
            return Ok(());
        }
    }
}

// Not a token mint account
Ok(())"#.to_string())
                } else if placeholder.text.contains("rugpull") {
                    Some(r#"// Assess token for rugpull risk
let mut risk_score = 0;

// Check token age
let token_age = chrono::Utc::now().signed_duration_since(token.metadata.launch_timestamp.unwrap_or_default());
if token_age.num_seconds() < self.config.min_token_age_seconds as i64 {
    risk_score += 20;
}

// Check liquidity
let total_liquidity: f64 = token.liquidity.values().sum();
if total_liquidity < self.config.min_liquidity_usd {
    risk_score += 30;
}

// Check holder concentration
if let Some(top_holder) = token.holders.first() {
    let top_holder_percentage = top_holder.percentage;
    if top_holder_percentage > 20.0 {
        risk_score += 20;
    }
    if top_holder_percentage > 50.0 {
        risk_score += 30;
    }
}

// Check for verified source code
if !token.metadata.is_verified {
    risk_score += 10;
}

// Cap risk score at 100
risk_score = risk_score.min(100);

// Update token risk score
token.score.rugpull_risk = risk_score;

// Return assessment
TokenRiskAssessment {
    token_mint: token.mint,
    risk_score,
    passed: risk_score <= self.config.max_risk_score,
    reasons: vec![
        format!("Token age: {} seconds", token_age.num_seconds()),
        format!("Liquidity: ${:.2}", total_liquidity),
        format!("Top holder: {}%", token.holders.first().map_or(0.0, |h| h.percentage)),
        format!("Verified: {}", token.metadata.is_verified),
    ],
    timestamp: chrono::Utc::now(),
}"#.to_string())
                } else {
                    None
                }
            },
            "risk" => {
                if placeholder.text.contains("circuit breaker") {
                    Some(r#"// Check circuit breakers
for breaker in &self.config.circuit_breakers {
    if !breaker.enabled {
        continue;
    }
    
    match breaker.name.as_str() {
        "drawdown" => {
            // Check drawdown circuit breaker
            let current_drawdown = self.calculate_drawdown()?;
            if current_drawdown > breaker.threshold {
                // Trigger circuit breaker
                self.trigger_circuit_breaker(
                    &breaker.name,
                    format!("Drawdown of {:.2}% exceeded threshold of {:.2}%", 
                            current_drawdown, breaker.threshold)
                )?;
                return Ok(false);
            }
        },
        "loss_streak" => {
            // Check loss streak circuit breaker
            let consecutive_losses = self.get_consecutive_losses();
            if consecutive_losses >= breaker.threshold as usize {
                // Trigger circuit breaker
                self.trigger_circuit_breaker(
                    &breaker.name,
                    format!("{} consecutive losses exceeded threshold of {}", 
                            consecutive_losses, breaker.threshold)
                )?;
                return Ok(false);
            }
        },
        "market_volatility" => {
            // Check market volatility circuit breaker
            let current_volatility = self.calculate_market_volatility()?;
            if current_volatility > breaker.threshold {
                // Trigger circuit breaker
                self.trigger_circuit_breaker(
                    &breaker.name,
                    format!("Market volatility of {:.2}% exceeded threshold of {:.2}%", 
                            current_volatility, breaker.threshold)
                )?;
                return Ok(false);
            }
        },
        _ => {
            warn!("Unknown circuit breaker: {}", breaker.name);
        }
    }
}

// All circuit breakers passed
Ok(true)"#.to_string())
                } else if placeholder.text.contains("risk") {
                    Some(r#"// Assess risk for a trading opportunity
let mut risk_assessment = RiskAssessment {
    approved: true,
    risk_score: 0,
    max_position_size: None,
    rejection_reason: None,
};

// Check token risk
if let Some(token_risk) = self.token_risk_scores.get(&token) {
    risk_assessment.risk_score += token_risk.risk_score;
}

// Check strategy allocation
let strategy_allocation = self.get_strategy_allocation(&strategy_id)?;
if strategy_allocation > self.config.max_strategy_allocation {
    risk_assessment.approved = false;
    risk_assessment.rejection_reason = Some(format!(
        "Strategy allocation of {:.2}% exceeds maximum of {:.2}%",
        strategy_allocation * 100.0,
        self.config.max_strategy_allocation * 100.0
    ));
    return Ok(risk_assessment);
}

// Check token concentration
let token_concentration = self.get_token_concentration(token)?;
if token_concentration > self.config.max_token_concentration {
    risk_assessment.approved = false;
    risk_assessment.rejection_reason = Some(format!(
        "Token concentration of {:.2}% exceeds maximum of {:.2}%",
        token_concentration * 100.0,
        self.config.max_token_concentration * 100.0
    ));
    return Ok(risk_assessment);
}

// Calculate maximum position size
let max_position_size = self.calculate_max_position_size(
    token,
    risk_assessment.risk_score,
    expected_profit_usd
)?;

risk_assessment.max_position_size = Some(max_position_size);

// Check if amount exceeds maximum position size
if amount > max_position_size {
    risk_assessment.approved = false;
    risk_assessment.rejection_reason = Some(format!(
        "Amount of {} exceeds maximum position size of {}",
        amount,
        max_position_size
    ));
}

Ok(risk_assessment)"#.to_string())
                } else {
                    None
                }
            },
            _ => None,
        };
        
        placeholder.replacement = replacement;
    }
    
    Ok(())
}

/// Replace placeholders in the codebase
async fn replace_placeholders(args: &Args, placeholders: &[Placeholder]) -> Result<()> {
    if args.dry_run {
        println!("Dry run mode - not replacing placeholders");
        return Ok(());
    }
    
    println!("Replacing {} placeholders", placeholders.len());
    
    // Group placeholders by file
    let mut placeholders_by_file: HashMap<String, Vec<&Placeholder>> = HashMap::new();
    for placeholder in placeholders {
        if placeholder.replacement.is_some() {
            placeholders_by_file
                .entry(placeholder.file.clone())
                .or_default()
                .push(placeholder);
        }
    }
    
    // Sort placeholders by line number (descending) to avoid line number changes
    for placeholders in placeholders_by_file.values_mut() {
        placeholders.sort_by(|a, b| b.line.cmp(&a.line));
    }
    
    // Replace placeholders in each file
    for (file, placeholders) in placeholders_by_file {
        replace_placeholders_in_file(&file, &placeholders)?;
    }
    
    Ok(())
}

/// Replace placeholders in a file
fn replace_placeholders_in_file(file: &str, placeholders: &[&Placeholder]) -> Result<()> {
    // Read file content
    let content = fs::read_to_string(file)?;
    let lines: Vec<&str> = content.lines().collect();
    
    // Create new content with replacements
    let mut new_lines = lines.clone();
    
    for placeholder in placeholders {
        if let Some(replacement) = &placeholder.replacement {
            // Replace the line
            if placeholder.line <= new_lines.len() {
                new_lines[placeholder.line - 1] = replacement;
            }
        }
    }
    
    // Write new content
    let new_content = new_lines.join("\n");
    fs::write(file, new_content)?;
    
    println!("Replaced {} placeholders in {}", placeholders.len(), file);
    
    Ok(())
}

/// Generate a replacement configuration file
fn generate_replacement_config(placeholders: &[Placeholder], output_path: &Path) -> Result<()> {
    let config = ReplacementConfig {
        placeholders: placeholders.to_vec(),
        ..Default::default()
    };
    
    let config_str = serde_json::to_string_pretty(&config)?;
    fs::write(output_path, config_str)?;
    
    println!("Generated replacement configuration file: {:?}", output_path);
    
    Ok(())
}

/// Print placeholders in a human-readable format
fn print_placeholders(placeholders: &[Placeholder]) {
    // Group by category
    let mut placeholders_by_category: HashMap<String, Vec<&Placeholder>> = HashMap::new();
    for placeholder in placeholders {
        placeholders_by_category
            .entry(placeholder.category.clone())
            .or_default()
            .push(placeholder);
    }
    
    // Sort categories by priority
    let mut categories: Vec<(String, u8)> = placeholders_by_category
        .keys()
        .map(|category| {
            let priority = placeholders_by_category[category]
                .iter()
                .map(|p| p.priority)
                .min()
                .unwrap_or(5);
            (category.clone(), priority)
        })
        .collect();
    
    categories.sort_by_key(|(_, priority)| *priority);
    
    // Print placeholders by category
    println!("\n=== Placeholders by Category ===\n");
    
    for (category, priority) in categories {
        let category_placeholders = placeholders_by_category[&category].clone();
        
        println!("Category: {} (Priority: {})", category, priority);
        println!("Found {} placeholders", category_placeholders.len());
        println!();
        
        // Print top 5 placeholders
        for (i, placeholder) in category_placeholders.iter().take(5).enumerate() {
            println!("{}. File: {}", i + 1, placeholder.file);
            println!("   Line: {}", placeholder.line);
            println!("   Text: {}", placeholder.text);
            println!();
        }
        
        if category_placeholders.len() > 5 {
            println!("... and {} more", category_placeholders.len() - 5);
        }
        
        println!("-----------------------------------\n");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Find placeholders
    let mut placeholders = find_placeholders(&args).await?;
    
    // Generate replacements
    generate_replacements(&mut placeholders)?;
    
    // Print placeholders
    print_placeholders(&placeholders);
    
    // Generate replacement configuration file if requested
    if args.generate_config {
        generate_replacement_config(&placeholders, &args.output_config)?;
    }
    
    // Replace placeholders
    replace_placeholders(&args, &placeholders).await?;
    
    Ok(())
}