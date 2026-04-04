//! Interactive setup command - User-friendly configuration

use crate::mlx::MlxConfig;
use crate::state::AppState;
use anyhow::Result;
use dialoguer::{Input, Select};

const PROVIDERS_CLOUD: &[(&str, &str)] = &[
    ("nvidia", "NVIDIA NIM - FREE tier available!"),
    ("openrouter", "100+ models including free options"),
    ("anthropic", "Claude - Powerful and capable"),
    ("openai", "GPT - Fast and reliable"),
    ("groq", "Groq - Lightning fast inference"),
    ("deepseek", "DeepSeek - Affordable option"),
];

const PROVIDERS_LOCAL: &[(&str, &str)] = &[
    ("ollama", "Ollama - FREE, works offline!"),
    ("mlx", "MLX - Apple Silicon, FREE"),
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
    ("mlx-community/llama-3.2-1b-instruct-4bit", "Llama 3.2 1B - Small & Fast (~700MB)"),
    ("mlx-community/llama-3.2-3b-instruct-4bit", "Llama 3.2 3B - Balanced (~2GB)"),
    ("mlx-community/Qwen2.5-1.5B-Instruct-4bit", "Qwen 2.5 1.5B - Efficient (~1GB)"),
    ("mlx-community/gemma-2b-it-4bit", "Gemma 2B - Google's model (~1.8GB)"),
    ("mlx-community/llama-3.1-8b-instruct-4bit", "Llama 3.1 8B - Most capable (~5GB)"),
    ("mlx-community/mistral-7b-instruct-v0.3-4bit", "Mistral 7B - Great for coding (~4GB)"),
];

const NVIDIA_MODELS: &[(&str, &str)] = &[
    ("meta/llama-3.1-8b-instruct", "Llama 3.1 8B - FREE, great quality"),
    ("meta/llama-3.1-70b-instruct", "Llama 3.1 70B - More powerful"),
    ("google/gemma-2-27b-it", "Gemma 2 27B - Google's model"),
    ("mistralai/mixtral-8x7b-instruct-v0.1", "Mixtral 8x7B - Fast mixture of experts"),
];

const OPENROUTER_MODELS: &[(&str, &str)] = &[
    ("google/gemini-2.5-flash-preview-05-20:free", "Gemini Flash - FREE & fast"),
    ("anthropic/claude-sonnet-4-6", "Claude Sonnet - Balanced & capable"),
    ("openai/gpt-4o", "GPT-4o - OpenAI's flagship"),
    ("meta-llama/llama-4-maverick:free", "Llama 4 Maverick - FREE option"),
    ("deepseek/deepseek-chat-v3-0324:free", "DeepSeek V3 - FREE, great value"),
];

const ANTHROPIC_MODELS: &[(&str, &str)] = &[
    ("claude-opus-4-6", "Claude Opus - Most capable, most expensive"),
    ("claude-sonnet-4-6", "Claude Sonnet - Best balance of speed/cost"),
    ("claude-haiku-4-5-20251001", "Claude Haiku - Fastest, cheapest"),
];

const OPENAI_MODELS: &[(&str, &str)] = &[
    ("gpt-4o", "GPT-4o - Most capable"),
    ("gpt-4o-mini", "GPT-4o Mini - Fast & affordable"),
    ("gpt-4-turbo", "GPT-4 Turbo - Good balance"),
];

const GROQ_MODELS: &[(&str, &str)] = &[
    ("llama-3.1-8b-instant", "Llama 3.1 8B - FREE & fast"),
    ("llama-3.1-70b-versatile", "Llama 3.1 70B - More powerful"),
    ("mixtral-8x7b-32768", "Mixtral - Mixture of experts"),
    ("gemma2-9b-it", "Gemma 2 9B - Google's model"),
];

const DEEPSEEK_MODELS: &[(&str, &str)] = &[
    ("deepseek-chat", "DeepSeek Chat - Balanced"),
    ("deepseek-coder", "DeepSeek Coder - Optimized for code"),
];

pub async fn run(state: &mut AppState) -> Result<i32> {
    println!();
    println!("\x1b[1m╭─────────────────────────────────────────────────────────────\x1b[0m");
    println!("\x1b[1m│\x1b[0m  \x1b[1mCode Buddy Setup Wizard\x1b[0m");
    println!("\x1b[1m╰─────────────────────────────────────────────────────────────\x1b[0m");
    println!();
    println!("Let's configure your AI assistant!\n");

    // Check if on Apple Silicon
    let is_apple_silicon = MlxConfig::is_apple_silicon();

    // Step 1: Select provider category
    println!("\x1b[1mStep 1:\x1b[0m Where should AI run?\n");

    let all_providers = if is_apple_silicon {
        vec![
            "\x1b[32mCloud\x1b[0m - Use AI via internet (needs API key)".to_string(),
            "\x1b[33mLocal\x1b[0m - Run AI on your computer (FREE!)".to_string(),
            "Other - Custom configuration".to_string(),
        ]
    } else {
        vec![
            "\x1b[32mCloud\x1b[0m - Use AI via internet (needs API key)".to_string(),
            "\x1b[33mLocal\x1b[0m - Run AI on your computer (FREE!)".to_string(),
            "Other - Custom configuration".to_string(),
        ]
    };

    let category_idx = Select::new()
        .with_prompt("Select where AI should run")
        .items(&all_providers)
        .default(0)
        .interact()?;

    let provider: &str;
    let models: &[(&str, &str)];

    match category_idx {
        0 => {
            // Cloud providers
            println!();
            let provider_options: Vec<String> = PROVIDERS_CLOUD
                .iter()
                .map(|(name, desc)| format!("\x1b[36m{}\x1b[0m - {}", name, desc))
                .collect();

            let provider_idx = Select::new()
                .with_prompt("Which cloud provider?")
                .items(&provider_options)
                .default(0)
                .interact()?;

            provider = PROVIDERS_CLOUD[provider_idx].0;
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
                    "\x1b[33mOllama\x1b[0m - Works on any computer".to_string(),
                    "\x1b[35mMLX\x1b[0m - Apple Silicon optimized".to_string(),
                ]
            } else {
                vec!["\x1b[33mOllama\x1b[0m - Run local AI for free".to_string()]
            };

            let local_idx = Select::new()
                .with_prompt("Which local provider?")
                .items(&local_providers)
                .default(0)
                .interact()?;

            if is_apple_silicon && local_idx == 1 {
                // MLX setup
                println!();
                println!("\x1b[1m╭─────────────────────────────────────────────────────────────\x1b[0m");
                println!("\x1b[1m│\x1b[0m  \x1b[1mMLX Setup (Apple Silicon)\x1b[0m");
                println!("\x1b[1m╰─────────────────────────────────────────────────────────────\x1b[0m");
                println!();
                println!("MLX runs AI models optimized for Apple Silicon chips.");
                println!("Models run entirely on your Mac - FREE and private!\n");

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
                let model_options: Vec<String> = MLX_MODELS
                    .iter()
                    .map(|(_, desc)| desc.to_string())
                    .collect();

                let model_idx = Select::new()
                    .with_prompt("Select a model to download")
                    .max_length(6)
                    .items(&model_options)
                    .default(1)
                    .interact()?;

                let (model_id, model_desc) = MLX_MODELS[model_idx];
                println!();
                println!("Downloading: {}", model_desc);

                // Download the model
                let mlx_config = MlxConfig::new();
                if let Err(e) = mlx_config.download_model(model_id).await {
                    eprintln!("Warning: Failed to download model: {}", e);
                } else {
                    println!();
                    println!("\x1b[32m✓ Model ready!\x1b[0m");
                }

                state.config.llm_provider = "mlx".to_string();
                state.config.model = Some(model_id.to_string());
                state.save_config()?;

                println!();
                println!("\x1b[1m╭─────────────────────────────────────────────────────────────\x1b[0m");
                println!("\x1b[1m│\x1b[0m  \x1b[32m✓ Setup Complete!\x1b[0m");
                println!("\x1b[1m╰─────────────────────────────────────────────────────────────\x1b[0m");
                println!();
                println!("  Provider: \x1b[36mMLX\x1b[0m (Apple Silicon)");
                println!("  Model:     \x1b[36m{}\x1b[0m", model_desc);
                println!();
                println!("  Run \x1b[32mcode-buddy\x1b[0m to start chatting!");
                println!();

                return Ok(0);
            } else {
                // Ollama
                println!();
                println!("\x1b[32m✓ Great choice! Ollama runs AI locally for FREE.\x1b[0m");
                println!();

                // Check if Ollama is running
                let ollama_check = std::process::Command::new("curl")
                    .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", "http://localhost:11434/api/tags"])
                    .output();

                let models_available = match ollama_check {
                    Ok(output) => {
                        let code = String::from_utf8_lossy(&output.stdout);
                        code.trim() == "200"
                    }
                    Err(_) => false,
                };

                if models_available {
                    println!("Ollama is running. You'll use your existing models.");
                    println!();
                    state.config.llm_provider = "ollama".to_string();
                    state.config.model = Some("llama3.2".to_string());
                } else {
                    println!("Ollama is installed but not running.");
                    println!("Start it with: \x1b[33mollama serve\x1b[0m");
                    println!("Then download a model: \x1b[33mollama pull llama3.2\x1b[0m");
                    println!();
                    state.config.llm_provider = "ollama".to_string();
                    state.config.model = Some("llama3.2".to_string());
                }

                state.save_config()?;

                println!("\x1b[1m╭─────────────────────────────────────────────────────────────\x1b[0m");
                println!("\x1b[1m│\x1b[0m  \x1b[32m✓ Setup Complete!\x1b[0m");
                println!("\x1b[1m╰─────────────────────────────────────────────────────────────\x1b[0m");
                println!();
                println!("  Provider: \x1b[36mOllama\x1b[0m (Local, FREE)");
                println!();
                println!("  Run \x1b[32mcode-buddy\x1b[0m to start chatting!");
                println!();

                return Ok(0);
            }
        }
        2 => {
            // Custom provider
            println!();
            let custom: String = Input::new()
                .with_prompt("Enter provider name (e.g., azure, vertex)")
                .interact()?;
            println!();
            println!("Provider: {}", custom);
            state.config.llm_provider = custom.clone();
            state.save_config()?;
            println!();
            println!("\x1b[32m✓ Setup complete!\x1b[0m");
            println!("Run \x1b[32mcode-buddy\x1b[0m to start chatting!");
            println!();
            return Ok(0);
        }
        _ => {
            provider = "nvidia";
            models = NVIDIA_MODELS;
        }
    }

    // For cloud providers, select model
    println!();
    let model_options: Vec<String> = models
        .iter()
        .map(|(_, desc)| desc.to_string())
        .collect();

    let model_idx = Select::new()
        .with_prompt("Which AI model?")
        .items(&model_options)
        .default(0)
        .interact()?;

    let (model_id, model_desc) = models[model_idx];
    state.config.model = Some(model_id.to_string());
    state.config.llm_provider = provider.to_string();
    println!();
    println!("Selected: \x1b[36m{}\x1b[0m - {}", provider, model_desc);

    // Step 3: API key if needed
    let needs_api_key = !["ollama", "mlx"].contains(&provider);

    if needs_api_key {
        println!();
        println!("\x1b[1mStep 3:\x1b[0m API Key\n");

        // Show where to get the API key
        let key_url = match provider {
            "nvidia" => "https://ngc.nvidia.com/",
            "openrouter" => "https://openrouter.ai/keys",
            "anthropic" => "https://console.anthropic.com/",
            "openai" => "https://platform.openai.com/api-keys",
            "groq" => "https://console.groq.com/",
            "deepseek" => "https://platform.deepseek.com/",
            _ => "your provider's website",
        };

        println!("Get your API key from: \x1b[33m{}\x1b[0m", key_url);
        println!();

        let api_key: String = Input::new()
            .with_prompt("Enter API key (or press Enter to skip)")
            .allow_empty(true)
            .interact()?;

        if !api_key.is_empty() {
            state.config.api_key = Some(api_key);
            println!();
            println!("\x1b[32m✓ API key saved!\x1b[0m");
        } else {
            println!();
            println!("\x1b[33m⚠ No API key entered.\x1b[0m");
            println!("Set it later with: \x1b[32mcode-buddy --login YOUR_KEY\x1b[0m");
        }
    } else {
        println!();
        println!("\x1b[32m✓ No API key needed for local models!\x1b[0m");
    }

    // Clear base_url for default providers
    state.config.base_url = None;

    // Save configuration
    if let Err(e) = state.save_config() {
        eprintln!("Warning: Failed to save config: {}", e);
    }

    // Summary
    println!();
    println!("\x1b[1m╭─────────────────────────────────────────────────────────────\x1b[0m");
    println!("\x1b[1m│\x1b[0m  \x1b[32m✓ Setup Complete!\x1b[0m");
    println!("\x1b[1m╰─────────────────────────────────────────────────────────────\x1b[0m");
    println!();
    println!("  Provider: \x1b[36m{}\x1b[0m", provider);
    println!("  Model:    \x1b[36m{}\x1b[0m", model_desc);
    if state.config.api_key.is_some() {
        println!("  API Key:  \x1b[32m✓ Configured\x1b[0m");
    } else {
        println!("  API Key:  \x1b[33m⚠ Not set\x1b[0m - run /login to add it");
    }
    println!();
    println!("  Run \x1b[32mcode-buddy\x1b[0m to start chatting!");
    println!();

    Ok(0)
}
