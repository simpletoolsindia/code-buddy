//! Authentication commands

use crate::cli::auth::AuthCommand;
use crate::state::AppState;
use anyhow::Result;
use std::io::{self, Write};

pub async fn run(subcommand: Option<AuthCommand>, state: &mut AppState) -> Result<i32> {
    match subcommand {
        Some(AuthCommand::Login { api_key }) => {
            login(api_key, state).await
        }
        Some(AuthCommand::Logout) => {
            logout(state).await
        }
        Some(AuthCommand::Status) => {
            status(state)
        }
        None => {
            println!("Auth command requires a subcommand:");
            println!("  code-buddy auth login [--api-key KEY]");
            println!("  code-buddy auth logout");
            println!("  code-buddy auth status");
            Ok(0)
        }
    }
}

pub async fn login(api_key: Option<String>, state: &mut AppState) -> Result<i32> {
    let key = if let Some(key) = api_key {
        key
    } else {
        print!("Enter API key: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    if key.is_empty() {
        eprintln!("Error: API key cannot be empty");
        return Ok(1);
    }

    // Validate key format (should start with sk-ant-)
    if !key.starts_with("sk-ant-") && !key.starts_with("sk-") {
        eprintln!("Warning: API key may not be in the correct format");
    }

    state.config.api_key = Some(key);
    if let Err(e) = state.save_config() {
        eprintln!("Warning: Failed to save config: {}", e);
    }

    println!("Successfully logged in!");
    Ok(0)
}

pub async fn logout(state: &mut AppState) -> Result<i32> {
    state.config.api_key = None;
    if let Err(e) = state.save_config() {
        eprintln!("Warning: Failed to save config: {}", e);
    }
    println!("Logged out successfully");
    Ok(0)
}

pub fn status(state: &AppState) -> Result<i32> {
    print!("Authentication: ");
    if state.config.api_key.is_some() {
        println!("✓ Logged in");
        // Show masked key
        if let Some(key) = &state.config.api_key {
            let masked = if key.len() > 8 {
                format!("{}...{}", &key[..8], &key[key.len()-4..])
            } else {
                "***".to_string()
            };
            println!("  API Key: {}", masked);
        }
    } else {
        println!("✗ Not logged in");
        println!("\nRun 'code-buddy auth login' or 'code-buddy login <api-key>' to authenticate");
    }
    Ok(0)
}
