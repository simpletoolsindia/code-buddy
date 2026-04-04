//! Code Buddy — AI coding assistant for local and open-source LLMs.
//!
//! Entry point. Parses CLI arguments, initializes telemetry and configuration,
//! then dispatches to the appropriate subcommand handler.

use clap::Parser;
use std::process;

mod args;
mod commands;

use args::{Cli, OutputFormat, Subcommand};
use code_buddy_config::AppConfig;
use code_buddy_telemetry::{LogFormat, TelemetryConfig};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Determine debug and color settings early (before config load,
    // so we can log config load errors correctly).
    let debug = cli.debug;
    let no_color = cli.no_color || std::env::var("NO_COLOR").is_ok();

    // Initialize telemetry first so all subsequent code can log.
    let log_format = match cli.output {
        Some(OutputFormat::Json) => LogFormat::Json,
        _ => LogFormat::Pretty,
    };
    let telemetry_cfg = TelemetryConfig {
        debug,
        format: log_format,
        filter_override: None,
    };
    if let Err(e) = code_buddy_telemetry::init(&telemetry_cfg) {
        eprintln!("Warning: could not initialize logging: {e}");
    }

    // Load configuration.
    let mut config = match AppConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading configuration: {e}");
            eprintln!("Run `code-buddy config show` to see the config location.");
            process::exit(1);
        }
    };

    // Apply CLI flags that override config.
    if debug {
        config.debug = true;
    }
    if no_color {
        config.no_color = true;
    }
    if let Some(ref provider) = cli.provider {
        if let Err(e) = config.set_field("provider", provider) {
            eprintln!("Invalid --provider value: {e}");
            process::exit(1);
        }
    }
    if let Some(ref model) = cli.model {
        config.model = Some(model.clone());
    }

    // Dispatch subcommand.
    let exit_code = match cli.subcommand {
        Some(Subcommand::Ask(args)) => {
            commands::ask::run(&config, args).await
        }
        Some(Subcommand::Run(args)) => {
            commands::run::run(&config, args).await
        }
        Some(Subcommand::Config(args)) => {
            commands::config_cmd::run(config, args).await
        }
        Some(Subcommand::Install(args)) => {
            commands::install::run(&config, args).await
        }
        None => {
            // No subcommand: default to interactive REPL.
            commands::run::run(&config, Default::default()).await
        }
    };

    process::exit(exit_code);
}
