//! Install command - Install native build

use crate::state::AppState;
use anyhow::{Context, Result};
use std::process::Command;

pub async fn run(target: Option<String>, _state: &mut AppState) -> Result<i32> {
    let target = target.unwrap_or_else(|| "stable".to_string());

    println!("Installing Claude Code...");

    match target.as_str() {
        "stable" => {
            println!("Installing stable version...");
            install_binary().await?;
        }
        "latest" => {
            println!("Installing latest version...");
            install_binary().await?;
        }
        version => {
            println!("Installing version {}...", version);
            install_specific_version(version).await?;
        }
    }

    Ok(0)
}

async fn install_binary() -> Result<i32> {
    // Download the latest release
    let output = Command::new("curl")
        .args(["-s", "https://api.github.com/repos/anthropics/claude-code/releases/latest"])
        .output()
        .context("Failed to fetch release info")?;

    if !output.status.success() {
        eprintln!("Failed to get release info");
        return Ok(1);
    }

    println!("Binary installation not implemented - use npm or GitHub releases");
    println!("Run: npm install -g @anthropic-ai/claude-code");
    Ok(0)
}

async fn install_specific_version(_version: &str) -> Result<i32> {
    println!("Installing specific version not implemented");
    println!("Run: npm install -g @anthropic-ai/claude-code@<version>");
    Ok(0)
}
