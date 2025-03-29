use serde::{Deserialize, Serialize};

/// Bot status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BotStatus {
    /// Bot is initializing
    Initializing,
    
    /// Bot is starting
    Starting,
    
    /// Bot is running
    Running,
    
    /// Bot is paused
    Paused,
    
    /// Bot is stopping
    Stopping,
    
    /// Bot is stopped
    Stopped,
    
    /// Bot has encountered an error
    Error,
}

/// Module status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleStatus {
    /// Module is not initialized
    Uninitialized,
    
    /// Module is initializing
    Initializing,
    
    /// Module is initialized
    Initialized,
    
    /// Module is starting
    Starting,
    
    /// Module is running
    Running,
    
    /// Module is stopping
    Stopping,
    
    /// Module is stopped
    Stopped,
    
    /// Module has encountered an error
    Error,
}