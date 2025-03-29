//! Mock implementation for Jito module
//!
//! This module provides mock implementations for Jito types that can be used
//! when the Jito feature is not enabled or when the jito-mock feature is enabled.
//! This allows the code to compile and run without the Jito dependencies.

// Stub module for Jito functionality
pub mod stub {
    use solana_sdk::signature::Signature;
    use solana_sdk::transaction::Transaction;
    use crate::ExecutionError;

    /// Stub function for submitting a transaction when Jito is not available
    pub async fn submit_transaction(
        transaction: &Transaction,
    ) -> Result<Signature, ExecutionError> {
        Err(ExecutionError::InvalidConfig("Jito support is not enabled. Enable the 'jito' feature to use Jito MEV bundles.".to_string()))
    }
}