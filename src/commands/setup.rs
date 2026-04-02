//! Interactive setup command

use crate::mlx::MlxConfig;
use crate::state::AppState;
use anyhow::Result;
use dialoguer::{Input, Select};

const PROVIDERS_CLOUD: &[&str] = &[
    "nvidia (NVIDIA NIM - FREE tier available)",
    "openrouter (100+ models, free tier)",
    "anthropic (Claude models)",
    "openai (GPT models)",
    "groq (Fast inference)",
    "deepseek (Affordable)",
];

const PROVIDERS_LOCAL: &[&str] = &[
    "ollama (Local, FREE - no API key needed)",
    "mlx (Apple Silicon, FREE - MLX optimized)",
];

const OLLAMA_MODELS: &[&str] = &[
    "llama3.2:latest",
    "llama3.2:1b",
    "llama3.2:3b",
    "qwen2.5:0.5b",
    "qwen3-coder:30b",
    "codellama:7b",
];

const MLX_MODELS: &[(&str, &str)] = &[
    ("mlx-community/llama-3.2-1b-instruct-4bit", "Llama 3.2 1B (4-bit, ~700MB)"),
    ("mlx-community/llama-3.2-3b-instruct-4bit", "Llama 3.2 3B (4-bit, ~2GB)"),
    ("mlx-community/Qwen2.5-1.5B-Instruct-4bit", "Qwen 2.5 1.5B (4-bit, ~1GB)"),
    ("mlx-community/gemma-2b-it-4bit", "Gemma 2B (4-bit, ~1.8GB)"),
    ("mlx-community/llama-3.1-8b-instruct-4bit", "Llama 3.1 8B (4-bit, ~5GB)"),
    ("mlx-community/mistral-7b-instruct-v0.3-4bit", "Mistral 7B v0.3 (4-bit, ~4GB)"),
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

    // Check if on Apple Silicon
    let is_apple_silicon = MlxConfig::is_apple_silicon();

    // Step 1: Select provider category
    println!("Step 1: Choose your LLM Provider\n");

    let all_providers = if is_apple_silicon {
        vec![
            "Cloud (API-based, requires API key)".to_string(),
            "Local (Runs on your machine, FREE)".to_string(),
            "Other (Custom provider)".to_string(),
        ]
    } else {
        vec![
            "Cloud (API-based, requires API key)".to_string(),
            "Local (Runs on your machine, FREE)".to_string(),
            "Other (Custom provider)".to_string(),
        ]
    };

    let category_idx = Select::new()
        .with_prompt("Select provider category (use arrow keys, Enter to select)")
        .items(&all_providers)
        .default(0)
        .interact()?;

    let provider: &str;
    let models: &[&str];

    match category_idx {
        0 => {
            // Cloud providers
            println!();
            let provider_idx = Select::new()
                .with_prompt("Select a cloud provider")
                .items(PROVIDERS_CLOUD)
                .default(0)
                .interact()?;

            provider = match provider_idx {
                0 => "nvidia",
                1 => "openrouter",
                2 => "anthropic",
                3 => "openai",
                4 => "groq",
                5 => "deepseek",
                _ => "nvidia",
            };

            models = match provider {
                "nvidia" => NVIDIA_MODELS,
                "openrouter" => OPENROUTER_MODELS,
                "anthropic" => ANTHROPIC_MODELS,
                "openai" => OPENAI_MODELS,
                "groq" => GROQ_MODELS,
                "deepseek" => DEEPSEEK_MODELS,
                _ => NVIDIA_MODELS,
            };
        }
        1 => {
            // Local providers
            println!();
            let local_providers: Vec<String> = if is_apple_silicon {
                vec![
                    "ollama (Linux/Windows/Intel Mac)".to_string(),
                    "mlx (Apple Silicon M1/M2/M3/M4)".to_string(),
                ]
            } else {
                vec!["ollama (Local models)".to_string()]
            };

            let local_idx = Select::new()
                .with_prompt("Select a local provider")
                .items(&local_providers)
                .default(0)
                .interact()?;

            if is_apple_silicon && local_idx == 1 {
                // MLX setup
                println!("\n=== MLX (Apple Silicon) Setup ===\n");
                println!("MLX provides optimized local inference on Apple Silicon Macs.");
                println!("It downloads models from HuggingFace mlx-community.\n");

                // Check if mlx-lm is installed
                let mlx_config = MlxConfig::new();
                if !mlx_config.check_mlx_lm_installed() {
                    let install = dialoguer::Confirm::new()
                        .with_prompt("mlx-lm is not installed. Install it now?")
                        .default(true)
                        .interact()?;

                    if install {
                        if let Err(e) = mlx_config.install_mlx_lm() {
                            eprintln!("Warning: Failed to install mlx-lm: {}", e);
                        }
                    }
                }

                // Show available models
                println!("\nAvailable MLX models:\n");
                for (i, (_, desc)) in MLX_MODELS.iter().enumerate() {
                    println!("{}. {}", i + 1, desc);
                }

                let model_idx = Select::new()
                    .with_prompt("\nSelect a model to download")
                    .max_length(6)
                    .items(&MLX_MODELS.iter().map(|(_, d)| *d).collect::<Vec<_>>())
                    .default(1)
                    .interact()?;

                let (model_id, _) = MLX_MODELS[model_idx];
                println!("\nSelected model: {}", model_id);

                // Download the model
                let mlx_config = MlxConfig::new();
                if let Err(e) = mlx_config.download_model(model_id).await {
                    eprintln!("Warning: Failed to download model: {}", e);
                } else {
                    println!("✓ Model downloaded successfully!");
                }

                state.config.llm_provider = "mlx".to_string();
                state.config.model = Some(model_id.to_string());
                state.save_config()?;

                println!("\n=== Setup Complete! ===\n");
                println!("Provider: mlx (Apple Silicon)");
                println!("Model: {}", model_id);
                println!("\nTest your setup:");
                println!("  code-buddy -p \"Hello, world!\"\n");

                return Ok(0);
            } else {
                // Ollama
                provider = "ollama";
                models = OLLAMA_MODELS;
            }
        }
        2 => {
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
        _ => {
            provider = "nvidia";
            models = NVIDIA_MODELS;
        }
    }

    state.config.llm_provider = provider.to_string();
    println!("Selected provider: {}\n", provider);

    // Step 2: Select model
    println!("Step 2: Choose a Model\n");

    let model_idx = Select::new()
        .with_prompt("Select a model (use arrow keys, Enter to select)")
        .items(models)
        .default(0)
        .interact()?;

    let selected_model = models[model_idx];
    let model_name = if selected_model.contains(" (") {
        selected_model.split(" (").next().unwrap_or(selected_model)
    } else {
        selected_model
    };

    state.config.model = Some(model_name.to_string());
    println!("Selected model: {}\n", model_name);

    // Step 3: API key if needed
    let needs_api_key = !["ollama", "mlx"].contains(&provider);

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
        println!("Step 3: API Key (Not needed for local models)\n");
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
