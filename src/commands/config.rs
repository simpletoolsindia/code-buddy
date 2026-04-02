//! Configuration commands

use crate::cli::config::ConfigCommand;
use crate::state::AppState;
use anyhow::Result;
use std::io::{self, Write};

pub async fn run(subcommand: Option<ConfigCommand>, state: &mut AppState) -> Result<i32> {
    match subcommand {
        Some(ConfigCommand::List) => {
            list_config(state)
        }
        Some(ConfigCommand::Get { key }) => {
            get_config(&key, state)
        }
        Some(ConfigCommand::Set { key, value }) => {
            set_config(&key, &value, state)
        }
        Some(ConfigCommand::Edit) => {
            edit_config(state)
        }
        None => {
            println!("Config command requires a subcommand:");
            println!("  code-buddy config list");
            println!("  code-buddy config get <key>");
            println!("  code-buddy config set <key> <value>");
            println!("  code-buddy config edit");
            Ok(0)
        }
    }
}

fn list_config(state: &AppState) -> Result<i32> {
    println!("=== Code Buddy Configuration ===\n");

    println!("api_key: {}", if state.config.api_key.is_some() { "***" } else { "not set" });
    println!("llm_provider: {}", state.config.llm_provider);
    println!("model: {:?}", state.config.model.as_deref().unwrap_or("default"));
    println!("permission_mode: {:?}", state.config.permission_mode.as_deref().unwrap_or("default"));
    println!("base_url: {:?}", state.config.base_url.as_deref().unwrap_or("api.anthropic.com"));
    println!("additional_dirs: {}", state.config.additional_dirs.len());

    for dir in &state.config.additional_dirs {
        println!("  - {}", dir.display());
    }

    println!("\nagents: {}", state.config.agents.len());

    Ok(0)
}

fn get_config(key: &str, state: &AppState) -> Result<i32> {
    match key {
        "api_key" => {
            println!("{}", state.config.api_key.as_deref().unwrap_or("not set"));
        }
        "llm_provider" => {
            println!("{}", state.config.llm_provider);
        }
        "model" => {
            println!("{:?}", state.config.model.as_deref().unwrap_or("default"));
        }
        "base_url" => {
            println!("{:?}", state.config.base_url.as_deref().unwrap_or("api.anthropic.com"));
        }
        "permission_mode" => {
            println!("{:?}", state.config.permission_mode.as_deref().unwrap_or("default"));
        }
        _ => {
            eprintln!("Unknown config key: {}", key);
            return Ok(1);
        }
    }
    Ok(0)
}

fn set_config(key: &str, value: &str, state: &mut AppState) -> Result<i32> {
    match key {
        "llm_provider" => {
            state.config.llm_provider = value.to_string();
            println!("Set llm_provider to: {}", value);
        }
        "model" => {
            if value.is_empty() {
                state.config.model = None;
                println!("Cleared model (will use provider default)");
            } else {
                state.config.model = Some(value.to_string());
                println!("Set model to: {}", value);
            }
        }
        "base_url" => {
            if value.is_empty() {
                state.config.base_url = None;
                println!("Cleared base_url (will use provider default)");
            } else {
                state.config.base_url = Some(value.to_string());
                println!("Set base_url to: {}", value);
            }
        }
        "api_key" => {
            state.config.api_key = Some(value.to_string());
            println!("Set api_key to: {}...", &value[..8.min(value.len())]);
        }
        "permission_mode" => {
            state.config.permission_mode = Some(value.to_string());
            println!("Set permission_mode to: {}", value);
            if value == "bypass" {
                println!("\n⚠️  Warning: Bypass permissions is dangerous! All commands will be auto-approved.\n");
            }
        }
        _ => {
            eprintln!("Cannot set config key: {}. Use code-buddy config edit for full config.", key);
            return Ok(1);
        }
    }

    state.save_config()?;
    Ok(0)
}

fn edit_config(state: &mut AppState) -> Result<i32> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    if let Some(config_path) = &state.config.config_path {
        println!("Opening config file: {}", config_path.display());
        let status = std::process::Command::new(&editor)
            .arg(config_path)
            .status()?;

        if !status.success() {
            eprintln!("Editor exited with error");
            return Ok(1);
        }

        // Reload config
        state.load_config()?;
        println!("Config updated successfully");
    } else {
        eprintln!("No config file found");
        return Ok(1);
    }

    Ok(0)
}
