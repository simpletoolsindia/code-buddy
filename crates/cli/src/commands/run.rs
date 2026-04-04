//! `run` subcommand — Claude Code-style interactive REPL with tool-calling support.
//!
//! # Visual design
//! Inspired by Claude Code's terminal UX:
//! - Branded header with `✻` sigil and box-drawing border
//! - Cyan `❯` prompt
//! - `indicatif` spinner during LLM inference
//! - Colored tool-call and status blocks
//! - `/status` reports web search availability
//!
//! Slash commands: /help /quit /exit /clear /model /provider /status /context /tools

use std::io::{self, Write};
use std::time::Duration;

use crate::args::RunArgs;
use code_buddy_config::AppConfig;
use code_buddy_providers::ProviderRegistry;
use code_buddy_runtime::{ConversationRuntime, RuntimeConfig, TextSink};
use code_buddy_tools::ToolRegistry;
use console::{Style, style};
use indicatif::{ProgressBar, ProgressStyle};
use tracing::debug;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/help",     "Show available slash commands"),
    ("/quit",     "Exit Code Buddy"),
    ("/exit",     "Exit Code Buddy"),
    ("/clear",    "Clear conversation history and screen"),
    ("/status",   "Show configuration and active tools"),
    ("/tools",    "List all registered tools"),
    ("/model",    "Show current model"),
    ("/provider", "Show current provider"),
    ("/context",  "Show conversation context size"),
];

enum SlashResult {
    Exit,
    Continue,
    Error(String),
}

// ── REPL entry point ──────────────────────────────────────────────────────────

pub async fn run(config: &AppConfig, args: RunArgs) -> i32 {
    print_banner(config);

    let provider = match ProviderRegistry::from_config(config) {
        Ok(p) => p,
        Err(e) => {
            if config.no_color {
                eprintln!("Error: {e}");
            } else {
                eprintln!(
                    "\n  {} Provider error: {e}",
                    style("✘").red().bold()
                );
                eprintln!(
                    "  Run {} to reconfigure.\n",
                    style("code-buddy setup").bold()
                );
            }
            return 1;
        }
    };

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut tools = ToolRegistry::new();

    if !args.no_tools {
        tools.register_builtin(cwd);
        tools.register_web_tools(
            config.brave_api_key.clone(),
            config.serpapi_key.clone(),
            config.firecrawl_api_key.clone(),
        );
        debug!("tools registered: {}", tools.tool_names().join(", "));
    } else if !config.no_color {
        println!(
            "  {} Tool calling disabled for this session.",
            style("◦").dim()
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
        // Prompt.
        if config.no_color {
            print!("\n❯ ");
        } else {
            print!("\n{} ", Style::new().cyan().bold().apply_to("❯"));
        }
        io::stdout().flush().unwrap_or(());

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                if config.no_color {
                    println!("\nGoodbye!");
                } else {
                    println!(
                        "\n\n  {} Goodbye!\n",
                        style("✻").magenta().bold()
                    );
                }
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

        // Slash command routing.
        if input.starts_with('/') {
            match handle_slash(&input, config, &mut runtime) {
                SlashResult::Exit => break,
                SlashResult::Continue => continue,
                SlashResult::Error(msg) => {
                    if config.no_color {
                        eprintln!("{msg}");
                    } else {
                        eprintln!("  {} {msg}", style("✘").red());
                    }
                }
            }
            continue;
        }

        // ── Spinner + LLM call ────────────────────────────────────────────────
        let spinner = if !config.no_color {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::with_template("  {spinner:.cyan} {msg}")
                    .expect("valid template")
                    .tick_strings(SPINNER_FRAMES),
            );
            pb.set_message(Style::new().dim().apply_to("Thinking…").to_string());
            pb.enable_steady_tick(Duration::from_millis(80));
            Some(pb)
        } else {
            None
        };

        println!();

        let spinner_clone = spinner.clone();
        let no_color = config.no_color;
        let sink = TextSink::new(Box::new(move |text: &str| {
            if let Some(ref pb) = spinner_clone {
                pb.suspend(|| {
                    print!("{text}");
                    io::stdout().flush().unwrap_or(());
                });
            } else {
                print!("{text}");
                io::stdout().flush().unwrap_or(());
            }
            let _ = no_color;
        }));

        match runtime.run_turn(&input, sink).await {
            Ok(summary) => {
                if let Some(ref pb) = spinner {
                    pb.finish_and_clear();
                }

                if !summary.response_text.is_empty()
                    && !summary.response_text.ends_with('\n')
                {
                    println!();
                }

                if summary.tool_calls_made > 0 && !config.no_color {
                    println!(
                        "\n  {}",
                        Style::new().dim().apply_to(format!(
                            "─── {} tool call{} across {} iteration{} ───",
                            summary.tool_calls_made,
                            if summary.tool_calls_made == 1 { "" } else { "s" },
                            summary.iterations,
                            if summary.iterations == 1 { "" } else { "s" },
                        ))
                    );
                }

                if config.debug {
                    eprintln!(
                        "  {} tokens in={} out={} tool_calls={} iters={}",
                        Style::new().dim().apply_to("◦"),
                        summary.input_tokens,
                        summary.output_tokens,
                        summary.tool_calls_made,
                        summary.iterations,
                    );
                }
            }
            Err(e) => {
                if let Some(ref pb) = spinner {
                    pb.finish_and_clear();
                }
                if config.no_color {
                    eprintln!("Error: {e}");
                } else {
                    eprintln!("\n  {} {e}", style("✘").red().bold());
                    eprintln!(
                        "  {} Check /status or run code-buddy setup.",
                        style("◦").dim()
                    );
                }
            }
        }
    }

    0
}

// ── Slash commands ────────────────────────────────────────────────────────────

fn handle_slash(
    raw: &str,
    config: &AppConfig,
    runtime: &mut ConversationRuntime,
) -> SlashResult {
    let cmd = raw.split_whitespace().next().unwrap_or(raw);
    let dim = Style::new().dim();
    let cyan = Style::new().cyan();
    let bold = Style::new().bold();

    match cmd {
        "/quit" | "/exit" => SlashResult::Exit,

        "/help" => {
            if config.no_color {
                for (c, d) in SLASH_COMMANDS {
                    println!("  {c:<14} {d}");
                }
            } else {
                println!();
                println!("  {}", bold.apply_to("Slash commands"));
                println!();
                for (c, d) in SLASH_COMMANDS {
                    println!(
                        "    {:<14} {}",
                        cyan.apply_to(c),
                        dim.apply_to(d)
                    );
                }
                println!();
            }
            SlashResult::Continue
        }

        "/clear" => {
            runtime.clear_history();
            print!("\x1b[2J\x1b[H");
            if config.no_color {
                println!("History cleared.");
            } else {
                println!("  {} Conversation history cleared.", style("✔").green());
            }
            SlashResult::Continue
        }

        "/status" => {
            if config.no_color {
                println!("Provider:  {}", config.provider);
                println!("Endpoint:  {}", config.resolved_endpoint());
                println!("Model:     {}", config.model.as_deref().unwrap_or("(not set)"));
                println!("Streaming: {}", config.streaming);
                println!("History:   {} messages", runtime.history().len());
            } else {
                println!();
                println!("  {}", bold.apply_to("Configuration"));
                let kv = |k: &str, v: String| {
                    println!("    {:<20} {}", dim.apply_to(k), v);
                };
                kv("Provider", cyan.apply_to(&config.provider).to_string());
                kv("Endpoint", config.resolved_endpoint());
                kv("Model", config.model.as_deref().unwrap_or("(not set)").to_string());
                kv("Streaming", config.streaming.to_string());
                kv("Context", format!("{} messages", runtime.history().len()));

                let mut tool_names: Vec<String> = runtime
                    .tool_definitions()
                    .into_iter()
                    .map(|d| d.name)
                    .collect();
                tool_names.sort();

                println!();
                println!("  {}", bold.apply_to("Tools"));
                if tool_names.is_empty() {
                    println!("    {} none", dim.apply_to("–"));
                } else {
                    for name in &tool_names {
                        println!("    {} {name}", style("●").green());
                    }
                }

                println!();
                println!("  {}", bold.apply_to("Web"));
                let search_status = if config.brave_api_key.is_some() {
                    style("✔ Brave Search").green().to_string()
                } else if config.serpapi_key.is_some() {
                    style("✔ SerpAPI").green().to_string()
                } else {
                    style("✘ no key (brave_api_key / serpapi_key)").red().to_string()
                };
                let fetch_status = if config.firecrawl_api_key.is_some() {
                    style("✔ Firecrawl").green().to_string()
                } else {
                    style("✔ plain HTTP").green().to_string()
                };
                println!("    {:<20} {}", dim.apply_to("web_search"), search_status);
                println!("    {:<20} {}", dim.apply_to("web_fetch"), fetch_status);
                println!();
            }
            SlashResult::Continue
        }

        "/tools" => {
            let mut tool_names: Vec<String> = runtime
                .tool_definitions()
                .into_iter()
                .map(|d| d.name)
                .collect();
            tool_names.sort();

            if tool_names.is_empty() {
                println!("  No tools registered.");
            } else if config.no_color {
                println!("Tools ({}):", tool_names.len());
                for name in &tool_names {
                    println!("  - {name}");
                }
            } else {
                println!();
                println!(
                    "  {} {} tools",
                    bold.apply_to("Active tools"),
                    tool_names.len()
                );
                for name in &tool_names {
                    println!("    {} {}", style("●").cyan(), name);
                }
                println!();
            }
            SlashResult::Continue
        }

        "/model" => {
            let model = config.model.as_deref().unwrap_or("(not set)");
            if config.no_color {
                println!("Model: {model}");
            } else {
                println!("\n  {} Model: {}\n", dim.apply_to("◦"), cyan.apply_to(model));
            }
            SlashResult::Continue
        }

        "/provider" => {
            if config.no_color {
                println!("Provider: {} ({})", config.provider, config.resolved_endpoint());
            } else {
                println!(
                    "\n  {} Provider: {} ({})\n",
                    dim.apply_to("◦"),
                    cyan.apply_to(&config.provider),
                    config.resolved_endpoint()
                );
            }
            SlashResult::Continue
        }

        "/context" => {
            let n = runtime.history().len();
            if config.no_color {
                println!("Context: {n} messages");
            } else {
                println!("\n  {} Context: {} messages\n", dim.apply_to("◦"), n);
            }
            SlashResult::Continue
        }

        _ => SlashResult::Error(format!(
            "Unknown command '{cmd}'. Type /help for commands."
        )),
    }
}

// ── Banner ────────────────────────────────────────────────────────────────────

fn print_banner(config: &AppConfig) {
    if config.no_color {
        println!("Code Buddy — AI coding assistant");
        println!(
            "Provider: {}  Model: {}",
            config.provider,
            config.model.as_deref().unwrap_or("(not set)")
        );
        println!("Type /help for commands, /quit to exit.");
        println!();
        return;
    }

    let dim = Style::new().dim();
    let provider_str = format!(
        "{}  {}  {} {}",
        Style::new().dim().apply_to("Provider"),
        Style::new().cyan().apply_to(&config.provider),
        Style::new().dim().apply_to("│ Model"),
        Style::new().cyan().apply_to(config.model.as_deref().unwrap_or("(not set)"))
    );

    println!();
    println!("  {}", dim.apply_to("╭────────────────────────────────────────────────────────────╮"));
    println!(
        "  {}  {}  {}",
        dim.apply_to("│"),
        style("✻").magenta().bold(),
        style("Welcome to Code Buddy — AI coding assistant            │").bold()
    );
    println!("  {}", dim.apply_to("╰────────────────────────────────────────────────────────────╯"));
    println!();
    println!("  {provider_str}");
    println!(
        "  {} Type {} for commands, {} to exit.",
        dim.apply_to("◦"),
        style("/help").bold(),
        style("/quit").bold()
    );
    println!();
}
