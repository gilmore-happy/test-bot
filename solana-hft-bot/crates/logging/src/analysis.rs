//! Real-time log analysis and alerting
//!
//! This module provides real-time analysis of logs, including:
//! - Critical error alerting via webhooks
//! - Performance anomaly detection
//! - Automated log scanning for known error patterns

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn, Level};

use crate::Logger;

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational alert
    Info,
    /// Warning alert
    Warning,
    /// Error alert
    Error,
    /// Critical alert
    Critical,
}

impl From<Level> for AlertSeverity {
    fn from(level: Level) -> Self {
        match level {
            Level::TRACE | Level::DEBUG | Level::INFO => AlertSeverity::Info,
            Level::WARN => AlertSeverity::Warning,
            Level::ERROR => AlertSeverity::Error,
        }
    }
}

/// Alert message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Alert ID
    pub id: String,
    /// Alert timestamp
    pub timestamp: DateTime<Utc>,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert source
    pub source: String,
    /// Alert message
    pub message: String,
    /// Alert details
    pub details: HashMap<String, String>,
    /// Correlation ID
    pub correlation_id: Option<String>,
}

/// Alert rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule ID
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Rule severity
    pub severity: AlertSeverity,
    /// Rule condition
    pub condition: AlertCondition,
    /// Rule actions
    pub actions: Vec<AlertAction>,
    /// Whether the rule is enabled
    pub enabled: bool,
}

/// Alert condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    /// Log message contains a pattern
    LogContains {
        /// Log level
        level: Option<Level>,
        /// Pattern to match
        pattern: String,
        /// Whether to use regex matching
        regex: bool,
    },
    /// Metric exceeds a threshold
    MetricThreshold {
        /// Metric name
        metric: String,
        /// Threshold operator
        operator: ThresholdOperator,
        /// Threshold value
        value: f64,
        /// Duration for which the condition must be true
        duration: Duration,
    },
    /// Metric changes by a percentage
    MetricChange {
        /// Metric name
        metric: String,
        /// Change operator
        operator: ThresholdOperator,
        /// Change percentage
        percentage: f64,
        /// Duration for which the condition must be true
        duration: Duration,
    },
    /// Composite condition (AND)
    And(Vec<AlertCondition>),
    /// Composite condition (OR)
    Or(Vec<AlertCondition>),
}

/// Threshold operator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThresholdOperator {
    /// Greater than
    GreaterThan,
    /// Greater than or equal to
    GreaterThanOrEqual,
    /// Less than
    LessThan,
    /// Less than or equal to
    LessThanOrEqual,
    /// Equal to
    Equal,
    /// Not equal to
    NotEqual,
}

/// Alert action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertAction {
    /// Log the alert
    Log,
    /// Send the alert to a webhook
    Webhook {
        /// Webhook URL
        url: String,
        /// Webhook headers
        headers: HashMap<String, String>,
    },
    /// Execute a command
    Command {
        /// Command to execute
        command: String,
        /// Command arguments
        args: Vec<String>,
    },
}

/// Real-time analyzer for logs and metrics
pub struct RealTimeAnalyzer {
    /// Alert rules
    rules: Vec<AlertRule>,
    /// Alert history
    alert_history: VecDeque<Alert>,
    /// Maximum alert history size
    max_history_size: usize,
    /// Metric history
    metric_history: HashMap<String, VecDeque<(DateTime<Utc>, f64)>>,
    /// Maximum metric history size
    max_metric_history_size: usize,
    /// Alert sender
    alert_tx: mpsc::Sender<Alert>,
    /// Known error patterns
    known_error_patterns: Vec<String>,
    /// Performance anomaly detection thresholds
    performance_anomaly_thresholds: HashMap<String, f64>,
}

impl RealTimeAnalyzer {
    /// Create a new real-time analyzer
    pub fn new(
        rules: Vec<AlertRule>,
        max_history_size: usize,
        max_metric_history_size: usize,
        known_error_patterns: Vec<String>,
        performance_anomaly_thresholds: HashMap<String, f64>,
    ) -> (Self, mpsc::Receiver<Alert>) {
        let (alert_tx, alert_rx) = mpsc::channel(100);
        
        (
            Self {
                rules,
                alert_history: VecDeque::with_capacity(max_history_size),
                max_history_size,
                metric_history: HashMap::new(),
                max_metric_history_size,
                alert_tx,
                known_error_patterns,
                performance_anomaly_thresholds,
            },
            alert_rx,
        )
    }
    
    /// Process a log message
    pub async fn process_log(
        &mut self,
        level: Level,
        message: &str,
        module_path: Option<&str>,
        file: Option<&str>,
        line: Option<u32>,
        correlation_id: Option<&str>,
        fields: &HashMap<String, String>,
    ) {
        // Check for known error patterns
        for pattern in &self.known_error_patterns {
            if message.contains(pattern) {
                self.trigger_alert(
                    AlertSeverity::Error,
                    "Known Error Pattern",
                    &format!("Log message contains known error pattern: {}", pattern),
                    module_path,
                    correlation_id,
                    fields,
                ).await;
                break;
            }
        }
        
        // Check alert rules
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            
            if self.check_condition(&rule.condition, level, message, fields).await {
                self.trigger_alert(
                    rule.severity,
                    &rule.name,
                    &rule.description,
                    module_path,
                    correlation_id,
                    fields,
                ).await;
                
                // Execute rule actions
                for action in &rule.actions {
                    self.execute_action(action, &rule.name, message, fields).await;
                }
            }
        }
    }
    
    /// Process a metric update
    pub async fn process_metric(&mut self, name: &str, value: f64, timestamp: DateTime<Utc>, fields: &HashMap<String, String>) {
        // Add to metric history
        let history = self.metric_history.entry(name.to_string()).or_insert_with(|| {
            VecDeque::with_capacity(self.max_metric_history_size)
        });
        
        if history.len() >= self.max_metric_history_size {
            history.pop_front();
        }
        
        history.push_back((timestamp, value));
        
        // Check for performance anomalies
        if let Some(threshold) = self.performance_anomaly_thresholds.get(name) {
            if history.len() >= 2 {
                let (_, prev_value) = history[history.len() - 2];
                let change_pct = (value - prev_value).abs() / prev_value.abs() * 100.0;
                
                if change_pct > *threshold {
                    self.trigger_alert(
                        AlertSeverity::Warning,
                        "Performance Anomaly",
                        &format!("Metric {} changed by {:.2}% (threshold: {:.2}%)", name, change_pct, threshold),
                        None,
                        None,
                        fields,
                    ).await;
                }
            }
        }
        
        // Check alert rules
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            
            if self.check_metric_condition(&rule.condition, name, value, &self.metric_history).await {
                self.trigger_alert(
                    rule.severity,
                    &rule.name,
                    &rule.description,
                    None,
                    None,
                    fields,
                ).await;
                
                // Execute rule actions
                for action in &rule.actions {
                    self.execute_action(action, &rule.name, &format!("Metric {} = {}", name, value), fields).await;
                }
            }
        }
    }
    
    /// Check if a condition is met
    async fn check_condition(
        &self,
        condition: &AlertCondition,
        level: Level,
        message: &str,
        fields: &HashMap<String, String>,
    ) -> bool {
        match condition {
            AlertCondition::LogContains { level: cond_level, pattern, regex } => {
                // Check level
                if let Some(cond_level) = cond_level {
                    if level < *cond_level {
                        return false;
                    }
                }
                
                // Check pattern
                if *regex {
                    // TODO: Implement regex matching
                    message.contains(pattern)
                } else {
                    message.contains(pattern)
                }
            },
            AlertCondition::MetricThreshold { .. } => false, // Not applicable for log messages
            AlertCondition::MetricChange { .. } => false, // Not applicable for log messages
            AlertCondition::And(conditions) => {
                for cond in conditions {
                    if !self.check_condition(cond, level, message, fields).await {
                        return false;
                    }
                }
                true
            },
            AlertCondition::Or(conditions) => {
                for cond in conditions {
                    if self.check_condition(cond, level, message, fields).await {
                        return true;
                    }
                }
                false
            },
        }
    }
    
    /// Check if a metric condition is met
    async fn check_metric_condition(
        &self,
        condition: &AlertCondition,
        name: &str,
        value: f64,
        history: &HashMap<String, VecDeque<(DateTime<Utc>, f64)>>,
    ) -> bool {
        match condition {
            AlertCondition::MetricThreshold { metric, operator, value: threshold, duration } => {
                if metric != name {
                    return false;
                }
                
                let meets_threshold = match operator {
                    ThresholdOperator::GreaterThan => value > *threshold,
                    ThresholdOperator::GreaterThanOrEqual => value >= *threshold,
                    ThresholdOperator::LessThan => value < *threshold,
                    ThresholdOperator::LessThanOrEqual => value <= *threshold,
                    ThresholdOperator::Equal => (value - *threshold).abs() < f64::EPSILON,
                    ThresholdOperator::NotEqual => (value - *threshold).abs() >= f64::EPSILON,
                };
                
                if !meets_threshold {
                    return false;
                }
                
                // Check if the condition has been true for the required duration
                if *duration == Duration::from_secs(0) {
                    return true;
                }
                
                if let Some(metric_history) = history.get(name) {
                    if metric_history.len() < 2 {
                        return false;
                    }
                    
                    let now = Utc::now();
                    let threshold_time = now - chrono::Duration::from_std(*duration).unwrap();
                    
                    // Find the earliest point that meets the threshold
                    for i in (0..metric_history.len()).rev() {
                        let (timestamp, val) = metric_history[i];
                        
                        let meets_condition = match operator {
                            ThresholdOperator::GreaterThan => val > *threshold,
                            ThresholdOperator::GreaterThanOrEqual => val >= *threshold,
                            ThresholdOperator::LessThan => val < *threshold,
                            ThresholdOperator::LessThanOrEqual => val <= *threshold,
                            ThresholdOperator::Equal => (val - *threshold).abs() < f64::EPSILON,
                            ThresholdOperator::NotEqual => (val - *threshold).abs() >= f64::EPSILON,
                        };
                        
                        if !meets_condition {
                            return false;
                        }
                        
                        if timestamp <= threshold_time {
                            return true;
                        }
                    }
                }
                
                false
            },
            AlertCondition::MetricChange { metric, operator, percentage, duration } => {
                if metric != name {
                    return false;
                }
                
                if let Some(metric_history) = history.get(name) {
                    if metric_history.len() < 2 {
                        return false;
                    }
                    
                    let now = Utc::now();
                    let threshold_time = now - chrono::Duration::from_std(*duration).unwrap();
                    
                    // Find the earliest point within the duration
                    let mut baseline_idx = 0;
                    for i in 0..metric_history.len() {
                        let (timestamp, _) = metric_history[i];
                        if timestamp >= threshold_time {
                            baseline_idx = i;
                            break;
                        }
                    }
                    
                    if baseline_idx >= metric_history.len() - 1 {
                        return false;
                    }
                    
                    let (_, baseline_value) = metric_history[baseline_idx];
                    let change_pct = (value - baseline_value) / baseline_value.abs() * 100.0;
                    
                    match operator {
                        ThresholdOperator::GreaterThan => change_pct > *percentage,
                        ThresholdOperator::GreaterThanOrEqual => change_pct >= *percentage,
                        ThresholdOperator::LessThan => change_pct < *percentage,
                        ThresholdOperator::LessThanOrEqual => change_pct <= *percentage,
                        ThresholdOperator::Equal => (change_pct - *percentage).abs() < f64::EPSILON,
                        ThresholdOperator::NotEqual => (change_pct - *percentage).abs() >= f64::EPSILON,
                    }
                } else {
                    false
                }
            },
            AlertCondition::LogContains { .. } => false, // Not applicable for metrics
            AlertCondition::And(conditions) => {
                for cond in conditions {
                    if !self.check_metric_condition(cond, name, value, history).await {
                        return false;
                    }
                }
                true
            },
            AlertCondition::Or(conditions) => {
                for cond in conditions {
                    if self.check_metric_condition(cond, name, value, history).await {
                        return true;
                    }
                }
                false
            },
        }
    }
    
    /// Trigger an alert
    async fn trigger_alert(
        &mut self,
        severity: AlertSeverity,
        source: &str,
        message: &str,
        module_path: Option<&str>,
        correlation_id: Option<&str>,
        fields: &HashMap<String, String>,
    ) {
        let alert = Alert {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            severity,
            source: source.to_string(),
            message: message.to_string(),
            details: fields.clone(),
            correlation_id: correlation_id.map(|s| s.to_string()),
        };
        
        // Add to alert history
        if self.alert_history.len() >= self.max_history_size {
            self.alert_history.pop_front();
        }
        self.alert_history.push_back(alert.clone());
        
        // Send alert
        if let Err(e) = self.alert_tx.send(alert).await {
            error!("Failed to send alert: {}", e);
        }
    }
    
    /// Execute an alert action
    async fn execute_action(&self, action: &AlertAction, rule_name: &str, message: &str, fields: &HashMap<String, String>) {
        match action {
            AlertAction::Log => {
                info!(
                    target: "alerts",
                    rule = rule_name,
                    message = message,
                    fields = ?fields,
                    "Alert triggered"
                );
            },
            AlertAction::Webhook { url, headers } => {
                // TODO: Implement webhook sending
                debug!("Would send webhook to {}: {}", url, message);
            },
            AlertAction::Command { command, args } => {
                // TODO: Implement command execution
                debug!("Would execute command {}: {}", command, message);
            },
        }
    }
    
    /// Get alert history
    pub fn get_alert_history(&self) -> Vec<Alert> {
        self.alert_history.iter().cloned().collect()
    }
    
    /// Get metric history
    pub fn get_metric_history(&self, name: &str) -> Option<Vec<(DateTime<Utc>, f64)>> {
        self.metric_history.get(name).map(|h| h.iter().cloned().collect())
    }
    
    /// Add a new alert rule
    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
    }
    
    /// Remove an alert rule
    pub fn remove_rule(&mut self, id: &str) -> bool {
        let len = self.rules.len();
        self.rules.retain(|r| r.id != id);
        self.rules.len() < len
    }
    
    /// Enable an alert rule
    pub fn enable_rule(&mut self, id: &str) -> bool {
        for rule in &mut self.rules {
            if rule.id == id {
                rule.enabled = true;
                return true;
            }
        }
        false
    }
    
    /// Disable an alert rule
    pub fn disable_rule(&mut self, id: &str) -> bool {
        for rule in &mut self.rules {
            if rule.id == id {
                rule.enabled = false;
                return true;
            }
        }
        false
    }
}

/// Start the real-time analyzer
pub async fn start_analyzer(
    rules: Vec<AlertRule>,
    max_history_size: usize,
    max_metric_history_size: usize,
    known_error_patterns: Vec<String>,
    performance_anomaly_thresholds: HashMap<String, f64>,
    webhook_urls: Vec<String>,
) -> mpsc::Sender<AnalyzerMessage> {
    let (analyzer, mut alert_rx) = RealTimeAnalyzer::new(
        rules,
        max_history_size,
        max_metric_history_size,
        known_error_patterns,
        performance_anomaly_thresholds,
    );
    
    let analyzer = Arc::new(RwLock::new(analyzer));
    let (msg_tx, mut msg_rx) = mpsc::channel(100);
    
    // Spawn alert handler
    let webhook_urls = webhook_urls.clone();
    tokio::spawn(async move {
        while let Some(alert) = alert_rx.recv().await {
            // Log the alert
            match alert.severity {
                AlertSeverity::Info => info!(
                    target: "alerts",
                    id = %alert.id,
                    source = %alert.source,
                    correlation_id = ?alert.correlation_id,
                    "Alert: {}", alert.message
                ),
                AlertSeverity::Warning => warn!(
                    target: "alerts",
                    id = %alert.id,
                    source = %alert.source,
                    correlation_id = ?alert.correlation_id,
                    "Alert: {}", alert.message
                ),
                AlertSeverity::Error | AlertSeverity::Critical => error!(
                    target: "alerts",
                    id = %alert.id,
                    source = %alert.source,
                    correlation_id = ?alert.correlation_id,
                    "Alert: {}", alert.message
                ),
            }
            
            // Send to webhooks
            for url in &webhook_urls {
                if let Err(e) = send_alert_to_webhook(url, &alert).await {
                    error!("Failed to send alert to webhook {}: {}", url, e);
                }
            }
        }
    });
    
    // Spawn message handler
    let analyzer_clone = analyzer.clone();
    tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            match msg {
                AnalyzerMessage::ProcessLog { level, message, module_path, file, line, correlation_id, fields } => {
                    analyzer_clone.write().process_log(
                        level,
                        &message,
                        module_path.as_deref(),
                        file.as_deref(),
                        line,
                        correlation_id.as_deref(),
                        &fields,
                    ).await;
                },
                AnalyzerMessage::ProcessMetric { name, value, timestamp, fields } => {
                    analyzer_clone.write().process_metric(
                        &name,
                        value,
                        timestamp,
                        &fields,
                    ).await;
                },
                AnalyzerMessage::AddRule(rule) => {
                    analyzer_clone.write().add_rule(rule);
                },
                AnalyzerMessage::RemoveRule(id) => {
                    analyzer_clone.write().remove_rule(&id);
                },
                AnalyzerMessage::EnableRule(id) => {
                    analyzer_clone.write().enable_rule(&id);
                },
                AnalyzerMessage::DisableRule(id) => {
                    analyzer_clone.write().disable_rule(&id);
                },
                AnalyzerMessage::GetAlertHistory(tx) => {
                    let history = analyzer_clone.read().get_alert_history();
                    let _ = tx.send(history);
                },
                AnalyzerMessage::GetMetricHistory { name, tx } => {
                    let history = analyzer_clone.read().get_metric_history(&name);
                    let _ = tx.send(history);
                },
            }
        }
    });
    
    msg_tx
}

/// Message for the analyzer
#[derive(Debug)]
pub enum AnalyzerMessage {
    /// Process a log message
    ProcessLog {
        /// Log level
        level: Level,
        /// Log message
        message: String,
        /// Module path
        module_path: Option<String>,
        /// File
        file: Option<String>,
        /// Line
        line: Option<u32>,
        /// Correlation ID
        correlation_id: Option<String>,
        /// Additional fields
        fields: HashMap<String, String>,
    },
    /// Process a metric update
    ProcessMetric {
        /// Metric name
        name: String,
        /// Metric value
        value: f64,
        /// Timestamp
        timestamp: DateTime<Utc>,
        /// Additional fields
        fields: HashMap<String, String>,
    },
    /// Add a new alert rule
    AddRule(AlertRule),
    /// Remove an alert rule
    RemoveRule(String),
    /// Enable an alert rule
    EnableRule(String),
    /// Disable an alert rule
    DisableRule(String),
    /// Get alert history
    GetAlertHistory(tokio::sync::oneshot::Sender<Vec<Alert>>),
    /// Get metric history
    GetMetricHistory {
        /// Metric name
        name: String,
        /// Response channel
        tx: tokio::sync::oneshot::Sender<Option<Vec<(DateTime<Utc>, f64)>>>,
    },
}

/// Send an alert to a webhook
async fn send_alert_to_webhook(url: &str, alert: &Alert) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    
    let payload = serde_json::json!({
        "id": alert.id,
        "timestamp": alert.timestamp.to_rfc3339(),
        "severity": format!("{:?}", alert.severity),
        "source": alert.source,
        "message": alert.message,
        "details": alert.details,
        "correlation_id": alert.correlation_id,
    });
    
    client.post(url)
        .json(&payload)
        .send()
        .await?;
        
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;
    
    #[tokio::test]
    async fn test_analyzer_process_log() {
        let rules = vec![
            AlertRule {
                id: "test-rule".to_string(),
                name: "Test Rule".to_string(),
                description: "Test rule for error messages".to_string(),
                severity: AlertSeverity::Error,
                condition: AlertCondition::LogContains {
                    level: Some(Level::ERROR),
                    pattern: "error".to_string(),
                    regex: false,
                },
                actions: vec![AlertAction::Log],
                enabled: true,
            },
        ];
        
        let known_error_patterns = vec!["known error".to_string()];
        let performance_anomaly_thresholds = HashMap::new();
        
        let (mut analyzer, _) = RealTimeAnalyzer::new(
            rules,
            100,
            100,
            known_error_patterns,
            performance_anomaly_thresholds,
        );
        
        // Process a log message that should trigger an alert
        analyzer.process_log(
            Level::ERROR,
            "This is an error message",
            Some("test_module"),
            Some("test_file.rs"),
            Some(42),
            Some("test-correlation-id"),
            &HashMap::new(),
        ).await;
        
        // Check that an alert was triggered
        assert_eq!(analyzer.alert_history.len(), 1);
        let alert = &analyzer.alert_history[0];
        assert_eq!(alert.severity, AlertSeverity::Error);
        assert_eq!(alert.source, "Test Rule");
    }
    
    #[tokio::test]
    async fn test_analyzer_process_metric() {
        let rules = vec![
            AlertRule {
                id: "test-metric-rule".to_string(),
                name: "Test Metric Rule".to_string(),
                description: "Test rule for high CPU usage".to_string(),
                severity: AlertSeverity::Warning,
                condition: AlertCondition::MetricThreshold {
                    metric: "cpu_usage".to_string(),
                    operator: ThresholdOperator::GreaterThan,
                    value: 90.0,
                    duration: Duration::from_secs(0),
                },
                actions: vec![AlertAction::Log],
                enabled: true,
            },
        ];
        
        let known_error_patterns = Vec::new();
        let performance_anomaly_thresholds = HashMap::new();
        
        let (mut analyzer, _) = RealTimeAnalyzer::new(
            rules,
            100,
            100,
            known_error_patterns,
            performance_anomaly_thresholds,
        );
        
        // Process a metric update that should trigger an alert
        analyzer.process_metric(
            "cpu_usage",
            95.0,
            Utc::now(),
            &HashMap::new(),
        ).await;
        
        // Check that an alert was triggered
        assert_eq!(analyzer.alert_history.len(), 1);
        let alert = &analyzer.alert_history[0];
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert_eq!(alert.source, "Test Metric Rule");
    }
}