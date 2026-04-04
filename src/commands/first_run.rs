//! First-run setup - Simple, friendly onboarding

use crate::state::AppState;
use anyhow::Result;
use std::io::{self, Write};
use std::process::Command;

/// Run first-run setup if needed
pub async fn run(state: &mut AppState) -> Result<()> {
    if !state.config.first_run {
        // Still check for Ollama availability for helpful suggestions
        if let Some(suggestion) = check_ollama_available() {
            // Only show suggestion if not already using Ollama
            if state.config.llm_provider != "ollama" {
                println!();
                println!("\x1b[33m💡 Hint: Ollama is running on your computer!\x1b[0m");
                println!("    {} - it's free and works offline.", suggestion);
                println!("    Run \x1b[32mcode-buddy --setup\x1b[0m to configure it.");
                println!();
            }
        }
        return Ok(());
    }

    print_welcome();
    let theme = ask_theme_preference()?;

    // Save the theme preference
    state.config.theme = Some(theme.clone());
    state.config.first_run = false;
    state.save_config()?;

    print_theme_selected(&theme);

    // Run LLM setup
    run_quick_setup(state).await?;

    Ok(())
}

/// Check if Ollama is available
fn check_ollama_available() -> Option<String> {
    // Try to connect to Ollama using a quick HTTP check
    use std::time::Duration;
    use std::net::TcpStream;
    use std::io::Read;

    // Quick TCP check first - is port 11434 open?
    if TcpStream::connect_timeout(
        &"127.0.0.1:11434".parse().ok()?,
        Duration::from_secs(1)
    ).is_err() {
        return None;
    }

    // Port is open, try to get model list
    let output = Command::new("curl")
        .args(["-s", "--max-time", "2", "http://localhost:11434/api/tags"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            // Parse available models
            if let Ok(json) = String::from_utf8(output.stdout) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) {
                    if let Some(models) = value.get("models").and_then(|m| m.as_array()) {
                        let count = models.len();
                        return Some(format!(
                            "Found {} model{} available locally",
                            count,
                            if count == 1 { "" } else { "s" }
                        ));
                    }
                }
            }
            return Some("Found Ollama running locally - it's free!".to_string());
        }
    }
    None
}

/// Run quick setup for LLM configuration
async fn run_quick_setup(state: &mut AppState) -> Result<()> {
    println!();
    println!("\x1b[1m╔════════════════════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[1m║              🚀 Let's set up your AI assistant!              ║\x1b[0m");
    println!("\x1b[1m╚════════════════════════════════════════════════════════════════╝\x1b[0m");
    println!();

    // Check for Ollama first
    if let Some(_) = check_ollama_available() {
        println!("\x1b[32m🎉 Great news! I found Ollama running on your computer.\x1b[0m");
        println!();
        println!("Ollama lets you use AI models for FREE - no API key needed!");
        println!();

        loop {
            print!("Use Ollama for free AI? (Y/n): ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();

            match input.as_str() {
                "" | "y" | "yes" => {
                    state.config.llm_provider = "ollama".to_string();
                    state.config.model = Some("llama3.2".to_string());
                    state.config.api_key = None;
                    state.save_config()?;

                    println!();
                    println!("\x1b[32m✓ Done! You're all set to use Code Buddy!\x1b[0m");
                    println!();
                    println!("Try it now:");
                    println!("  \x1b[36mcode-buddy\x1b[0m  - Start chatting with Code Buddy");
                    println!();
                    return Ok(());
                }
                "n" | "no" => {
                    break;
                }
                _ => {
                    println!("Please enter Y or n");
                }
            }
        }
    }

    // Show cloud provider options
    println!("No local AI found. Let's configure a cloud provider.");
    println!();
    println!("Choose how you want to use Code Buddy:");
    println!();
    println!("  \x1b[32m1\x1b[0m) NVIDIA NIM     - Free tier available (no credit card)");
    println!("  \x1b[32m2\x1b[0m) OpenRouter     - Many free models available");
    println!("  \x1b[32m3\x1b[0m) Anthropic      - Claude (requires API key)");
    println!("  \x1b[32m4\x1b[0m) OpenAI         - GPT models (requires API key)");
    println!("  \x1b[32m5\x1b[0m) Skip for now    - Configure later");
    println!();

    loop {
        print!("Enter choice (1-5): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "1" => {
                state.config.llm_provider = "nvidia".to_string();
                state.config.model = Some("meta/llama-3.1-8b-instruct".to_string());
                println!();
                println!("\x1b[32m✓ Configured NVIDIA NIM (free tier)!\x1b[0m");
                println!();
                println!("You'll need an API key from: https://ngc.nvidia.com/");
                println!("Run \x1b[32mcode-buddy --login\x1b[0m to set your API key.");
                println!();
                println!("Try it now:");
                println!("  \x1b[36mcode-buddy\x1b[0m  - Start chatting with Code Buddy");
                println!();
                return Ok(());
            }
            "2" => {
                state.config.llm_provider = "openrouter".to_string();
                state.config.model = Some("google/gemini-2.5-flash-preview-05-20:free".to_string());
                println!();
                println!("\x1b[32m✓ Configured OpenRouter with free model!\x1b[0m");
                println!();
                println!("Get a free API key from: https://openrouter.ai/keys");
                println!("Run \x1b[32mcode-buddy --login\x1b[0m to set your API key.");
                println!();
                println!("Try it now:");
                println!("  \x1b[36mcode-buddy\x1b[0m  - Start chatting with Code Buddy");
                println!();
                return Ok(());
            }
            "3" => {
                state.config.llm_provider = "anthropic".to_string();
                state.config.model = Some("claude-sonnet-4-6".to_string());
                println!();
                println!("\x1b[32m✓ Configured Anthropic Claude!\x1b[0m");
                println!();
                println!("Get an API key from: https://console.anthropic.com/");
                println!("Run \x1b[32mcode-buddy --login\x1b[0m to set your API key.");
                println!();
                println!("Try it now:");
                println!("  \x1b[36mcode-buddy\x1b[0m  - Start chatting with Code Buddy");
                println!();
                return Ok(());
            }
            "4" => {
                state.config.llm_provider = "openai".to_string();
                state.config.model = Some("gpt-4o-mini".to_string());
                println!();
                println!("\x1b[32m✓ Configured OpenAI GPT!\x1b[0m");
                println!();
                println!("Get an API key from: https://platform.openai.com/api-keys");
                println!("Run \x1b[32mcode-buddy --login\x1b[0m to set your API key.");
                println!();
                println!("Try it now:");
                println!("  \x1b[36mcode-buddy\x1b[0m  - Start chatting with Code Buddy");
                println!();
                return Ok(());
            }
            "5" => {
                println!();
                println!("No problem! Configure later with:");
                println!("  \x1b[32mcode-buddy --setup\x1b[0m  - Full setup wizard");
                println!("  \x1b[32mcode-buddy --login\x1b[0m  - Set API key");
                println!();
                return Ok(());
            }
            _ => {
                println!("Invalid choice. Please enter 1, 2, 3, 4, or 5.");
            }
        }
    }
}

/// Print welcome message
fn print_welcome() {
    println!();
    println!("\x1b[1m\x1b[36m╔════════════════════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[1m\x1b[36m║                                                                ║\x1b[0m");
    println!("\x1b[1m\x1b[36m║     ██████╗ ███████╗██╗   ██╗███╗   ███╗                      ║\x1b[0m");
    println!("\x1b[1m\x1b[36m║     ██╔══██╗██╔════╝██║   ██║████╗ ████║                      ║\x1b[0m");
    println!("\x1b[1m\x1b[36m║     ██║  ██║█████╗  ██║   ██║██╔████╔██║                      ║\x1b[0m");
    println!("\x1b[1m\x1b[36m║     ██║  ██║██╔══╝  ██║   ██║██║╚██╔╝██║                      ║\x1b[0m");
    println!("\x1b[1m\x1b[36m║     ██████╔╝███████╗╚██████╔╝██║ ╚═╝ ██║                      ║\x1b[0m");
    println!("\x1b[1m\x1b[36m║     ╚═════╝ ╚══════╝ ╚═════╝ ╚═╝     ╚═╝                      ║\x1b[0m");
    println!("\x1b[1m\x1b[36m║                                                                ║\x1b[0m");
    println!("\x1b[1m\x1b[36m║              \x1b[0m\x1b[1mYour AI Coding Companion\x1b[0m\x1b[36m                      ║\x1b[0m");
    println!("\x1b[1m\x1b[36m║                                                                ║\x1b[0m");
    println!("\x1b[1m\x1b[36m╚════════════════════════════════════════════════════════════════╝\x1b[0m");
    println!();
    println!("\x1b[1mWelcome to Code Buddy!\x1b[0m Let's get you set up in just a moment.");
    println!();
}

/// Ask user for theme preference
fn ask_theme_preference() -> Result<String> {
    println!("Choose your preferred theme:");
    println!();
    println!("  \x1b[32m1\x1b[0m) Dark   - Dark background with light text");
    println!("  \x1b[32m2\x1b[0m) Light  - Light background with dark text");
    println!("  \x1b[32m3\x1b[0m) Auto   - Follow system preference");
    println!();

    loop {
        print!("Enter choice (1/2/3) or name (dark/light/auto): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        match input.as_str() {
            "1" | "dark" => return Ok("dark".to_string()),
            "2" | "light" => return Ok("light".to_string()),
            "3" | "auto" => return Ok("auto".to_string()),
            _ => {
                println!("Invalid choice. Please enter 1, 2, or 3.");
                println!();
            }
        }
    }
}

/// Print the selected theme
fn print_theme_selected(theme: &str) {
    println!();
    match theme {
        "dark" => {
            println!("\x1b[32m✓\x1b[0m Theme set to Dark mode");
        }
        "light" => {
            println!("\x1b[32m✓\x1b[0m Theme set to Light mode");
        }
        "auto" => {
            println!("\x1b[32m✓\x1b[0m Theme set to Auto (follows system)");
        }
        _ => {}
    }
    println!("You can change this anytime with: \x1b[33mcode-buddy /theme\x1b[0m");
    println!();
}

/// Check if first run setup is needed
pub fn is_first_run(state: &AppState) -> bool {
    state.config.first_run
}
