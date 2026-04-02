//! Code Buddy - Rust Implementation
//!
//! A high-performance CLI tool for AI-assisted coding.

mod cli;
mod commands;
mod api;
mod config;
mod state;
mod tools;
mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use std::process;
use tracing::{error, info, Level};
use tracing_subscriber::{fmt, EnvFilter};

use crate::cli::Cli;
use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(Level::INFO.into())
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Handle --self-update flag (update the binary itself)
    if cli.self_update {
        match commands::update::perform_update().await {
            Ok(code) => process::exit(code),
            Err(e) => {
                eprintln!("Update failed: {}", e);
                process::exit(1);
            }
        }
    }

    // Load configuration
    let config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            eprintln!("Error: Failed to load configuration");
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // Initialize application state
    let mut state = AppState::new(config);

    // Check for updates on startup (only in interactive mode)
    let check_update = matches!(cli.command, None | Some(cli::CommandEnum::Interactive));
    if check_update && !cli.print {
        // Spawn background update check
        let handle = tokio::spawn(async {
            match commands::update::check_update_silent().await {
                Ok(Some(latest)) => {
                    println!();
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("  ⚠️  Update available: {} → {}", env!("CARGO_PKG_VERSION"), latest);
                    println!("  Run 'code-buddy --self-update' to update");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!();
                }
                _ => {}
            }
        });

        // Execute command
        let result = run_command(cli, &mut state).await;

        // Wait for update check to complete
        let _ = handle.await;
    } else {
        // Execute command
        let result = run_command(cli, &mut state).await;

        match result {
            Ok(exit_code) => process::exit(exit_code),
            Err(e) => {
                error!("Command failed: {}", e);
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    }
}

/// Run the appropriate command based on CLI arguments
async fn run_command(cli: Cli, mut state: &mut AppState) -> Result<i32> {
    use cli::CommandEnum;

    info!("Running command: {:?}", cli.command);

    // Handle global options
    if cli.debug {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive(Level::DEBUG.into())
            )
            .init();
    }

    // Apply permission mode from CLI flags
    if cli.allow_dangerously_skip_permissions {
        state.config.permission_mode = Some("bypass".to_string());
        println!("\n⚠️  Bypass permissions enabled - dangerous!\n");
    } else if let Some(mode) = &cli.permission_mode {
        state.config.permission_mode = Some(format!("{:?}", mode).to_lowercase());
    }

    // Handle print mode (-p flag with prompt)
    if cli.print {
        let prompt = cli.prompt.unwrap_or_default().join(" ");
        if prompt.is_empty() {
            eprintln!("Error: -p/--print requires a prompt argument");
            return Ok(1);
        }
        return commands::print::run(vec![prompt], cli.output_format, &mut state).await;
    }

    // Execute subcommand if provided, otherwise enter interactive mode
    match cli.command {
        Some(CommandEnum::Mcp(subcommand)) => {
            commands::mcp::run(Some(subcommand), &mut state).await
        }

        Some(CommandEnum::Agents { list }) => {
            commands::agents::run(list, &mut state).await
        }

        Some(CommandEnum::Auth(subcommand)) => {
            commands::auth::run(Some(subcommand), &mut state).await
        }

        Some(CommandEnum::Setup) => {
            commands::setup_run(&mut state).await
        }

        Some(CommandEnum::Reset { all }) => {
            commands::reset_run(&mut state, all).await
        }

        Some(CommandEnum::Interactive) => {
            commands::repl_run(&mut state).await
        }

        Some(CommandEnum::Doctor) => {
            commands::doctor::run(&mut state).await
        }

        Some(CommandEnum::Install { target }) => {
            commands::install::run(target, &mut state).await
        }

        Some(CommandEnum::Update { yes }) => {
            if yes {
                commands::update::perform_update().await
            } else {
                commands::update::run(&mut state).await
            }
        }

        Some(CommandEnum::Config(subcommand)) => {
            commands::config::run(Some(subcommand), &mut state).await
        }

        Some(CommandEnum::Model { model }) => {
            commands::model::run(model, &mut state).await
        }

        Some(CommandEnum::Login { api_key }) => {
            commands::auth::login(api_key, &mut state).await
        }

        Some(CommandEnum::Logout) => {
            commands::auth::logout(&mut state).await
        }

        Some(CommandEnum::Status) => {
            commands::status::run(&mut state).await
        }

        Some(CommandEnum::Version) => {
            commands::version::run()
        }

        None => {
            // Interactive REPL mode
            commands::repl_run(&mut state).await
        }
    }
}
