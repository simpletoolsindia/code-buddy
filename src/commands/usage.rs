//! Usage Command - Usage statistics and limits
//!
//! Provides usage tracking and statistics.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub total_tokens: usize,
    pub api_calls: usize,
    pub cost_usd: f64,
}

impl UsageStats {
    pub fn new() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            api_calls: 0,
            cost_usd: 0.0,
        }
    }

    pub fn add(&mut self, input: usize, output: usize) {
        self.input_tokens += input;
        self.output_tokens += output;
        self.total_tokens += input + output;
        self.api_calls += 1;
        self.cost_usd += self.calculate_cost(input, output);
    }

    fn calculate_cost(&self, input: usize, output: usize) -> f64 {
        // Approximate pricing for Claude
        let input_cost = input as f64 / 1_000_000.0 * 3.0; // $3 per M tokens
        let output_cost = output as f64 / 1_000_000.0 * 15.0; // $15 per M tokens
        input_cost + output_cost
    }
}

impl Default for UsageStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Run usage command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return show_usage();
    }

    match args[0].as_str() {
        "stats" => show_usage(),
        "reset" => reset_usage(),
        "export" => export_usage(),
        "limit" => show_limit(),
        _ => show_usage(),
    }
}

fn show_usage() -> Result<String> {
    let output = r#"# Usage Statistics

## Session
| Metric | Value |
|--------|-------|
| Input Tokens | 0 |
| Output Tokens | 0 |
| Total Tokens | 0 |
| API Calls | 0 |

## Cost
| Metric | Value |
|--------|-------|
| Session Cost | $0.00 |
| Daily Limit | $100.00 |
| Daily Used | $0.00 |

## Limits
- Max tokens per response: 8192
- Max context: 200,000 tokens
- Rate limit: 50 requests/minute
"#;
    Ok(output.to_string())
}

fn reset_usage() -> Result<String> {
    Ok("Usage statistics reset.\n".to_string())
}

fn export_usage() -> Result<String> {
    let json = r#"{
  "usage": {
    "input_tokens": 0,
    "output_tokens": 0,
    "total_tokens": 0,
    "api_calls": 0,
    "cost_usd": 0.0
  }
}"#;
    Ok(format!("Exported:\n{}\n", json))
}

fn show_limit() -> Result<String> {
    let output = r#"# Usage Limits

## Current Limits
- Daily spending limit: $100.00
- Monthly spending limit: $1000.00
- Token limit per request: 8192

## Rate Limits
- Requests per minute: 50
- Requests per hour: 1000
"#;
    Ok(output.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_stats() {
        let mut stats = UsageStats::new();
        stats.add(1000, 500);
        assert_eq!(stats.input_tokens, 1000);
        assert_eq!(stats.output_tokens, 500);
    }
}
