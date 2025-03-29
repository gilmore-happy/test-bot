# Jito Integration Dependency Issues

This document outlines the dependency issues encountered during the integration of Jito into the Solana HFT Bot and provides solutions to resolve them.

## Issues Encountered

1. **Solana SDK Version Conflict**:
   - `jito-tip-distribution` requires `solana-sdk = "=2.2.0"`
   - `jito-shredstream-proxy` requires `solana-sdk = "=2.2.1"`
   - This conflict prevents both dependencies from being used together

2. **Package Name Mismatch**:
   - The package name in the shredstream-proxy repository is `jito-shredstream-proxy`, not `jito-shredstream`
   - This caused issues when trying to import the package

3. **Tag Version Issues**:
   - The tag `v0.1.0` doesn't exist in the shredstream-proxy repository
   - We needed to use the `master` branch instead

## Solutions

### Short-term Solutions

1. **Use Feature Flags to Conditionally Include Dependencies**:
   ```rust
   [features]
   default = ["jito-mock"]  # Use mock by default
   jito = ["dep:jito-tip-distribution", "dep:jito-bundle", "dep:jito-searcher-client"]  # Basic Jito features
   jito-full = ["jito", "dep:jito-shredstream-proxy"]  # Full Jito features
   jito-mock = []  # Mock implementation
   ```

2. **Create Mock Implementations**:
   - Create mock implementations of the Jito services for testing
   - This allows development to continue without resolving all dependency issues

3. **Use Separate Test Scripts**:
   - `test-with-jito.sh`: Test with basic Jito features
   - `test-with-jito-full.sh`: Test with full Jito features
   - `test-without-jito.sh`: Test with mock implementations

### Long-term Solutions

1. **Fork and Align Dependencies**:
   - Fork the Jito repositories
   - Update them to use the same version of Solana SDK
   - Submit pull requests to the original repositories

2. **Use Cargo Patch**:
   - Use Cargo's patch feature to override the Solana SDK version
   - This requires careful management of the patch section in Cargo.toml

   ```toml
   [patch."https://github.com/jito-foundation/jito-solana.git"]
   solana-sdk = { git = "https://github.com/solana-labs/solana.git", tag = "v1.17.20" }

   [patch."https://github.com/jito-labs/shredstream-proxy.git"]
   solana-sdk = { git = "https://github.com/solana-labs/solana.git", tag = "v1.17.20" }
   ```

3. **Use Workspace Dependencies**:
   - Define all dependencies at the workspace level
   - Use `workspace = true` in crate-specific Cargo.toml files
   - This ensures all crates use the same version of dependencies

## Implementation Strategy

1. **Phase 1: Basic Integration**
   - Integrate basic Jito features (MEV bundles)
   - Use mock implementations for ShredStream
   - Test with `test-with-jito.sh`

2. **Phase 2: Full Integration**
   - Resolve dependency conflicts
   - Integrate ShredStream
   - Test with `test-with-jito-full.sh`

3. **Phase 3: Production Deployment**
   - Optimize performance
   - Add monitoring and metrics
   - Deploy to production

## Conclusion

The Jito integration is challenging due to dependency conflicts, but can be managed through careful use of feature flags, mock implementations, and eventually resolving the conflicts through forking or patching. The implementation strategy provides a path forward that allows development to continue while working towards a full integration.