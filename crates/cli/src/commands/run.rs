//! `run` subcommand — simple interactive mode for all users.
//!
//! # UX goals (non-technical users)
//! - Clear banner showing what model is active
//! - "Thinking…" shown while AI is working
//! - Tool call names shown so users know what's happening
//! - Bell rings when AI needs attention
//! - Simple slash commands (type /help to see them)

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
    ("/help",     "Show all commands"),
    ("/quit",     "Exit Code Buddy"),
    ("/exit",     "Exit Code Buddy"),
    ("/clear",    "Start a new conversation"),
    ("/status",   "Check current model and provider"),
    ("/tools",    "See what tools are available"),
];

enum SlashResult {
    Exit,
    Continue,
    Error(String),
}

// ── REPL entry point ──────────────────────────────────────────────────────────

#[allow(clippy::too_many_lines, clippy::if_not_else)]
pub async fn run(config: &AppConfig, args: RunArgs) -> i32 {
    print_banner(config);

    // Check provider works
    let provider = match ProviderRegistry::from_config(config) {
        Ok(p) => p,
        Err(e) => {
            if config.no_color {
                eprintln!("Error: {e}");
            } else {
                eprintln!(
                    "\n  {}  Could not connect to {}",
                    style("✘").red().bold(),
                    style(&config.provider).bold()
                );
                eprintln!("  Error: {e}");
                eprintln!("  Run {} to fix this.\n", style("code-buddy setup").bold());
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
    }

    let rt_config = RuntimeConfig {
        model: config.model.clone().unwrap_or_else(|| "local-model".to_string()),
        max_tokens: config.max_tokens.unwrap_or(4096),
        temperature: config.temperature,
        system_prompt: config.system_prompt.clone(),
        streaming: config.streaming,
        debug: config.debug,
        on_tool_call: if config.no_color {
            None
        } else {
            Some(Box::new(|name: &str| {
                print!("\r\x1b[K");
                println!("  {}  Running {}…", style("▶").yellow(), style(name).bold());
                let _ = io::stdout().flush();
            }))
        },
        ..Default::default()
    };

    let mut runtime = ConversationRuntime::new(provider, tools, rt_config);

    // Show active tools
    let tool_names: Vec<String> = runtime.tool_definitions().into_iter().map(|d| d.name).collect();
    if !tool_names.is_empty() && !config.no_color {
        println!("  {}  Tools: {}", style("●").cyan(), tool_names.join(", "));
        println!();
    } else if !config.no_color {
        println!("  {}  No tools enabled (--no-tools mode).", style("◦").dim());
        println!();
    }

    // ── Main input loop ───────────────────────────────────────────────────────
    loop {
        print_prompt(config);
        io::stdout().flush().unwrap_or(());

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                // Ctrl+D — clean exit
                println!();
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

        // ── Slash commands ──────────────────────────────────────────────────
        if input.starts_with('/') {
            match handle_slash(&input, config, &mut runtime) {
                SlashResult::Exit => break,
                SlashResult::Continue => continue,
                SlashResult::Error(msg) => {
                    if config.no_color {
                        eprintln!("{msg}");
                    } else {
                        eprintln!("  {}  {msg}", style("✘").red());
                    }
                }
            }
            continue;
        }

        // ── Thinking indicator ────────────────────────────────────────────────
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
            println!("  Thinking…");
            None
        };

        println!();

        let spinner_clone = spinner.clone();

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

                // Tool call summary
                if summary.tool_calls_made > 0 && !config.no_color {
                    println!();
                    let tools_msg = if summary.tool_calls_made == 1 {
                        "1 tool used"
                    } else {
                        &format!("{} tools used", summary.tool_calls_made)
                    };
                    let steps_msg = if summary.iterations == 1 { "step" } else { "steps" };
                    println!(
                        "  {}  {} — completed in {} {}",
                        style("✔").green(),
                        tools_msg,
                        summary.iterations,
                        steps_msg,
                    );
                }

                if config.debug {
                    eprintln!(
                        "  {} tokens: {} in / {} out",
                        Style::new().dim().apply_to("◦"),
                        summary.input_tokens,
                        summary.output_tokens,
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
                    eprintln!();
                    eprintln!("  {}  Error: {e}", style("✘").red().bold());
                    match e.to_string().contains("context") || e.to_string().contains("too large") {
                        true => {
                            eprintln!("  {}  Context too long. Type {} to start fresh.", style("ℹ").yellow(), style("/clear").bold());
                        }
                        false => {
                            eprintln!("  {}  Run {} to check your setup.", style("ℹ").yellow(), style("/status").bold());
                        }
                    }
                }
            }
        }

        // Bell to get attention before next prompt
        ring_bell();
    }

    if !config.no_color {
        println!();
        println!("  {}  Goodbye! Run {} to chat again.\n", style("✻").magenta(), style("code-buddy").bold());
    } else {
        println!("Goodbye!\n");
    }
    0
}

// ── Slash commands ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
fn handle_slash(
    raw: &str,
    config: &AppConfig,
    runtime: &mut ConversationRuntime,
) -> SlashResult {
    let cmd = raw.split_whitespace().next().unwrap_or(raw);

    match cmd {
        "/quit" | "/exit" => SlashResult::Exit,

        "/help" => {
            if config.no_color {
                println!("Commands:");
                for (c, d) in SLASH_COMMANDS {
                    println!("  {c:<14} {d}");
                }
                println!("  /model     Show current model");
                println!("  /provider  Show current provider");
            } else {
                println!();
                println!("  {}  Commands you can type:", style("?").cyan());
                println!();
                for (c, d) in SLASH_COMMANDS {
                    println!(
                        "    {:<14}  {}",
                        style(c).cyan().bold(),
                        style(d).dim()
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
                println!("Started a new conversation.");
            } else {
                println!("  {}  New conversation started.", style("✔").green());
            }
            SlashResult::Continue
        }

        "/status" => {
            if config.no_color {
                println!("Provider:  {}", config.provider);
                println!("Endpoint:  {}", config.resolved_endpoint());
                println!("Model:     {}", config.model.as_deref().unwrap_or("(not set)"));
                println!("Tools:     {} enabled", runtime.tool_definitions().len());
            } else {
                println!();
                println!("  {}  Current setup:", style("●").cyan());
                println!();
                println!(
                    "    {:<14}  {}",
                    style("Provider").dim(),
                    style(&config.provider).bold()
                );
                println!(
                    "    {:<14}  {}",
                    style("Model").dim(),
                    style(config.model.as_deref().unwrap_or("(not set)")).bold()
                );
                println!(
                    "    {:<14}  {}",
                    style("URL").dim(),
                    config.resolved_endpoint()
                );

                let tool_names: Vec<String> = runtime
                    .tool_definitions()
                    .into_iter()
                    .map(|d| d.name)
                    .collect();
                println!(
                    "    {:<14}  {}",
                    style("Tools").dim(),
                    if tool_names.is_empty() {
                        "none".to_string()
                    } else {
                        tool_names.join(", ")
                    }
                );

                let web = if config.brave_api_key.is_some() || config.serpapi_key.is_some() {
                    style("enabled").green().to_string()
                } else {
                    style("not configured").dim().to_string()
                };
                println!("    {:<14}  {}", style("Web search").dim(), web);
                println!();
            }
            SlashResult::Continue
        }

        "/tools" => {
            let tool_names: Vec<String> = runtime
                .tool_definitions()
                .into_iter()
                .map(|d| d.name)
                .collect();

            if config.no_color {
                println!("Available tools:");
                for name in &tool_names {
                    println!("  - {name}");
                }
            } else {
                println!();
                if tool_names.is_empty() {
                    println!("  {}  No tools enabled.", style("ℹ").yellow());
                } else {
                    println!(
                        "  {}  {} tool{} available:",
                        style("●").cyan(),
                        tool_names.len(),
                        if tool_names.len() == 1 { "" } else { "s" }
                    );
                    for name in &tool_names {
                        println!("    • {}", style(name).cyan());
                    }
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
                println!();
                println!(
                    "  {}  Model: {}\n",
                    style("◦").dim(),
                    style(model).bold()
                );
            }
            SlashResult::Continue
        }

        "/provider" => {
            if config.no_color {
                println!("Provider: {} ({})", config.provider, config.resolved_endpoint());
            } else {
                println!();
                println!(
                    "  {}  Provider: {} ({})\n",
                    style("◦").dim(),
                    style(&config.provider).bold(),
                    config.resolved_endpoint()
                );
            }
            SlashResult::Continue
        }

        _ => SlashResult::Error(format!(
            "Unknown command '{cmd}'. Type {} for help.",
            style("/help").bold()
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn print_prompt(config: &AppConfig) {
    if config.no_color {
        print!("\n> ");
    } else {
        print!("\n{}", Style::new().cyan().bold().apply_to("❯ "));
    }
}

/// Ring the terminal bell to get the user's attention.
fn ring_bell() {
    print!("\x07");
    let _ = std::io::stdout().flush();
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
    let model_name = config.model.as_deref().unwrap_or("(not set)");
    let provider_name = &config.provider;

    println!();
    println!("  {}", dim.apply_to("╭────────────────────────────────────────────────────────────╮"));
    println!(
        "  {}  {}  {}",
        dim.apply_to("│"),
        style("✻").magenta().bold(),
        style("Code Buddy — AI coding assistant                          │").bold()
    );
    println!("  {}", dim.apply_to("╰────────────────────────────────────────────────────────────╯"));
    println!();
    println!("  {}  {}", style("Provider").dim(), style(provider_name).cyan().bold());
    println!("  {}  {}", style("Model").dim(), style(model_name).cyan());
    println!();
    println!(
        "  {} Type {} for help, {} to quit.",
        dim.apply_to("◦"),
        style("/help").bold(),
        style("/quit").bold()
    );
    println!();
}
