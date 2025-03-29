use serde::{Deserialize, Serialize};
use crate::token::Token;

/// Token score components
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TokenScore {
    /// Overall score (0-100)
    pub overall: u8,
    
    /// Liquidity score (0-100)
    pub liquidity: u8,
    
    /// Volatility score (0-100)
    pub volatility: u8,
    
    /// Social score (0-100)
    pub social: u8,
    
    /// Code quality score (0-100)
    pub code_quality: u8,
    
    /// Rugpull risk (0-100, higher is worse)
    pub rugpull_risk: u8,
}

impl Default for TokenScore {
    fn default() -> Self {
        Self {
            overall: 50,
            liquidity: 50,
            volatility: 50,
            social: 50,
            code_quality: 50,
            rugpull_risk: 50,
        }
    }
}

/// Scoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfig {
    /// Weight for liquidity score
    pub liquidity_weight: f64,
    
    /// Weight for volatility score
    pub volatility_weight: f64,
    
    /// Weight for social score
    pub social_weight: f64,
    
    /// Weight for code quality score
    pub code_quality_weight: f64,
    
    /// Threshold for high liquidity (USD)
    pub high_liquidity_threshold: f64,
    
    /// Threshold for medium liquidity (USD)
    pub medium_liquidity_threshold: f64,
    
    /// Threshold for high volatility (%)
    pub high_volatility_threshold: f64,
    
    /// Threshold for medium volatility (%)
    pub medium_volatility_threshold: f64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            liquidity_weight: 0.4,
            volatility_weight: 0.2,
            social_weight: 0.2,
            code_quality_weight: 0.2,
            high_liquidity_threshold: 1_000_000.0, // $1M
            medium_liquidity_threshold: 100_000.0, // $100K
            high_volatility_threshold: 0.2,        // 20%
            medium_volatility_threshold: 0.1,      // 10%
        }
    }
}

/// Token scorer for evaluating tokens
pub struct TokenScorer {
    config: ScoringConfig,
}

impl TokenScorer {
    /// Create a new token scorer with the provided configuration
    pub fn new(config: ScoringConfig) -> Self {
        Self { config }
    }
    
    /// Score a token
    pub fn score(&self, token: &mut Token) -> TokenScore {
        // Score liquidity
        let liquidity_score = self.score_liquidity(token);
        
        // Score volatility
        let volatility_score = self.score_volatility(token);
        
        // Score social
        let social_score = self.score_social(token);
        
        // Score code quality
        let code_quality_score = self.score_code_quality(token);
        
        // Score rugpull risk
        let rugpull_risk = self.score_rugpull_risk(token);
        
        // Calculate overall score
        let overall = self.calculate_overall_score(
            liquidity_score,
            volatility_score,
            social_score,
            code_quality_score,
            rugpull_risk,
        );
        
        TokenScore {
            overall,
            liquidity: liquidity_score,
            volatility: volatility_score,
            social: social_score,
            code_quality: code_quality_score,
            rugpull_risk,
        }
    }
    
    /// Score token liquidity
    fn score_liquidity(&self, token: &Token) -> u8 {
        // Sum liquidity across all DEXes
        let total_liquidity: f64 = token.total_liquidity();
        
        if total_liquidity >= self.config.high_liquidity_threshold {
            90 // High liquidity
        } else if total_liquidity >= self.config.medium_liquidity_threshold {
            70 // Medium liquidity
        } else if total_liquidity > 0.0 {
            40 // Low liquidity
        } else {
            10 // No liquidity
        }
    }
    
    /// Score token volatility
    fn score_volatility(&self, _token: &Token) -> u8 {
        // In a real implementation, calculate volatility from price history
        // For now, return a default value
        50
    }
    
    /// Score token social metrics
    fn score_social(&self, token: &Token) -> u8 {
        // In a real implementation, score based on social metrics
        // For now, use some basic heuristics
        
        let mut score = 50; // Default score
        
        // Check if verified
        if token.metadata.is_verified {
            score += 20;
        }
        
        // Check for website and social presence
        if token.metadata.website.is_some() {
            score += 10;
        }
        
        if token.metadata.twitter.is_some() {
            score += 10;
        }
        
        // Cap at 100
        score.min(100)
    }
    
    /// Score token code quality
    fn score_code_quality(&self, _token: &Token) -> u8 {
        // In a real implementation, analyze contract code quality
        // For now, return a default value
        60
    }
    
    /// Score token rugpull risk
    fn score_rugpull_risk(&self, token: &Token) -> u8 {
        let mut risk_score = 0;
        
        // In a real implementation, analyze various risk factors
        // For now, use some basic heuristics
        
        // Check token age
        if let Some(launch_time) = token.metadata.launch_timestamp {
            let age = chrono::Utc::now().signed_duration_since(launch_time);
            
            if age.num_days() < 1 {
                risk_score += 30; // Very new token
            } else if age.num_days() < 7 {
                risk_score += 20; // New token
            } else if age.num_days() < 30 {
                risk_score += 10; // Somewhat new token
            }
        } else {
            risk_score += 20; // Unknown launch time
        }
        
        // Check token verification
        if !token.metadata.is_verified {
            risk_score += 20;
        }
        
        // Check token liquidity
        let total_liquidity: f64 = token.total_liquidity();
        
        if total_liquidity == 0.0 {
            risk_score += 30; // No liquidity
        } else if total_liquidity < 10_000.0 {
            risk_score += 20; // Very low liquidity
        } else if total_liquidity < 100_000.0 {
            risk_score += 10; // Low liquidity
        }
        
        // Cap at 100
        risk_score.min(100)
    }
    
    /// Calculate overall score
    fn calculate_overall_score(
        &self,
        liquidity_score: u8,
        volatility_score: u8,
        social_score: u8,
        code_quality_score: u8,
        rugpull_risk: u8,
    ) -> u8 {
        // Apply weights to component scores
        let weighted_score = 
            (liquidity_score as f64) * self.config.liquidity_weight +
            (volatility_score as f64) * self.config.volatility_weight +
            (social_score as f64) * self.config.social_weight +
            (code_quality_score as f64) * self.config.code_quality_weight;
        
        // Apply rugpull risk penalty
        let risk_penalty = (rugpull_risk as f64) / 100.0;
        
        // Final score
        let final_score = weighted_score * (1.0 - risk_penalty);
        
        // Convert to u8 and cap at 100
        (final_score as u8).min(100)
    }
}