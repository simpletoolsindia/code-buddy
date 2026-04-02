//! Reset configuration command

use crate::state::AppState;
use anyhow::Result;
use dialoguer::Select;

pub async fn run(state: &mut AppState, reset_all: bool) -> Result<i32> {
    if reset_all {
        println!("\n=== Reset All Configuration ===\n");
        println!("This will clear:");
        println!("  - LLM Provider");
        println!("  - Model");
        println!("  - API Key");
        println!("  - Base URL");
        println!("  - All settings\n");

        state.config.llm_provider = "ollama".to_string();
        state.config.model = None;
        state.config.api_key = None;
        state.config.base_url = None;

        if let Err(e) = state.save_config() {
            eprintln!("Failed to save config: {}", e);
            return Ok(1);
        }

        println!("✓ All settings have been reset!");
        println!("Run 'code-buddy setup' to reconfigure.\n");
    } else {
        println!("\n=== Reset Options ===\n");
        println!("Choose what to reset:\n");

        let choice = Select::new()
            .with_prompt("Select what to reset")
            .items(&[
                "Reset LLM Provider (set to ollama)",
                "Reset Model (clear model selection)",
                "Reset API Key (remove stored key)",
                "Reset Base URL (use default)",
                "Reset All (full factory reset)",
                "Cancel",
            ])
            .default(5)
            .interact()?;

        match choice {
            0 => {
                state.config.llm_provider = "ollama".to_string();
                if let Err(e) = state.save_config() {
                    eprintln!("Failed to save config: {}", e);
                    return Ok(1);
                }
                println!("\n✓ LLM Provider reset to 'ollama'\n");
            }
            1 => {
                state.config.model = None;
                if let Err(e) = state.save_config() {
                    eprintln!("Failed to save config: {}", e);
                    return Ok(1);
                }
                println!("\n✓ Model selection cleared\n");
            }
            2 => {
                state.config.api_key = None;
                if let Err(e) = state.save_config() {
                    eprintln!("Failed to save config: {}", e);
                    return Ok(1);
                }
                println!("\n✓ API Key removed\n");
            }
            3 => {
                state.config.base_url = None;
                if let Err(e) = state.save_config() {
                    eprintln!("Failed to save config: {}", e);
                    return Ok(1);
                }
                println!("\n✓ Base URL cleared (will use provider default)\n");
            }
            4 => {
                // Full reset
                state.config.llm_provider = "ollama".to_string();
                state.config.model = None;
                state.config.api_key = None;
                state.config.base_url = None;
                if let Err(e) = state.save_config() {
                    eprintln!("Failed to save config: {}", e);
                    return Ok(1);
                }
                println!("\n✓ Full factory reset complete!\n");
                println!("Run 'code-buddy setup' to reconfigure.\n");
            }
            _ => {
                println!("\nCancelled.\n");
            }
        }
    }

    Ok(0)
}
