//! `run` subcommand — simple interactive mode for all users.
//!
//! # UX goals (non-technical users)
//! - Clear banner showing what model is active
//! - "Thinking…" shown while AI is working
//! - Tool call names shown so users know what's happening
//! - Bell rings when AI needs attention
//! - Simple slash commands (type /help to see them)
//! - Tab completion for slash commands
//! - Interactive /provider and /model commands to switch LLM mid-session

use std::io::{self, IsTerminal, Write};
use std::time::Duration;

use crate::args::RunArgs;
use code_buddy_config::AppConfig;
use code_buddy_providers::ProviderRegistry;
use code_buddy_providers::model_list as ml;
use code_buddy_runtime::{ConversationRuntime, RuntimeConfig, TextSink};
use code_buddy_tools::ToolRegistry;
use console::{Style, style};
use dialoguer::{Input, Select, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Result as RlResult};
use tracing::debug;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// All slash commands with their descriptions.
const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/help",     "Show all commands"),
    ("/quit",     "Exit Code Buddy"),
    ("/exit",     "Exit Code Buddy"),
    ("/clear",    "Start a new conversation"),
    ("/status",   "Check current model and provider"),
    ("/tools",    "See what tools are available"),
    ("/provider", "Switch to a different AI provider"),
    ("/model",    "Switch to a different model"),
];

enum SlashResult {
    Exit,
    Continue,
    Error(String),
    ConfigChanged,
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

    // ── Interactive input with tab completion ────────────────────────────────
    let use_rustyline = std::io::stdin().is_terminal() && !config.no_color;
    let mut rl_editor = if use_rustyline {
        Some(build_editor())
    } else {
        None
    };

    // ── Main input loop ───────────────────────────────────────────────────────
    loop {
        print_prompt(config);
        io::stdout().flush().unwrap_or(());

        let input = if let Some(ref mut rl) = rl_editor {
            match rl.readline("") {
                Ok(line) => line,
                Err(ReadlineError::Eof) => {
                    println!();
                    break;
                }
                Err(_) => continue,
            }
        } else {
            // Non-TTY or no-color: fall back to simple read_line
            let mut line = String::new();
            if io::stdin().read_line(&mut line).is_err() {
                continue;
            }
            line.trim().to_string()
        };

        if input.is_empty() {
            continue;
        }

        // ── Slash commands ──────────────────────────────────────────────────
        if input.starts_with('/') {
            match handle_slash(&input, config, &mut runtime).await {
                SlashResult::Exit => break,
                SlashResult::Continue => continue,
                SlashResult::Error(msg) => {
                    if config.no_color {
                        eprintln!("{msg}");
                    } else {
                        eprintln!("  {}  {msg}", style("✘").red());
                    }
                }
                SlashResult::ConfigChanged => {
                    if !config.no_color {
                        println!(
                            "  {}  Settings saved. Restart {} to use the new config.\n",
                            style("✔").green(),
                            style("code-buddy").bold()
                        );
                    } else {
                        println!("Settings saved. Restart code-buddy to use the new config.\n");
                    }
                }
            }
            continue;
        }

        // Add to rustyline history (only non-slash commands)
        if let Some(ref mut rl) = rl_editor {
            let _ = rl.add_history_entry(&input);
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
                    if e.to_string().contains("context") || e.to_string().contains("too large") {
                        eprintln!("  {}  Context too long. Type {} to start fresh.", style("ℹ").yellow(), style("/clear").bold());
                    } else {
                        eprintln!("  {}  Run {} to check your setup.", style("ℹ").yellow(), style("/status").bold());
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

// ── Rustyline tab completion ────────────────────────────────────────────────────

/// Slash command completer for tab completion.
#[derive(Clone)]
struct SlashCompleter;

impl SlashCompleter {
    fn new() -> Self {
        Self
    }
}

impl rustyline::Helper for SlashCompleter {}

impl Completer for SlashCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> RlResult<(usize, Vec<Self::Candidate>)> {
        if line.is_empty() || line.starts_with('/') {
            let prefix = line.trim_start_matches('/');
            let completions: Vec<Pair> = SLASH_COMMANDS
                .iter()
                .filter(|(cmd, _)| {
                    cmd.starts_with('/') && cmd.starts_with(&format!("/{prefix}"))
                })
                .map(|(cmd, _desc)| Pair {
                    display: (*cmd).to_string(),
                    replacement: (*cmd).to_string(),
                })
                .collect();
            Ok((pos, completions))
        } else {
            Ok((pos, vec![]))
        }
    }
}

impl Hinter for SlashCompleter {
    type Hint = String;
}

impl Highlighter for SlashCompleter {}

impl Validator for SlashCompleter {}

fn build_editor() -> Editor<SlashCompleter, DefaultHistory> {
    let completer = SlashCompleter::new();
    let mut editor = Editor::<SlashCompleter, DefaultHistory>::new()
        .expect("failed to create editor");
    editor.set_helper(Some(completer));
    editor
}

// ── Slash commands ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
async fn handle_slash(
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
                    style("free (DuckDuckGo)").cyan().to_string()
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
            if !std::io::stdin().is_terminal() || config.no_color {
                let model = config.model.as_deref().unwrap_or("(not set)");
                println!("Model: {model}");
                return SlashResult::Continue;
            }
            match interactive_model_switch(config).await {
                Ok(true) => SlashResult::ConfigChanged,
                Ok(false) => SlashResult::Continue,
                Err(e) => SlashResult::Error(e),
            }
        }

        "/provider" => {
            if !std::io::stdin().is_terminal() || config.no_color {
                println!("Provider: {} ({})", config.provider, config.resolved_endpoint());
                return SlashResult::Continue;
            }
            match interactive_provider_switch(config).await {
                Ok(true) => SlashResult::ConfigChanged,
                Ok(false) => SlashResult::Continue,
                Err(e) => SlashResult::Error(e),
            }
        }

        _ => SlashResult::Error(format!(
            "Unknown command '{cmd}'. Type {} for help.",
            style("/help").bold()
        )),
    }
}

// ── Interactive provider switch ───────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
async fn interactive_provider_switch(config: &AppConfig) -> Result<bool, String> {
    let theme = ColorfulTheme::default();
    println!();

    let providers = vec![
        "NVIDIA          — Free credits, fast, no setup (recommended)",
        "OpenRouter      — 200+ models, supports free models",
        "OpenAI          — GPT-4o, o1, o3 (requires account)",
        "LM Studio       — Run models on your own computer (free)",
        "Ollama          — Another free local option (no GPU needed)",
        "Custom endpoint — Connect to any OpenAI-compatible API",
    ];
    let provider_keys = ["nvidia", "openrouter", "openai", "lm-studio", "ollama", "custom"];

    println!("  {}  Choose a new AI provider:", style("?").cyan());
    let idx = Select::with_theme(&theme)
        .with_prompt("  Type the number and press Enter")
        .items(&providers)
        .default(0)
        .interact()
        .map_err(|e| format!("Selection cancelled: {e}"))?;

    let new_provider = provider_keys[idx].to_string();

    // Step 2: API key or URL for the new provider
    let mut api_key: Option<String> = None;
    let mut endpoint: Option<String> = None;

    match new_provider.as_str() {
        "nvidia" => {
            ring_bell();
            let key: String = Input::with_theme(&theme)
                .with_prompt("  Paste your NVIDIA API key (or press Enter to skip)")
                .interact_text()
                .map_err(|e| format!("Cancelled: {e}"))?;
            if !key.trim().is_empty() {
                api_key = Some(key.trim().to_string());
            }
        }
        "openrouter" => {
            ring_bell();
            let key: String = Input::with_theme(&theme)
                .with_prompt("  Paste your OpenRouter API key (or press Enter to skip)")
                .interact_text()
                .map_err(|e| format!("Cancelled: {e}"))?;
            if !key.trim().is_empty() {
                api_key = Some(key.trim().to_string());
            }
        }
        "openai" => {
            ring_bell();
            let key: String = Input::with_theme(&theme)
                .with_prompt("  Paste your OpenAI API key (or press Enter to skip)")
                .interact_text()
                .map_err(|e| format!("Cancelled: {e}"))?;
            if !key.trim().is_empty() {
                api_key = Some(key.trim().to_string());
            }
        }
        "lm-studio" => {
            let ep: String = Input::with_theme(&theme)
                .with_prompt("  Server URL (press Enter for localhost:1234)")
                .default("http://localhost:1234".to_string())
                .interact_text()
                .map_err(|e| format!("Cancelled: {e}"))?;
            if ep != "http://localhost:1234" {
                endpoint = Some(ep.trim().to_string());
            }
        }
        "ollama" => {
            let ep: String = Input::with_theme(&theme)
                .with_prompt("  Server URL (press Enter for localhost:11434)")
                .default("http://localhost:11434".to_string())
                .interact_text()
                .map_err(|e| format!("Cancelled: {e}"))?;
            if ep != "http://localhost:11434" {
                endpoint = Some(ep.trim().to_string());
            }
        }
        "custom" => {
            let ep: String = Input::with_theme(&theme)
                .with_prompt("  Enter the API URL")
                .default("http://localhost:8080/v1".to_string())
                .interact_text()
                .map_err(|e| format!("Cancelled: {e}"))?;
            endpoint = Some(ep.trim().to_string());
            let key: String = Input::with_theme(&theme)
                .with_prompt("  API key (press Enter to skip)")
                .default(String::new())
                .interact_text()
                .map_err(|e| format!("Cancelled: {e}"))?;
            if !key.trim().is_empty() {
                api_key = Some(key.trim().to_string());
            }
        }
        _ => {}
    }

    // Save updated config
    let mut updated = config.clone();
    updated.provider = new_provider;
    updated.api_key = api_key;
    updated.endpoint = endpoint;
    // Clear model so user picks a new one
    updated.model = None;

    // Fetch and select a new model
    println!("  {}  Fetching models from new provider…", style("●").cyan());
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        indicatif::ProgressStyle::with_template("  {spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    spinner.set_message("Connecting…");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let models = fetch_models_for_provider(&updated).await;
    spinner.finish_and_clear();

    let model = if models.is_empty() {
        "local-model".to_string()
    } else {
        let display: Vec<String> = models.iter().take(15).cloned().collect();
        let choices: Vec<&str> = display.iter().map(std::string::String::as_str).collect();
        let idx = Select::with_theme(&theme)
            .with_prompt("  Choose a model:")
            .items(&choices)
            .default(0)
            .interact()
            .map_err(|e| format!("Cancelled: {e}"))?;
        display[idx].clone()
    };

    updated.model = Some(model);

    updated.save().map_err(|e| format!("Failed to save config: {e}"))?;

    println!();
    println!(
        "  {}  Provider switched to {}",
        style("✔").green(),
        style(&updated.provider).cyan()
    );
    println!("  {}  Config saved.\n", style("✔").green());

    Ok(true)
}

// ── Interactive model switch ──────────────────────────────────────────────────

async fn interactive_model_switch(config: &AppConfig) -> Result<bool, String> {
    let theme = ColorfulTheme::default();
    println!();
    println!("  {}  Fetching available models…", style("●").cyan());

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        indicatif::ProgressStyle::with_template("  {spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    spinner.set_message("Connecting…");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let models = fetch_models_for_provider(config).await;
    spinner.finish_and_clear();

    if models.is_empty() {
        println!();
        let m: String = Input::with_theme(&theme)
            .with_prompt("  Could not reach provider — enter model name manually")
            .default("local-model".to_string())
            .interact_text()
            .map_err(|e| format!("Cancelled: {e}"))?;

        let mut updated = config.clone();
        updated.model = Some(m.clone());
        updated.save().map_err(|e| format!("Failed to save config: {e}"))?;
        println!("  {}  Model set to {m}", style("✔").green());
        return Ok(true);
    }

    let display: Vec<String> = models.iter().take(20).cloned().collect();
    let choices: Vec<&str> = display.iter().map(std::string::String::as_str).collect();

    println!();
    println!("  {}  Choose a new model:", style("?").cyan());
    let idx = Select::with_theme(&theme)
        .with_prompt("  Type the number and press Enter")
        .items(&choices)
        .default(0)
        .interact()
        .map_err(|e| format!("Cancelled: {e}"))?;

    let new_model = display[idx].clone();

    let mut updated = config.clone();
    updated.model = Some(new_model.clone());
    updated.save().map_err(|e| format!("Failed to save config: {e}"))?;

    println!();
    println!("  {}  Model set to {}", style("✔").green(), style(&new_model).cyan());
    println!("  {}  Config saved. Restart {} to use the new model.\n", style("✔").green(), style("code-buddy").bold());

    Ok(true)
}

// ── Model fetching for interactive commands ────────────────────────────────────

async fn fetch_models_for_provider(config: &AppConfig) -> Vec<String> {
    match config.provider.as_str() {
        "lm-studio" => ml::fetch_lm_studio_models(config.endpoint.as_deref()).await,
        "ollama" => ml::fetch_ollama_models(config.endpoint.as_deref()).await,
        "openrouter" => {
            if let Some(ref key) = config.api_key {
                ml::fetch_openrouter_models(key).await
            } else {
                ml::openrouter_fallback_pub()
            }
        }
        "openai" => {
            if let Some(ref key) = config.api_key {
                ml::fetch_openai_models(key).await
            } else {
                ml::openai_fallback_pub()
            }
        }
        "nvidia" => {
            if let Some(ref key) = config.api_key {
                ml::fetch_nvidia_models(key).await
            } else {
                ml::nvidia_models()
            }
        }
        _ => vec![],
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
