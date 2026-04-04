//! Status command - Show system and authentication status

use crate::mlx::MlxConfig;
use crate::state::AppState;
use anyhow::Result;

pub async fn run(state: &mut AppState) -> Result<i32> {
    println!();
    println!("\x1b[1m╭─────────────────────────────────────────────────────────────\x1b[0m");
    println!("\x1b[1m│\x1b[0m  \x1b[1mCode Buddy Status\x1b[0m");
    println!("\x1b[1m╰─────────────────────────────────────────────────────────────\x1b[0m");
    println!();

    // Provider
    let provider_display: String = if state.config.base_url.is_some() && !state.config.base_url.as_ref().unwrap().is_empty() {
        // Custom URL set - show both provider name and URL
        let base = state.config.base_url.as_ref().unwrap();
        if base.contains("nvidia") {
            "NVIDIA NIM (custom endpoint)".to_string()
        } else if base.contains("openrouter") {
            "OpenRouter (custom endpoint)".to_string()
        } else if base.contains("anthropic") {
            "Anthropic (custom endpoint)".to_string()
        } else if base.contains("openai") {
            "OpenAI (custom endpoint)".to_string()
        } else {
            format!("Custom ({})", base)
        }
    } else {
        match state.config.llm_provider.as_str() {
            "ollama" => "Ollama (local, free!)".to_string(),
            "openrouter" => "OpenRouter".to_string(),
            "anthropic" => "Anthropic (Claude)".to_string(),
            "openai" => "OpenAI (GPT)".to_string(),
            "nvidia" => "NVIDIA NIM".to_string(),
            "groq" => "Groq".to_string(),
            "deepseek" => "DeepSeek".to_string(),
            "mlx" => "MLX (Apple Silicon)".to_string(),
            other => other.to_string(),
        }
    };
    println!("\x1b[1mProvider:\x1b[0m \x1b[36m{}\x1b[0m", provider_display);

    // Model
    let model = state.config.model.as_deref().unwrap_or("default");
    println!("\x1b[1mModel:\x1b[0m    \x1b[36m{}\x1b[0m", model);

    // API Key
    if state.config.api_key.is_some() {
        println!("\x1b[1mAPI Key:\x1b[0m  \x1b[32m✓ Configured\x1b[0m");
    } else if state.config.llm_provider == "ollama" || state.config.llm_provider == "mlx" {
        println!("\x1b[1mAPI Key:\x1b[0m  \x1b[32m✓ Not needed (local)\x1b[0m");
    } else {
        println!("\x1b[1mAPI Key:\x1b[0m  \x1b[31m⚠ Not set\x1b[0m");
    }

    // Messages count
    println!();
    println!("\x1b[1mConversation:\x1b[0m {} messages", state.conversation_history.len());

    // MLX-specific info
    if state.config.llm_provider == "mlx" {
        let mlx_config = MlxConfig::new();
        println!();
        println!("\x1b[1m╭─ MLX Info ────────────────────────────────────────────────\x1b[0m");
        println!("│  mlx-lm installed: {}", if mlx_config.check_mlx_lm_installed() { "\x1b[32mYes\x1b[0m" } else { "\x1b[31mNo\x1b[0m" });
        if !mlx_config.cached_models.is_empty() {
            println!("│  Cached models:");
            for model in &mlx_config.cached_models {
                println!("│    - {}", model);
            }
        }
        println!("\x1b[1m╰─────────────────────────────────────────────────────────────\x1b[0m");
    }

    // System info
    println!();
    println!("\x1b[1m╭─ System ───────────────────────────────────────────────────\x1b[0m");
    println!("│  Version: {}", env!("CARGO_PKG_VERSION"));
    println!("\x1b[1m╰─────────────────────────────────────────────────────────────\x1b[0m");

    // MLX availability notice
    if MlxConfig::is_apple_silicon() {
        println!();
        println!("\x1b[33m💡\x1b[0m MLX is available on your Apple Silicon Mac!");
        println!("   Run \x1b[32mcode-buddy --mlx\x1b[0m to set up local AI for free.");
    }

    // Help
    println!();
    println!("\x1b[90mRun \x1b[32mcode-buddy --setup\x1b[0m\x1b[90m to change settings.\x1b[0m");
    println!("\x1b[90mRun \x1b[32mcode-buddy --doctor\x1b[0m\x1b[90m to check for problems.\x1b[0m");
    println!();

    Ok(0)
}
