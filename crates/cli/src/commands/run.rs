//! `run` subcommand — start an interactive REPL session.

use crate::args::RunArgs;
use code_buddy_config::AppConfig;
use code_buddy_providers::ProviderRegistry;
use code_buddy_transport::{InputMessage, MessageRequest, StreamEvent};
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

    println!(
        "Provider: {}  |  Endpoint: {}",
        config.provider,
        config.resolved_endpoint()
    );
    if let Some(ref model) = config.model {
        println!("Model: {model}");
    }
    println!();

    let provider = match ProviderRegistry::from_config(config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error initializing provider: {e}");
            return 1;
        }
    };

    let model = config
        .model
        .clone()
        .unwrap_or_else(|| "local-model".to_string());

    let mut history: Vec<InputMessage> = Vec::new();

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
            match handle_slash_command(input, config, &mut history) {
                SlashResult::Exit => break,
                SlashResult::Continue => continue,
                SlashResult::Error(msg) => eprintln!("{msg}"),
            }
            continue;
        }

        history.push(InputMessage::user_text(input));

        let mut request = MessageRequest {
            model: model.clone(),
            max_tokens: config.max_tokens.unwrap_or(4096),
            messages: history.clone(),
            system: config.system_prompt.clone(),
            tools: None,
            tool_choice: None,
            stream: config.streaming,
            temperature: config.temperature,
        };

        if config.streaming {
            request.stream = true;
            let mut response_text = String::new();

            match provider.stream(&request).await {
                Err(e) => {
                    history.pop(); // remove the failed user message
                    eprintln!("Stream error: {e}");
                    continue;
                }
                Ok(mut source) => {
                    let mut stream_error = false;
                    loop {
                        match source.next_event().await {
                            Ok(None) => break,
                            Ok(Some(StreamEvent::TextDelta(text))) => {
                                print!("{text}");
                                io::stdout().flush().unwrap_or(());
                                response_text.push_str(&text);
                            }
                            Ok(Some(StreamEvent::MessageStop)) => {
                                println!();
                                break;
                            }
                            Ok(Some(StreamEvent::Usage(u))) => {
                                debug!(
                                    "usage: in={} out={}",
                                    u.input_tokens, u.output_tokens
                                );
                            }
                            Ok(Some(StreamEvent::ToolUseDelta { name, input_json, .. })) => {
                                debug!("tool call: {name}({input_json})");
                            }
                            Err(e) => {
                                eprintln!("\nStream error: {e}");
                                stream_error = true;
                                break;
                            }
                        }
                    }

                    if stream_error {
                        // Remove the user message — the turn did not complete cleanly.
                        history.pop();
                    } else if !response_text.is_empty() {
                        history.push(InputMessage::assistant_text(response_text));
                    }
                }
            }
        } else {
            match provider.send(&request).await {
                Err(e) => {
                    history.pop(); // remove the failed user message
                    eprintln!("Error: {e}");
                    continue;
                }
                Ok(response) => {
                    let text = response.text_content();
                    println!("{text}");
                    if config.debug {
                        eprintln!(
                            "[tokens: in={} out={}]",
                            response.usage.input_tokens, response.usage.output_tokens
                        );
                    }
                    if !text.is_empty() {
                        history.push(InputMessage::assistant_text(text));
                    }
                }
            }
        }

        println!();
    }

    0
}

enum SlashResult {
    Exit,
    Continue,
    Error(String),
}

fn handle_slash_command(
    input: &str,
    config: &AppConfig,
    history: &mut Vec<InputMessage>,
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
            history.clear();
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
            println!("  History:   {} messages", history.len());
            println!();
            SlashResult::Continue
        }
        "/context" => {
            println!("\nContext: {} messages in history", history.len());
            println!();
            SlashResult::Continue
        }
        _ => SlashResult::Error(format!("Unknown command '{cmd}'. Type /help for commands.")),
    }
}
