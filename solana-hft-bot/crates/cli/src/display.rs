//! Display utilities for rendering output

use clap::ValueEnum;
use solana_hft_core::{
    BotStatus, ExecutionMode, Strategy, 
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
    pub fn render_metrics(&self, metrics: &solana_hft_core::metrics::MetricsSnapshot) {
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
                
                let mut rows = Vec::new();
                
                // Add general metrics
                rows.push(MetricsRow { 
                    name: "Uptime (seconds)".to_string(), 
                    value: metrics.uptime_seconds.to_string() 
                });
                
                // Add execution metrics
                rows.push(MetricsRow { 
                    name: "Transactions Sent".to_string(), 
                    value: metrics.execution.transactions_sent.to_string() 
                });
                rows.push(MetricsRow { 
                    name: "Transactions Confirmed".to_string(), 
                    value: metrics.execution.transactions_confirmed.to_string() 
                });
                rows.push(MetricsRow { 
                    name: "Transactions Failed".to_string(), 
                    value: metrics.execution.transactions_failed.to_string() 
                });
                rows.push(MetricsRow { 
                    name: "Avg Confirmation Time (ms)".to_string(), 
                    value: format!("{:.2}", metrics.execution.avg_confirmation_time_ms) 
                });
                
                // Add network metrics
                rows.push(MetricsRow { 
                    name: "Network Requests Sent".to_string(), 
                    value: metrics.network.requests_sent.to_string() 
                });
                rows.push(MetricsRow { 
                    name: "Network Requests Failed".to_string(), 
                    value: metrics.network.requests_failed.to_string() 
                });
                rows.push(MetricsRow { 
                    name: "Avg Network Latency (ms)".to_string(), 
                    value: format!("{:.2}", metrics.network.avg_latency_ms) 
                });
                
                // Add arbitrage metrics
                rows.push(MetricsRow { 
                    name: "Arbitrage Opportunities Found".to_string(), 
                    value: metrics.arbitrage.opportunities_found.to_string() 
                });
                rows.push(MetricsRow { 
                    name: "Arbitrage Opportunities Executed".to_string(), 
                    value: metrics.arbitrage.opportunities_executed.to_string() 
                });
                rows.push(MetricsRow { 
                    name: "Total Profit (SOL)".to_string(), 
                    value: format!("{:.6}", metrics.arbitrage.total_profit_sol) 
                });
                
                let table = Table::new(rows)
                    .with(Style::modern())
                    .with(Columns::single().set_alignment(Alignment::center()));
                
                println!("{}", table);
                
                // Render strategy metrics if available
                if !metrics.strategy_metrics.is_empty() {
                    println!("\nStrategy Metrics:");
                    
                    #[derive(Tabled)]
                    struct StrategyRow {
                        #[tabled(rename = "Strategy")]
                        name: String,
                        #[tabled(rename = "Executions")]
                        executions: u64,
                        #[tabled(rename = "Success Rate")]
                        success_rate: String,
                        #[tabled(rename = "Avg Profit")]
                        avg_profit: String,
                    }
                    
                    let strategy_rows = metrics.strategy_metrics
                        .iter()
                        .map(|(strategy, stats)| StrategyRow {
                            name: strategy.clone(),
                            executions: stats.executions,
                            success_rate: format!("{:.2}%", stats.success_rate * 100.0),
                            avg_profit: format!("{:.6} SOL", stats.avg_profit),
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
    pub fn render_strategies(&self, strategies: &[Arc<dyn Strategy>]) {
        match self.format {
            DisplayFormat::Text => {
                #[derive(Tabled)]
                struct StrategyRow {
                    #[tabled(rename = "ID")]
                    id: String,
                    #[tabled(rename = "Name")]
                    name: String,
                    #[tabled(rename = "Type")]
                    strategy_type: String,
                    #[tabled(rename = "Enabled")]
                    enabled: String,
                }
                
                let rows = strategies
                    .iter()
                    .map(|s| StrategyRow {
                        id: s.id().to_string(),
                        name: s.name().to_string(),
                        strategy_type: s.strategy_type().to_string(),
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
                        "type": s.strategy_type(),
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
                        "type": s.strategy_type(),
                        "enabled": s.is_enabled(),
                    }))
                    .collect::<Vec<_>>();
                
                println!("{}", serde_yaml::to_string(&output).unwrap());
            },
        }
    }
    
    /// Format bot status for display
    pub fn format_status(status: BotStatus) -> String {
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
    pub fn format_execution_mode(mode: ExecutionMode) -> String {
        use console::style;
        
        match mode {
            ExecutionMode::Normal => style("Normal").green().to_string(),
            ExecutionMode::Simulation => style("Simulation").yellow().to_string(),
            ExecutionMode::Analysis => style("Analysis").cyan().to_string(),
            ExecutionMode::Test => style("Test").magenta().to_string(),
        }
    }
}