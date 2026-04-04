//! `run` subcommand — start an interactive REPL session.

use crate::args::RunArgs;
use code_buddy_config::AppConfig;
use std::io::{self, Write};
use tracing::debug;

const BANNER: &str = r#"
  ╔══════════════════════════════════════════════════════════╗
  ║            Code Buddy — Local-First AI Assistant         ║
  ╠══════════════════════════════════════════════════════════╣
  ║  /help for commands  •  /quit to exit                    ║
  ╚══════════════════════════════════════════════════════════╝
"#;

const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/help", "Show available commands"),
    ("/quit", "Exit Code Buddy"),
    ("/exit", "Exit Code Buddy"),
    ("/clear", "Clear conversation history"),
    ("/status", "Show current configuration"),
    ("/model", "Show or change model"),
    ("/provider", "Show or change provider"),
    ("/context", "Show context usage"),
];

pub async fn run(config: &AppConfig, _args: RunArgs) -> i32 {
    if !config.no_color {
        print!("{BANNER}");
    }

    println!("Provider: {}  |  Endpoint: {}", config.provider, config.resolved_endpoint());
    if let Some(ref model) = config.model {
        println!("Model: {model}");
    }
    println!("(Phase 2 will add live LLM connections)\n");

    loop {
        print!("❯ ");
        io::stdout().flush().unwrap_or(());

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                println!("\nGoodbye!");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error reading input: {e}");
                return 1;
            }
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input.starts_with('/') {
            match handle_slash_command(input, config) {
                SlashResult::Exit => break,
                SlashResult::Continue => continue,
                SlashResult::Error(msg) => eprintln!("{msg}"),
            }
        } else {
            debug!("user prompt: {input}");
            println!("[Provider integration coming in Phase 2]");
            println!("You said: {input}\n");
        }
    }

    0
}

enum SlashResult {
    Exit,
    Continue,
    Error(String),
}

fn handle_slash_command(input: &str, config: &AppConfig) -> SlashResult {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();

    match cmd.as_str() {
        "/quit" | "/exit" => {
            println!("Goodbye!");
            SlashResult::Exit
        }
        "/help" => {
            println!("\nAvailable commands:");
            for (cmd, desc) in SLASH_COMMANDS {
                println!("  {cmd:<12} — {desc}");
            }
            println!();
            SlashResult::Continue
        }
        "/status" => {
            println!("\nConfiguration:");
            println!("  Provider:  {}", config.provider);
            println!("  Endpoint:  {}", config.resolved_endpoint());
            println!(
                "  Model:     {}",
                config.model.as_deref().unwrap_or("(not set)")
            );
            println!("  Streaming: {}", config.streaming);
            println!("  Debug:     {}", config.debug);
            println!();
            SlashResult::Continue
        }
        "/clear" => {
            print!("\x1b[2J\x1b[H");
            SlashResult::Continue
        }
        _ => {
            SlashResult::Error(format!("Unknown command '{cmd}'. Type /help for commands."))
        }
    }
}
