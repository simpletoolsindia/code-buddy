//! Model selection command

use crate::state::AppState;
use anyhow::Result;

pub async fn run(model: Option<String>, state: &mut AppState) -> Result<i32> {
    if let Some(model) = model {
        println!("Setting default model to: {}", model);
        state.config.model = Some(model.clone());
        // Save to config file
        if let Err(e) = state.save_config() {
            eprintln!("Warning: Failed to save config: {}", e);
        }
        println!("Model updated successfully");
    } else {
        // Show current model
        let current = state.config.model.as_deref().unwrap_or("claude-sonnet-4-5");
        println!("Current model: {}", current);
        println!("\nAvailable models:");
        println!("  claude-opus-4-6     - Most capable, slower");
        println!("  claude-sonnet-4-5   - Balanced (default)");
        println!("  claude-haiku-4-5-20251001 - Fastest, less capable");
        println!("\nUsage: code-buddy model <model-name>");
    }
    Ok(0)
}
