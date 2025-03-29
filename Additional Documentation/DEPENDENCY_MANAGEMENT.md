# Dependency Management in Solana HFT Bot

This document explains the multi-repository structure of the Solana HFT Bot project and how external dependencies are managed, with a particular focus on Jito MEV integration.

## Multi-Repository Structure

The Solana HFT Bot relies on several external repositories, particularly for Jito MEV integration. Understanding this structure is crucial for development and maintenance.

### Jito Dependencies

Jito functionality is split across multiple repositories:

1. **jito-foundation/jito-solana**
   - Contains the `tip-distributor` package used for tip distribution
   - Forked from the main Solana repository with MEV-specific enhancements
   - Used for: Tip calculation and distribution to validators

2. **jito-foundation/jito-rust-rpc**
   - Contains the `jito-bundle` package for bundle creation and submission
   - Used for: Creating and submitting MEV bundles to Jito's infrastructure

3. **jito-foundation/jito-labs**
   - Contains the `searcher_client` package for interacting with Jito's searcher API
   - Used for: Communicating with Jito's MEV infrastructure as a searcher

### Version Management Strategy

To ensure stability and reproducibility, we use specific commit hashes rather than branches for all external dependencies:

```toml
jito-tip-distribution = { git = "https://github.com/jito-foundation/jito-solana.git", branch = "master", package = "tip-distributor" }
jito-bundle = { git = "https://github.com/jito-foundation/jito-rust-rpc.git", branch = "master" }
jito-searcher-client = { git = "https://github.com/jito-foundation/jito-labs.git", branch = "master", package = "searcher_client" }
```

For production use, it's recommended to pin these dependencies to specific commit hashes once stable versions are identified:

```toml
jito-tip-distribution = { git = "https://github.com/jito-foundation/jito-solana.git", rev = "specific-commit-hash", package = "tip-distributor" }
```

This approach provides several benefits:

- Ensures consistent builds across different environments
- Prevents unexpected changes when upstream repositories update
- Makes it easier to track which versions of dependencies are being used
- Allows for controlled updates of dependencies

## Feature Flags

The project uses feature flags to conditionally include Jito functionality:

### Workspace Features

In the root `Cargo.toml`:

```toml
[workspace.features]
default = []
jito = ["jito-tip-distribution", "jito-bundle", "jito-searcher-client"]
```

### Crate-Specific Features

In the execution crate's `Cargo.toml`:

```toml
[features]
default = ["jito-mock"]  # Use mock by default for easier development
jito = ["dep:jito-tip-distribution", "dep:jito-bundle", "dep:jito-searcher-client"]
jito-mock = []  # Use mock implementations instead of actual Jito dependencies
```

## Mock Implementations

For development and testing without Jito dependencies, we provide mock implementations:

- `jito_mock.rs`: Mock implementations of Jito types
- `jito_optimizer_mock.rs`: Mock implementation of the Jito optimizer

These mocks are conditionally included using the `jito-mock` feature:

```rust
#[cfg(any(not(feature = "jito"), feature = "jito-mock"))]
mod jito_mock;
```

## Testing Strategy

We have separate test scripts for testing with and without Jito dependencies:

- `test-with-jito.sh`/`test-with-jito.ps1`: Tests with real Jito dependencies
- `test-without-jito.sh`/`test-without-jito.ps1`: Tests with mock implementations

## Updating Dependencies

When updating dependencies, follow these steps:

1. Identify the specific commit hash to use for each repository
2. Update the `rev` values in `Cargo.toml`
3. Run both test scripts to ensure compatibility
4. Document the changes in the commit message

## Troubleshooting

Common issues with the multi-repository structure:

1. **Compilation Errors**: Ensure you're using the correct feature flags
2. **Missing Types**: Check that you're importing from the correct modules
3. **Version Mismatches**: Verify that the commit hashes are compatible with each other
