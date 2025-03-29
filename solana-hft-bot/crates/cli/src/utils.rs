//! Utility functions for the CLI

use std::path::Path;
use std::fs;
use std::io::{self, Write};

use anyhow::{anyhow, Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Password, Select};
use indicatif::{ProgressBar, ProgressStyle};
use solana_hft_core::{BotConfig, ExecutionMode, LogLevel};
use tokio::time::Duration;

/// Load configuration from file
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<BotConfig> {
    let path = path.as_ref();
    
    if !path.exists() {
        return Err(anyhow!("Configuration file not found: {:?}", path));
    }
    
    let config_str = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {:?}", path))?;
    
    // Determine file format from extension
    let config = if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        serde_json::from_str(&config_str)
            .with_context(|| format!("Failed to parse JSON config: {:?}", path))?
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
        toml::from_str(&config_str)
            .with_context(|| format!("Failed to parse TOML config: {:?}", path))?
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("yaml") 
          || path.extension().and_then(|ext| ext.to_str()) == Some("yml") {
        serde_yaml::from_str(&config_str)
            .with_context(|| format!("Failed to parse YAML config: {:?}", path))?
    } else {
        return Err(anyhow!("Unsupported config file format: {:?}", path));
    };
    
    Ok(config)
}

/// Save configuration to file
pub fn save_config<P: AsRef<Path>>(config: &BotConfig, path: P) -> Result<()> {
    let path = path.as_ref();
    
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }
    }
    
    // Determine file format from extension and serialize accordingly
    let config_str = if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        serde_json::to_string_pretty(config)
            .with_context(|| "Failed to serialize config to JSON")?
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
        toml::to_string(config)
            .with_context(|| "Failed to serialize config to TOML")?
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("yaml") 
          || path.extension().and_then(|ext| ext.to_str()) == Some("yml") {
        serde_yaml::to_string(config)
            .with_context(|| "Failed to serialize config to YAML")?
    } else {
        return Err(anyhow!("Unsupported config file format: {:?}", path));
    };
    
    fs::write(path, config_str)
        .with_context(|| format!("Failed to write config to file: {:?}", path))?;
    
    Ok(())
}

/// Create a new configuration interactively
pub fn create_config_interactive() -> Result<BotConfig> {
    println!("{}", style("Solana HFT Bot Configuration").bold().cyan());
    println!("Let's set up your configuration...\n");
    
    let mut config = BotConfig::default();
    
    // Basic settings
    config.bot_name = Input::<String>::new()
        .with_prompt("Bot Name")
        .default(config.bot_name.clone())
        .interact_text()?;
    
    // Execution mode
    let execution_modes = vec![
        "Normal - Real trading with real funds",
        "Simulation - Simulated trading with no real transactions",
        "Analysis - Only analyze opportunities without executing trades",
        "Test - Test mode for development",
    ];
    
    let selected_mode = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select execution mode")
        .default(0)
        .items(&execution_modes)
        .interact()?;
    
    config.execution_mode = match selected_mode {
        0 => ExecutionMode::Normal,
        1 => ExecutionMode::Simulation,
        2 => ExecutionMode::Analysis,
        3 => ExecutionMode::Test,
        _ => ExecutionMode::Simulation, // Default to simulation if something goes wrong
    };
    
    // Risk level
    config.risk_level = Input::<u8>::new()
        .with_prompt("Risk Level (0-10)")
        .default(config.risk_level)
        .validate_with(|input: &u8| {
            if *input <= 10 {
                Ok(())
            } else {
                Err("Risk level must be between 0 and 10")
            }
        })
        .interact_text()?;
    
    // Log level
    let log_levels = vec![
        "Error - Only errors",
        "Warn - Warnings and errors",
        "Info - Informational messages (recommended)",
        "Debug - Detailed information for debugging",
        "Trace - Very verbose debugging information",
    ];
    
    let selected_log_level = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select log level")
        .default(2) // Default to Info
        .items(&log_levels)
        .interact()?;
    
    config.log_level = match selected_log_level {
        0 => LogLevel::Error,
        1 => LogLevel::Warn,
        2 => LogLevel::Info,
        3 => LogLevel::Debug,
        4 => LogLevel::Trace,
        _ => LogLevel::Info, // Default to Info if something goes wrong
    };
    
    // RPC endpoints
    let default_endpoint = config.rpc_endpoints.first().cloned().unwrap_or_default();
    let endpoint = Input::<String>::new()
        .with_prompt("Primary RPC Endpoint")
        .default(default_endpoint)
        .interact_text()?;
    
    config.rpc_endpoints = vec![endpoint];
    
    let add_more = Confirm::new()
        .with_prompt("Add additional RPC endpoints?")
        .default(false)
        .interact()?;
    
    if add_more {
        loop {
            let endpoint = Input::<String>::new()
                .with_prompt("Additional RPC Endpoint (leave empty to finish)")
                .allow_empty(true)
                .interact_text()?;
            
            if endpoint.is_empty() {
                break;
            }
            
            config.rpc_endpoints.push(endpoint);
        }
    }
    
    // Strategies
    let available_strategies = vec![
        "Triangular Arbitrage",
        "Cross-Exchange Arbitrage",
        "Circular Arbitrage",
        "Market Making",
        "Liquidity Provision",
    ];
    
    let selected_strategies = MultiSelect::new()
        .with_prompt("Select strategies to enable")
        .items(&available_strategies)
        .defaults(&[true, true, false, false, false])
        .interact()?;
    
    config.enabled_strategies = selected_strategies.iter()
        .map(|&i| available_strategies[i].to_string())
        .collect();
    
    println!("\n{}", style("Configuration created successfully!").green());
    
    Ok(config)
}

/// Create a spinner with the specified message
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    
    pb
}

/// Print a success message
pub fn print_success(message: &str) {
    println!("{} {}", style("✓").green().bold(), message);
}

/// Print an error message
pub fn print_error(message: &str) {
    eprintln!("{} {}", style("✗").red().bold(), message);
}

/// Print a warning message
pub fn print_warning(message: &str) {
    println!("{} {}", style("!").yellow().bold(), message);
}

/// Print an info message
pub fn print_info(message: &str) {
    println!("{} {}", style("i").blue().bold(), message);
}

/// Prompt for confirmation with yes/no
pub fn confirm(message: &str, default: bool) -> Result<bool> {
    Ok(Confirm::new()
        .with_prompt(message)
        .default(default)
        .interact()?)
}

/// Get a secure password from the user
pub fn get_password(prompt: &str) -> Result<String> {
    Ok(Password::new()
        .with_prompt(prompt)
        .with_confirmation("Confirm password", "Passwords don't match")
        .interact()?)
}

/// Clear the terminal screen
pub fn clear_screen() -> Result<()> {
    // Cross-platform clear screen
    if cfg!(windows) {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "cls"])
            .status();
    } else {
        let _ = std::process::Command::new("clear")
            .status();
    }
    
    // Fallback if the command fails
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush()?;
    
    Ok(())
}