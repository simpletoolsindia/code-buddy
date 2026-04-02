//! MCP server management commands

use crate::cli::mcp::McpCommand;
use crate::state::AppState;
use anyhow::{Context, Result};
use std::fs;

pub async fn run(subcommand: Option<McpCommand>, state: &mut AppState) -> Result<i32> {
    match subcommand {
        Some(McpCommand::Add { name, command }) => {
            // Parse command into command and args
            let parts: Vec<&str> = command.split_whitespace().collect();
            let cmd = parts.first().unwrap_or(&"");
            let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
            add_server(&name, cmd, &args, state).await
        }
        Some(McpCommand::AddJson { name, json }) => {
            add_server_json(&name, &json, state).await
        }
        Some(McpCommand::AddFromClaudeDesktop) => {
            add_from_desktop(state).await
        }
        Some(McpCommand::List) => {
            list_servers(state)
        }
        Some(McpCommand::Get { name }) => {
            get_server(&name, state)
        }
        Some(McpCommand::Remove { name }) => {
            remove_server(&name, state).await
        }
        Some(McpCommand::Serve) => {
            serve(state).await
        }
        Some(McpCommand::ResetProjectChoices) => {
            reset_project_choices(state)
        }
        None => {
            println!("MCP command requires a subcommand:");
            println!("  code-buddy mcp add <name> <command-or-url> [args...]");
            println!("  code-buddy mcp add-json <name> <json-config>");
            println!("  code-buddy mcp add-from-claude-desktop");
            println!("  code-buddy mcp list");
            println!("  code-buddy mcp get <name>");
            println!("  code-buddy mcp remove <name>");
            println!("  code-buddy mcp serve");
            println!("  code-buddy mcp reset-project-choices");
            Ok(0)
        }
    }
}

async fn add_server(name: &str, command_or_url: &str, args: &[String], state: &mut AppState) -> Result<i32> {
    println!("Adding MCP server: {} -> {}", name, command_or_url);

    let server_config = serde_json::json!({
        "name": name,
        "command": command_or_url,
        "args": args,
    });

    state.config.mcp_servers.insert(name.to_string(), server_config);

    if let Err(e) = state.save_config() {
        eprintln!("Failed to save config: {}", e);
        return Ok(1);
    }

    println!("Server '{}' added successfully", name);
    Ok(0)
}

async fn add_server_json(name: &str, json: &str, state: &mut AppState) -> Result<i32> {
    println!("Adding MCP server from JSON: {}", name);

    let config: serde_json::Value = serde_json::from_str(json)
        .context("Invalid JSON config")?;

    state.config.mcp_servers.insert(name.to_string(), config);

    if let Err(e) = state.save_config() {
        eprintln!("Failed to save config: {}", e);
        return Ok(1);
    }

    println!("Server '{}' added successfully", name);
    Ok(0)
}

async fn add_from_desktop(state: &mut AppState) -> Result<i32> {
    println!("Importing MCP servers from Claude Desktop...");

    // Check common locations for Claude Desktop config
    let config_paths = vec![
        dirs::home_dir()
            .map(|h| h.join("Library/Application Support/Claude/claude_desktop_config.json")),
        dirs::home_dir()
            .map(|h| h.join(".config/Claude/claude_desktop_config.json")),
    ];

    let mut found = false;
    for path in config_paths.into_iter().flatten() {
        if path.exists() {
            println!("Found config at: {}", path.display());
            let content = fs::read_to_string(&path).context("Failed to read config")?;
            let config: serde_json::Value = serde_json::from_str(&content)?;

            if let Some(mcp) = config.get("mcp_servers").and_then(|v| v.as_object()) {
                for (name, server_config) in mcp {
                    println!("Importing server: {}", name);
                    state.config.mcp_servers.insert(name.clone(), server_config.clone());
                }
            }

            if let Err(e) = state.save_config() {
                eprintln!("Failed to save config: {}", e);
                return Ok(1);
            }

            found = true;
            println!("Import complete!");
            break;
        }
    }

    if !found {
        eprintln!("Claude Desktop config not found.");
        eprintln!("Make sure Claude Desktop is installed and has been run at least once.");
        return Ok(1);
    }

    Ok(0)
}

fn list_servers(state: &AppState) -> Result<i32> {
    println!("=== MCP Servers ===\n");

    if state.config.mcp_servers.is_empty() {
        println!("No MCP servers configured.");
        println!("\nAdd a server with:");
        println!("  code-buddy mcp add <name> <command-or-url> [args...]");
    } else {
        for (name, config) in &state.config.mcp_servers {
            println!("Server: {}", name);
            if let Some(cmd) = config.get("command").or(config.get("url")) {
                println!("  Command/URL: {}", cmd);
            }
            if let Some(args) = config.get("args").and_then(|v| v.as_array()) {
                println!("  Args: {}", args.iter().map(|v| v.as_str().unwrap_or("")).collect::<Vec<_>>().join(" "));
            }
            println!();
        }
    }

    Ok(0)
}

fn get_server(name: &str, state: &AppState) -> Result<i32> {
    match state.config.mcp_servers.get(name) {
        Some(config) => {
            println!("=== Server: {} ===", name);
            println!("{}", serde_json::to_string_pretty(config)?);
        }
        None => {
            eprintln!("Server '{}' not found", name);
            return Ok(1);
        }
    }
    Ok(0)
}

async fn remove_server(name: &str, state: &mut AppState) -> Result<i32> {
    if state.config.mcp_servers.remove(name).is_some() {
        if let Err(e) = state.save_config() {
            eprintln!("Failed to save config: {}", e);
            return Ok(1);
        }
        println!("Server '{}' removed successfully", name);
    } else {
        eprintln!("Server '{}' not found", name);
        return Ok(1);
    }
    Ok(0)
}

async fn serve(_state: &mut AppState) -> Result<i32> {
    println!("Starting MCP server...");
    println!("MCP server functionality requires the full implementation.");
    println!("Use 'code-buddy mcp serve' for MCP support.");
    Ok(0)
}

fn reset_project_choices(state: &mut AppState) -> Result<i32> {
    state.config.project_choices.clear();
    if let Err(e) = state.save_config() {
        eprintln!("Failed to save config: {}", e);
        return Ok(1);
    }
    println!("Project choices reset successfully");
    Ok(0)
}
