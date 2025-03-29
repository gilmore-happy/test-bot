use crate::token::Token;
use std::fmt;
use chrono::Utc;

/// Token filter trait for applying filtering criteria
pub trait TokenFilter: Send + Sync {
    /// Apply the filter to a token
    fn apply(&self, token: &Token) -> bool;
    
    /// Get the filter name
    fn name(&self) -> &str;
    
    /// Get the filter description
    fn description(&self) -> String;
}

/// Filter for minimum liquidity
pub struct MinimumLiquidityFilter {
    min_liquidity_usd: f64,
    name: String,
}

impl MinimumLiquidityFilter {
    pub fn new(min_liquidity_usd: f64) -> Self {
        Self {
            min_liquidity_usd,
            name: format!("MinimumLiquidity(${:.2})", min_liquidity_usd),
        }
    }
}

impl TokenFilter for MinimumLiquidityFilter {
    fn apply(&self, token: &Token) -> bool {
        // Sum liquidity across all DEXes
        let total_liquidity: f64 = token.total_liquidity();
        
        total_liquidity >= self.min_liquidity_usd
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> String {
        format!("Requires token to have at least ${:.2} in liquidity", self.min_liquidity_usd)
    }
}

/// Filter for token age
pub struct TokenAgeFilter {
    min_age_seconds: u64,
    name: String,
}

impl TokenAgeFilter {
    pub fn new(min_age_seconds: u64) -> Self {
        Self {
            min_age_seconds,
            name: format!("TokenAge({}s)", min_age_seconds),
        }
    }
}

impl TokenFilter for TokenAgeFilter {
    fn apply(&self, token: &Token) -> bool {
        // If launch timestamp is not available, use tracked_since
        let start_time = token.metadata.launch_timestamp
            .unwrap_or(token.tracked_since);
        
        let age = Utc::now().signed_duration_since(start_time);
        
        age.num_seconds() >= self.min_age_seconds as i64
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> String {
        let duration_str = if self.min_age_seconds >= 86400 {
            format!("{} days", self.min_age_seconds / 86400)
        } else if self.min_age_seconds >= 3600 {
            format!("{} hours", self.min_age_seconds / 3600)
        } else if self.min_age_seconds >= 60 {
            format!("{} minutes", self.min_age_seconds / 60)
        } else {
            format!("{} seconds", self.min_age_seconds)
        };
        
        format!("Requires token to be at least {} old", duration_str)
    }
}

/// Filter for rugpull risk
pub struct RugpullRiskFilter {
    max_risk: u8,
    name: String,
}

impl RugpullRiskFilter {
    pub fn new(max_risk: u8) -> Self {
        Self {
            max_risk,
            name: format!("RugpullRisk({})", max_risk),
        }
    }
}

impl TokenFilter for RugpullRiskFilter {
    fn apply(&self, token: &Token) -> bool {
        token.score.rugpull_risk <= self.max_risk
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> String {
        format!("Requires token to have rugpull risk score <= {}", self.max_risk)
    }
}

/// Filter for minimum market cap
pub struct MinimumMarketCapFilter {
    min_market_cap_usd: f64,
    name: String,
}

impl MinimumMarketCapFilter {
    pub fn new(min_market_cap_usd: f64) -> Self {
        Self {
            min_market_cap_usd,
            name: format!("MinimumMarketCap(${:.2})", min_market_cap_usd),
        }
    }
}

impl TokenFilter for MinimumMarketCapFilter {
    fn apply(&self, token: &Token) -> bool {
        match token.market_cap_usd {
            Some(market_cap) => market_cap >= self.min_market_cap_usd,
            None => false, // If market cap is unknown, filter it out
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> String {
        format!("Requires token to have market cap of at least ${:.2}", self.min_market_cap_usd)
    }
}

/// Filter for minimum trading volume
pub struct MinimumVolumeFilter {
    min_volume_usd: f64,
    name: String,
}

impl MinimumVolumeFilter {
    pub fn new(min_volume_usd: f64) -> Self {
        Self {
            min_volume_usd,
            name: format!("MinimumVolume(${:.2})", min_volume_usd),
        }
    }
}

impl TokenFilter for MinimumVolumeFilter {
    fn apply(&self, token: &Token) -> bool {
        match token.volume_24h_usd {
            Some(volume) => volume >= self.min_volume_usd,
            None => false, // If volume is unknown, filter it out
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> String {
        format!("Requires token to have 24h trading volume of at least ${:.2}", self.min_volume_usd)
    }
}

/// Filter for verified tokens
pub struct VerifiedTokenFilter {
    name: String,
}

impl VerifiedTokenFilter {
    pub fn new() -> Self {
        Self {
            name: "VerifiedToken".to_string(),
        }
    }
}

impl TokenFilter for VerifiedTokenFilter {
    fn apply(&self, token: &Token) -> bool {
        token.metadata.is_verified
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> String {
        "Requires token to be verified".to_string()
    }
}

/// Filter for tokens with specific tags
pub struct TagFilter {
    tags: Vec<String>,
    require_all: bool,
    name: String,
}

impl TagFilter {
    pub fn new(tags: Vec<String>, require_all: bool) -> Self {
        let tag_list = tags.join(",");
        let name = if require_all {
            format!("AllTags({})", tag_list)
        } else {
            format!("AnyTag({})", tag_list)
        };
        
        Self {
            tags,
            require_all,
            name,
        }
    }
}

impl TokenFilter for TagFilter {
    fn apply(&self, token: &Token) -> bool {
        if self.require_all {
            // All specified tags must be present
            self.tags.iter().all(|tag| token.tags.contains(tag))
        } else {
            // At least one specified tag must be present
            self.tags.iter().any(|tag| token.tags.contains(tag))
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> String {
        if self.require_all {
            format!("Requires token to have all of these tags: {}", self.tags.join(", "))
        } else {
            format!("Requires token to have at least one of these tags: {}", self.tags.join(", "))
        }
    }
}

/// Composite filter that combines multiple filters with AND logic
pub struct AndFilter {
    filters: Vec<Box<dyn TokenFilter + Send + Sync>>,
    name: String,
}

impl AndFilter {
    pub fn new(filters: Vec<Box<dyn TokenFilter + Send + Sync>>) -> Self {
        let filter_names: Vec<String> = filters.iter()
            .map(|f| f.name().to_string())
            .collect();
        
        let name = format!("And({})", filter_names.join(","));
        
        Self {
            filters,
            name,
        }
    }
}

impl TokenFilter for AndFilter {
    fn apply(&self, token: &Token) -> bool {
        self.filters.iter().all(|filter| filter.apply(token))
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> String {
        let descriptions: Vec<String> = self.filters.iter()
            .map(|f| f.description())
            .collect();
        
        format!("Requires ALL of the following conditions:\n- {}", descriptions.join("\n- "))
    }
}

/// Composite filter that combines multiple filters with OR logic
pub struct OrFilter {
    filters: Vec<Box<dyn TokenFilter + Send + Sync>>,
    name: String,
}

impl OrFilter {
    pub fn new(filters: Vec<Box<dyn TokenFilter + Send + Sync>>) -> Self {
        let filter_names: Vec<String> = filters.iter()
            .map(|f| f.name().to_string())
            .collect();
        
        let name = format!("Or({})", filter_names.join(","));
        
        Self {
            filters,
            name,
        }
    }
}

impl TokenFilter for OrFilter {
    fn apply(&self, token: &Token) -> bool {
        self.filters.iter().any(|filter| filter.apply(token))
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> String {
        let descriptions: Vec<String> = self.filters.iter()
            .map(|f| f.description())
            .collect();
        
        format!("Requires ANY of the following conditions:\n- {}", descriptions.join("\n- "))
    }
}

/// Filter chain for applying multiple filters in sequence
pub struct FilterChain {
    filters: Vec<Box<dyn TokenFilter + Send + Sync>>,
}

impl FilterChain {
    /// Create a new filter chain
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }
    
    /// Add a filter to the chain
    pub fn add_filter(&mut self, filter: Box<dyn TokenFilter + Send + Sync>) {
        self.filters.push(filter);
    }
    
    /// Apply all filters to a token
    pub fn apply(&self, token: &Token) -> bool {
        self.filters.iter().all(|filter| filter.apply(token))
    }
    
    /// Get all filters
    pub fn get_filters(&self) -> &[Box<dyn TokenFilter + Send + Sync>] {
        &self.filters
    }
    
    /// Get the number of filters
    pub fn filter_count(&self) -> usize {
        self.filters.len()
    }
}

/// Create default token filters based on configuration
pub fn create_default_filters(config: &crate::ScreeningConfig) -> Vec<Box<dyn TokenFilter + Send + Sync>> {
    let mut filters = Vec::new();
    
    // Add default filters
    filters.push(Box::new(MinimumLiquidityFilter::new(
        config.min_liquidity_usd,
    )));
    
    filters.push(Box::new(TokenAgeFilter::new(
        config.min_token_age_seconds,
    )));
    
    filters.push(Box::new(RugpullRiskFilter::new(
        config.max_rugpull_risk,
    )));
    
    // Add more filters based on configuration
    
    filters
}