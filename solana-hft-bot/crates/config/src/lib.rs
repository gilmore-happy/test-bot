//! Configuration system for Solana HFT Bot
//!
//! This module provides a flexible, hierarchical configuration system
//! that can detect hardware capabilities and adapt the bot's behavior accordingly.

pub mod error;
pub mod hardware;
pub mod loader;
pub mod manager;
pub mod network;
pub mod profiles;
pub mod schema;
pub mod sources;
pub mod utils;

pub use error::ConfigError;
pub use hardware::{HardwareCapabilities, HardwareFeatureFlags};
pub use loader::ConfigLoader;
pub use manager::ConfigurationManager;
pub use network::{EndpointManager, NetworkOptimizer};
pub use profiles::HardwareProfile;
pub use schema::BotConfig;