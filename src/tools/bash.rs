//! Bash tool - Execute shell commands

use anyhow::{Context, Result};
use std::process::Command;

use super::Tool;

pub struct BashTool {
    timeout_seconds: u64,
}

impl BashTool {
    pub fn new() -> Self {
        Self { timeout_seconds: 120 }
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for BashTool {
    fn name(&self) -> &str {
        "Bash"
    }

    fn description(&self) -> &str {
        "Execute shell commands"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            anyhow::bail!("Bash tool requires a command argument");
        }

        let command = &args[0];
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .args(&args[1..])
            .output()
            .context("Failed to execute command")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let result = if output.status.success() {
            stdout.to_string()
        } else {
            format!("Error (exit {}):\n{}\n{}", output.status, stderr, stdout)
        };

        Ok(result)
    }
}
