//! Status command - Show system and authentication status

use crate::state::AppState;
use anyhow::Result;

pub async fn run(state: &mut AppState) -> Result<i32> {
    println!("=== Code Buddy Status ===\n");

    // Check authentication
    print!("Authentication: ");
    if state.config.api_key.is_some() {
        println!("✓ Logged in");
    } else {
        println!("✗ Not logged in");
    }

    // Check LLM provider
    println!("\nLLM Provider: {}", state.config.llm_provider);
    println!("Model: {}", state.config.model.as_deref().unwrap_or("claude-sonnet-4-5"));

    // Check additional directories
    if !state.config.additional_dirs.is_empty() {
        println!("\nAdditional directories:");
        for dir in &state.config.additional_dirs {
            println!("  - {}", dir.display());
        }
    }

    // System info
    println!("\n=== System Info ===");
    println!("OS: {}", os_info::get());
    println!("Version: {}", env!("CARGO_PKG_VERSION"));

    Ok(0)
}
