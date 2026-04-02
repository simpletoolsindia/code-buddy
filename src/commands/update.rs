//! Update command - Update CLI to latest version

use crate::state::AppState;
use anyhow::{Context, Result};
use std::process::Command;

pub async fn run(_state: &mut AppState) -> Result<i32> {
    println!("Checking for updates...");

    let output = Command::new("npm")
        .args(["view", "@anthropic-ai/claude-code", "version"])
        .output()
        .context("Failed to check npm")?;

    if !output.status.success() {
        eprintln!("Failed to check for updates");
        return Ok(1);
    }

    let latest = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let current = env!("CARGO_PKG_VERSION");

    if latest == current {
        println!("You're running the latest version: {}", current);
    } else {
        println!("Update available: {} -> {}", current, latest);
        println!("Run: npm install -g @anthropic-ai/claude-code");
    }

    Ok(0)
}
