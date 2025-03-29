# Alternatives to Git Repository Dependencies for Jito Integration

## Current Approach
Currently, the project uses Git repository dependencies for Jito components:

```toml
# Jito MEV libraries - using specific commits for compatibility
jito-tip-distribution = { git = "https://github.com/jito-foundation/jito-solana.git", rev = "d7f139c", package = "solana-tip-distributor" }
jito-bundle = { git = "https://github.com/jito-labs/jito-rust-rpc.git", rev = "81ebd23", package = "jito-sdk-rust" }
jito-searcher-client = { git = "https://github.com/jito-labs/searcher-examples.git", rev = "51fb639", package = "jito-searcher-client" }
jito-shredstream-proxy = { git = "https://github.com/jito-labs/shredstream-proxy.git", branch = "master", package = "jito-shredstream-proxy" }
```

This approach has several drawbacks:
- Requires network access during builds
- Can lead to build failures if repositories change or become unavailable
- Complicates CI/CD pipelines and offline builds

## Alternative 1: Vendoring Dependencies

The most reliable alternative is to vendor the dependencies by copying the source code directly into your project:

### Implementation Steps:

1. Create a vendor directory structure:
```bash
mkdir -p solana-hft-bot/vendor/jito
```

2. Clone the repositories locally at the specific commits:
```bash
git clone https://github.com/jito-foundation/jito-solana.git temp-jito-solana
cd temp-jito-solana
git checkout d7f139c
cd ..

git clone https://github.com/jito-labs/jito-rust-rpc.git temp-jito-rust-rpc
cd temp-jito-rust-rpc
git checkout 81ebd23
cd ..

git clone https://github.com/jito-labs/searcher-examples.git temp-searcher-examples
cd temp-searcher-examples
git checkout 51fb639
cd ..
```

3. Copy the relevant code to your vendor directory:
```bash
cp -r temp-jito-solana/programs/tip-distributor solana-hft-bot/vendor/jito/tip-distributor
cp -r temp-jito-rust-rpc/sdk solana-hft-bot/vendor/jito/bundle
cp -r temp-searcher-examples/searcher-client solana-hft-bot/vendor/jito/searcher-client
```

4. Update your Cargo.toml to use path dependencies:
```toml
[dependencies]
jito-tip-distribution = { path = "vendor/jito/tip-distributor", optional = true }
jito-bundle = { path = "vendor/jito/bundle", optional = true }
jito-searcher-client = { path = "vendor/jito/searcher-client", optional = true }
```

5. Clean up the temporary repositories:
```bash
rm -rf temp-jito-solana temp-jito-rust-rpc temp-searcher-examples
```

### Advantages:
- No network dependency during builds
- Complete control over the code
- Stable builds regardless of external repository changes
- Easier to audit and review the code

### Disadvantages:
- Increases repository size
- Manual process to update dependencies
- May need to modify the vendored code to fix any compilation issues

## Alternative 2: Enhanced Mock Implementation

Expand the existing mock implementation to fully replicate the Jito API surface:

### Implementation Steps:

1. Create comprehensive mock implementations for all Jito types:

```rust
// In jito_mock.rs
pub struct JitoClient {
    // Mock implementation fields
    config: JitoConfig,
    rpc_client: Arc<RpcClient>,
}

impl JitoClient {
    pub fn new(config: JitoConfig, rpc_client: Arc<RpcClient>) -> Self {
        Self { config, rpc_client }
    }
    
    pub async fn submit_bundle(&self, transactions: Vec<Transaction>) -> Result<BundleReceipt, ClientError> {
        // Mock implementation that simulates success
        Ok(BundleReceipt {
            uuid: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            status: BundleStatus::Received,
        })
    }
    
    // Implement other methods...
}

// Mock all other necessary types
pub struct BundleReceipt {
    pub uuid: String,
    pub timestamp: u64,
    pub status: BundleStatus,
}

pub enum BundleStatus {
    Received,
    Pending,
    Confirmed,
    Failed,
}

// ... other mock types
```

2. Update your feature flags to use the mock implementation:

```toml
[features]
default = ["jito-mock"]  # Use mock by default
jito = ["dep:jito-tip-distribution", "dep:jito-bundle", "dep:jito-searcher-client"]  # Real implementation
jito-mock = []  # Mock implementation
```

### Advantages:
- No external dependencies
- Complete control over behavior
- Can simulate various scenarios for testing
- Faster compilation times

### Disadvantages:
- Not using the actual Jito code
- Need to keep mock implementation in sync with Jito API changes
- May not accurately represent real-world behavior

## Alternative 3: Git Submodules

Use Git submodules to manage the dependencies locally:

### Implementation Steps:

1. Add the repositories as submodules:
```bash
git submodule add https://github.com/jito-foundation/jito-solana.git external/jito-solana
git submodule add https://github.com/jito-labs/jito-rust-rpc.git external/jito-rust-rpc
git submodule add https://github.com/jito-labs/searcher-examples.git external/searcher-examples
```

2. Checkout the specific commits:
```bash
cd external/jito-solana
git checkout d7f139c
cd ../..

cd external/jito-rust-rpc
git checkout 81ebd23
cd ../..

cd external/searcher-examples
git checkout 51fb639
cd ../..
```

3. Update your Cargo.toml to use path dependencies:
```toml
[dependencies]
jito-tip-distribution = { path = "external/jito-solana/programs/tip-distributor", optional = true }
jito-bundle = { path = "external/jito-rust-rpc/sdk", optional = true }
jito-searcher-client = { path = "external/searcher-examples/searcher-client", optional = true }
```

### Advantages:
- Explicit version control
- Easier to update dependencies
- Doesn't require copying code
- Works well with Git workflows

### Disadvantages:
- Increases repository complexity
- Requires additional Git knowledge
- Can be tricky to set up in CI/CD pipelines

## Alternative 4: Simplified Interface Approach

Create a simplified interface that only exposes the functionality you actually need:

### Implementation Steps:

1. Define a minimal interface for Jito functionality:

```rust
// In jito_interface.rs
pub trait JitoInterface {
    fn submit_bundle(&self, transactions: Vec<Transaction>) -> Result<BundleReceipt, ClientError>;
    // Other methods you need...
}

// Factory function to create the appropriate implementation
pub fn create_jito_client(config: &Config) -> Box<dyn JitoInterface> {
    #[cfg(feature = "jito")]
    {
        Box::new(RealJitoClient::new(config))
    }
    
    #[cfg(not(feature = "jito"))]
    {
        Box::new(MockJitoClient::new(config))
    }
}
```

2. Implement the interface for both real and mock implementations:

```rust
// Real implementation when jito feature is enabled
#[cfg(feature = "jito")]
pub struct RealJitoClient {
    inner: jito_bundle::BundleClient,
}

#[cfg(feature = "jito")]
impl JitoInterface for RealJitoClient {
    fn submit_bundle(&self, transactions: Vec<Transaction>) -> Result<BundleReceipt, ClientError> {
        // Implement using real Jito dependencies
    }
}

// Mock implementation otherwise
#[cfg(not(feature = "jito"))]
pub struct MockJitoClient {
    // Mock implementation
}

#[cfg(not(feature = "jito"))]
impl JitoInterface for MockJitoClient {
    fn submit_bundle(&self, transactions: Vec<Transaction>) -> Result<BundleReceipt, ClientError> {
        // Implement with mock behavior
    }
}
```

### Advantages:
- Minimizes the surface area where you need to handle conditional compilation
- Cleaner code organization
- Easier to test
- Can work with any of the other approaches

### Disadvantages:
- May not expose all functionality
- Requires additional abstraction layer

## Recommendation

The **vendoring approach (Alternative 1)** combined with the **simplified interface approach (Alternative 4)** provides the most reliable solution:

1. Vendor the dependencies to eliminate external repository dependencies
2. Create a simplified interface to abstract away the implementation details
3. Use feature flags to conditionally compile the real or mock implementation

This approach gives you:
- Reliable builds without network dependencies
- Clean abstraction of Jito functionality
- Flexibility to use either real or mock implementations
- Better control over the codebase

## Implementation Plan

1. Vendor the dependencies as described in Alternative 1
2. Create the simplified interface as described in Alternative 4
3. Update the feature flags to support both real and mock implementations
4. Update the codebase to use the interface instead of direct Jito types

This approach will resolve the dependency issues while maintaining the flexibility to use either the real Jito code or a mock implementation.