//! Interactive REPL with slash commands support

use crate::api::ApiClient;
use crate::state::AppState;
use anyhow::Result;
use std::io::{self, Write};

const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/help", "Show available commands"),
    ("/quit", "Exit Code Buddy"),
    ("/exit", "Exit Code Buddy"),
    ("/clear", "Clear conversation history"),
    ("/status", "Show current configuration"),
    ("/model", "Change model"),
    ("/provider", "Change LLM provider"),
    ("/history", "Show conversation history"),
    ("/reset", "Reset conversation"),
    ("/models", "List available models"),
    ("/cost", "Show estimated costs"),
    ("/compact", "Compact context window"),
    ("/context", "Show context usage"),
    ("/system", "Show system prompt"),
    ("/set", "Set configuration option"),
];

pub async fn run(state: &mut AppState) -> Result<i32> {
    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    Code Buddy REPL                             ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Type your prompts or use /commands                          ║");
    println!("║  Type /help for available commands                          ║");
    println!("║  Type /quit or /exit to leave                             ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Show current config
    show_status(state);
    println!();

    loop {
        print!("❯ ");
        io::stdout().flush()?;

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                println!("\nGoodbye!");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("\nError reading input: {}", e);
                break;
            }
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        // Handle slash commands
        if input.starts_with('/') {
            let result = handle_slash_command(input, state).await?;
            if result == 1 {
                break;
            }
            continue;
        }

        // Handle regular prompt
        match handle_prompt(input, state).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(0)
}

async fn handle_slash_command(input: &str, state: &mut AppState) -> Result<i32> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts.first().unwrap_or(&"").to_lowercase();

    match cmd.as_str() {
        "/help" | "/?" => {
            show_help();
        }
        "/quit" | "/exit" | "/q" => {
            println!("Goodbye!");
            return Ok(1);
        }
        "/clear" | "/cls" => {
            state.clear_history();
            println!("✓ Conversation history cleared\n");
        }
        "/status" => {
            show_status(state);
        }
        "/model" => {
            if let Some(model) = parts.get(1) {
                state.config.model = Some(model.to_string());
                state.save_config()?;
                println!("✓ Model set to: {}\n", model);
            } else {
                let current = state.config.model.as_deref().unwrap_or("default");
                println!("Current model: {}\n", current);
                println!("Usage: /model <model-name>\n");
            }
        }
        "/provider" => {
            if let Some(provider) = parts.get(1) {
                state.config.llm_provider = provider.to_string();
                state.save_config()?;
                println!("✓ Provider set to: {}\n", provider);
            } else {
                println!("Current provider: {}\n", state.config.llm_provider);
                println!("Usage: /provider <provider-name>\n");
            }
        }
        "/history" => {
            println!("=== Conversation History ===\n");
            for (i, msg) in state.conversation_history.iter().enumerate() {
                let role = if msg.role == "user" { "You" } else { "Buddy" };
                let preview = if msg.content.len() > 50 {
                    format!("{}...", &msg.content[..50])
                } else {
                    msg.content.clone()
                };
                println!("[{}] {}: {}", i + 1, role, preview);
            }
            println!();
        }
        "/reset" => {
            state.clear_history();
            println!("✓ Conversation reset\n");
        }
        "/models" => {
            println!("=== Available Models ===\n");
            println!("Current provider: {}\n", state.config.llm_provider);
            println!("Use 'code-buddy model <name>' to change model\n");
        }
        "/cost" => {
            println!("=== Cost Estimation ===\n");
            let input_tokens: u32 = state.conversation_history.iter()
                .filter(|m| m.role == "user")
                .map(|m| (m.content.len() / 4) as u32)
                .sum();
            let output_tokens: u32 = state.conversation_history.iter()
                .filter(|m| m.role == "assistant")
                .map(|m| (m.content.len() / 4) as u32)
                .sum();
            println!("Input tokens (estimated): {}", input_tokens);
            println!("Output tokens (estimated): {}", output_tokens);
            println!("Total tokens: {}", input_tokens + output_tokens);
            println!();
        }
        "/context" => {
            println!("=== Context Usage ===\n");
            let total = state.conversation_history.len();
            println!("Messages in context: {}", total);
            println!("Provider: {}", state.config.llm_provider);
            println!();
        }
        "/system" => {
            println!("=== System Configuration ===\n");
            println!("Provider: {}", state.config.llm_provider);
            println!("Model: {:?}", state.config.model.as_deref().unwrap_or("default"));
            println!("API Key: {}\n", if state.config.api_key.is_some() { "Configured" } else { "Not set" });
        }
        "/set" => {
            if parts.len() >= 3 {
                let key = parts[1];
                let value = parts[2];
                match key {
                    "provider" => {
                        state.config.llm_provider = value.to_string();
                        state.save_config()?;
                        println!("✓ Provider set to: {}\n", value);
                    }
                    "model" => {
                        state.config.model = Some(value.to_string());
                        state.save_config()?;
                        println!("✓ Model set to: {}\n", value);
                    }
                    _ => {
                        println!("Unknown setting: {}\n", key);
                    }
                }
            } else {
                println!("Usage: /set <key> <value>\n");
            }
        }
        _ => {
            println!("Unknown command: {}\n", cmd);
            println!("Type /help for available commands\n");
        }
    }

    Ok(0)
}

async fn handle_prompt(prompt: &str, state: &mut AppState) -> Result<()> {
    print!("\n");

    let api_client = ApiClient::new(state)?;
    let response = api_client.complete(prompt, &state.config, state).await?;

    println!("\nBuddy: {}", response.content);
    println!("[Tokens: {}]\n", response.usage.total_tokens);

    // Update conversation history
    state.add_message("user", prompt);
    state.add_message("assistant", &response.content);

    Ok(())
}

fn show_help() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    Available Commands                         ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    for (cmd, desc) in SLASH_COMMANDS {
        println!("║  {:15}  {}", cmd, desc);
    }
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
}

fn show_status(state: &AppState) {
    println!("=== Current Configuration ===");
    println!("Provider: {}", state.config.llm_provider);
    println!("Model: {:?}", state.config.model.as_deref().unwrap_or("default"));
    println!("API Key: {}", if state.config.api_key.is_some() { "Configured ✓" } else { "Not set ✗" });
    println!("Messages in history: {}", state.conversation_history.len());
    println!("Config file: {:?}", state.config.config_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default());
}
