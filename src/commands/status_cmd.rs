//! Status Command - Display status information
//!
//! Provides session and system status information.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Run status command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return show_status();
    }

    match args[0].as_str() {
        "session" => session_status(),
        "model" => model_status(),
        "tools" => tools_status(),
        "config" => config_status(),
        "version" => version_status(),
        _ => show_status(),
    }
}

fn show_status() -> Result<String> {
    let output = r#"# Code Buddy Status

## Session
- Messages: 0
- Context: 0%
- Agent: default

## Model
- Current: claude-sonnet-4-5
- Provider: anthropic
- Vision: enabled

## Tools
- Enabled: 17
- Denied: 0

## Config
- Permission mode: Ask
- Auto compact: enabled
- Fast mode: disabled
"#;
    Ok(output.to_string())
}

fn session_status() -> Result<String> {
    let output = r#"# Session Status

| Metric | Value |
|--------|-------|
| Session ID | abc123 |
| Messages | 0 |
| Context Usage | 0% |
| Current Agent | default |
| Session Start | Now |
"#;
    Ok(output.to_string())
}

fn model_status() -> Result<String> {
    let output = r#"# Model Status

**Current Model:** claude-sonnet-4-5

| Property | Value |
|----------|-------|
| Provider | anthropic |
| Context Window | 200k tokens |
| Vision | Enabled |
| Tools | Enabled |
"#;
    Ok(output.to_string())
}

fn tools_status() -> Result<String> {
    let output = r#"# Tools Status

**Enabled Tools:**
- Read, Write, Edit, Glob, Grep
- Bash, WebSearch, WebFetch
- AskUserQuestion, NotebookEdit
- ListMcpResources, ReadMcpResource
- TaskCreate, TaskComplete

**Denied Tools:**
None
"#;
    Ok(output.to_string())
}

fn config_status() -> Result<String> {
    let output = r#"# Config Status

| Setting | Value |
|---------|-------|
| Permission Mode | Ask |
| Auto Compact | true |
| Compact Threshold | 80% |
| Fast Mode | false |
| Temperature | 0.7 |
"#;
    Ok(output.to_string())
}

fn version_status() -> Result<String> {
    Ok(format!(
        "# Version\n\ncode-buddy {}\n",
        env!("CARGO_PKG_VERSION")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status() {
        let status = show_status().unwrap();
        assert!(status.contains("Code Buddy Status"));
    }
}
