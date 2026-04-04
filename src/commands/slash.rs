//! Slash Commands
//!
//! Built-in slash commands similar to Claude Code:
//! - /simplify - Review code for quality and efficiency
//! - /review - Full code review
//! - /context - Show context usage
//! - /cost - Show token usage and costs

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Slash command result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommandResult {
    pub command: String,
    pub output: String,
    pub success: bool,
}

/// Token pricing (approximate, per 1M tokens)
const PRICING: &[(&str, f64, f64)] = &[
    ("claude-opus-4-6", 15.0, 75.0),     // input, output
    ("claude-sonnet-4-6", 3.0, 15.0),
    ("claude-haiku-4-5", 0.25, 1.25),
];

/// Run a slash command
pub fn run_slash_command(
    command: &str,
    args: &[String],
    state: &crate::state::AppState,
) -> Result<SlashCommandResult> {
    let parts: Vec<&str> = command.trim_start_matches('/').splitn(2, ' ').collect();
    let cmd = parts.first().copied().unwrap_or("");
    let _cmd_args = parts.get(1).copied();

    match cmd.to_lowercase().as_str() {
        "simplify" => run_simplify(args, state),
        "review" => run_review(args, state),
        "context" => run_context(state),
        "cost" => run_cost(state),
        "help" => Ok(SlashCommandResult {
            command: command.to_string(),
            output: get_help_text().to_string(),
            success: true,
        }),
        _ => Err(anyhow::anyhow!("Unknown slash command: /{}", cmd)),
    }
}

/// Run /simplify command - review code for quality issues
fn run_simplify(args: &[String], state: &crate::state::AppState) -> Result<SlashCommandResult> {
    let target = args.first().map(|s| s.as_str()).unwrap_or("*");

    let output = format!(
        r#"# /simplify - Code Quality Review

Analyzing: {}

## Review Focus Areas

1. **Code Reuse** - Look for duplicated code patterns
2. **Quality Issues** - Identify potential bugs, anti-patterns
3. **Efficiency** - Check for performance improvements

## How to Use

/simplify             - Review all changed files
/simplify <file>      - Review specific file
/simplify <dir>       - Review directory

## Note

This is a slash command that triggers AI analysis.
The actual code review will be performed by the LLM with full context awareness.

To run: Ask me to "simplify the code in <target>"
"#,
        target
    );

    Ok(SlashCommandResult {
        command: "/simplify".to_string(),
        output,
        success: true,
    })
}

/// Run /review command - full code review
fn run_review(args: &[String], state: &crate::state::AppState) -> Result<SlashCommandResult> {
    let scope = args.first().map(|s| s.as_str()).unwrap_or("changes");

    let output = format!(
        r#"# /review - Full Code Review

Reviewing: {}

## Review Checklist

- [ ] Correctness - Does the code work as intended?
- [ ] Security - Any security vulnerabilities?
- [ ] Performance - Any bottlenecks or inefficiencies?
- [ ] Maintainability - Is the code easy to understand?
- [ ] Testing - Are there adequate tests?
- [ ] Documentation - Is the code documented?
- [ ] Error Handling - Are errors handled properly?
- [ ] Edge Cases - Are boundary conditions handled?

## Review Scope

- `changes` - Review only changed files (git diff)
- `all` - Full codebase review
- `<file>` - Review specific file

## How to Use

/review changes     - Review git changes
/review all         - Full codebase review
/review <file>      - Review specific file

To run: Ask me to "review the code"
"#,
        scope
    );

    Ok(SlashCommandResult {
        command: "/review".to_string(),
        output,
        success: true,
    })
}

/// Run /context command - show context usage
fn run_context(state: &crate::state::AppState) -> Result<SlashCommandResult> {
    let input_tokens = state.conversation_history.iter()
        .filter(|m| m.role == "user")
        .map(|m| m.content.len() / 4)
        .sum::<usize>();
    let output_tokens = state.conversation_history.iter()
        .filter(|m| m.role == "assistant")
        .map(|m| m.content.len() / 4)
        .sum::<usize>();
    let total_tokens = input_tokens + output_tokens;
    let message_count = state.conversation_history.len();
    let response_count = state.conversation_history.iter()
        .filter(|m| m.role == "assistant")
        .count();

    // Get model info
    let model = state.config.model.as_deref().unwrap_or("claude-sonnet-4-6");

    // Calculate context usage (assuming 200K context window)
    let context_window = 200_000;
    let usage_percent = (total_tokens as f64 / context_window as f64 * 100.0).min(100.0);

    let output = format!(
        r#"# /context - Context Window Usage

## Current Session

- Model: {}
- Messages: {}
- Responses: {}
- Estimated Input Tokens: ~{}
- Estimated Output Tokens: ~{}
- Total Estimated Tokens: ~{}
- Context Window: {} tokens
- Usage: {:.1}%

## Context Tips

- Use /compact to compress conversation history
- Break large tasks into smaller steps
- Use /clear to start fresh if context is full
- Consider using a smaller model for simple tasks

## Storage Location

Session history: ~/.config/code-buddy/sessions/
"#,
        model,
        message_count,
        response_count,
        input_tokens,
        output_tokens,
        total_tokens,
        context_window,
        usage_percent
    );

    Ok(SlashCommandResult {
        command: "/context".to_string(),
        output,
        success: true,
    })
}

/// Run /cost command - show token usage and costs
fn run_cost(state: &crate::state::AppState) -> Result<SlashCommandResult> {
    let input_tokens: usize = state.conversation_history.iter()
        .filter(|m| m.role == "user")
        .map(|m| m.content.len() / 4)
        .sum();
    let output_tokens: usize = state.conversation_history.iter()
        .filter(|m| m.role == "assistant")
        .map(|m| m.content.len() / 4)
        .sum();

    // Find pricing for current model
    let model = state.config.model.as_deref().unwrap_or("claude-sonnet-4-6");
    let (_model_name, input_price, output_price) = PRICING
        .iter()
        .find(|(name, _, _)| model.contains(name))
        .copied()
        .unwrap_or(("claude-sonnet-4-6", 3.0, 15.0));

    let input_cost = (input_tokens as f64 / 1_000_000.0) * input_price;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * output_price;
    let total_cost = input_cost + output_cost;

    let output = format!(
        r#"# /cost - Token Usage and Cost

## Current Session

- Model: {}
- Input Tokens (est.): ~{}
- Output Tokens (est.): ~{}
- Total Tokens (est.): ~{}

## Pricing (per 1M tokens)

| Model | Input | Output |
|-------|-------|--------|
| claude-opus-4-6 | $15.00 | $75.00 |
| claude-sonnet-4-6 | $3.00 | $15.00 |
| claude-haiku-4-5 | $0.25 | $1.25 |

## Estimated Cost

- Input Cost: ${:.6}
- Output Cost: ${:.6}
- **Total: ${:.6}**

## Note

Costs are estimates based on approximate token counts.
Actual costs may vary based on exact tokenization.

To view detailed usage: code-buddy status
"#,
        model,
        input_tokens,
        output_tokens,
        input_tokens + output_tokens,
        input_cost,
        output_cost,
        total_cost
    );

    Ok(SlashCommandResult {
        command: "/cost".to_string(),
        output,
        success: true,
    })
}

/// Get help text for all slash commands
pub fn get_help_text() -> &'static str {
    r#"# Slash Commands

Available built-in slash commands:

## Code Quality
- `/simplify` - Review code for quality, reuse, and efficiency improvements
- `/review` - Full code review with checklist

## Session Management
- `/context` - Show context window usage
- `/cost` - Show token usage and estimated costs
- `/help` - Show this help message

## Usage

Type a slash command at the prompt:
  /simplify
  /review
  /context
  /cost

Or use it inline:
  /simplify src/main.rs
  /review changes
"#
}

/// List all available slash commands
pub fn list_commands() -> Vec<(&'static str, &'static str)> {
    vec![
        ("simplify", "Review code for quality issues"),
        ("review", "Full code review"),
        ("context", "Show context usage"),
        ("cost", "Show token usage and costs"),
        ("help", "Show this help"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_commands() {
        let commands = list_commands();
        assert!(!commands.is_empty());
        assert!(commands.iter().any(|(cmd, _)| *cmd == "simplify"));
        assert!(commands.iter().any(|(cmd, _)| *cmd == "review"));
    }

    #[test]
    fn test_pricing_lookup() {
        let (_name, input, output) = PRICING
            .iter()
            .find(|(name, _, _)| *name == "claude-sonnet-4-6")
            .copied()
            .unwrap();
        assert_eq!(input, 3.0);
        assert_eq!(output, 15.0);
    }
}
