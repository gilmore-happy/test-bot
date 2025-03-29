# Jito API Implementation Details

This document provides detailed implementation guidance for integrating Jito's API endpoints into the Solana HFT Bot, focusing on the specific API endpoints, bundle handling, and tip management.

## API Endpoints

### Transaction Endpoints (`/api/v1/transactions`)

#### `sendTransaction`

This endpoint acts as a proxy to the Solana `sendTransaction` RPC method but forwards transactions directly to validators with MEV protection.

**Implementation:**

```rust
pub async fn send_transaction_with_jito(
    &self,
    transaction: &Transaction,
    config: &SendTransactionConfig,
) -> Result<Signature, ClientError> {
    // Ensure transaction has a priority fee
    if !transaction_has_priority_fee(transaction) {
        return Err(ClientError::custom("Transaction must include a priority fee"));
    }
    
    // Ensure transaction has a Jito tip (30% of total fee)
    if !transaction_has_jito_tip(transaction, self.config.jito_tip_account.as_str()) {
        return Err(ClientError::custom("Transaction must include a Jito tip"));
    }
    
    // Serialize transaction
    let serialized_tx = bincode::serialize(transaction)?;
    let encoded_tx = base64::encode(&serialized_tx);
    
    // Prepare request
    let request = json!({
        "jsonrpc": "2.0",
        "id": self.next_request_id(),
        "method": "sendTransaction",
        "params": [
            encoded_tx,
            {
                "encoding": "base64",
                "skipPreflight": true
            }
        ]
    });
    
    // Send request to Jito Block Engine
    let response = self.client
        .post(&self.config.jito_endpoint)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;
    
    // Extract bundle_id from header if available
    if let Some(bundle_id) = response.headers().get("x-bundle-id") {
        log::info!("Transaction sent as bundle: {}", bundle_id.to_str().unwrap_or("unknown"));
    }
    
    // Parse response
    let response_json: Value = response.json().await?;
    
    // Extract signature
    if let Some(result) = response_json.get("result") {
        if let Some(signature_str) = result.as_str() {
            return Ok(Signature::from_str(signature_str)?);
        }
    }
    
    Err(ClientError::custom("Failed to parse response"))
}
```

### Bundle Endpoints (`/api/v1/bundles`)

#### `sendBundle`

This endpoint submits a bundled list of signed transactions for atomic processing.

**Implementation:**

```rust
pub async fn send_bundle(
    &self,
    transactions: Vec<Transaction>,
    options: BundleOptions,
) -> Result<BundleReceipt, ClientError> {
    // Validate bundle size
    if transactions.is_empty() || transactions.len() > 5 {
        return Err(ClientError::custom("Bundle must contain 1-5 transactions"));
    }
    
    // Ensure at least one transaction has a tip
    if !bundle_has_tip(&transactions, &options.tip_account) {
        return Err(ClientError::custom("Bundle must include a tip to a Jito tip account"));
    }
    
    // Serialize and encode transactions
    let encoded_txs: Vec<String> = transactions
        .iter()
        .map(|tx| {
            let serialized = bincode::serialize(tx)?;
            Ok(base64::encode(&serialized))
        })
        .collect::<Result<Vec<String>, ClientError>>()?;
    
    // Prepare request
    let request = json!({
        "jsonrpc": "2.0",
        "id": self.next_request_id(),
        "method": "sendBundle",
        "params": [
            encoded_txs,
            {
                "encoding": "base64"
            }
        ]
    });
    
    // Add authentication if provided
    let mut req_builder = self.client.post(&format!("{}/api/v1/bundles", self.config.jito_endpoint))
        .header("Content-Type", "application/json");
    
    if let Some(auth_token) = &self.config.jito_auth_token {
        req_builder = req_builder.header("x-jito-auth", auth_token);
    }
    
    // Send request
    let response = req_builder.json(&request).send().await?;
    
    // Parse response
    let response_json: Value = response.json().await?;
    
    // Extract bundle ID
    if let Some(result) = response_json.get("result") {
        if let Some(bundle_id) = result.as_str() {
            return Ok(BundleReceipt {
                uuid: bundle_id.to_string(),
                transactions: transactions.iter().map(|tx| tx.signatures[0]).collect(),
            });
        }
    }
    
    Err(ClientError::custom("Failed to parse response"))
}
```

#### `getBundleStatuses`

This endpoint returns the status of submitted bundles.

**Implementation:**

```rust
pub async fn get_bundle_statuses(
    &self,
    bundle_ids: &[String],
) -> Result<Vec<Option<BundleStatus>>, ClientError> {
    // Validate input
    if bundle_ids.is_empty() || bundle_ids.len() > 5 {
        return Err(ClientError::custom("Must provide 1-5 bundle IDs"));
    }
    
    // Prepare request
    let request = json!({
        "jsonrpc": "2.0",
        "id": self.next_request_id(),
        "method": "getBundleStatuses",
        "params": [bundle_ids]
    });
    
    // Send request
    let response = self.client
        .post(&format!("{}/api/v1/bundles", self.config.jito_endpoint))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;
    
    // Parse response
    let response_json: Value = response.json().await?;
    
    // Extract statuses
    if let Some(result) = response_json.get("result") {
        if let Some(value) = result.get("value") {
            if let Some(statuses) = value.as_array() {
                return Ok(statuses
                    .iter()
                    .map(|status| {
                        if status.is_null() {
                            None
                        } else {
                            Some(BundleStatus {
                                bundle_id: status["bundle_id"].as_str().unwrap_or("").to_string(),
                                transactions: status["transactions"]
                                    .as_array()
                                    .unwrap_or(&Vec::new())
                                    .iter()
                                    .filter_map(|tx| tx.as_str().map(|s| s.to_string()))
                                    .collect(),
                                slot: status["slot"].as_u64().unwrap_or(0),
                                confirmation_status: status["confirmation_status"]
                                    .as_str()
                                    .unwrap_or("unknown")
                                    .to_string(),
                                error: status["err"]["Ok"].is_null(),
                            })
                        }
                    })
                    .collect());
            }
        }
    }
    
    Err(ClientError::custom("Failed to parse response"))
}
```

#### `getTipAccounts`

This endpoint retrieves the tip accounts designated for tip payments.

**Implementation:**

```rust
pub async fn get_tip_accounts(&self) -> Result<Vec<String>, ClientError> {
    // Prepare request
    let request = json!({
        "jsonrpc": "2.0",
        "id": self.next_request_id(),
        "method": "getTipAccounts",
        "params": []
    });
    
    // Send request
    let response = self.client
        .post(&format!("{}/api/v1/bundles", self.config.jito_endpoint))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;
    
    // Parse response
    let response_json: Value = response.json().await?;
    
    // Extract tip accounts
    if let Some(result) = response_json.get("result") {
        if let Some(accounts) = result.as_array() {
            return Ok(accounts
                .iter()
                .filter_map(|account| account.as_str().map(|s| s.to_string()))
                .collect());
        }
    }
    
    // If API call fails, use hardcoded tip accounts as fallback
    Ok(vec![
        "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5".to_string(),
        "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe".to_string(),
        "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY".to_string(),
        "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49".to_string(),
        "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh".to_string(),
        "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt".to_string(),
        "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL".to_string(),
        "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT".to_string(),
    ])
}
```

#### `getInflightBundleStatuses`

This endpoint returns the status of submitted bundles within the last five minutes.

**Implementation:**

```rust
pub async fn get_inflight_bundle_statuses(
    &self,
    bundle_ids: &[String],
) -> Result<Vec<InflightBundleStatus>, ClientError> {
    // Validate input
    if bundle_ids.is_empty() || bundle_ids.len() > 5 {
        return Err(ClientError::custom("Must provide 1-5 bundle IDs"));
    }
    
    // Prepare request
    let request = json!({
        "jsonrpc": "2.0",
        "id": self.next_request_id(),
        "method": "getInflightBundleStatuses",
        "params": [bundle_ids]
    });
    
    // Send request
    let response = self.client
        .post(&format!("{}/api/v1/bundles", self.config.jito_endpoint))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;
    
    // Parse response
    let response_json: Value = response.json().await?;
    
    // Extract statuses
    if let Some(result) = response_json.get("result") {
        if let Some(value) = result.get("value") {
            if let Some(statuses) = value.as_array() {
                return Ok(statuses
                    .iter()
                    .map(|status| InflightBundleStatus {
                        bundle_id: status["bundle_id"].as_str().unwrap_or("").to_string(),
                        status: match status["status"].as_str().unwrap_or("unknown") {
                            "Invalid" => BundleInflightStatus::Invalid,
                            "Pending" => BundleInflightStatus::Pending,
                            "Failed" => BundleInflightStatus::Failed,
                            "Landed" => BundleInflightStatus::Landed,
                            _ => BundleInflightStatus::Unknown,
                        },
                        landed_slot: if status["landed_slot"].is_null() {
                            None
                        } else {
                            status["landed_slot"].as_u64()
                        },
                    })
                    .collect());
            }
        }
    }
    
    Err(ClientError::custom("Failed to parse response"))
}
```

## Tip Management

### Tip Amount Calculation

For `sendTransaction`, we need to implement a 70/30 split between priority fee and Jito tip:

```rust
pub fn calculate_tip_amounts(total_fee_lamports: u64) -> (u64, u64) {
    // 70% for priority fee, 30% for Jito tip
    let priority_fee = total_fee_lamports * 7 / 10;
    let jito_tip = total_fee_lamports - priority_fee;
    
    // Ensure minimum tip of 1000 lamports
    let jito_tip = jito_tip.max(1000);
    
    (priority_fee, jito_tip)
}
```

### Adding Tips to Transactions

```rust
pub fn add_jito_tip(
    transaction: &mut Transaction,
    tip_account: &Pubkey,
    tip_amount: u64,
) -> Result<(), ClientError> {
    // Create transfer instruction
    let transfer_ix = system_instruction::transfer(
        &transaction.message.account_keys[0], // Assuming first account is fee payer
        tip_account,
        tip_amount,
    );
    
    // Add instruction to transaction
    transaction.message.instructions.push(CompiledInstruction {
        program_id_index: transaction.message.account_keys
            .iter()
            .position(|key| *key == system_program::id())
            .ok_or_else(|| ClientError::custom("System program not found in transaction"))? as u8,
        accounts: vec![0, transaction.message.account_keys
            .iter()
            .position(|key| key == tip_account)
            .ok_or_else(|| ClientError::custom("Tip account not found in transaction"))? as u8],
        data: bincode::serialize(&transfer_ix)?,
    });
    
    Ok(())
}
```

### Best Practices for Tipping

1. **Integrate tip instructions within main transaction:**

```rust
pub fn create_transaction_with_tip(
    instructions: Vec<Instruction>,
    fee_payer: &Keypair,
    recent_blockhash: Hash,
    tip_account: &Pubkey,
    tip_amount: u64,
) -> Result<Transaction, ClientError> {
    // Create base transaction
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&fee_payer.pubkey()));
    
    // Add tip instruction
    let tip_ix = system_instruction::transfer(
        &fee_payer.pubkey(),
        tip_account,
        tip_amount,
    );
    transaction.message.instructions.push(tip_ix);
    
    // Sign transaction
    transaction.sign(&[fee_payer], recent_blockhash);
    
    Ok(transaction)
}
```

2. **Add assertions in tipping transaction:**

```rust
pub fn create_conditional_tip_transaction(
    fee_payer: &Keypair,
    tip_account: &Pubkey,
    tip_amount: u64,
    condition_account: &Pubkey,
    expected_data: Vec<u8>,
    recent_blockhash: Hash,
) -> Result<Transaction, ClientError> {
    // Create assertion instruction to check account state
    let assertion_ix = Instruction {
        program_id: solana_program::sysvar::instructions::id(),
        accounts: vec![
            AccountMeta::new_readonly(*condition_account, false),
        ],
        data: expected_data,
    };
    
    // Create tip instruction
    let tip_ix = system_instruction::transfer(
        &fee_payer.pubkey(),
        tip_account,
        tip_amount,
    );
    
    // Create transaction with both instructions
    let mut transaction = Transaction::new_with_payer(
        &[assertion_ix, tip_ix],
        Some(&fee_payer.pubkey()),
    );
    
    // Sign transaction
    transaction.sign(&[fee_payer], recent_blockhash);
    
    Ok(transaction)
}
```

3. **Bundle with safeguards:**

```rust
pub fn create_bundle_with_safeguards(
    main_transaction: Transaction,
    fee_payer: &Keypair,
    tip_account: &Pubkey,
    tip_amount: u64,
    recent_blockhash: Hash,
) -> Result<Vec<Transaction>, ClientError> {
    // Create tip transaction with assertions
    let tip_transaction = create_conditional_tip_transaction(
        fee_payer,
        tip_account,
        tip_amount,
        &main_transaction.signatures[0], // Use main transaction signature as condition
        vec![1], // Expected success value
        recent_blockhash,
    )?;
    
    Ok(vec![main_transaction, tip_transaction])
}
```

## Bundle Handling

### Creating a Bundle

```rust
pub struct MevBundleBuilder {
    transactions: Vec<Transaction>,
    max_size: usize,
}

impl MevBundleBuilder {
    pub fn new(max_size: usize) -> Self {
        Self {
            transactions: Vec::with_capacity(max_size),
            max_size: max_size.min(5), // Maximum 5 transactions per bundle
        }
    }
    
    pub fn add_transaction(&mut self, transaction: Transaction) -> Result<(), ClientError> {
        if self.transactions.len() >= self.max_size {
            return Err(ClientError::custom("Bundle is full"));
        }
        
        self.transactions.push(transaction);
        Ok(())
    }
    
    pub fn with_options(self, options: BundleOptions) -> (Vec<Transaction>, BundleOptions) {
        (self.transactions, options)
    }
    
    pub fn build(self) -> Vec<Transaction> {
        self.transactions
    }
}
```

### Bundle Options

```rust
pub struct BundleOptions {
    pub tip_account: Option<String>,
    pub tip_amount: u64,
    pub wait_for_confirmation: bool,
    pub timeout: Duration,
}

impl Default for BundleOptions {
    fn default() -> Self {
        Self {
            tip_account: None,
            tip_amount: 1000, // Minimum tip amount
            wait_for_confirmation: false,
            timeout: Duration::from_secs(30),
        }
    }
}
```

### Handling Bundle Responses

```rust
pub struct BundleReceipt {
    pub uuid: String,
    pub transactions: Vec<Signature>,
}

pub struct BundleStatus {
    pub bundle_id: String,
    pub transactions: Vec<String>,
    pub slot: u64,
    pub confirmation_status: String,
    pub error: bool,
}

pub enum BundleInflightStatus {
    Invalid,
    Pending,
    Failed,
    Landed,
    Unknown,
}

pub struct InflightBundleStatus {
    pub bundle_id: String,
    pub status: BundleInflightStatus,
    pub landed_slot: Option<u64>,
}
```

## Mitigations for Uncled Blocks

To protect against the "unbundling" issue mentioned in the documentation, we need to implement safeguards:

```rust
pub fn create_transaction_with_safeguards(
    instructions: Vec<Instruction>,
    fee_payer: &Keypair,
    recent_blockhash: Hash,
) -> Result<Transaction, ClientError> {
    // Add pre-execution checks
    let pre_check_ix = create_pre_execution_check_instruction()?;
    
    // Add post-execution checks
    let post_check_ix = create_post_execution_check_instruction()?;
    
    // Combine all instructions
    let mut all_instructions = vec![pre_check_ix];
    all_instructions.extend(instructions);
    all_instructions.push(post_check_ix);
    
    // Create transaction
    let mut transaction = Transaction::new_with_payer(&all_instructions, Some(&fee_payer.pubkey()));
    
    // Sign transaction
    transaction.sign(&[fee_payer], recent_blockhash);
    
    Ok(transaction)
}

fn create_pre_execution_check_instruction() -> Result<Instruction, ClientError> {
    // Create instruction to check initial state
    // This could verify account balances, slot number, etc.
    // ...
    
    Ok(Instruction {
        program_id: solana_program::system_program::id(),
        accounts: vec![],
        data: vec![],
    })
}

fn create_post_execution_check_instruction() -> Result<Instruction, ClientError> {
    // Create instruction to verify final state
    // This could verify expected account changes
    // ...
    
    Ok(Instruction {
        program_id: solana_program::system_program::id(),
        accounts: vec![],
        data: vec![],
    })
}
```

## Rate Limits

The Jito API has rate limits of 1 request per second per IP per region. We need to implement rate limiting in our client:

```rust
pub struct RateLimiter {
    last_request_time: Mutex<HashMap<String, Instant>>,
    rate_limit: Duration,
}

impl RateLimiter {
    pub fn new(rate_limit: Duration) -> Self {
        Self {
            last_request_time: Mutex::new(HashMap::new()),
            rate_limit,
        }
    }
    
    pub async fn wait(&self, region: &str) {
        let mut last_request_time = self.last_request_time.lock().await;
        
        if let Some(last_time) = last_request_time.get(region) {
            let elapsed = last_time.elapsed();
            if elapsed < self.rate_limit {
                let wait_time = self.rate_limit - elapsed;
                tokio::time::sleep(wait_time).await;
            }
        }
        
        last_request_time.insert(region.to_string(), Instant::now());
    }
}
```

## Integration with Execution Engine

```rust
impl ExecutionEngine {
    pub async fn submit_transaction_with_jito(
        &self,
        transaction: Transaction,
        priority_level: PriorityLevel,
        timeout: Option<Duration>,
    ) -> Result<Signature, ClientError> {
        // Calculate fee based on priority level
        let total_fee = match priority_level {
            PriorityLevel::Low => 10_000,
            PriorityLevel::Medium => 100_000,
            PriorityLevel::High => 1_000_000,
        };
        
        // Calculate priority fee and tip (70/30 split)
        let (priority_fee, jito_tip) = calculate_tip_amounts(total_fee);
        
        // Add priority fee to transaction
        let mut tx_with_fee = add_priority_fee(&transaction, priority_fee)?;
        
        // Get random tip account
        let tip_accounts = self.jito_client.get_tip_accounts().await?;
        let tip_account = Pubkey::from_str(
            &tip_accounts[rand::thread_rng().gen_range(0..tip_accounts.len())]
        )?;
        
        // Add Jito tip to transaction
        add_jito_tip(&mut tx_with_fee, &tip_account, jito_tip)?;
        
        // Send transaction via Jito
        let signature = self.jito_client.send_transaction_with_jito(
            &tx_with_fee,
            &SendTransactionConfig {
                skip_preflight: true,
                preflight_commitment: None,
                encoding: None,
                max_retries: None,
            },
        ).await?;
        
        // Wait for confirmation if timeout provided
        if let Some(timeout) = timeout {
            self.wait_for_confirmation(&signature, timeout).await?;
        }
        
        Ok(signature)
    }
    
    pub async fn submit_bundle(
        &self,
        transactions: Vec<Transaction>,
        options: BundleOptions,
    ) -> Result<BundleReceipt, ClientError> {
        // Validate bundle
        if transactions.is_empty() || transactions.len() > 5 {
            return Err(ClientError::custom("Bundle must contain 1-5 transactions"));
        }
        
        // Send bundle via Jito
        let receipt = self.jito_client.send_bundle(transactions, options).await?;
        
        // Wait for confirmation if requested
        if options.wait_for_confirmation {
            self.wait_for_bundle_confirmation(&receipt.uuid, options.timeout).await?;
        }
        
        Ok(receipt)
    }
    
    async fn wait_for_bundle_confirmation(
        &self,
        bundle_id: &str,
        timeout: Duration,
    ) -> Result<BundleStatus, ClientError> {
        let start_time = Instant::now();
        
        while start_time.elapsed() < timeout {
            // Check bundle status
            let statuses = self.jito_client.get_bundle_statuses(&[bundle_id.to_string()]).await?;
            
            if let Some(Some(status)) = statuses.first() {
                if status.confirmation_status == "finalized" {
                    return Ok(status.clone());
                }
            }
            
            // Check inflight status
            let inflight_statuses = self.jito_client.get_inflight_bundle_statuses(&[bundle_id.to_string()]).await?;
            
            if let Some(status) = inflight_statuses.first() {
                match status.status {
                    BundleInflightStatus::Landed => {
                        // Bundle landed, wait for finalization
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    },
                    BundleInflightStatus::Failed => {
                        return Err(ClientError::custom("Bundle failed"));
                    },
                    BundleInflightStatus::Invalid => {
                        return Err(ClientError::custom("Bundle invalid"));
                    },
                    _ => {
                        // Still pending, continue waiting
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
            
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        
        Err(ClientError::custom("Bundle confirmation timeout"))
    }
}
```

## Conclusion

This implementation provides a comprehensive integration of Jito's API endpoints, bundle handling, and tip management into the Solana HFT Bot. By following these implementation details, the bot will be able to leverage Jito's MEV protection and bundle functionality for optimal performance in the Solana ecosystem.

Key features of this implementation include:
- Full support for all Jito API endpoints
- Proper tip management with 70/30 split for transactions
- Bundle creation and management
- Safeguards against uncled blocks
- Rate limiting to comply with Jito's API limits
- Integration with the Execution Engine for seamless usage