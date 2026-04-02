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
mod plugins;
pub mod mlx;
pub mod hooks;
pub mod vision;
pub mod computer;
pub mod agents;
pub mod skills;

use anyhow::Result;
use clap::Parser;
use std::process;
use tracing::{error, info, Level};
use tracing_subscriber::EnvFilter;

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

    // Handle MLX-specific flags
    if cli.mlx || cli.mlx_model.is_some() || cli.mlx_download.is_some() || cli.mlx_list_models {
        let result = run_mlx_command(&cli).await;
        match result {
            Ok(_) => process::exit(0),
            Err(e) => {
                eprintln!("MLX Error: {}", e);
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
            if let Ok(Some(latest)) = commands::update::check_update_silent().await {
                println!();
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("  ⚠️  Update available: {} → {}", env!("CARGO_PKG_VERSION"), latest);
                println!("  Run 'code-buddy --self-update' to update");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!();
            }
        });

        // Execute command
        let _result = run_command(cli, &mut state).await;

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

        Some(CommandEnum::Setup) => {
            commands::setup_run(state).await
        }

        Some(CommandEnum::Reset { all }) => {
            commands::reset_run(state, all).await
        }

        Some(CommandEnum::Interactive) => {
            commands::repl_run(state).await
        }

        Some(CommandEnum::Doctor) => {
            commands::doctor::run(state).await
        }

        Some(CommandEnum::Install { target }) => {
            commands::install::run(target, state).await
        }

        Some(CommandEnum::Update { yes }) => {
            if yes {
                commands::update::perform_update().await
            } else {
                commands::update::run(state).await
            }
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

        Some(CommandEnum::Plugin(cli_plugin_cmd)) => {
            use cli::plugin::PluginCommand as CliPluginCmd;
            match cli_plugin_cmd {
                CliPluginCmd::List { json, all } => {
                    commands::plugin::run(
                        Some(commands::plugin::PluginSubcommand::List { json, all }),
                        state,
                    ).await
                }
                CliPluginCmd::Add { source, scope } => {
                    let scope = scope.map(|s| match s {
                        cli::plugin::PluginScopeArg::User => "user".to_string(),
                        cli::plugin::PluginScopeArg::Project => "project".to_string(),
                    });
                    commands::plugin::run(
                        Some(commands::plugin::PluginSubcommand::Add { source, scope }),
                        state,
                    ).await
                }
                CliPluginCmd::Remove { name } => {
                    commands::plugin::run(
                        Some(commands::plugin::PluginSubcommand::Remove { name }),
                        state,
                    ).await
                }
                CliPluginCmd::Enable { name } => {
                    commands::plugin::run(
                        Some(commands::plugin::PluginSubcommand::Enable { name }),
                        state,
                    ).await
                }
                CliPluginCmd::Disable { name } => {
                    commands::plugin::run(
                        Some(commands::plugin::PluginSubcommand::Disable { name }),
                        state,
                    ).await
                }
                CliPluginCmd::Update { name } => {
                    commands::plugin::run(
                        Some(commands::plugin::PluginSubcommand::Update { name }),
                        state,
                    ).await
                }
                CliPluginCmd::Search { query } => {
                    commands::plugin::run(
                        Some(commands::plugin::PluginSubcommand::Search { query }),
                        state,
                    ).await
                }
                CliPluginCmd::Skills => {
                    commands::plugin::run(
                        Some(commands::plugin::PluginSubcommand::Skills),
                        state,
                    ).await
                }
                CliPluginCmd::Marketplace { subcmd } => {
                    let msc = subcmd.map(|s| match s {
                        cli::plugin::MarketplaceSubcommand::List =>
                            commands::plugin::MarketplaceAction::List,
                        cli::plugin::MarketplaceSubcommand::Add { source } =>
                            commands::plugin::MarketplaceAction::Add { source },
                        cli::plugin::MarketplaceSubcommand::Remove { name } =>
                            commands::plugin::MarketplaceAction::Remove { name },
                        cli::plugin::MarketplaceSubcommand::Update { name } =>
                            commands::plugin::MarketplaceAction::Update { name },
                    });
                    commands::plugin::run(
                        Some(commands::plugin::PluginSubcommand::Marketplace { subcmd: msc }),
                        state,
                    ).await
                }
                CliPluginCmd::Validate { path } => {
                    commands::plugin::run(
                        Some(commands::plugin::PluginSubcommand::Validate { path }),
                        state,
                    ).await
                }
                CliPluginCmd::Reload => {
                    commands::plugin::run(
                        Some(commands::plugin::PluginSubcommand::Reload),
                        state,
                    ).await
                }
            }
        }

        Some(CommandEnum::Memory { action, args }) => {
            let mut all_args = Vec::new();
            if let Some(a) = action {
                all_args.push(a);
            }
            all_args.extend(args);
            match commands::memory_run(&all_args) {
                Ok(_) => Ok(0),
                Err(e) => {
                    eprintln!("Memory error: {}", e);
                    Ok(1)
                }
            }
        }

        None => {
            // Interactive REPL mode
            commands::repl_run(state).await
        }
    }
}

/// Handle MLX-specific commands
async fn run_mlx_command(cli: &Cli) -> Result<()> {
    use crate::mlx::{self, MlxConfig, MLX_COMMUNITY_MODELS};

    let mut config = MlxConfig::new();
    config.detect().await?;

    // List models
    if cli.mlx_list_models {
        println!("=== Popular MLX Models (mlx-community) ===");
        println!();
        for (i, (id, name)) in MLX_COMMUNITY_MODELS.iter().enumerate() {
            println!("{}. {}", i + 1, name);
            println!("   {}", id);
            println!();
        }
        return Ok(());
    }

    // Download model
    if let Some(model_id) = &cli.mlx_download {
        config.download_model(model_id).await?;
        return Ok(());
    }

    // MLX status/setup
    if cli.mlx {
        if !config.available {
            println!("MLX is only available on Apple Silicon Macs (M1/M2/M3/M4)");
            println!();
            println!("For cloud-based inference, use:");
            println!("  code-buddy --provider openai -p 'your prompt'");
            println!("  code-buddy --provider anthropic -p 'your prompt'");
            return Ok(());
        }

        // Interactive setup
        if let Some(model) = mlx::interactive_model_setup(&config).await? {
            println!();
            println!("✓ Model '{}' is ready!", model);
            println!();
            println!("To use it, run:");
            println!("  code-buddy --provider mlx --model {} -p 'your prompt'", model);
        }
        return Ok(());
    }

    // Set MLX model
    if let Some(model) = &cli.mlx_model {
        if !config.available {
            anyhow::bail!("MLX is only available on Apple Silicon Macs");
        }

        println!("Setting MLX model to: {}", model);
        println!();

        // Download if not cached
        if !config.cached_models.iter().any(|m| m.contains(model)) {
            println!("Model not found in cache. Downloading...");
            config.download_model(model).await?;
        }

        // Save to config
        let mut app_config = crate::config::Config::load()?;
        app_config.llm_provider = "mlx".to_string();
        app_config.model = Some(model.clone());
        app_config.save()?;
        println!();
        println!("✓ MLX model '{}' configured!", model);
        println!("  Run 'code-buddy -p \"your prompt\"' to use it");
    }

    Ok(())
}
