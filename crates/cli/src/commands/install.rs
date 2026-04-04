//! `install` subcommand — post-install setup and verification.

use crate::args::InstallArgs;
use code_buddy_config::AppConfig;

pub async fn run(_config: &AppConfig, args: InstallArgs) -> i32 {
    if args.verify_only {
        println!("Verifying Code Buddy installation...");
    } else {
        println!("Running Code Buddy post-install setup...");
    }

    // Check binary location
    let binary = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "(unknown)".to_string());
    println!("  Binary:      {binary}");

    // Check config path
    if let Some(config_dir) = dirs::config_dir() {
        let config_path = config_dir.join("code-buddy").join("config.toml");
        if config_path.exists() {
            println!("  Config:      {} (found)", config_path.display());
        } else {
            println!("  Config:      {} (not yet created — run 'config set' to configure)", config_path.display());
        }
    }

    // Verify config loads
    match AppConfig::load() {
        Ok(config) => {
            println!("  Config load: OK");
            println!("  Provider:    {}", config.provider);
            println!("  Endpoint:    {}", config.resolved_endpoint());
        }
        Err(e) => {
            eprintln!("  Config load: FAILED — {e}");
            return 1;
        }
    }

    println!();
    if args.verify_only {
        println!("Installation verification complete.");
    } else {
        println!("Setup complete. Run `code-buddy --help` to get started.");
        println!();
        println!("Quick start:");
        println!("  code-buddy config set provider lm-studio");
        println!("  code-buddy config set model mistral-7b-instruct");
        println!("  code-buddy run");
    }

    0
}
