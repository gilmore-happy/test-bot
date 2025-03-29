                                                                                                                                                                                                                //! Kernel bypass networking module
//! 
//! This module provides implementations for kernel bypass networking
//! using technologies like DPDK and io_uring.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Kernel bypass mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BypassMode {
    /// No kernel bypass
    None,
    
    /// DPDK for kernel bypass
    Dpdk,
    
    /// io_uring for async I/O
    IoUring,
    
    /// Both DPDK and io_uring
    DpdkAndIoUring,
}

/// Kernel bypass configuration
#[derive(Debug, Clone)]
pub struct BypassConfig {
    /// Bypass mode
    pub mode: BypassMode,
    
    /// DPDK configuration
    pub dpdk_config: Option<super::dpdk::DpdkConfig>,
    
    /// io_uring configuration
    pub io_uring_config: Option<super::io_uring::IoUringConfig>,
}

impl Default for BypassConfig {
    fn default() -> Self {
        Self {
            mode: BypassMode::None,
            dpdk_config: None,
            io_uring_config: None,
        }
    }
}

/// Kernel bypass context
pub struct BypassContext {
    /// Configuration
    config: BypassConfig,
    
    /// DPDK context
    dpdk: Option<super::dpdk::Dpdk>,
    
    /// io_uring context
    io_uring: Option<super::io_uring::IoUring>,
}

impl BypassContext {
    /// Create a new kernel bypass context
    pub fn new(config: BypassConfig) -> Self {
        Self {
            config,
            dpdk: None,
            io_uring: None,
        }
    }
    
    /// Initialize kernel bypass
    pub fn init(&mut self) -> Result<()> {
        info!("Initializing kernel bypass with mode: {:?}", self.config.mode);
        
        match self.config.mode {
            BypassMode::None => {
                // No kernel bypass
                Ok(())
            }
            BypassMode::Dpdk => {
                // Initialize DPDK
                let dpdk_config = self.config.dpdk_config
                    .clone()
                    .unwrap_or_default();
                
                self.dpdk = Some(super::dpdk::Dpdk::new(dpdk_config)?);
                
                Ok(())
            }
            BypassMode::IoUring => {
                // Initialize io_uring
                let io_uring_config = self.config.io_uring_config
                    .clone()
                    .unwrap_or_default();
                
                self.io_uring = Some(super::io_uring::IoUring::new(io_uring_config)?);
                
                Ok(())
            }
            BypassMode::DpdkAndIoUring => {
                // Initialize both DPDK and io_uring
                let dpdk_config = self.config.dpdk_config
                    .clone()
                    .unwrap_or_default();
                
                let io_uring_config = self.config.io_uring_config
                    .clone()
                    .unwrap_or_default();
                
                self.dpdk = Some(super::dpdk::Dpdk::new(dpdk_config)?);
                self.io_uring = Some(super::io_uring::IoUring::new(io_uring_config)?);
                
                Ok(())
            }
        }
    }
    
    /// Get DPDK context
    pub fn dpdk(&self) -> Option<Arc<super::dpdk::DpdkContext>> {
        self.dpdk.as_ref().map(|dpdk| dpdk.context())
    }
    
    /// Get io_uring context
    pub fn io_uring(&self) -> Option<Arc<std::sync::Mutex<super::io_uring::IoUringContext>>> {
        self.io_uring.as_ref().map(|io_uring| io_uring.context())
    }
    
    /// Cleanup kernel bypass resources
    pub fn cleanup(&mut self) {
        info!("Cleaning up kernel bypass resources");
        
        // Drop DPDK and io_uring contexts
        self.dpdk = None;
        self.io_uring = None;
    }
}

impl Drop for BypassContext {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Kernel bypass manager
pub struct BypassManager {
    /// Context
    context: Arc<std::sync::Mutex<BypassContext>>,
}

impl BypassManager {
    /// Create a new kernel bypass manager
    pub fn new(config: BypassConfig) -> Result<Self> {
        let mut context = BypassContext::new(config);
        context.init()?;
        
        Ok(Self {
            context: Arc::new(std::sync::Mutex::new(context)),
        })
    }
    
    /// Get a reference to the kernel bypass context
    pub fn context(&self) -> Arc<std::sync::Mutex<BypassContext>> {
        self.context.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bypass_config_default() {
        let config = BypassConfig::default();
        assert_eq!(config.mode, BypassMode::None);
        assert!(config.dpdk_config.is_none());
        assert!(config.io_uring_config.is_none());
    }

    #[test]
    fn test_bypass_context_new() {
        let config = BypassConfig::default();
        let context = BypassContext::new(config);
        assert!(context.dpdk.is_none());
        assert!(context.io_uring.is_none());
    }
}
