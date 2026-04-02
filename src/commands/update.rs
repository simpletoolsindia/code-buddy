//! Update command - Check for and apply updates

use crate::state::AppState;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Command;

const REPO_OWNER: &str = "simpletoolsindia";
const REPO_NAME: &str = "code-buddy";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubResponse {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    assets: Vec<GithubAsset>,
}

pub async fn run(state: &mut AppState) -> Result<i32> {
    let verbose = state.config.llm_provider == "debug";

    check_and_update(verbose).await
}

pub async fn check_and_update(verbose: bool) -> Result<i32> {
    let current = env!("CARGO_PKG_VERSION");

    println!("Checking for updates...");

    // Check GitHub for latest release
    match get_latest_release().await {
        Ok(release) => {
            let latest = release.tag_name.trim_start_matches('v');

            if verbose {
                println!("Current version: {}", current);
                println!("Latest version:  {}", latest);
            }

            if is_newer_version(latest, current) {
                println!();
                println!("╔══════════════════════════════════════════════════════════════╗");
                println!("║                    Update Available!                           ║");
                println!("╠══════════════════════════════════════════════════════════════╣");
                println!("║  Current: {}                                                ║", current);
                println!("║  Latest:   {}                                                ║", latest);
                println!("╚══════════════════════════════════════════════════════════════╝");
                println!();

                if let Some(body) = &release.body {
                    if !body.is_empty() {
                        println!("Release notes:");
                        println!("{}", body.lines().take(10).collect::<Vec<_>>().join("\n"));
                        if body.lines().count() > 10 {
                            println!("... (see {} for full notes)", release.html_url);
                        }
                        println!();
                    }
                }

                println!("To update, run:");
                println!();
                println!("  curl -fsSL https://raw.githubusercontent.com/{}/{}/main/install-simple.sh | bash", REPO_OWNER, REPO_NAME);
                println!();
                println!("Or:");
                println!("  cargo install --git https://github.com/{}/{}.git", REPO_OWNER, REPO_NAME);
                println!();

                Ok(1) // Return 1 to indicate update available
            } else {
                println!("✓ You're running the latest version: {}", current);
                Ok(0)
            }
        }
        Err(e) => {
            println!("✓ You're running version: {}", current);
            if verbose {
                eprintln!("(Could not check for updates: {})", e);
            }
            Ok(0)
        }
    }
}

/// Check for updates silently, return true if update available
pub async fn check_update_silent() -> Result<Option<String>> {
    let current = env!("CARGO_PKG_VERSION");

    match get_latest_release().await {
        Ok(release) => {
            let latest = release.tag_name.trim_start_matches('v');
            if is_newer_version(latest, current) {
                Ok(Some(latest.to_string()))
            } else {
                Ok(None)
            }
        }
        Err(_) => Ok(None),
    }
}

async fn get_latest_release() -> Result<GithubResponse> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        REPO_OWNER, REPO_NAME
    );

    let output = reqwest::get(&url)
        .await
        .context("Failed to fetch releases")?
        .json::<GithubResponse>()
        .await
        .context("Failed to parse release info")?;

    Ok(output)
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest_parts = parse_version(latest);
    let current_parts = parse_version(current);

    for (l, c) in latest_parts.iter().zip(current_parts.iter()) {
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }

    // If all parts equal, check if latest has more parts (e.g., 1.0.1 > 1.0)
    latest_parts.len() > current_parts.len()
}

fn parse_version(version: &str) -> Vec<u32> {
    version
        .split(['-', '+'])
        .next()
        .unwrap_or(version)
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// Perform the update
pub async fn perform_update() -> Result<i32> {
    println!("Updating Code Buddy...\n");

    // Check if cargo is available
    if Command::new("cargo").arg("--version").output().is_err() {
        // Fallback to install script
        println!("Using installer script...\n");

        let install_cmd = format!(
            "curl -fsSL https://raw.githubusercontent.com/{}/{}/main/install-simple.sh | bash",
            REPO_OWNER, REPO_NAME
        );

        println!("Running: {}\n", install_cmd);

        let output = Command::new("sh")
            .arg("-c")
            .arg(&install_cmd)
            .output()
            .context("Failed to run installer")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Update failed: {}", stderr);
            return Ok(1);
        }

        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        // Use cargo install
        let install_cmd = format!(
            "cargo install --git https://github.com/{}/{}.git --force code-buddy",
            REPO_OWNER, REPO_NAME
        );

        println!("Running: {}\n", install_cmd);

        let output = Command::new("sh")
            .arg("-c")
            .arg(&install_cmd)
            .output()
            .context("Failed to install via cargo")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Update failed: {}", stderr);
            return Ok(1);
        }

        println!("{}", String::from_utf8_lossy(&output.stdout));
    }

    // Verify installation
    let version = Command::new("code-buddy")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    println!();
    println!("✓ Update complete! Version: {}", version);

    Ok(0)
}
