//! `run` subcommand — interactive REPL with tool-calling support.
//!
//! Slash commands: /help /quit /exit /clear /model /provider /status /context

use std::io::{self, Write};

use crate::args::RunArgs;
use code_buddy_config::AppConfig;
use code_buddy_providers::ProviderRegistry;
use code_buddy_runtime::{ConversationRuntime, RuntimeConfig, TextSink};
use code_buddy_tools::ToolRegistry;
use tracing::debug;

const BANNER: &str = r#"
  ╔══════════════════════════════════════════════════════════╗
  ║            Code Buddy — Local-First AI Assistant         ║
  ╠══════════════════════════════════════════════════════════╣
  ║  /help for commands  •  /quit to exit                    ║
  ╚══════════════════════════════════════════════════════════╝
"#;

const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/help",     "Show available commands"),
    ("/quit",     "Exit Code Buddy"),
    ("/exit",     "Exit Code Buddy"),
    ("/clear",    "Clear conversation history"),
    ("/status",   "Show current configuration"),
    ("/model",    "Show current model"),
    ("/provider", "Show current provider"),
    ("/context",  "Show context (message count)"),
];

enum SlashResult {
    Exit,
    Continue,
    Error(String),
}

pub async fn run(config: &AppConfig, args: RunArgs) -> i32 {
    if !config.no_color {
        print!("{BANNER}");
    }

    println!(
        "Provider: {}  |  Endpoint: {}",
        config.provider,
        config.resolved_endpoint()
    );
    if let Some(ref model) = config.model {
        println!("Model: {model}");
    }
    if args.no_tools {
        println!("Tool calling: disabled");
    }
    println!();

    let provider = match ProviderRegistry::from_config(config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error initializing provider: {e}");
            return 1;
        }
    };

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut tools = ToolRegistry::new();
    if !args.no_tools {
        tools.register_builtin(cwd);
        debug!(
            "tools registered: {}",
            tools
                .definitions()
                .iter()
                .map(|d| d.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let rt_config = RuntimeConfig {
        model: config.model.clone().unwrap_or_else(|| "local-model".to_string()),
        max_tokens: config.max_tokens.unwrap_or(4096),
        temperature: config.temperature,
        system_prompt: config.system_prompt.clone(),
        streaming: config.streaming,
        debug: config.debug,
        ..Default::default()
    };

    let mut runtime = ConversationRuntime::new(provider, tools, rt_config);

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

        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }

        if input.starts_with('/') {
            match handle_slash_command(&input, config, &mut runtime) {
                SlashResult::Exit => break,
                SlashResult::Continue => continue,
                SlashResult::Error(msg) => eprintln!("{msg}"),
            }
            continue;
        }

        let sink = TextSink::new(Box::new(|text: &str| {
            print!("{text}");
            io::stdout().flush().unwrap_or(());
        }));

        match runtime.run_turn(&input, sink).await {
            Ok(summary) => {
                if !summary.response_text.ends_with('\n') {
                    println!();
                }
                if config.debug {
                    eprintln!(
                        "[tokens: in={} out={} | tool_calls={} iterations={}]",
                        summary.input_tokens,
                        summary.output_tokens,
                        summary.tool_calls_made,
                        summary.iterations,
                    );
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
            }
        }

        println!();
    }

    0
}

fn handle_slash_command(
    input: &str,
    config: &AppConfig,
    runtime: &mut ConversationRuntime,
) -> SlashResult {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();

    match cmd.as_str() {
        "/quit" | "/exit" => {
            println!("Goodbye!");
            SlashResult::Exit
        }
        "/help" => {
            println!("\nAvailable commands:");
            for (c, desc) in SLASH_COMMANDS {
                println!("  {c:<12} — {desc}");
            }
            println!();
            SlashResult::Continue
        }
        "/clear" => {
            runtime.clear_history();
            print!("\x1b[2J\x1b[H");
            println!("Conversation history cleared.");
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
            println!("  History:   {} messages", runtime.history().len());
            println!();
            SlashResult::Continue
        }
        "/model" => {
            println!(
                "\nModel: {}",
                config.model.as_deref().unwrap_or("(not set)")
            );
            SlashResult::Continue
        }
        "/provider" => {
            println!(
                "\nProvider: {} ({})",
                config.provider,
                config.resolved_endpoint()
            );
            SlashResult::Continue
        }
        "/context" => {
            println!("\nContext: {} messages in history", runtime.history().len());
            println!();
            SlashResult::Continue
        }
        _ => SlashResult::Error(format!(
            "Unknown command '{cmd}'. Type /help for commands."
        )),
    }
}
