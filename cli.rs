// crates/cli/src/main.rs
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
use clap::{Parser, Subcommand};
use console::{style, Term};
use dialoguer::{Confirm, Input, MultiSelect, Password, Select};
use indicatif::{ProgressBar, ProgressStyle};
use solana_hft_core::{BotConfig, DefaultConfigs, ExecutionMode, HftBot};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod display;
mod interactive;
mod utils;

use display::{DisplayFormat, Renderer};
use interactive::InteractiveShell;
use utils::load_config;

/// Solana HFT Bot CLI
#[derive(Parser, Debug)]
#[clap(name = "solana-hft-bot", version, about, long_about = None)]
struct Cli {
    /// Sets a custom config file
    #[clap(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    
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
    },
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
}

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Initialize logging
    initialize_logging(&cli.log_level)?;
    
    // Load configuration
    let config_path = cli.config
        .unwrap_or_else(|| PathBuf::from("config.json"));
    
    let bot = if let Some(Commands::GenerateConfig { output }) = &cli.command {
        // Generate default config
        let default_config = BotConfig::default();
        let config_json = serde_json::to_string_pretty(&default_config)?;
        std::fs::write(output, config_json)?;
        
        println!("Default configuration generated: {:?}", output);
        return Ok(());
    } else if config_path.exists() {
        // Create bot from config
        Arc::new(Mutex::new(HftBot::new(config_path).await?))
    } else {
        // Config doesn't exist - prompt to create one
        if Confirm::new()
            .with_prompt(format!("Configuration file {:?} not found. Create a default one?", config_path))
            .interact()?
        {
            let default_config = BotConfig::default();
            let config_json = serde_json::to_string_pretty(&default_config)?;
            std::fs::write(&config_path, config_json)?;
            
            println!("Default configuration created: {:?}", config_path);
            
            Arc::new(Mutex::new(HftBot::new(config_path).await?))
        } else {
            return Err(anyhow!("Configuration file not found: {:?}", config_path));
        }
    };
    
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
        Commands::Start { simulation } => {
            // Initialize a progress bar
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            
            pb.set_message("Starting HFT Bot...");
            pb.enable_steady_tick(Duration::from_millis(100));
            
            // Set execution mode if simulation is enabled
            if *simulation {
                let mut bot = bot.lock().await;
                bot.set_execution_mode(ExecutionMode::Simulation);
                pb.set_message("Starting HFT Bot (Simulation Mode)...");
            }
            
            // Start the bot
            {
                let mut bot = bot.lock().await;
                bot.start().await?;
            }
            
            pb.finish_with_message(format!("HFT Bot started successfully{}!", 
                if *simulation { " in simulation mode" } else { "" }));
            
            // Keep running until user presses Ctrl+C
            println!("Press Ctrl+C to stop the bot");
            
            // Wait for Ctrl+C
            tokio::signal::ctrl_c().await?;
            
            // Stop the bot
            let mut bot = bot.lock().await;
            bot.stop().await?;
            
            println!("HFT Bot stopped");
        },
        Commands::Stop => {
            let mut bot = bot.lock().await;
            bot.stop().await?;
            println!("HFT Bot stopped");
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
            renderer.render_metrics(metrics);
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
    }
    
    Ok(())
}

/// Process strategy subcommand
async fn process_strategy_command(command: &StrategyCommands, bot: Arc<Mutex<HftBot>>) -> Result<()> {
    match command {
        StrategyCommands::List { format } => {
            let bot = bot.lock().await;
            let strategy_manager = bot.strategy_manager.clone();
            let strategies = strategy_manager.get_strategies();
            
            // Render strategies
            let renderer = Renderer::new(*format);
            renderer.render_strategies(strategies);
        },
        StrategyCommands::Enable { id } => {
            let bot = bot.lock().await;
            let strategy_manager = bot.strategy_manager.clone();
            strategy_manager.enable_strategy(id)?;
            
            println!("Strategy {} enabled", id);
        },
        StrategyCommands::Disable { id } => {
            let bot = bot.lock().await;
            let strategy_manager = bot.strategy_manager.clone();
            strategy_manager.disable_strategy(id)?;
            
            println!("Strategy {} disabled", id);
        },
    }
    
    Ok(())
}

// crates/cli/src/display.rs
//! Display utilities for rendering output

use clap::ValueEnum;
use solana_hft_core::{
    BotStatus, CoreMetricsSnapshot, ExecutionMode, Strategy, 
};
use std::sync::Arc;
use tabled::{
    Table, Tabled,
    settings::{
        object::Columns,
        formatting::Alignment,
        style::{Border, Style},
    },
};

/// Output format for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DisplayFormat {
    /// Text format (tables)
    Text,
    
    /// JSON format
    Json,
    
    /// YAML format
    Yaml,
}

/// Renderer for formatting output
pub struct Renderer {
    /// Output format
    format: DisplayFormat,
}

impl Renderer {
    /// Create a new renderer
    pub fn new(format: DisplayFormat) -> Self {
        Self { format }
    }
    
    /// Render bot status
    pub fn render_status(&self, status: BotStatus, mode: ExecutionMode) {
        match self.format {
            DisplayFormat::Text => {
                println!("Bot Status: {}", Self::format_status(status));
                println!("Execution Mode: {}", Self::format_execution_mode(mode));
            },
            DisplayFormat::Json => {
                let output = serde_json::json!({
                    "status": status,
                    "executionMode": mode,
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            },
            DisplayFormat::Yaml => {
                let output = serde_json::json!({
                    "status": status,
                    "executionMode": mode,
                });
                println!("{}", serde_yaml::to_string(&output).unwrap());
            },
        }
    }
    
    /// Render metrics
    pub fn render_metrics(&self, metrics: CoreMetricsSnapshot) {
        match self.format {
            DisplayFormat::Text => {
                // Create a table structure
                #[derive(Tabled)]
                struct MetricsRow {
                    #[tabled(rename = "Metric")]
                    name: String,
                    #[tabled(rename = "Value")]
                    value: String,
                }
                
                let rows = vec![
                    MetricsRow { name: "Signals Processed".to_string(), value: metrics.signals_processed.to_string() },
                    MetricsRow { name: "Signals Rejected".to_string(), value: metrics.signals_rejected.to_string() },
                    MetricsRow { name: "Strategies Run".to_string(), value: metrics.strategies_run.to_string() },
                    MetricsRow { name: "Strategy Errors".to_string(), value: metrics.strategy_errors.to_string() },
                    MetricsRow { name: "Avg Strategy Time (ms)".to_string(), value: metrics.avg_strategy_time_ms.to_string() },
                    MetricsRow { name: "Signal Queue Size".to_string(), value: metrics.signal_queue_size.to_string() },
                ];
                
                let table = Table::new(rows)
                    .with(Style::modern())
                    .with(Columns::single().set_alignment(Alignment::center()));
                
                println!("{}", table);
                
                // Render signals by strategy
                if !metrics.signals_by_strategy.is_empty() {
                    println!("\nSignals by Strategy:");
                    
                    #[derive(Tabled)]
                    struct StrategyRow {
                        #[tabled(rename = "Strategy")]
                        name: String,
                        #[tabled(rename = "Signals")]
                        signals: u64,
                    }
                    
                    let strategy_rows = metrics.signals_by_strategy
                        .iter()
                        .map(|(strategy, count)| StrategyRow {
                            name: strategy.clone(),
                            signals: *count,
                        })
                        .collect::<Vec<_>>();
                    
                    let table = Table::new(strategy_rows)
                        .with(Style::modern())
                        .with(Columns::single().set_alignment(Alignment::center()));
                    
                    println!("{}", table);
                }
            },
            DisplayFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&metrics).unwrap());
            },
            DisplayFormat::Yaml => {
                println!("{}", serde_yaml::to_string(&metrics).unwrap());
            },
        }
    }
    
    /// Render strategies
    pub fn render_strategies(&self, strategies: Vec<Arc<dyn Strategy>>) {
        match self.format {
            DisplayFormat::Text => {
                #[derive(Tabled)]
                struct StrategyRow {
                    #[tabled(rename = "ID")]
                    id: String,
                    #[tabled(rename = "Name")]
                    name: String,
                    #[tabled(rename = "Enabled")]
                    enabled: String,
                }
                
                let rows = strategies
                    .iter()
                    .map(|s| StrategyRow {
                        id: s.id().to_string(),
                        name: s.name().to_string(),
                        enabled: if s.is_enabled() { "Yes".to_string() } else { "No".to_string() },
                    })
                    .collect::<Vec<_>>();
                
                let table = Table::new(rows)
                    .with(Style::modern())
                    .with(Columns::single().set_alignment(Alignment::center()));
                
                println!("{}", table);
            },
            DisplayFormat::Json => {
                let output = strategies
                    .iter()
                    .map(|s| serde_json::json!({
                        "id": s.id(),
                        "name": s.name(),
                        "enabled": s.is_enabled(),
                    }))
                    .collect::<Vec<_>>();
                
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            },
            DisplayFormat::Yaml => {
                let output = strategies
                    .iter()
                    .map(|s| serde_json::json!({
                        "id": s.id(),
                        "name": s.name(),
                        "enabled": s.is_enabled(),
                    }))
                    .collect::<Vec<_>>();
                
                println!("{}", serde_yaml::to_string(&output).unwrap());
            },
        }
    }
    
    /// Format bot status for display
    fn format_status(status: BotStatus) -> String {
        use console::style;
        
        match status {
            BotStatus::Initializing => style("Initializing").yellow().to_string(),
            BotStatus::Starting => style("Starting").yellow().to_string(),
            BotStatus::Running => style("Running").green().to_string(),
            BotStatus::Paused => style("Paused").yellow().to_string(),
            BotStatus::Stopping => style("Stopping").yellow().to_string(),
            BotStatus::Stopped => style("Stopped").red().to_string(),
            BotStatus::Error => style("Error").red().bold().to_string(),
        }
    }
    
    /// Format execution mode for display
    fn format_execution_mode(mode: ExecutionMode) -> String {
        use console::style;
        
        match mode {
            ExecutionMode::Normal => style("Normal").green().to_string(),
            ExecutionMode::Simulation => style("Simulation").yellow().to_string(),
            ExecutionMode::Analysis => style("Analysis").cyan().to_string(),
            ExecutionMode::Test => style("Test").magenta().to_string(),
        }
    }
}

// crates/cli/src/interactive.rs
//! Interactive shell for the HFT bot

use std::sync::Arc;

use anyhow::{anyhow, Result};
use rustyline::{error::ReadlineError, Editor};
use solana_hft_core::{ExecutionMode, HftBot};
use tokio::sync::Mutex;

/// Interactive shell commands
enum ShellCommand {
    /// Start the bot
    Start,
    
    /// Start in simulation mode
    StartSimulation,
    
    /// Stop the bot
    Stop,
    
    /// Show current status
    Status,
    
    /// Show metrics
    Metrics,
    
    /// List strategies
    Strategies,
    
    /// Enable a strategy
    EnableStrategy(String),
    
    /// Disable a strategy
    DisableStrategy(String),
    
    /// Set execution mode
    SetMode(ExecutionMode),
    
    /// Show help
    Help,
    
    /// Exit the shell
    Exit,
    
    /// Unknown command
    Unknown(String),
}

impl ShellCommand {
    /// Parse a command string
    fn parse(input: &str) -> Self {
        let parts = input.trim().split_whitespace().collect::<Vec<_>>();
        let command = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();
        
        match command.as_str() {
            "start" => {
                if parts.get(1) == Some(&"simulation") {
                    Self::StartSimulation
                } else {
                    Self::Start
                }
            },
            "stop" => Self::Stop,
            "status" => Self::Status,
            "metrics" => Self::Metrics,
            "strategies" | "list" => Self::Strategies,
            "enable" => {
                if let Some(id) = parts.get(1) {
                    Self::EnableStrategy(id.to_string())
                } else {
                    Self::Unknown("enable requires a strategy ID".to_string())
                }
            },
            "disable" => {
                if let Some(id) = parts.get(1) {
                    Self::DisableStrategy(id.to_string())
                } else {
                    Self::Unknown("disable requires a strategy ID".to_string())
                }
            },
            "mode" => {
                if let Some(mode) = parts.get(1) {
                    match mode.to_lowercase().as_str() {
                        "normal" => Self::SetMode(ExecutionMode::Normal),
                        "simulation" => Self::SetMode(ExecutionMode::Simulation),
                        "analysis" => Self::SetMode(ExecutionMode::Analysis),
                        "test" => Self::SetMode(ExecutionMode::Test),
                        _ => Self::Unknown(format!("unknown mode: {}", mode)),
                    }
                } else {
                    Self::Unknown("mode requires a mode name".to_string())
                }
            },
            "help" | "?" => Self::Help,
            "exit" | "quit" => Self::Exit,
            "" => Self::Help,
            _ => Self::Unknown(format!("unknown command: {}", command)),
        }
    }
}

/// Interactive shell for managing the HFT bot
pub struct InteractiveShell {
    /// Bot instance
    bot: Arc<Mutex<HftBot>>,
    
    /// Readline editor
    editor: Editor<()>,
    
    /// Running flag
    running: bool,
}

impl InteractiveShell {
    /// Create a new interactive shell
    pub fn new(bot: Arc<Mutex<HftBot>>) -> Self {
        Self {
            bot,
            editor: Editor::<()>::new().expect("Failed to create editor"),
            running: true,
        }
    }
    
    /// Run the interactive shell
    pub async fn run(&mut self) -> Result<()> {
        println!("Solana HFT Bot Interactive Shell");
        println!("Type 'help' for a list of commands");
        
        while self.running {
            let readline = self.editor.readline("hft> ");
            
            match readline {
                Ok(line) => {
                    self.editor.add_history_entry(line.as_str());
                    let command = ShellCommand::parse(&line);
                    self.handle_command(command).await?;
                },
                Err(ReadlineError::Interrupted) => {
                    // Ctrl-C
                    println!("Type 'exit' to quit");
                },
                Err(ReadlineError::Eof) => {
                    // Ctrl-D
                    self.running = false;
                },
                Err(err) => {
                    println!("Error: {:?}", err);
                    self.running = false;
                },
            }
        }
        
        println!("Goodbye!");
        
        Ok(())
    }
    
    /// Handle shell command
    async fn handle_command(&mut self, command: ShellCommand) -> Result<()> {
        match command {
            ShellCommand::Start => {
                println!("Starting HFT Bot...");
                let mut bot = self.bot.lock().await;
                bot.start().await?;
                println!("HFT Bot started");
            },
            ShellCommand::StartSimulation => {
                println!("Starting HFT Bot in simulation mode...");
                let mut bot = self.bot.lock().await;
                bot.set_execution_mode(ExecutionMode::Simulation);
                bot.start().await?;
                println!("HFT Bot started in simulation mode");
            },
            ShellCommand::Stop => {
                println!("Stopping HFT Bot...");
                let mut bot = self.bot.lock().await;
                bot.stop().await?;
                println!("HFT Bot stopped");
            },
            ShellCommand::Status => {
                let bot = self.bot.lock().await;
                let status = bot.get_status();
                let mode = bot.get_execution_mode();
                println!("Status: {}", super::display::Renderer::format_status(status));
                println!("Mode: {}", super::display::Renderer::format_execution_mode(mode));
            },
            ShellCommand::Metrics => {
                let bot = self.bot.lock().await;
                let metrics = bot.get_metrics();
                super::display::Renderer::new(super::display::DisplayFormat::Text)
                    .render_metrics(metrics);
            },
            ShellCommand::Strategies => {
                let bot = self.bot.lock().await;
                let strategies = bot.strategy_manager.get_strategies();
                super::display::Renderer::new(super::display::DisplayFormat::Text)
                    .render_strategies(strategies);
            },
            ShellCommand::EnableStrategy(id) => {
                let bot = self.bot.lock().await;
                match bot.strategy_manager.enable_strategy(&id) {
                    Ok(_) => println!("Strategy {} enabled", id),
                    Err(e) => println!("Error: {}", e),
                }
            },
            ShellCommand::DisableStrategy(id) => {
                let bot = self.bot.lock().await;
                match bot.strategy_manager.disable_strategy(&id) {
                    Ok(_) => println!("Strategy {} disabled", id),
                    Err(e) => println!("Error: {}", e),
                }
            },
            ShellCommand::SetMode(mode) => {
                let mut bot = self.bot.lock().await;
                bot.set_execution_mode(mode);
                println!("Execution mode set to: {}", 
                    super::display::Renderer::format_execution_mode(mode));
            },
            ShellCommand::Help => {
                self.print_help();
            },
            ShellCommand::Exit => {
                self.running = false;
            },
            ShellCommand::Unknown(message) => {
                println!("Error: {}", message);
                self.print_help();
            },
        }
        
        Ok(())
    }
    
    /// Print help information
    fn print_help(&self) {
        println!("Available commands:");
        println!("  start                     Start the HFT bot");
        println!("  start simulation          Start the HFT bot in simulation mode");
        println!("  stop                      Stop the HFT bot");
        println!("  status                    Show current status");
        println!("  metrics                   Show performance metrics");
        println!("  strategies                List available strategies");
        println!("  enable [strategy_id]      Enable a strategy");
        println!("  disable [strategy_id]     Disable a strategy");
        println!("  mode [normal|simulation|analysis|test]  Set execution mode");
        println!("  help                      Show this help");
        println!("  exit                      Exit the shell");
    }
}

// crates/cli/src/utils.rs
//! Utility functions

use anyhow::{anyhow, Result};
use solana_hft_core::BotConfig;
use std::path::PathBuf;

/// Load configuration from file
pub fn load_config(path: &PathBuf) -> Result<BotConfig> {
    if !path.exists() {
        return Err(anyhow!("Configuration file not found: {:?}", path));
    }
    
    let config_str = std::fs::read_to_string(path)?;
    let config: BotConfig = serde_json::from_str(&config_str)?;
    
    Ok(config)
}