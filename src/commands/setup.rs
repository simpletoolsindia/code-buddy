//! Interactive setup command

use crate::state::AppState;
use anyhow::Result;
use dialoguer::{Input, Select};

const PROVIDERS: &[&str] = &[
    "ollama (Local, FREE - no API key needed)",
    "nvidia (NVIDIA NIM - FREE tier available)",
    "openrouter (100+ models, free tier)",
    "anthropic (Claude models)",
    "openai (GPT models)",
    "groq (Fast inference)",
    "deepseek (Affordable)",
    "other (Custom provider)",
];

const OLLAMA_MODELS: &[&str] = &[
    "llama3.2:latest",
    "llama3.2:1b",
    "llama3.2:3b",
    "qwen2.5:0.5b",
    "qwen3-coder:30b",
    "codellama:7b",
];

const NVIDIA_MODELS: &[&str] = &[
    "meta/llama-3.1-8b-instruct (FREE)",
    "meta/llama-3.1-70b-instruct",
    "meta/llama-3.1-nemotron-70b-instruct (FREE)",
    "nvidia/llama-3.1-nemotron-80b-instruct",
    "mistralai/mixtral-8x7b-instruct-v0.1",
    "google/gemma-2-27b-it",
];

const OPENROUTER_MODELS: &[&str] = &[
    "google/gemini-2.5-flash-preview-05-20:free (FREE)",
    "anthropic/claude-sonnet-4-6",
    "openai/gpt-4o",
    "meta-llama/llama-4-maverick:free (FREE)",
    "deepseek/deepseek-chat-v3-0324:free (FREE)",
    "mistralai/mistral-nemo:free (FREE)",
];

const ANTHROPIC_MODELS: &[&str] = &[
    "claude-opus-4-6 (Most capable)",
    "claude-sonnet-4-6 (Balanced)",
    "claude-haiku-4-5-20251001 (Fastest)",
];

const OPENAI_MODELS: &[&str] = &[
    "gpt-4o (Most capable)",
    "gpt-4o-mini (Fast)",
    "gpt-4-turbo",
];

const GROQ_MODELS: &[&str] = &[
    "llama-3.1-8b-instant (Fast, FREE)",
    "llama-3.1-70b-versatile",
    "mixtral-8x7b-32768",
    "gemma2-9b-it",
];

const DEEPSEEK_MODELS: &[&str] = &[
    "deepseek-chat (Balanced)",
    "deepseek-coder (Code specialized)",
];

pub async fn run(state: &mut AppState) -> Result<i32> {
    println!("\n=== Code Buddy Interactive Setup ===\n");
    println!("Let's configure your Code Buddy assistant.\n");

    // Step 1: Select provider
    println!("Step 1: Choose your LLM Provider\n");

    let provider_idx = Select::new()
        .with_prompt("Select a provider (use arrow keys, Enter to select)")
        .items(PROVIDERS)
        .default(1) // nvidia as default
        .interact()?;

    let provider = match provider_idx {
        0 => "ollama",
        1 => "nvidia",
        2 => "openrouter",
        3 => "anthropic",
        4 => "openai",
        5 => "groq",
        6 => "deepseek",
        7 => {
            // Custom provider
            let custom: String = Input::new()
                .with_prompt("Enter custom provider name (e.g., azure, vertex)")
                .interact()?;
            println!("Custom provider: {}", custom);
            state.config.llm_provider = custom.clone();
            state.save_config()?;
            println!("\n✓ Setup complete!");
            return Ok(0);
        }
        _ => "nvidia",
    };

    state.config.llm_provider = provider.to_string();
    println!("Selected provider: {}\n", provider);

    // Step 2: Select model based on provider
    println!("Step 2: Choose a Model\n");

    let models = match provider {
        "ollama" => OLLAMA_MODELS,
        "nvidia" => NVIDIA_MODELS,
        "openrouter" => OPENROUTER_MODELS,
        "anthropic" => ANTHROPIC_MODELS,
        "openai" => OPENAI_MODELS,
        "groq" => GROQ_MODELS,
        "deepseek" => DEEPSEEK_MODELS,
        _ => OLLAMA_MODELS,
    };

    let model_idx = Select::new()
        .with_prompt("Select a model (use arrow keys, Enter to select)")
        .items(models)
        .default(0)
        .interact()?;

    let selected_model = models[model_idx];
    // Extract just the model name (before the description)
    let model_name = if selected_model.contains(" (") {
        selected_model.split(" (").next().unwrap_or(selected_model)
    } else {
        selected_model
    };

    state.config.model = Some(model_name.to_string());
    println!("Selected model: {}\n", model_name);

    // Step 3: API key if needed
    let needs_api_key = !["ollama"].contains(&provider);

    if needs_api_key {
        println!("Step 3: API Key\n");

        let api_key: String = Input::new()
            .with_prompt("Enter your API key (or press Enter to skip)")
            .allow_empty(true)
            .interact()?;

        if !api_key.is_empty() {
            state.config.api_key = Some(api_key.clone());
            println!("API key saved: {}...", &api_key[..8.min(api_key.len())]);
        } else {
            println!("No API key entered. You can set it later with:");
            println!("  code-buddy config set api_key YOUR_KEY");
        }
    } else {
        println!("Step 3: API Key (Not needed for Ollama)\n");
        println!("✓ No API key needed for local models!");
    }

    // Clear base_url for default providers
    state.config.base_url = None;

    // Save configuration
    if let Err(e) = state.save_config() {
        eprintln!("Warning: Failed to save config: {}", e);
    }

    // Summary
    println!("\n=== Setup Complete! ===\n");
    println!("Provider: {}", state.config.llm_provider);
    println!("Model: {}", state.config.model.as_deref().unwrap_or("default"));
    println!("API Key: {}\n", if state.config.api_key.is_some() { "Configured" } else { "Not set" });

    println!("Test your setup:");
    println!("  code-buddy -p \"Hello, world!\"\n");

    Ok(0)
}
