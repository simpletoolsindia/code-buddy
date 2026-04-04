//! `setup` subcommand — interactive first-run wizard.
//!
//! Guides the user through:
//! 1. Provider selection
//! 2. API key entry (providers that require one)
//! 3. Live model-list fetch → fuzzy model selection
//! 4. Optional web search key (Brave / `SerpAPI`)
//! 5. Optional Firecrawl key for page fetching
//! 6. Writes the result to `~/.config/code-buddy/config.toml`
//!
//! Falls back gracefully when not running in a TTY.

use std::io::IsTerminal;

use code_buddy_config::AppConfig;
use code_buddy_providers::model_list as ml;
use console::style;
use dialoguer::{FuzzySelect, Input, Password, Select, theme::ColorfulTheme};

use crate::args::SetupArgs;

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

    print_wizard_banner();

    let theme = ColorfulTheme::default();

    // ── Step 1: Provider ──────────────────────────────────────────────────────
    let providers = vec![
        "lm-studio   (local, no API key needed)",
        "ollama      (local, no API key needed)",
        "openrouter  (cloud, supports 200+ models)",
        "openai      (cloud, GPT-4o / o1 / o3)",
        "nvidia      (cloud, NIM endpoint)",
        "custom      (custom OpenAI-compat endpoint)",
    ];
    let provider_keys = [
        "lm-studio",
        "ollama",
        "openrouter",
        "openai",
        "nvidia",
        "custom",
    ];

    println!(
        "\n  {} Select a provider:",
        style("Step 1").cyan().bold()
    );
    let provider_idx = Select::with_theme(&theme)
        .items(&providers)
        .default(0)
        .interact()
        .unwrap_or(0);
    let provider = provider_keys[provider_idx].to_string();

    // ── Step 2: API key / endpoint ────────────────────────────────────────────
    let mut api_key: Option<String> = None;
    let mut endpoint: Option<String> = None;

    match provider.as_str() {
        "openrouter" => {
            println!(
                "\n  {} OpenRouter API key (from openrouter.ai/keys):",
                style("Step 2").cyan().bold()
            );
            let key: String = Password::with_theme(&theme)
                .with_prompt("  API key")
                .interact()
                .unwrap_or_default();
            if !key.is_empty() {
                api_key = Some(key);
            }
        }
        "openai" => {
            println!(
                "\n  {} OpenAI API key:",
                style("Step 2").cyan().bold()
            );
            let key: String = Password::with_theme(&theme)
                .with_prompt("  API key (sk-...)")
                .interact()
                .unwrap_or_default();
            if !key.is_empty() {
                api_key = Some(key);
            }
        }
        "nvidia" => {
            println!(
                "\n  {} NVIDIA NIM API key:",
                style("Step 2").cyan().bold()
            );
            let key: String = Password::with_theme(&theme)
                .with_prompt("  API key (nvapi-...)")
                .interact()
                .unwrap_or_default();
            if !key.is_empty() {
                api_key = Some(key);
            }
        }
        "custom" => {
            println!(
                "\n  {} Custom OpenAI-compatible endpoint:",
                style("Step 2").cyan().bold()
            );
            let ep: String = Input::with_theme(&theme)
                .with_prompt("  Base URL (e.g. http://localhost:8080/v1)")
                .default("http://localhost:8080/v1".to_string())
                .interact_text()
                .unwrap_or_default();
            if !ep.is_empty() {
                endpoint = Some(ep);
            }
            let key: String = Input::with_theme(&theme)
                .with_prompt("  API key (leave blank if not required)")
                .default(String::new())
                .interact_text()
                .unwrap_or_default();
            if !key.is_empty() {
                api_key = Some(key);
            }
        }
        "ollama" => {
            println!(
                "\n  {} Ollama endpoint (default: http://localhost:11434):",
                style("Step 2").cyan().bold()
            );
            let ep: String = Input::with_theme(&theme)
                .with_prompt("  Endpoint")
                .default("http://localhost:11434".to_string())
                .interact_text()
                .unwrap_or_default();
            if ep != "http://localhost:11434" {
                endpoint = Some(ep);
            }
        }
        _ => {}
    }

    // ── Step 3: Model selection ───────────────────────────────────────────────
    println!(
        "\n  {} Fetching available models…",
        style("Step 3").cyan().bold()
    );

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        indicatif::ProgressStyle::with_template("  {spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    spinner.set_message("Querying provider API…");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let mut models = match provider.as_str() {
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
        "nvidia" => ml::nvidia_models(),
        _ => vec![],
    };

    spinner.finish_and_clear();

    let selected_model = if models.is_empty() {
        println!(
            "  {} Could not fetch models. Enter the model name manually:",
            style("ℹ").yellow()
        );
        let m: String = Input::with_theme(&theme)
            .with_prompt("  Model name")
            .default("local-model".to_string())
            .interact_text()
            .unwrap_or_else(|_| "local-model".to_string());
        m
    } else {
        models.push("[ Enter manually ]".to_string());
        let idx = FuzzySelect::with_theme(&theme)
            .with_prompt("  Search and select a model")
            .items(&models)
            .default(0)
            .interact()
            .unwrap_or(0);

        if idx == models.len() - 1 {
            Input::with_theme(&theme)
                .with_prompt("  Model name")
                .interact_text()
                .unwrap_or_else(|_| "local-model".to_string())
        } else {
            models[idx].clone()
        }
    };

    // ── Step 4: Web search key (optional) ────────────────────────────────────
    println!(
        "\n  {} Web search (optional — enables the web_search tool):",
        style("Step 4").cyan().bold()
    );
    println!("  Get a free key at: https://brave.com/search/api/");
    let brave_key: String = Input::with_theme(&theme)
        .with_prompt("  Brave Search API key (leave blank to skip)")
        .default(String::new())
        .interact_text()
        .unwrap_or_default();

    // ── Step 5: Firecrawl key (optional) ─────────────────────────────────────
    println!(
        "\n  {} Firecrawl (optional — improves web_fetch quality):",
        style("Step 5").cyan().bold()
    );
    println!("  Get a key at: https://firecrawl.dev");
    let firecrawl_key: String = Input::with_theme(&theme)
        .with_prompt("  Firecrawl API key (leave blank to skip)")
        .default(String::new())
        .interact_text()
        .unwrap_or_default();

    // ── Write config ──────────────────────────────────────────────────────────
    let mut config = AppConfig {
        provider: provider.clone(),
        model: Some(selected_model.clone()),
        endpoint,
        api_key,
        ..Default::default()
    };
    if !brave_key.is_empty() {
        config.brave_api_key = Some(brave_key);
    }
    if !firecrawl_key.is_empty() {
        config.firecrawl_api_key = Some(firecrawl_key);
    }

    match config.save() {
        Ok(path) => {
            println!();
            println!(
                "  {} Configuration saved to {}",
                style("✔").green().bold(),
                style(path.display().to_string()).underlined()
            );
            println!();
            println!("  Summary:");
            println!("    Provider : {}", style(&provider).cyan());
            println!("    Model    : {}", style(&selected_model).cyan());
            println!();
            println!(
                "  Run {} to start coding!",
                style("code-buddy").bold()
            );
            println!();
            0
        }
        Err(e) => {
            eprintln!(
                "\n  {} Failed to save config: {e}",
                style("✘").red().bold()
            );
            1
        }
    }
}

fn print_wizard_banner() {
    println!();
    println!(
        "  {}",
        style("╭──────────────────────────────────────────╮").dim()
    );
    println!(
        "  {} {}  {}",
        style("│").dim(),
        style("✻").magenta().bold(),
        style("Code Buddy — Setup Wizard                │").bold()
    );
    println!(
        "  {}",
        style("╰──────────────────────────────────────────╯").dim()
    );
    println!();
    println!("  This wizard will configure your AI provider, model, and optional");
    println!("  web search keys. Settings are saved to ~/.config/code-buddy/config.toml");
    println!("  and can be changed at any time with `code-buddy config set <field> <value>`.");
}
