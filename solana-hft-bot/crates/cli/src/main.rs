//! Command Line Interface for Solana HFT Bot
//!
//! This module provides a user-friendly CLI for managing the HFT bot:
//! - Start/stop the bot
//! - Configure settings
//! - View performance metrics
//! - Manage strategies
//! - Check status and logs

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use solana_hft_core::{
    BotConfig, BotStatus, ExecutionMode, HftBot, LogLevel,
    init_logging, init as init_core
};
use tokio::sync::Mutex;
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod display;
mod interactive;
mod utils;

use display::{DisplayFormat, Renderer};
use interactive::InteractiveShell;
use utils::{load_config, save_config, create_config_interactive, create_spinner, print_success, print_error};

/// Solana HFT Bot CLI
#[derive(Parser, Debug)]
#[clap(name = "solana-hft-bot", version, about, long_about = None)]
struct Cli {
    /// Sets a custom config file
    #[clap(short, long, value_name = "FILE", default_value = "config.json")]
    config: PathBuf,
    
    /// Sets log level
    #[clap(short, long, value_name = "LEVEL", default_value = "info")]
    log_level: String,
    
    /// Subcommand to execute
    #[clap(subcommand)]
    command: Option<Commands>,
}

/// CLI commands
#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the HFT bot
    Start {
        /// Run in simulation mode (no real transactions)
        #[clap(short, long)]
        simulation: bool,
        
        /// Run in analysis mode (only analyze opportunities)
        #[clap(short, long)]
        analysis: bool,
    },
    
    /// Stop the HFT bot
    Stop,
    
    /// Show current status
    Status {
        /// Output format
        #[clap(short, long, value_enum, default_value = "text")]
        format: DisplayFormat,
    },
    
    /// Show performance metrics
    Metrics {
        /// Output format
        #[clap(short, long, value_enum, default_value = "text")]
        format: DisplayFormat,
    },
    
    /// Manage trading strategies
    Strategy {
        /// Strategy subcommand
        #[clap(subcommand)]
        command: StrategyCommands,
    },
    
    /// Interactive shell
    Shell,
    
    /// Generate a default configuration
    GenerateConfig {
        /// Output file
        #[clap(short, long, value_name = "FILE")]
        output: PathBuf,
        
        /// Create interactively with prompts
        #[clap(short, long)]
        interactive: bool,
    },
    
    /// Initialize all modules
    Init,
}

/// Strategy management commands
#[derive(Subcommand, Debug)]
enum StrategyCommands {
    /// List all strategies
    List {
        /// Output format
        #[clap(short, long, value_enum, default_value = "text")]
        format: DisplayFormat,
    },
    
    /// Enable a strategy
    Enable {
        /// Strategy ID
        #[clap(value_name = "STRATEGY_ID")]
        id: String,
    },
    
    /// Disable a strategy
    Disable {
        /// Strategy ID
        #[clap(value_name = "STRATEGY_ID")]
        id: String,
    },
    
    /// Show detailed information about a strategy
    Info {
        /// Strategy ID
        #[clap(value_name = "STRATEGY_ID")]
        id: String,
        
        /// Output format
        #[clap(short, long, value_enum, default_value = "text")]
        format: DisplayFormat,
    },
    
    /// Run a specific strategy once
    Run {
        /// Strategy ID
        #[clap(value_name = "STRATEGY_ID")]
        id: String,
        
        /// Strategy parameters as JSON
        #[clap(short, long)]
        params: Option<String>,
    },
}

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Initialize logging
    initialize_logging(&cli.log_level)?;
    
    // Load configuration
    let config_path = &cli.config;
    
    let bot = if let Some(Commands::GenerateConfig { output, interactive }) = &cli.command {
        // Generate default config
        let config = if *interactive {
            create_config_interactive()?
        } else {
            BotConfig::default()
        };
        
        save_config(&config, output)?;
        
        print_success(&format!("Configuration generated: {:?}", output));
        return Ok(());
    } else if config_path.exists() {
        // Create bot from config
        info!("Loading configuration from {:?}", config_path);
        let config = load_config(config_path)?;
        Arc::new(Mutex::new(HftBot::new(config).await?))
    } else {
        // Config doesn't exist - prompt to create one
        if utils::confirm(&format!("Configuration file {:?} not found. Create a default one?", config_path), true)? {
            let config = create_config_interactive()?;
            save_config(&config, config_path)?;
            
            print_success(&format!("Default configuration created: {:?}", config_path));
            
            Arc::new(Mutex::new(HftBot::new(config).await?))
        } else {
            return Err(anyhow!("Configuration file not found: {:?}", config_path));
        }
    };
    
    // Initialize all modules
    initialize_modules();
    
    // Process command
    match &cli.command {
        Some(cmd) => process_command(cmd, bot).await?,
        None => {
            // If no command specified, show help
            Cli::parse_from(vec!["solana-hft-bot", "--help"]);
        }
    }
    
    Ok(())
}

/// Initialize all modules
fn initialize_modules() {
    // Initialize core module
    init_core();
    
    // Initialize other modules
    solana_hft_network::init();
    solana_hft_execution::init();
    solana_hft_arbitrage::init();
    solana_hft_monitoring::init();
    solana_hft_screening::init();
    solana_hft_risk::init();
    solana_hft_rpc::init();
    solana_hft_security::init();
    solana_hft_metrics::init();
    
    info!("All modules initialized successfully");
}

/// Initialize logging with the specified level
fn initialize_logging(level: &str) -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("solana_hft={}", level)));
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();
    
    Ok(())
}

/// Process CLI command
async fn process_command(command: &Commands, bot: Arc<Mutex<HftBot>>) -> Result<()> {
    match command {
        Commands::Start { simulation, analysis } => {
            // Initialize a progress bar
            let pb = create_spinner("Starting HFT Bot...");
            
            // Set execution mode based on flags
            let mode = if *simulation {
                ExecutionMode::Simulation
            } else if *analysis {
                ExecutionMode::Analysis
            } else {
                ExecutionMode::Normal
            };
            
            // Update progress message
            let mode_str = match mode {
                ExecutionMode::Normal => "Normal Mode",
                ExecutionMode::Simulation => "Simulation Mode",
                ExecutionMode::Analysis => "Analysis Mode",
                ExecutionMode::Test => "Test Mode",
            };
            pb.set_message(format!("Starting HFT Bot ({})...", mode_str));
            
            // Set execution mode and start the bot
            {
                let mut bot = bot.lock().await;
                bot.set_execution_mode(mode);
                bot.start().await?;
            }
            
            pb.finish_with_message(format!("HFT Bot started successfully in {}!", mode_str));
            
            // Keep running until user presses Ctrl+C
            println!("Press Ctrl+C to stop the bot");
            
            // Wait for Ctrl+C
            tokio::signal::ctrl_c().await?;
            
            // Stop the bot
            let mut bot = bot.lock().await;
            bot.stop().await?;
            
            print_success("HFT Bot stopped");
        },
        Commands::Stop => {
            let pb = create_spinner("Stopping HFT Bot...");
            
            let mut bot = bot.lock().await;
            bot.stop().await?;
            
            pb.finish_with_message("HFT Bot stopped successfully");
        },
        Commands::Status { format } => {
            let bot = bot.lock().await;
            let status = bot.get_status();
            let execution_mode = bot.get_execution_mode();
            
            // Render status
            let renderer = Renderer::new(*format);
            renderer.render_status(status, execution_mode);
        },
        Commands::Metrics { format } => {
            let bot = bot.lock().await;
            let metrics = bot.get_metrics();
            
            // Render metrics
            let renderer = Renderer::new(*format);
            renderer.render_metrics(&metrics);
        },
        Commands::Strategy { command } => {
            process_strategy_command(command, bot).await?;
        },
        Commands::Shell => {
            // Run interactive shell
            let mut shell = InteractiveShell::new(bot);
            shell.run().await?;
        },
        Commands::GenerateConfig { .. } => {
            // Already handled above
        },
        Commands::Init => {
            print_success("All modules initialized successfully");
        },
    }
    
    Ok(())
}

/// Process strategy subcommand
async fn process_strategy_command(command: &StrategyCommands, bot: Arc<Mutex<HftBot>>) -> Result<()> {
    match command {
        StrategyCommands::List { format } => {
            let bot = bot.lock().await;
            let strategies = bot.get_strategies();
            
            // Render strategies
            let renderer = Renderer::new(*format);
            renderer.render_strategies(&strategies);
        },
        StrategyCommands::Enable { id } => {
            let bot = bot.lock().await;
            match bot.enable_strategy(id) {
                Ok(_) => print_success(&format!("Strategy {} enabled", id)),
                Err(e) => print_error(&format!("Failed to enable strategy: {}", e)),
            }
        },
        StrategyCommands::Disable { id } => {
            let bot = bot.lock().await;
            match bot.disable_strategy(id) {
                Ok(_) => print_success(&format!("Strategy {} disabled", id)),
                Err(e) => print_error(&format!("Failed to disable strategy: {}", e)),
            }
        },
        StrategyCommands::Info { id, format } => {
            let bot = bot.lock().await;
            
            match bot.get_strategy_by_id(id) {
                Some(strategy) => {
                    match format {
                        DisplayFormat::Text => {
                            println!("\n{} {}", style("Strategy:").bold(), style(&strategy.name()).cyan());
                            println!("{} {}", style("ID:").bold(), strategy.id());
                            println!("{} {}", style("Type:").bold(), strategy.strategy_type());
                            println!("{} {}", style("Enabled:").bold(),
                                if strategy.is_enabled() { style("Yes").green() } else { style("No").red() });
                            
                            // Show strategy parameters if available
                            if let Some(params) = strategy.get_parameters() {
                                println!("\n{}", style("Parameters:").bold());
                                for (key, value) in params {
                                    println!("  {}: {}", style(key).bold(), value);
                                }
                            }
                            
                            // Show strategy performance if available
                            if let Some(perf) = strategy.get_performance_stats() {
                                println!("\n{}", style("Performance:").bold());
                                println!("  {}: {}", style("Executions").bold(), perf.executions);
                                println!("  {}: {:.2}%", style("Success Rate").bold(), perf.success_rate * 100.0);
                                println!("  {}: {:.6} SOL", style("Total Profit").bold(), perf.total_profit);
                                println!("  {}: {:.6} SOL", style("Average Profit").bold(), perf.avg_profit);
                                println!("  {}: {:.2} ms", style("Average Execution Time").bold(), perf.avg_execution_time_ms);
                            }
                        },
                        DisplayFormat::Json => {
                            let output = serde_json::json!({
                                "id": strategy.id(),
                                "name": strategy.name(),
                                "type": strategy.strategy_type(),
                                "enabled": strategy.is_enabled(),
                                "parameters": strategy.get_parameters(),
                                "performance": strategy.get_performance_stats(),
                            });
                            println!("{}", serde_json::to_string_pretty(&output).unwrap());
                        },
                        DisplayFormat::Yaml => {
                            let output = serde_json::json!({
                                "id": strategy.id(),
                                "name": strategy.name(),
                                "type": strategy.strategy_type(),
                                "enabled": strategy.is_enabled(),
                                "parameters": strategy.get_parameters(),
                                "performance": strategy.get_performance_stats(),
                            });
                            println!("{}", serde_yaml::to_string(&output).unwrap());
                        },
                    }
                },
                None => print_error(&format!("Strategy with ID '{}' not found", id)),
            }
        },
        StrategyCommands::Run { id, params } => {
            let pb = create_spinner(&format!("Running strategy {}...", id));
            
            let bot = bot.lock().await;
            let result = match params {
                Some(params_json) => {
                    let params: serde_json::Value = serde_json::from_str(params_json)?;
                    bot.run_strategy_once(id, Some(&params)).await
                },
                None => bot.run_strategy_once(id, None).await,
            };
            
            match result {
                Ok(outcome) => {
                    pb.finish_with_message(format!("Strategy {} executed successfully", id));
                    println!("\n{}", style("Execution Result:").bold());
                    println!("  {}: {}", style("Success").bold(),
                        if outcome.success { style("Yes").green() } else { style("No").red() });
                    println!("  {}: {:.6} SOL", style("Profit").bold(), outcome.profit);
                    println!("  {}: {:.2} ms", style("Execution Time").bold(), outcome.execution_time_ms);
                    
                    if let Some(details) = outcome.details {
                        println!("\n{}", style("Details:").bold());
                        println!("{}", serde_json::to_string_pretty(&details).unwrap());
                    }
                },
                Err(e) => {
                    pb.finish_with_message(format!("Strategy {} execution failed", id));
                    print_error(&format!("Error: {}", e));
                },
            }
        },
    }
    
    Ok(())
}