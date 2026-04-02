//! Grep tool - Search file contents using ripgrep

use anyhow::{Context, Result};
use std::process::Command;

use super::Tool;

/// Grep tool using ripgrep
pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    fn description(&self) -> &str {
        "Search file contents using ripgrep"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            anyhow::bail!("Grep tool requires a pattern argument");
        }

        let pattern = &args[0];
        let path = args.get(1).map(|p| p.as_str()).unwrap_or(".");
        let flags = args.get(2..).map(|a| a.join(" ")).unwrap_or_default();

        let mut cmd = Command::new("rg");

        // Add common flags
        cmd.arg("--color=never")
           .arg("--line-number")
           .arg(pattern)
           .arg(path);

        // Add additional flags
        if !flags.is_empty() {
            for flag in flags.split_whitespace() {
                cmd.arg(flag);
            }
        }

        let output = cmd.output()
            .context("Failed to execute ripgrep")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            // grep returns 1 when no matches found
            if output.status.code() == Some(1) {
                Ok(String::new())
            } else {
                anyhow::bail!("Grep failed: {}", String::from_utf8_lossy(&output.stderr))
            }
        }
    }
}
