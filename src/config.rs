//! Configuration management

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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

    /// Allowed directories for file operations
    #[serde(default)]
    pub allowed_directories: Vec<PathBuf>,

    /// Max tokens for API requests
    pub max_tokens: Option<u32>,

    /// Temperature for API requests
    pub temperature: Option<f32>,

    /// Custom system prompt
    pub system_prompt: Option<String>,

    /// Conversation window size
    pub conversation_window: Option<usize>,

    /// Auto-compact enabled (auto-summarize long conversations)
    #[serde(default)]
    pub auto_compact: bool,

    /// Compact threshold as percentage of context window (default: 85)
    #[serde(default = "default_compact_threshold")]
    pub compact_threshold: u8,

    /// Maximum messages to keep after compaction
    #[serde(default = "default_compact_messages")]
    pub compact_messages: usize,

    /// Debug mode
    #[serde(default)]
    pub debug: bool,

    /// Verbose output
    #[serde(default)]
    pub verbose: bool,

    /// Streaming mode
    #[serde(default = "default_true")]
    pub streaming: bool,

    /// Disable color output
    #[serde(default)]
    pub no_color: bool,

    /// JSON output mode
    #[serde(default)]
    pub json: bool,

    /// Skip SSL verification
    #[serde(default)]
    pub insecure_ssl: bool,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub request_timeout_seconds: u64,

    /// Max retries for API requests
    #[serde(default = "default_retries")]
    pub max_retries: u32,

    /// Theme preference (dark, light, or null for system)
    #[serde(default)]
    pub theme: Option<String>,

    /// First run flag - show theme selection on first launch
    #[serde(default)]
    pub first_run: bool,
}

fn default_provider() -> String {
    "anthropic".to_string()
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    120
}

fn default_retries() -> u32 {
    3
}

fn default_compact_threshold() -> u8 {
    85
}

fn default_compact_messages() -> usize {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    pub original_messages: usize,
    pub compacted_messages: usize,
    pub summary: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
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
            allowed_directories: Vec::new(),
            max_tokens: std::env::var("MAX_TOKENS").ok().and_then(|s| s.parse().ok()),
            temperature: std::env::var("TEMPERATURE").ok().and_then(|s| s.parse().ok()),
            system_prompt: std::env::var("SYSTEM_PROMPT").ok(),
            conversation_window: std::env::var("CONVERSATION_WINDOW").ok().and_then(|s| s.parse().ok()),
            auto_compact: std::env::var("AUTO_COMPACT")
                .map(|s| s == "true" || s == "1")
                .unwrap_or(true),
            compact_threshold: std::env::var("COMPACT_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(85),
            compact_messages: std::env::var("COMPACT_MESSAGES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),
            debug: std::env::var("DEBUG").map(|s| s == "true" || s == "1").unwrap_or(false),
            verbose: false,
            streaming: true,
            no_color: std::env::var("NO_COLOR").map(|s| s == "true" || s == "1").unwrap_or(false),
            json: false,
            insecure_ssl: false,
            request_timeout_seconds: std::env::var("REQUEST_TIMEOUT_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(120),
            max_retries: std::env::var("MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            theme: std::env::var("CODE_BUDDY_THEME").ok(),
            first_run: true,
        }
    }
}

impl Config {
    /// Create configuration from environment variables
    pub fn from_env() -> Self {
        // Check for explicit API keys in priority order
        let api_key = if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            Some(key)
        } else if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            Some(key)
        } else if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
            Some(key)
        } else if let Ok(key) = std::env::var("NVIDIA_API_KEY") {
            Some(key)
        } else if let Ok(key) = std::env::var("GROQ_API_KEY") {
            Some(key)
        } else if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
            Some(key)
        } else {
            std::env::var("TOGETHER_API_KEY").ok()
        };

        // Determine provider from explicit key or LLM_PROVIDER env var
        let llm_provider = if api_key.is_some() {
            if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                "anthropic".to_string()
            } else if std::env::var("OPENAI_API_KEY").is_ok() {
                "openai".to_string()
            } else if std::env::var("OPENROUTER_API_KEY").is_ok() {
                "openrouter".to_string()
            } else if std::env::var("NVIDIA_API_KEY").is_ok() {
                "nvidia".to_string()
            } else if std::env::var("GROQ_API_KEY").is_ok() {
                "groq".to_string()
            } else if std::env::var("DEEPSEEK_API_KEY").is_ok() {
                "deepseek".to_string()
            } else if std::env::var("TOGETHER_API_KEY").is_ok() {
                "together".to_string()
            } else {
                std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string())
            }
        } else {
            std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string())
        };

        Self {
            api_key,
            llm_provider,
            model: std::env::var("ANTHROPIC_MODEL").ok(),
            base_url: std::env::var("ANTHROPIC_BASE_URL").ok(),
            permission_mode: std::env::var("PERMISSION_MODE").ok(),
            additional_dirs: Vec::new(),
            mcp_servers: HashMap::new(),
            agents: HashMap::new(),
            project_choices: HashMap::new(),
            config_path: None,
            session_history: Vec::new(),
            allowed_directories: Vec::new(),
            max_tokens: std::env::var("MAX_TOKENS").ok().and_then(|s| s.parse().ok()),
            temperature: std::env::var("TEMPERATURE").ok().and_then(|s| s.parse().ok()),
            system_prompt: std::env::var("SYSTEM_PROMPT").ok(),
            conversation_window: std::env::var("CONVERSATION_WINDOW").ok().and_then(|s| s.parse().ok()),
            auto_compact: std::env::var("AUTO_COMPACT")
                .map(|s| s == "true" || s == "1")
                .unwrap_or(true),
            compact_threshold: std::env::var("COMPACT_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(85),
            compact_messages: std::env::var("COMPACT_MESSAGES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),
            debug: std::env::var("DEBUG").map(|s| s == "true" || s == "1").unwrap_or(false),
            verbose: std::env::var("VERBOSE").map(|s| s == "true" || s == "1").unwrap_or(false),
            streaming: true,
            no_color: std::env::var("NO_COLOR").map(|s| s == "true" || s == "1").unwrap_or(false),
            json: false,
            insecure_ssl: false,
            request_timeout_seconds: std::env::var("REQUEST_TIMEOUT_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(120),
            max_retries: std::env::var("MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            theme: std::env::var("CODE_BUDDY_THEME").ok(),
            first_run: true,
        }
    }

    /// Add an allowed directory
    pub fn add_allowed_dir(&mut self, dir: impl Into<PathBuf>) {
        self.allowed_directories.push(dir.into());
    }

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

    /// Get config directory (uses ~/.codebuddy to avoid Claude Code conflicts)
    pub fn config_dir() -> Result<PathBuf> {
        // Use ~/.codebuddy on Linux, ~/Library/Application Support/codebuddy on macOS
        let dir = if cfg!(target_os = "macos") {
            dirs::home_dir()
                .context("Could not find home directory")?
                .join("Library/Application Support/codebuddy")
        } else {
            dirs::home_dir()
                .context("Could not find home directory")?
                .join(".codebuddy")
        };

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
            .unwrap_or_else(|| Self::default_config_path()
                .unwrap_or_else(|e| {
                    // Fallback to a safe default path if config_dir is not available
                    // This should rarely happen on any reasonable system
                    eprintln!("Warning: Could not determine config directory: {}", e);
                    PathBuf::from("~/.code-buddy/config.json")
                })
            );

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(&path, content)
            .context("Failed to write config file")?;

        Ok(())
    }

    /// Get the cache directory (uses codebuddy to avoid Claude Code conflicts)
    pub fn cache_dir() -> Result<PathBuf> {
        let dir = if cfg!(target_os = "macos") {
            dirs::home_dir()
                .context("Could not find home directory")?
                .join("Library/Caches/codebuddy")
        } else {
            dirs::cache_dir()
                .context("Could not find cache directory")?
                .join("codebuddy")
        };

        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        Ok(dir)
    }

    /// Get the data directory (uses codebuddy to avoid Claude Code conflicts)
    pub fn data_dir() -> Result<PathBuf> {
        let dir = if cfg!(target_os = "macos") {
            dirs::home_dir()
                .context("Could not find home directory")?
                .join("Library/Application Support/codebuddy")
        } else {
            dirs::data_dir()
                .context("Could not find data directory")?
                .join("codebuddy")
        };

        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        Ok(dir)
    }
}

