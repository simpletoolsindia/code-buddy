//! Claude Code - Rust Implementation
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

/// Run the appropriate command based on CLI arguments
async fn run_command(cli: Cli, state: &mut AppState) -> Result<i32> {
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

    // Handle print mode (-p flag with prompt)
    if cli.print {
        let prompt = cli.prompt.unwrap_or_default().join(" ");
        if prompt.is_empty() {
            eprintln!("Error: -p/--print requires a prompt argument");
            return Ok(1);
        }
        return commands::print::run(vec![prompt], cli.output_format, state).await;
    }

    // Execute subcommand if provided, otherwise enter interactive mode
    match cli.command {
        Some(CommandEnum::Mcp(subcommand)) => {
            commands::mcp::run(Some(subcommand), state).await
        }

        Some(CommandEnum::Agents { list }) => {
            commands::agents::run(list, state).await
        }

        Some(CommandEnum::Auth(subcommand)) => {
            commands::auth::run(Some(subcommand), state).await
        }

        Some(CommandEnum::Doctor) => {
            commands::doctor::run(state).await
        }

        Some(CommandEnum::Install { target }) => {
            commands::install::run(target, state).await
        }

        Some(CommandEnum::Update) => {
            commands::update::run(state).await
        }

        Some(CommandEnum::Config(subcommand)) => {
            commands::config::run(Some(subcommand), state).await
        }

        Some(CommandEnum::Model { model }) => {
            commands::model::run(model, state).await
        }

        Some(CommandEnum::Login { api_key }) => {
            commands::auth::login(api_key, state).await
        }

        Some(CommandEnum::Logout) => {
            commands::auth::logout(state).await
        }

        Some(CommandEnum::Status) => {
            commands::status::run(state).await
        }

        Some(CommandEnum::Version) => {
            commands::version::run()
        }

        Some(CommandEnum::Help) => {
            cli::Cli::print_help();
            Ok(0)
        }

        None => {
            // Interactive mode - this would normally start the REPL
            println!("Code Buddy interactive mode (not yet implemented)");
            println!("Use -p/--print <prompt> for non-interactive mode");
            Ok(0)
        }
    }
}
