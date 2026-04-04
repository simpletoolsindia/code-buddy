//! `setup` subcommand — simple first-run wizard for all users.
//!
//! Guides non-technical users through:
//! 1. Choose your AI provider (numbered list)
//! 2. Enter API key or server URL
//! 3. Pick a model (popular choices shown first)
//! 4. Optional: enable web search
//!
//! Falls back gracefully when not running in a TTY.

use std::io::{IsTerminal, Write};

use code_buddy_config::AppConfig;
use code_buddy_providers::model_list as ml;
use console::style;
use dialoguer::{Input, Password, Select, theme::ColorfulTheme};

use crate::args::SetupArgs;

/// Ring the terminal bell to get the user's attention.
fn ring_bell() {
    print!("\x07");
    let _ = std::io::stdout().flush();
}

#[allow(clippy::too_many_lines)]
pub async fn run(_args: SetupArgs) -> i32 {
    if !std::io::stdin().is_terminal() {
        eprintln!(
            "{}  setup requires an interactive terminal (TTY). \
             Run `code-buddy setup` in your shell.",
            style("✘").red().bold()
        );
        return 1;
    }

    print_intro();

    let theme = ColorfulTheme::default();

    // ── Step 1: Provider ──────────────────────────────────────────────────────
    println!();
    println!("  {}  Which AI service do you want to use?", style("1.").cyan().bold());
    println!();
    let providers = vec![
        "NVIDIA          — Free credits, fast, no setup (recommended)",
        "OpenRouter      — 200+ models, supports free models",
        "OpenAI          — GPT-4o, o1, o3 (requires account)",
        "LM Studio       — Run models on your own computer (free)",
        "Ollama          — Another free local option (no GPU needed)",
        "Custom endpoint — Connect to any OpenAI-compatible API",
    ];
    let provider_keys = [
        "nvidia",
        "openrouter",
        "openai",
        "lm-studio",
        "ollama",
        "custom",
    ];

    let provider_idx = match Select::with_theme(&theme)
        .items(&providers)
        .default(0)
        .interact()
    {
        Ok(i) => i,
        Err(_) => {
            eprintln!("\n  {} Setup cancelled.\n", style("✘").red());
            return 1;
        }
    };
    let provider = provider_keys[provider_idx].to_string();

    // ── Step 2: API key or URL ─────────────────────────────────────────────────
    let mut api_key: Option<String> = None;
    let mut endpoint: Option<String> = None;

    match provider.as_str() {
        "nvidia" => {
            println!();
            println!("  {}  NVIDIA needs an API key to connect.", style("2.").cyan().bold());
            println!();
            println!("  Get your free key at: https://build.nvidia.com/");
            println!("  Sign up → Click your profile → Copy API Key");
            println!();
            ring_bell();
            let key: String = match Password::with_theme(&theme)
                .with_prompt("  Paste your NVIDIA API key (starts with nvapi-)")
                .interact()
            {
                Ok(k) => k,
                Err(_) => {
                    eprintln!("\n  {} Setup cancelled.\n", style("✘").red());
                    return 1;
                }
            };
            if !key.trim().is_empty() {
                api_key = Some(key.trim().to_string());
            }
        }
        "openrouter" => {
            println!();
            println!("  {}  OpenRouter needs an API key.", style("2.").cyan().bold());
            println!();
            println!("  Get a key at: https://openrouter.ai/keys");
            println!();
            ring_bell();
            let key: String = match Password::with_theme(&theme)
                .with_prompt("  Paste your OpenRouter API key")
                .interact()
            {
                Ok(k) => k,
                Err(_) => {
                    eprintln!("\n  {} Setup cancelled.\n", style("✘").red());
                    return 1;
                }
            };
            if !key.trim().is_empty() {
                api_key = Some(key.trim().to_string());
            }
        }
        "openai" => {
            println!();
            println!("  {}  OpenAI needs an API key.", style("2.").cyan().bold());
            println!();
            println!("  Get a key at: https://platform.openai.com/api-keys");
            println!();
            ring_bell();
            let key: String = match Password::with_theme(&theme)
                .with_prompt("  Paste your OpenAI API key (starts with sk-)")
                .interact()
            {
                Ok(k) => k,
                Err(_) => {
                    eprintln!("\n  {} Setup cancelled.\n", style("✘").red());
                    return 1;
                }
            };
            if !key.trim().is_empty() {
                api_key = Some(key.trim().to_string());
            }
        }
        "lm-studio" => {
            println!();
            println!("  {}  LM Studio URL", style("2.").cyan().bold());
            println!();
            println!("  {}  Start LM Studio on your computer, then press Enter.", style("ℹ").yellow());
            println!("  {}  Or enter a remote URL like http://192.168.1.100:1234", style("ℹ").yellow());
            println!();
            let ep: String = match Input::with_theme(&theme)
                .with_prompt("  Press Enter for local (localhost:1234)")
                .default("http://localhost:1234".to_string())
                .interact_text()
            {
                Ok(e) => e,
                Err(_) => "http://localhost:1234".to_string(),
            };
            if ep != "http://localhost:1234" {
                endpoint = Some(ep.trim().to_string());
            }
        }
        "ollama" => {
            println!();
            println!("  {}  Ollama URL", style("2.").cyan().bold());
            println!();
            println!("  {}  Start Ollama (run 'ollama serve'), then press Enter.", style("ℹ").yellow());
            println!("  {}  Or enter a remote URL like http://192.168.1.100:11434", style("ℹ").yellow());
            println!();
            let ep: String = match Input::with_theme(&theme)
                .with_prompt("  Press Enter for local (localhost:11434)")
                .default("http://localhost:11434".to_string())
                .interact_text()
            {
                Ok(e) => e,
                Err(_) => "http://localhost:11434".to_string(),
            };
            if ep != "http://localhost:11434" {
                endpoint = Some(ep.trim().to_string());
            }
        }
        "custom" => {
            println!();
            println!("  {}  Custom API URL", style("2.").cyan().bold());
            println!();
            let ep: String = match Input::with_theme(&theme)
                .with_prompt("  Enter the API URL")
                .default("http://localhost:8080/v1".to_string())
                .interact_text()
            {
                Ok(e) => e,
                Err(_) => "http://localhost:8080/v1".to_string(),
            };
            endpoint = Some(ep.trim().to_string());
            let key: String = match Input::with_theme(&theme)
                .with_prompt("  API key (press Enter to skip if not needed)")
                .default(String::new())
                .interact_text()
            {
                Ok(k) => k,
                Err(_) => String::new(),
            };
            if !key.trim().is_empty() {
                api_key = Some(key.trim().to_string());
            }
        }
        _ => {}
    }

    // ── Step 3: Model selection ───────────────────────────────────────────────
    println!();
    println!("  {}  Finding the best models for you…", style("3.").cyan().bold());
    println!();

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        indicatif::ProgressStyle::with_template("  {spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    spinner.set_message("Connecting to provider…");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let models = match provider.as_str() {
        "lm-studio" => {
            let host = endpoint.as_deref();
            ml::fetch_lm_studio_models(host).await
        }
        "ollama" => {
            let host = endpoint.as_deref();
            ml::fetch_ollama_models(host).await
        }
        "openrouter" => {
            if let Some(ref key) = api_key {
                ml::fetch_openrouter_models(key).await
            } else {
                ml::openrouter_fallback_pub()
            }
        }
        "openai" => {
            if let Some(ref key) = api_key {
                ml::fetch_openai_models(key).await
            } else {
                ml::openai_fallback_pub()
            }
        }
        "nvidia" => {
            if let Some(ref key) = api_key {
                ml::fetch_nvidia_models(key).await
            } else {
                ml::nvidia_models()
            }
        }
        _ => vec![],
    };

    spinner.finish_and_clear();

    if models.is_empty() {
        println!();
        println!("  {}  Could not reach the provider. Enter model name manually:", style("⚠").yellow());
        println!();
        let m: String = match Input::with_theme(&theme)
            .with_prompt("  Model name")
            .default("local-model".to_string())
            .interact_text()
        {
            Ok(m) => m,
            Err(_) => "local-model".to_string(),
        };
        print_summary_and_save(&provider, &m, endpoint, api_key);
        return 0;
    }

    // Show top 15 models in a simple numbered list
    let mut display_models: Vec<String> = if models.len() > 15 {
        models.iter().take(15).cloned().collect()
    } else {
        models.clone()
    };
    display_models.push("[ Other model ]".to_string());

    println!();
    println!("  {}  Choose a model:", style("3.").cyan().bold());
    println!();
    let help_texts: Vec<String> = display_models
        .iter()
        .map(|m| {
            if m.contains("llama-3.3-70b") || m.contains("mistral-large") || m.contains("gpt-4o") {
                format!("{m}  (popular)", )
            } else {
                m.clone()
            }
        })
        .collect();

    let model_idx = match Select::with_theme(&theme)
        .with_prompt("  Type the number and press Enter")
        .items(&help_texts)
        .default(0)
        .interact()
    {
        Ok(i) => i,
        Err(_) => {
            eprintln!("\n  {} Setup cancelled.\n", style("✘").red());
            return 1;
        }
    };

    let selected_model = if model_idx >= display_models.len() - 1 {
        println!();
        let m: String = match Input::with_theme(&theme)
            .with_prompt("  Enter the model name")
            .default("local-model".to_string())
            .interact_text()
        {
            Ok(m) => m,
            Err(_) => "local-model".to_string(),
        };
        m
    } else {
        display_models[model_idx].clone()
    };

    // ── Step 4: Web search (optional) ────────────────────────────────────────
    println!();
    println!("  {}  Enable web search?", style("4.").cyan().bold());
    println!();
    println!("  This lets Code Buddy search the internet to answer your questions.");
    println!("  {}  Get a free key at: https://brave.com/search/api/", style("ℹ").yellow());
    println!();
    let enable_search = match Select::with_theme(&theme)
        .with_prompt("  Enable web search?")
        .items(&["No (skip for now)", "Yes, I have a Brave Search key"])
        .default(0)
        .interact()
    {
        Ok(i) => i,
        Err(_) => 0,
    };

    let mut brave_api_key: Option<String> = None;

    if enable_search == 1 {
        println!();
        println!("  {}  Paste your Brave Search API key:", style("•").cyan());
        ring_bell();
        let key: String = match Password::with_theme(&theme)
            .with_prompt("  API key")
            .interact()
        {
            Ok(k) => k,
            Err(_) => String::new(),
        };
        if !key.trim().is_empty() {
            brave_api_key = Some(key.trim().to_string());
        }
    }

    // ── Save ──────────────────────────────────────────────────────────────────
    print_summary_and_save(
        &provider,
        &selected_model,
        endpoint,
        api_key.clone(),
    );

    // Show web search result
    if brave_api_key.is_some() {
        println!();
        println!(
            "  {}  Web search enabled!",
            style("✔").green()
        );
    }

    println!();
    println!("  {}  You're all set! Run {} to start.", style("✔").green(), style("code-buddy").bold());
    println!();
    0
}

fn print_summary_and_save(
    provider: &str,
    model: &str,
    endpoint: Option<String>,
    api_key: Option<String>,
) {
    let config = AppConfig {
        provider: provider.to_string(),
        model: Some(model.to_string()),
        endpoint,
        api_key,
        ..Default::default()
    };

    match config.save() {
        Ok(path) => {
            println!();
            println!(
                "  {}  Saved to: {}",
                style("✔").green(),
                style(path.display().to_string()).underlined()
            );
            println!();
            println!("  Configuration:");
            println!("    Provider : {}", style(&config.provider).cyan());
            println!("    Model    : {}", style(config.model.as_deref().unwrap_or("–")).cyan());
            println!();
        }
        Err(e) => {
            eprintln!(
                "\n  {}  Failed to save config: {e}\n",
                style("✘").red()
            );
        }
    }
}

fn print_intro() {
    println!();
    println!(
        "  {}",
        style("╭────────────────────────────────────────────────────────────╮").dim()
    );
    println!(
        "  {}  {}  {}",
        style("│").dim(),
        style("✻").magenta().bold(),
        style("Welcome to Code Buddy!                              │").bold()
    );
    println!(
        "  {}",
        style("╰────────────────────────────────────────────────────────────╯").dim()
    );
    println!();
    println!(
        "  {}  Let's set up Code Buddy in a few quick steps.",
        style("●").green()
    );
    println!();
    println!(
        "  This will save your settings to {}",
        style("~/.config/code-buddy/config.toml").underlined()
    );
    println!(
        "  You can change settings anytime with {}",
        style("code-buddy config set <field> <value>").dim()
    );
    println!();
}
