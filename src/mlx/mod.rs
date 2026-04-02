//! MLX Apple Silicon Support
//!
//! This module provides native LLM inference on Apple Silicon Macs using the
//! MLX framework. It supports:
//! - Automatic Apple Silicon detection
//! - Model download from HuggingFace mlx-community
//! - Local inference using mlx-lm Python package
//! - Model management and caching

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// MLX configuration and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlxConfig {
    /// Whether MLX is available on this system
    pub available: bool,

    /// Whether MLX is the preferred provider
    pub enabled: bool,

    /// Currently loaded model (if any)
    pub loaded_model: Option<String>,

    /// Model cache directory
    pub model_dir: PathBuf,

    /// Available models in cache
    pub cached_models: Vec<String>,

    /// Python path for mlx-lm
    pub python_path: Option<String>,
}

/// Model info from HuggingFace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub size: Option<String>,
    pub description: Option<String>,
    pub downloads: Option<u64>,
}

/// Popular mlx-community models for quick access
pub const MLX_COMMUNITY_MODELS: &[(&str, &str)] = &[
    // Small efficient models
    ("mlx-community/llama-3.2-1b-instruct-4bit", "Llama 3.2 1B (4-bit, ~700MB)"),
    ("mlx-community/llama-3.2-3b-instruct-4bit", "Llama 3.2 3B (4-bit, ~2GB)"),
    ("mlx-community/Qwen2.5-0.5B-Instruct-4bit", "Qwen 2.5 0.5B (4-bit, ~400MB)"),
    ("mlx-community/Qwen2.5-1.5B-Instruct-4bit", "Qwen 2.5 1.5B (4-bit, ~1GB)"),
    ("mlx-community/gemma-2b-it-4bit", "Gemma 2B (4-bit, ~1.8GB)"),

    // Medium models
    ("mlx-community/llama-3.1-8b-instruct-4bit", "Llama 3.1 8B (4-bit, ~5GB)"),
    ("mlx-community/Qwen2.5-7B-Instruct-4bit", "Qwen 2.5 7B (4-bit, ~4GB)"),
    ("mlx-community/mistral-7b-instruct-v0.3-4bit", "Mistral 7B v0.3 (4-bit, ~4GB)"),

    // Large models
    ("mlx-community/llama-3.1-70b-instruct-4bit", "Llama 3.1 70B (4-bit, ~40GB)"),
    ("mlx-community/Qwen2.5-72B-Instruct-4bit", "Qwen 2.5 72B (4-bit, ~42GB)"),
];

impl MlxConfig {
    /// Create new MLX configuration
    pub fn new() -> Self {
        Self {
            available: false,
            enabled: false,
            loaded_model: None,
            model_dir: Self::default_model_dir(),
            cached_models: Vec::new(),
            python_path: None,
        }
    }

    /// Get default model directory
    pub fn default_model_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("~/.cache"))
            .join("mlx-models")
    }

    /// Detect if running on Apple Silicon Mac
    pub fn is_apple_silicon() -> bool {
        // Check for macOS
        if !cfg!(target_os = "macos") {
            return false;
        }

        // On macOS, check if we're running on ARM64 architecture
        // Using sysctl to check hw.optional.arm64 (1 = ARM64 available)
        if let Ok(output) = Command::new("sysctl")
            .args(["-n", "hw.optional.arm64"])
            .output()
        {
            let result = String::from_utf8_lossy(&output.stdout);
            return result.trim() == "1";
        }

        // Fallback: check uname for arm64
        if let Ok(output) = Command::new("uname")
            .args(["-m"])
            .output()
        {
            let arch = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
            return arch == "arm64";
        }

        false
    }

    /// Check if mlx-lm is installed
    pub fn check_mlx_lm_installed(&self) -> bool {
        let python = self.python_path.as_deref().unwrap_or("python3");

        Command::new(python)
            .args(["-c", "import mlx_lm"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if MLX is available on this system
    pub async fn detect(&mut self) -> Result<()> {
        self.available = Self::is_apple_silicon();

        if self.available {
            // Check if mlx-lm is installed
            if !self.check_mlx_lm_installed() {
                println!();
                println!("⚠️  MLX is available but mlx-lm is not installed.");
                println!("   Install with: pip install mlx-lm");
                println!();
            }

            // Scan for cached models
            self.scan_cached_models()?;
        }

        Ok(())
    }

    /// Scan for cached MLX models
    pub fn scan_cached_models(&mut self) -> Result<()> {
        let model_dir = &self.model_dir;

        if !model_dir.exists() {
            return Ok(());
        }

        self.cached_models = std::fs::read_dir(model_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| {
                entry.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(String::from)
            })
            .collect();

        Ok(())
    }

    /// Install mlx-lm package
    pub fn install_mlx_lm(&self) -> Result<()> {
        let python = self.python_path.as_deref().unwrap_or("python3");

        println!("Installing mlx-lm...");
        println!("This may take a few minutes...");

        let output = Command::new(python)
            .args(["-m", "pip", "install", "mlx-lm"])
            .output()
            .context("Failed to run pip install")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to install mlx-lm: {}", stderr);
        }

        println!("✓ mlx-lm installed successfully!");
        Ok(())
    }

    /// Download a model from HuggingFace
    pub async fn download_model(&self, model_id: &str) -> Result<()> {
        let python = self.python_path.as_deref().unwrap_or("python3");

        println!("Downloading model: {}", model_id);
        println!("This may take a while for large models...");

        let output = Command::new(python)
            .args(["-c", &format!("from mlx_lm.hub import download; download('{model_id}')")])
            .output()
            .context("Failed to download model")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to download model: {}", stderr);
        }

        println!("✓ Model downloaded successfully!");
        Ok(())
    }

    /// List available models from HuggingFace mlx-community
    pub async fn list_huggingface_models(&self) -> Result<Vec<ModelInfo>> {
        let client = reqwest::Client::new();

        // Fetch from HuggingFace API
        let url = "https://huggingface.co/api/models?org=mlx-community&sort=downloads&direction=-1&limit=20";

        let response = client
            .get(url)
            .header("User-Agent", "code-buddy/1.0")
            .send()
            .await
            .context("Failed to fetch models from HuggingFace")?;

        #[derive(Deserialize)]
        struct HfModel {
            id: String,
            #[serde(default)]
            downloads: Option<u64>,
        }

        let models: Vec<HfModel> = response.json().await.context("Failed to parse response")?;

        let mlx_models: Vec<ModelInfo> = models
            .into_iter()
            .map(|m| ModelInfo {
                id: m.id.clone(),
                name: m.id.replace("mlx-community/", ""),
                size: None,
                description: None,
                downloads: m.downloads,
            })
            .collect();

        Ok(mlx_models)
    }

    /// Generate text using MLX model
    pub async fn generate(&self, prompt: &str, model: &str) -> Result<String> {
        let python = self.python_path.as_deref().unwrap_or("python3");

        // Escape the prompt for Python
        let escaped_prompt = prompt
            .replace("\\", "\\\\")
            .replace("'", "\\'")
            .replace("\n", "\\n");

        let script = format!(
            r#"from mlx_lm import generate
model_path = '{}'
response = generate(model_path, prompt='{}')
print(response, end='', flush=True)
"#,
            model, escaped_prompt
        );

        let output = Command::new(python)
            .args(["-c", &script])
            .output()
            .context("Failed to generate with mlx-lm")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("MLX generation failed: {}", stderr);
        }

        let result = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(result)
    }

    /// Generate streaming response
    pub async fn generate_streaming<F>(&self, prompt: &str, model: &str, callback: F) -> Result<()>
    where
        F: Fn(&str) + Send + Sync,
    {
        let python = self.python_path.as_deref().unwrap_or("python3");

        let escaped_prompt = prompt
            .replace("\\", "\\\\")
            .replace("'", "\\'")
            .replace("\n", "\\n");

        let script = format!(
            r#"from mlx_lm import generate
model_path = '{}'
for chunk in generate(model_path, prompt='{}', stream=True):
    print(chunk, end='', flush=True)
"#,
            model, escaped_prompt
        );

        let mut child = Command::new(python)
            .args(["-c", &script])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn mlx-lm process")?;

        use std::io::{BufRead, BufReader};

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    callback(&line);
                }
            }
        }

        let status = child.wait()?;
        if !status.success() {
            anyhow::bail!("MLX generation failed with exit code: {}", status);
        }

        Ok(())
    }
}

impl Default for MlxConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Print MLX status information
pub fn print_mlx_status(config: &MlxConfig) {
    println!("=== MLX (Apple Silicon) Status ===");
    println!();

    if config.available {
        println!("✓ Running on Apple Silicon Mac");
        println!();

        if config.check_mlx_lm_installed() {
            println!("✓ mlx-lm installed");
        } else {
            println!("✗ mlx-lm not installed");
            println!("  Install with: pip install mlx-lm");
        }

        println!();
        println!("Model directory: {}", config.model_dir.display());

        if !config.cached_models.is_empty() {
            println!();
            println!("Cached models:");
            for model in &config.cached_models {
                println!("  - {}", model);
            }
        }

        if let Some(model) = &config.loaded_model {
            println!();
            println!("Currently loaded: {}", model);
        }
    } else {
        println!("✗ Not running on Apple Silicon Mac");
        println!();
        println!("MLX provides native LLM inference on Apple Silicon.");
        println!("This feature is only available on M1/M2/M3/M4 Macs.");
    }
    println!();
}

/// Interactive model selection
pub async fn interactive_model_setup(config: &MlxConfig) -> Result<Option<String>> {
    use dialoguer::Select;

    if !config.available {
        println!("MLX is only available on Apple Silicon Macs.");
        return Ok(None);
    }

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           MLX Model Setup (Apple Silicon)                    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Check if mlx-lm is installed
    if !config.check_mlx_lm_installed() {
        println!("mlx-lm is not installed. Would you like to install it?");
        println!();

        let install = dialoguer::Confirm::new()
            .with_prompt("Install mlx-lm?")
            .default(true)
            .interact()?;

        if install {
            config.install_mlx_lm()?;
        } else {
            println!("Skipping mlx-lm installation.");
            return Ok(None);
        }
    }

    // Show popular models
    println!();
    println!("Popular MLX models:");
    println!();

    let model_descriptions: Vec<String> = MLX_COMMUNITY_MODELS
        .iter()
        .map(|(_, desc)| desc.to_string())
        .collect();

    let selection = Select::new()
        .with_prompt("Select a model to download (or enter custom ID)")
        .items(&model_descriptions)
        .default(2) // Default to 3B model
        .interact()?;

    let (model_id, _) = MLX_COMMUNITY_MODELS[selection];

    println!();
    println!("Selected: {}", model_id);

    // Download the model
    config.download_model(model_id).await?;

    Ok(Some(model_id.to_string()))
}

/// Check if mlx-lm should be suggested based on platform
pub fn should_suggest_mlx() -> bool {
    MlxConfig::is_apple_silicon()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mlx_config_default() {
        let config = MlxConfig::new();
        assert!(!config.available);
        assert!(!config.enabled);
        assert!(config.loaded_model.is_none());
        assert!(config.cached_models.is_empty());
    }

    #[test]
    fn test_default_model_dir() {
        let dir = MlxConfig::default_model_dir();
        assert!(dir.to_string_lossy().contains("mlx-models"));
    }

    #[test]
    fn test_mlx_community_models() {
        assert!(!MLX_COMMUNITY_MODELS.is_empty());
        for (id, name) in MLX_COMMUNITY_MODELS {
            assert!(id.starts_with("mlx-community/"));
            assert!(!name.is_empty());
        }
    }
}
