//! `config` subcommand — read and write configuration.

use crate::args::{ConfigAction, ConfigArgs};
use code_buddy_config::AppConfig;
use dirs::config_dir;

pub async fn run(mut config: AppConfig, args: ConfigArgs) -> i32 {
    match args.action {
        ConfigAction::Show => {
            show_config(&config);
            0
        }
        ConfigAction::Get { field } => {
            match config.get_field(&field) {
                Some(val) => {
                    println!("{val}");
                    0
                }
                None => {
                    eprintln!("Unknown config field '{field}'.");
                    1
                }
            }
        }
        ConfigAction::Set { field, value } => {
            match config.set_field(&field, &value) {
                Ok(()) => {
                    if let Err(e) = config.validate() {
                        eprintln!("Invalid value for '{field}': {e}");
                        return 1;
                    }
                    match config.save() {
                        Ok(_) => {
                            println!("Set {field} = {value}");
                            0
                        }
                        Err(e) => {
                            eprintln!("Failed to save config: {e}");
                            1
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Invalid value: {e}");
                    1
                }
            }
        }
        ConfigAction::Path => {
            match config_dir() {
                Some(dir) => {
                    println!("{}", dir.join("code-buddy").join("config.toml").display());
                    0
                }
                None => {
                    eprintln!("Cannot determine config directory.");
                    1
                }
            }
        }
    }
}

fn show_config(config: &AppConfig) {
    println!("Current configuration:");
    println!("  provider:        {}", config.provider);
    println!(
        "  model:           {}",
        config.model.as_deref().unwrap_or("(not set)")
    );
    println!(
        "  endpoint:        {}",
        config
            .endpoint
            .as_deref()
            .unwrap_or(&format!("(default: {})", config.default_endpoint()))
    );
    println!(
        "  api_key:         {}",
        if config.api_key.is_some() {
            "<set>"
        } else {
            "(not set)"
        }
    );
    println!("  timeout_seconds: {}", config.timeout_seconds);
    println!("  max_retries:     {}", config.max_retries);
    println!("  streaming:       {}", config.streaming);
    println!("  debug:           {}", config.debug);
    println!("  max_tokens:      {}", config.max_tokens.map_or("(default)".to_string(), |v| v.to_string()));
    println!(
        "  temperature:     {}",
        config.temperature.map_or("(default)".to_string(), |v| format!("{v:.2}"))
    );
    println!(
        "  system_prompt:   {}",
        config.system_prompt.as_deref().unwrap_or("(not set)")
    );

    println!();
    println!("To change a value: code-buddy config set <field> <value>");
    println!("Config file path:  code-buddy config path");
}
