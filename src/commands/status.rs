//! Status command - Show system and authentication status

use crate::mlx::MlxConfig;
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
    println!("Model: {}", state.config.model.as_deref().unwrap_or("default"));

    // Show MLX-specific info
    if state.config.llm_provider == "mlx" {
        let mlx_config = MlxConfig::new();
        println!("\nMLX Info:");
        println!("  mlx-lm installed: {}", if mlx_config.check_mlx_lm_installed() { "Yes" } else { "No" });
        println!("  Model cache: {}", mlx_config.model_dir.display());
        if !mlx_config.cached_models.is_empty() {
            println!("  Cached models:");
            for model in &mlx_config.cached_models {
                println!("    - {}", model);
            }
        }
    }

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

    // Show MLX availability
    if MlxConfig::is_apple_silicon() {
        println!("\nMLX (Apple Silicon): Available");
        println!("  Run 'code-buddy --mlx' to set up MLX models");
    }

    Ok(0)
}
