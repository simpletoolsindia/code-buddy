//! Configuration management

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// API key for authentication
    pub api_key: Option<String>,

    /// LLM provider (anthropic, openai, openrouter, etc.)
    #[serde(default = "default_provider")]
    pub llm_provider: String,

    /// Model to use
    pub model: Option<String>,

    /// Custom API base URL
    pub base_url: Option<String>,

    /// Permission mode
    #[serde(default)]
    pub permission_mode: Option<String>,

    /// Additional directories to allow
    #[serde(default)]
    pub additional_dirs: Vec<PathBuf>,

    /// MCP server configurations
    #[serde(default)]
    pub mcp_servers: HashMap<String, serde_json::Value>,

    /// Custom agents
    #[serde(default)]
    pub agents: HashMap<String, AgentConfig>,

    /// Project-specific choices
    #[serde(default)]
    pub project_choices: HashMap<String, serde_json::Value>,

    /// Config file path
    #[serde(skip)]
    pub config_path: Option<PathBuf>,

    /// Session history
    #[serde(default)]
    pub session_history: Vec<Message>,
}

fn default_provider() -> String {
    "anthropic".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub model: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let llm_provider = if let Ok(val) = std::env::var("LLM_PROVIDER") {
            val
        } else if let Ok(val) = std::env::var("ANTHROPIC_LLM_PROVIDER") {
            val
        } else {
            "anthropic".to_string()
        };

        Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            llm_provider,
            model: std::env::var("ANTHROPIC_MODEL").ok(),
            base_url: std::env::var("ANTHROPIC_BASE_URL").ok(),
            permission_mode: None,
            additional_dirs: Vec::new(),
            mcp_servers: HashMap::new(),
            agents: HashMap::new(),
            project_choices: HashMap::new(),
            config_path: None,
            session_history: Vec::new(),
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let config_path = Self::find_config_file()?;

        if let Some(path) = config_path {
            let content = fs::read_to_string(&path)
                .context("Failed to read config file")?;

            let mut config: Config = serde_json::from_str(&content)
                .context("Failed to parse config file")?;

            config.config_path = Some(path);
            Ok(config)
        } else {
            // Create default config
            let mut config = Self::default();
            config.config_path = Some(Self::default_config_path()?);
            Ok(config)
        }
    }

    /// Find the config file
    fn find_config_file() -> Result<Option<PathBuf>> {
        let paths = vec![
            Self::config_dir()?.join("config.json"),
            Self::config_dir()?.join("config.local.json"),
        ];

        for path in paths {
            if path.exists() {
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    /// Get config directory
    pub fn config_dir() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("code-buddy");

        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        Ok(dir)
    }

    /// Get default config file path
    fn default_config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.json"))
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let path = self.config_path.clone()
            .unwrap_or_else(|| Self::default_config_path().unwrap());

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(&path, content)
            .context("Failed to write config file")?;

        Ok(())
    }

    /// Get the cache directory
    pub fn cache_dir() -> Result<PathBuf> {
        let dir = dirs::cache_dir()
            .context("Could not find cache directory")?
            .join("code-buddy");

        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        Ok(dir)
    }

    /// Get the data directory
    pub fn data_dir() -> Result<PathBuf> {
        let dir = dirs::data_dir()
            .context("Could not find data directory")?
            .join("code-buddy");

        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        Ok(dir)
    }
}
