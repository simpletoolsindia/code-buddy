//! Doctor command - Health checks

use crate::mlx::MlxConfig;
use crate::state::AppState;
use anyhow::Result;
use std::process::Command;

pub async fn run(state: &mut AppState) -> Result<i32> {
    println!("=== Code Buddy Doctor ===\n");

    let mut has_errors = false;

    // Check Node.js
    print!("Node.js: ");
    match Command::new("node").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("✓ {}", version);
        }
        _ => {
            println!("✗ Not found");
            has_errors = true;
        }
    }

    // Check npm
    print!("npm: ");
    match Command::new("npm").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("✓ {}", version);
        }
        _ => {
            println!("✗ Not found");
            has_errors = true;
        }
    }

    // Check git
    print!("Git: ");
    match Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("✓ {}", version);
        }
        _ => {
            println!("✗ Not found");
            has_errors = true;
        }
    }

    // Check Python (needed for mlx-lm)
    print!("Python: ");
    match Command::new("python3").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("✓ {}", version);
        }
        _ => {
            println!("✗ Not found");
            has_errors = true;
        }
    }

    // Check MLX (Apple Silicon)
    print!("Apple Silicon (MLX): ");
    if MlxConfig::is_apple_silicon() {
        println!("✓ Detected");
        let mlx_config = MlxConfig::new();
        print!("  mlx-lm: ");
        if mlx_config.check_mlx_lm_installed() {
            println!("✓ Installed");
        } else {
            println!("✗ Not installed");
            println!("  Install with: pip install mlx-lm");
        }
    } else {
        println!("✗ Not an Apple Silicon Mac");
    }

    // Check Ollama
    print!("Ollama: ");
    match Command::new("ollama").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("✓ {}", version);
        }
        _ => {
            println!("✗ Not installed (optional)");
        }
    }

    // Check API key
    print!("API Key: ");
    if state.config.api_key.is_some() {
        println!("✓ Configured");
    } else {
        println!("✗ Not configured");
        println!("  Run: code-buddy login <api-key>");
        // Only error if using cloud provider
        if !["ollama", "mlx"].contains(&state.config.llm_provider.as_str()) {
            has_errors = true;
        }
    }

    // Check LLM provider
    print!("LLM Provider: ");
    println!("{}", state.config.llm_provider);
    if state.config.llm_provider == "mlx" && !MlxConfig::is_apple_silicon() {
        println!("  ⚠️  MLX only works on Apple Silicon!");
    }

    // Check model
    print!("Model: ");
    println!("{}", state.config.model.as_deref().unwrap_or("default"));

    // Check config file
    print!("Config file: ");
    if let Some(config_path) = &state.config.config_path {
        println!("✓ {}", config_path.display());
    } else {
        println!("✗ Not found");
    }

    // Check network
    print!("Network: ");
    match reqwest::get("https://api.anthropic.com").await {
        Ok(resp) if resp.status().is_success() => {
            println!("✓ Connected");
        }
        Ok(resp) => {
            println!("✗ Status {}", resp.status());
            has_errors = true;
        }
        Err(e) => {
            println!("✗ {}", e);
            has_errors = true;
        }
    }

    println!();
    if has_errors {
        println!("Some checks failed. Please review the issues above.");
        Ok(1)
    } else {
        println!("All checks passed!");
        Ok(0)
    }
}
