//! Interactive shell for the HFT bot

use std::sync::Arc;

use anyhow::{anyhow, Result};
use console::style;
use rustyline::{error::ReadlineError, Editor};
use solana_hft_core::{BotStatus, ExecutionMode, HftBot, Strategy};
use tokio::sync::Mutex;

use crate::display::Renderer;

/// Interactive shell commands
#[derive(Debug)]
pub enum ShellCommand {
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
    
    /// Set risk level
    SetRiskLevel(u8),
    
    /// Show detailed information about a strategy
    StrategyInfo(String),
    
    /// Show help
    Help,
    
    /// Exit the shell
    Exit,
    
    /// Unknown command
    Unknown(String),
}

impl ShellCommand {
    /// Parse a command string
    pub fn parse(input: &str) -> Self {
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
            "risk" => {
                if let Some(level) = parts.get(1) {
                    if let Ok(level) = level.parse::<u8>() {
                        if level <= 10 {
                            Self::SetRiskLevel(level)
                        } else {
                            Self::Unknown("risk level must be between 0 and 10".to_string())
                        }
                    } else {
                        Self::Unknown("risk level must be a number between 0 and 10".to_string())
                    }
                } else {
                    Self::Unknown("risk requires a level (0-10)".to_string())
                }
            },
            "info" => {
                if let Some(id) = parts.get(1) {
                    Self::StrategyInfo(id.to_string())
                } else {
                    Self::Unknown("info requires a strategy ID".to_string())
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
        println!("{}", style("Solana HFT Bot Interactive Shell").bold().cyan());
        println!("Type {} for a list of commands", style("help").green());
        
        // Display initial status
        let bot = self.bot.lock().await;
        let status = bot.get_status();
        let mode = bot.get_execution_mode();
        println!("\nCurrent Status: {}", Renderer::format_status(status));
        println!("Execution Mode: {}", Renderer::format_execution_mode(mode));
        drop(bot);
        
        while self.running {
            let readline = self.editor.readline(&style("hft> ").green().to_string());
            
            match readline {
                Ok(line) => {
                    self.editor.add_history_entry(line.as_str());
                    let command = ShellCommand::parse(&line);
                    if let Err(e) = self.handle_command(command).await {
                        println!("{}: {}", style("Error").red().bold(), e);
                    }
                },
                Err(ReadlineError::Interrupted) => {
                    // Ctrl-C
                    println!("Type {} to quit", style("exit").green());
                },
                Err(ReadlineError::Eof) => {
                    // Ctrl-D
                    self.running = false;
                },
                Err(err) => {
                    println!("{}: {:?}", style("Error").red().bold(), err);
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
                println!("{}", style("HFT Bot started").green());
            },
            ShellCommand::StartSimulation => {
                println!("Starting HFT Bot in simulation mode...");
                let mut bot = self.bot.lock().await;
                bot.set_execution_mode(ExecutionMode::Simulation);
                bot.start().await?;
                println!("{}", style("HFT Bot started in simulation mode").yellow());
            },
            ShellCommand::Stop => {
                println!("Stopping HFT Bot...");
                let mut bot = self.bot.lock().await;
                bot.stop().await?;
                println!("{}", style("HFT Bot stopped").red());
            },
            ShellCommand::Status => {
                let bot = self.bot.lock().await;
                let status = bot.get_status();
                let mode = bot.get_execution_mode();
                
                println!("Status: {}", Renderer::format_status(status));
                println!("Mode: {}", Renderer::format_execution_mode(mode));
                
                // Show additional status information
                if status == BotStatus::Running {
                    let uptime = bot.get_uptime_seconds();
                    let hours = uptime / 3600;
                    let minutes = (uptime % 3600) / 60;
                    let seconds = uptime % 60;
                    
                    println!("Uptime: {}h {}m {}s", hours, minutes, seconds);
                    
                    // Show active strategies
                    let active_strategies = bot.get_active_strategies_count();
                    println!("Active Strategies: {}", active_strategies);
                }
            },
            ShellCommand::Metrics => {
                let bot = self.bot.lock().await;
                let metrics = bot.get_metrics();
                Renderer::new(crate::display::DisplayFormat::Text)
                    .render_metrics(&metrics);
            },
            ShellCommand::Strategies => {
                let bot = self.bot.lock().await;
                let strategies = bot.get_strategies();
                Renderer::new(crate::display::DisplayFormat::Text)
                    .render_strategies(&strategies);
            },
            ShellCommand::EnableStrategy(id) => {
                let bot = self.bot.lock().await;
                match bot.enable_strategy(&id) {
                    Ok(_) => println!("Strategy {} {}", style(&id).cyan(), style("enabled").green()),
                    Err(e) => println!("{}: {}", style("Error").red().bold(), e),
                }
            },
            ShellCommand::DisableStrategy(id) => {
                let bot = self.bot.lock().await;
                match bot.disable_strategy(&id) {
                    Ok(_) => println!("Strategy {} {}", style(&id).cyan(), style("disabled").yellow()),
                    Err(e) => println!("{}: {}", style("Error").red().bold(), e),
                }
            },
            ShellCommand::SetMode(mode) => {
                let mut bot = self.bot.lock().await;
                bot.set_execution_mode(mode);
                println!("Execution mode set to: {}", 
                    Renderer::format_execution_mode(mode));
            },
            ShellCommand::SetRiskLevel(level) => {
                let mut bot = self.bot.lock().await;
                bot.set_risk_level(level);
                
                let risk_description = match level {
                    0..=2 => style("Very Low").green(),
                    3..=4 => style("Low").green(),
                    5..=6 => style("Medium").yellow(),
                    7..=8 => style("High").red(),
                    9..=10 => style("Very High").red().bold(),
                    _ => unreachable!(),
                };
                
                println!("Risk level set to: {} ({})", level, risk_description);
            },
            ShellCommand::StrategyInfo(id) => {
                let bot = self.bot.lock().await;
                match bot.get_strategy_by_id(&id) {
                    Some(strategy) => {
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
                    None => println!("{}: Strategy with ID '{}' not found", style("Error").red().bold(), id),
                }
            },
            ShellCommand::Help => {
                self.print_help();
            },
            ShellCommand::Exit => {
                // Check if bot is running and confirm exit
                let bot = self.bot.lock().await;
                let status = bot.get_status();
                drop(bot);
                
                if status == BotStatus::Running {
                    println!("{}", style("Warning: Bot is still running.").yellow().bold());
                    println!("Use {} to stop the bot before exiting.", style("stop").green());
                    println!("Or type {} again to exit anyway.", style("exit").red());
                    
                    // Set a flag to exit on next exit command
                    self.editor.add_history_entry("exit");
                } else {
                    self.running = false;
                }
            },
            ShellCommand::Unknown(message) => {
                println!("{}: {}", style("Error").red().bold(), message);
                println!("Type {} for a list of commands", style("help").green());
            },
        }
        
        Ok(())
    }
    
    /// Print help information
    fn print_help(&self) {
        println!("\n{}", style("Available Commands:").bold().cyan());
        println!("  {:<30} Start the HFT bot", style("start").green());
        println!("  {:<30} Start the HFT bot in simulation mode", style("start simulation").green());
        println!("  {:<30} Stop the HFT bot", style("stop").green());
        println!("  {:<30} Show current status", style("status").green());
        println!("  {:<30} Show performance metrics", style("metrics").green());
        println!("  {:<30} List available strategies", style("strategies").green());
        println!("  {:<30} Enable a strategy", style("enable <strategy_id>").green());
        println!("  {:<30} Disable a strategy", style("disable <strategy_id>").green());
        println!("  {:<30} Show detailed info about a strategy", style("info <strategy_id>").green());
        println!("  {:<30} Set execution mode", style("mode <normal|simulation|analysis|test>").green());
        println!("  {:<30} Set risk level (0-10)", style("risk <level>").green());
        println!("  {:<30} Show this help", style("help").green());
        println!("  {:<30} Exit the shell", style("exit").green());
        println!("");
    }
}