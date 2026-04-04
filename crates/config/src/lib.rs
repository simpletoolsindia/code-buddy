//! Configuration management for Code Buddy.
//!
//! Config is loaded from `~/.config/code-buddy/config.toml` (or
//! `$XDG_CONFIG_HOME/code-buddy/config.toml`). Environment variables
//! override file values. All fields are validated on load.

use code_buddy_errors::ConfigError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::debug;

/// The name of the config directory and file.
const APP_NAME: &str = "code-buddy";
const CONFIG_FILE: &str = "config.toml";

/// Full application configuration.
///
/// TOML keys match field names exactly. Environment variables follow
/// the pattern `CODE_BUDDY_<FIELD_NAME_UPPER>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// LLM provider identifier.
    /// Valid values: `lm-studio`, `openrouter`, `nvidia`, `openai`, `custom`.
    pub provider: String,

    /// Model name or identifier to request.
    pub model: Option<String>,

    /// Base URL for the provider API endpoint.
    /// Defaults to the standard URL for the selected provider.
    pub endpoint: Option<String>,

    /// API key for authenticated providers.
    /// For local providers like `lm-studio`, this is optional.
    /// Environment override: `CODE_BUDDY_API_KEY`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Request timeout in seconds.
    /// TOML alias: `timeout`
    #[serde(alias = "timeout")]
    pub timeout_seconds: u64,

    /// Maximum number of automatic retries on transient errors.
    /// TOML alias: `retries`
    #[serde(alias = "retries")]
    pub max_retries: u32,

    /// Enable debug-level logging.
    pub debug: bool,

    /// Enable streaming responses (token-by-token output).
    pub streaming: bool,

    /// Maximum tokens for model output.
    pub max_tokens: Option<u32>,

    /// Sampling temperature (0.0–2.0).
    pub temperature: Option<f32>,

    /// Custom system prompt prepended to every conversation.
    pub system_prompt: Option<String>,

    /// Disable ANSI color output.
    pub no_color: bool,

    /// Show verbose request/response diagnostics.
    pub verbose: bool,

    /// Brave Search API key for the `web_search` tool.
    /// Environment override: `BRAVE_SEARCH_API_KEY`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brave_api_key: Option<String>,

    /// SerpAPI key (fallback for web search when Brave key is absent).
    /// Environment override: `SERPAPI_KEY`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serpapi_key: Option<String>,

    /// Firecrawl API key for high-quality `web_fetch` output.
    /// Environment override: `FIRECRAWL_API_KEY`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firecrawl_api_key: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider: "lm-studio".to_string(),
            model: None,
            endpoint: None,
            api_key: None,
            timeout_seconds: 120,
            max_retries: 3,
            debug: false,
            streaming: true,
            max_tokens: None,
            temperature: None,
            system_prompt: None,
            no_color: false,
            verbose: false,
            brave_api_key: None,
            serpapi_key: None,
            firecrawl_api_key: None,
        }
    }
}

impl AppConfig {
    /// Load configuration from the default file path, then apply environment overrides.
    ///
    /// This is the primary entry point for startup. It:
    /// 1. Resolves the config file path.
    /// 2. Reads and parses the TOML file (if it exists; missing file is not an error).
    /// 3. Applies environment variable overrides.
    /// 4. Validates the resulting config.
    ///
    /// # Errors
    /// Returns [`ConfigError`] if the file cannot be read or parsed, or if validation fails.
    pub fn load() -> Result<Self, ConfigError> {
        let path = config_file_path()?;
        Self::load_from(&path)
    }

    /// Load configuration from a specific file path.
    ///
    /// # Errors
    /// Returns [`ConfigError`] if reading or parsing fails.
    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        let mut config = if path.exists() {
            debug!("Loading config from {}", path.display());
            let content =
                std::fs::read_to_string(path).map_err(|e| ConfigError::Read {
                    path: path.display().to_string(),
                    source: e,
                })?;
            toml::from_str::<Self>(&content).map_err(|e| ConfigError::Parse {
                path: path.display().to_string(),
                source: Box::new(e),
            })?
        } else {
            debug!(
                "Config file not found at {}, using defaults",
                path.display()
            );
            Self::default()
        };

        config.apply_env_overrides();
        config.validate()?;

        Ok(config)
    }

    /// Save the config to the default file path.
    ///
    /// Creates the config directory if it does not exist.
    ///
    /// # Errors
    /// Returns [`ConfigError`] if the directory cannot be created or the file cannot be written.
    pub fn save(&self) -> Result<std::path::PathBuf, ConfigError> {
        let path = config_file_path()?;
        self.save_to(&path)?;
        Ok(path)
    }

    /// Save the config to a specific file path.
    ///
    /// # Errors
    /// Returns [`ConfigError`] if the file cannot be written.
    pub fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::Write {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| ConfigError::Validation {
            field: "(serialization)".to_string(),
            reason: e.to_string(),
        })?;
        std::fs::write(path, content).map_err(|e| ConfigError::Write {
            path: path.display().to_string(),
            source: e,
        })?;
        debug!("Config saved to {}", path.display());
        Ok(())
    }

    /// Apply environment variable overrides to the current config.
    ///
    /// Environment variables take precedence over file values.
    /// All config fields are overridable via `CODE_BUDDY_<FIELD_UPPER>`.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("CODE_BUDDY_API_KEY") {
            if !val.is_empty() {
                self.api_key = Some(val);
            }
        }
        if let Ok(val) = std::env::var("CODE_BUDDY_PROVIDER") {
            if !val.is_empty() {
                self.provider = val;
            }
        }
        if let Ok(val) = std::env::var("CODE_BUDDY_MODEL") {
            if !val.is_empty() {
                self.model = Some(val);
            }
        }
        if let Ok(val) = std::env::var("CODE_BUDDY_ENDPOINT") {
            if !val.is_empty() {
                self.endpoint = Some(val);
            }
        }
        if let Ok(val) = std::env::var("CODE_BUDDY_DEBUG") {
            self.debug = matches!(val.to_lowercase().as_str(), "1" | "true" | "yes");
        }
        if let Ok(val) = std::env::var("CODE_BUDDY_NO_COLOR") {
            self.no_color = matches!(val.to_lowercase().as_str(), "1" | "true" | "yes");
        }
        if let Ok(val) = std::env::var("CODE_BUDDY_STREAMING") {
            self.streaming = matches!(val.to_lowercase().as_str(), "1" | "true" | "yes");
        }
        if let Ok(val) = std::env::var("CODE_BUDDY_VERBOSE") {
            self.verbose = matches!(val.to_lowercase().as_str(), "1" | "true" | "yes");
        }
        if let Ok(val) = std::env::var("CODE_BUDDY_TIMEOUT_SECONDS") {
            if let Ok(v) = val.parse::<u64>() {
                self.timeout_seconds = v;
            }
        }
        if let Ok(val) = std::env::var("CODE_BUDDY_MAX_RETRIES") {
            if let Ok(v) = val.parse::<u32>() {
                self.max_retries = v;
            }
        }
        if let Ok(val) = std::env::var("CODE_BUDDY_MAX_TOKENS") {
            if let Ok(v) = val.parse::<u32>() {
                self.max_tokens = Some(v);
            }
        }
        if let Ok(val) = std::env::var("CODE_BUDDY_TEMPERATURE") {
            if let Ok(v) = val.parse::<f32>() {
                self.temperature = Some(v);
            }
        }
        if let Ok(val) = std::env::var("CODE_BUDDY_SYSTEM_PROMPT") {
            if !val.is_empty() {
                self.system_prompt = Some(val);
            }
        }
        if let Ok(val) = std::env::var("BRAVE_SEARCH_API_KEY") {
            if !val.is_empty() {
                self.brave_api_key = Some(val);
            }
        }
        if let Ok(val) = std::env::var("SERPAPI_KEY") {
            if !val.is_empty() {
                self.serpapi_key = Some(val);
            }
        }
        if let Ok(val) = std::env::var("FIRECRAWL_API_KEY") {
            if !val.is_empty() {
                self.firecrawl_api_key = Some(val);
            }
        }
    }

    /// Validate all configuration fields.
    ///
    /// # Errors
    /// Returns [`ConfigError::Validation`] if any field has an invalid value.
    pub fn validate(&self) -> Result<(), ConfigError> {
        validate_provider(&self.provider)?;

        if self.timeout_seconds == 0 {
            return Err(ConfigError::Validation {
                field: "timeout_seconds".to_string(),
                reason: "must be greater than 0".to_string(),
            });
        }

        if self.max_retries > 10 {
            return Err(ConfigError::Validation {
                field: "max_retries".to_string(),
                reason: "must be 10 or less".to_string(),
            });
        }

        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(ConfigError::Validation {
                    field: "temperature".to_string(),
                    reason: format!("must be between 0.0 and 2.0, got {temp}"),
                });
            }
        }

        Ok(())
    }

    /// Return the default endpoint URL for the configured provider.
    #[must_use]
    pub fn default_endpoint(&self) -> &'static str {
        match self.provider.as_str() {
            "lm-studio" => "http://localhost:1234/v1",
            "openrouter" => "https://openrouter.ai/api/v1",
            "nvidia" => "https://integrate.api.nvidia.com/v1",
            "openai" => "https://api.openai.com/v1",
            _ => "http://localhost:1234/v1",
        }
    }

    /// Return the resolved endpoint (config value or provider default).
    #[must_use]
    pub fn resolved_endpoint(&self) -> String {
        self.endpoint
            .clone()
            .unwrap_or_else(|| self.default_endpoint().to_string())
    }

    /// Set a config field by name. Used by the `config set` CLI command.
    ///
    /// # Errors
    /// Returns [`ConfigError::Validation`] if the field name is unknown or the value is invalid.
    pub fn set_field(&mut self, field: &str, value: &str) -> Result<(), ConfigError> {
        match field {
            "provider" => {
                validate_provider(value)?;
                self.provider = value.to_string();
            }
            "model" => {
                self.model = if value.is_empty() { None } else { Some(value.to_string()) };
            }
            "endpoint" => {
                self.endpoint = if value.is_empty() { None } else { Some(value.to_string()) };
            }
            "api_key" => {
                self.api_key = if value.is_empty() { None } else { Some(value.to_string()) };
            }
            "timeout_seconds" => {
                let v: u64 = value.parse().map_err(|_| ConfigError::Validation {
                    field: field.to_string(),
                    reason: format!("expected an integer, got '{value}'"),
                })?;
                self.timeout_seconds = v;
            }
            "max_retries" => {
                let v: u32 = value.parse().map_err(|_| ConfigError::Validation {
                    field: field.to_string(),
                    reason: format!("expected an integer, got '{value}'"),
                })?;
                self.max_retries = v;
            }
            "debug" => {
                self.debug = parse_bool(value, field)?;
            }
            "streaming" => {
                self.streaming = parse_bool(value, field)?;
            }
            "no_color" => {
                self.no_color = parse_bool(value, field)?;
            }
            "max_tokens" => {
                self.max_tokens = if value.is_empty() {
                    None
                } else {
                    let v: u32 = value.parse().map_err(|_| ConfigError::Validation {
                        field: field.to_string(),
                        reason: format!("expected an integer, got '{value}'"),
                    })?;
                    Some(v)
                };
            }
            "temperature" => {
                self.temperature = if value.is_empty() {
                    None
                } else {
                    let v: f32 = value.parse().map_err(|_| ConfigError::Validation {
                        field: field.to_string(),
                        reason: format!("expected a float, got '{value}'"),
                    })?;
                    if !(0.0..=2.0).contains(&v) {
                        return Err(ConfigError::Validation {
                            field: field.to_string(),
                            reason: format!("must be between 0.0 and 2.0, got {v}"),
                        });
                    }
                    Some(v)
                };
            }
            "system_prompt" => {
                self.system_prompt = if value.is_empty() { None } else { Some(value.to_string()) };
            }
            "brave_api_key" => {
                self.brave_api_key = if value.is_empty() { None } else { Some(value.to_string()) };
            }
            "serpapi_key" => {
                self.serpapi_key = if value.is_empty() { None } else { Some(value.to_string()) };
            }
            "firecrawl_api_key" => {
                self.firecrawl_api_key = if value.is_empty() { None } else { Some(value.to_string()) };
            }
            _ => {
                return Err(ConfigError::Validation {
                    field: field.to_string(),
                    reason: format!(
                        "unknown field '{field}'. Valid fields: provider, model, endpoint, api_key, \
                         timeout_seconds, max_retries, debug, streaming, no_color, max_tokens, \
                         temperature, system_prompt, brave_api_key, serpapi_key, firecrawl_api_key"
                    ),
                });
            }
        }
        Ok(())
    }

    /// Get a config field value as a string. Used by the `config get` CLI command.
    #[must_use]
    pub fn get_field(&self, field: &str) -> Option<String> {
        match field {
            "provider" => Some(self.provider.clone()),
            "model" => self.model.clone(),
            "endpoint" => self.endpoint.clone(),
            "api_key" => self.api_key.as_deref().map(|_| "<redacted>".to_string()),
            "timeout_seconds" => Some(self.timeout_seconds.to_string()),
            "max_retries" => Some(self.max_retries.to_string()),
            "debug" => Some(self.debug.to_string()),
            "streaming" => Some(self.streaming.to_string()),
            "no_color" => Some(self.no_color.to_string()),
            "max_tokens" => self.max_tokens.map(|v| v.to_string()),
            "temperature" => self.temperature.map(|v| format!("{v:.2}")),
            "system_prompt" => self.system_prompt.clone(),
            "brave_api_key" => self.brave_api_key.as_deref().map(|_| "<redacted>".to_string()),
            "serpapi_key" => self.serpapi_key.as_deref().map(|_| "<redacted>".to_string()),
            "firecrawl_api_key" => self.firecrawl_api_key.as_deref().map(|_| "<redacted>".to_string()),
            _ => None,
        }
    }
}

/// Resolve the config file path.
fn config_file_path() -> Result<PathBuf, ConfigError> {
    let config_dir = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
    Ok(config_dir.join(APP_NAME).join(CONFIG_FILE))
}

/// Validate the provider name.
fn validate_provider(provider: &str) -> Result<(), ConfigError> {
    match provider {
        "lm-studio" | "ollama" | "openrouter" | "nvidia" | "openai" | "custom" => Ok(()),
        _ => Err(ConfigError::Validation {
            field: "provider".to_string(),
            reason: format!(
                "unknown provider '{provider}'. Valid: lm-studio, ollama, openrouter, nvidia, openai, custom"
            ),
        }),
    }
}

/// Parse a boolean from user-provided string.
fn parse_bool(value: &str, field: &str) -> Result<bool, ConfigError> {
    match value.to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(ConfigError::Validation {
            field: field.to_string(),
            reason: format!("expected true/false, got '{value}'"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_config(content: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    #[test]
    fn default_config_is_valid() {
        let config = AppConfig::default();
        config.validate().expect("default config should be valid");
    }

    #[test]
    fn load_valid_toml() {
        let file = write_temp_config(
            r#"
provider = "openrouter"
model = "mistralai/mistral-7b"
timeout_seconds = 60
streaming = true
"#,
        );
        let config = AppConfig::load_from(file.path()).expect("should load");
        assert_eq!(config.provider, "openrouter");
        assert_eq!(config.model.as_deref(), Some("mistralai/mistral-7b"));
        assert_eq!(config.timeout_seconds, 60);
    }

    #[test]
    fn load_toml_short_aliases() {
        let file = write_temp_config(
            r#"
provider = "openai"
timeout = 45
retries = 2
"#,
        );
        let config = AppConfig::load_from(file.path()).expect("short alias keys should load");
        assert_eq!(config.timeout_seconds, 45, "timeout alias should set timeout_seconds");
        assert_eq!(config.max_retries, 2, "retries alias should set max_retries");
    }

    #[test]
    fn load_missing_file_uses_defaults() {
        let path = PathBuf::from("/tmp/code-buddy-nonexistent-test-config.toml");
        let config = AppConfig::load_from(&path).expect("missing file should use defaults");
        assert_eq!(config.provider, "lm-studio");
    }

    #[test]
    fn invalid_provider_fails_validation() {
        let file = write_temp_config(r#"provider = "unknown-provider""#);
        let result = AppConfig::load_from(file.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unknown provider"));
    }

    #[test]
    fn invalid_temperature_fails_validation() {
        let file = write_temp_config(r#"temperature = 5.0"#);
        let result = AppConfig::load_from(file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("temperature"));
    }

    #[test]
    fn set_field_provider() {
        let mut config = AppConfig::default();
        config.set_field("provider", "openai").unwrap();
        assert_eq!(config.provider, "openai");
    }

    #[test]
    fn set_field_unknown_fails() {
        let mut config = AppConfig::default();
        let result = config.set_field("nonexistent_field", "value");
        assert!(result.is_err());
    }

    #[test]
    fn get_field_api_key_is_redacted() {
        let mut config = AppConfig::default();
        config.api_key = Some("sk-supersecret123".to_string());
        let val = config.get_field("api_key").unwrap();
        assert_eq!(val, "<redacted>");
    }

    #[test]
    fn resolved_endpoint_uses_default_for_lm_studio() {
        let config = AppConfig::default();
        assert_eq!(config.resolved_endpoint(), "http://localhost:1234/v1");
    }

    #[test]
    fn resolved_endpoint_uses_config_value() {
        let mut config = AppConfig::default();
        config.endpoint = Some("http://localhost:8080/v1".to_string());
        assert_eq!(config.resolved_endpoint(), "http://localhost:8080/v1");
    }

    #[test]
    fn env_override_api_key() {
        std::env::set_var("CODE_BUDDY_API_KEY", "env-test-key");
        let mut config = AppConfig::default();
        config.apply_env_overrides();
        assert_eq!(config.api_key.as_deref(), Some("env-test-key"));
        std::env::remove_var("CODE_BUDDY_API_KEY");
    }

    #[test]
    fn env_override_timeout_seconds() {
        std::env::set_var("CODE_BUDDY_TIMEOUT_SECONDS", "120");
        let mut config = AppConfig::default();
        config.apply_env_overrides();
        assert_eq!(config.timeout_seconds, 120);
        std::env::remove_var("CODE_BUDDY_TIMEOUT_SECONDS");
    }

    #[test]
    fn env_override_max_retries() {
        std::env::set_var("CODE_BUDDY_MAX_RETRIES", "5");
        let mut config = AppConfig::default();
        config.apply_env_overrides();
        assert_eq!(config.max_retries, 5);
        std::env::remove_var("CODE_BUDDY_MAX_RETRIES");
    }

    #[test]
    fn env_override_max_tokens() {
        std::env::set_var("CODE_BUDDY_MAX_TOKENS", "2048");
        let mut config = AppConfig::default();
        config.apply_env_overrides();
        assert_eq!(config.max_tokens, Some(2048));
        std::env::remove_var("CODE_BUDDY_MAX_TOKENS");
    }

    #[test]
    fn env_override_temperature() {
        std::env::set_var("CODE_BUDDY_TEMPERATURE", "0.5");
        let mut config = AppConfig::default();
        config.apply_env_overrides();
        assert!((config.temperature.unwrap() - 0.5_f32).abs() < 1e-6);
        std::env::remove_var("CODE_BUDDY_TEMPERATURE");
    }

    #[test]
    fn env_override_system_prompt() {
        std::env::set_var("CODE_BUDDY_SYSTEM_PROMPT", "You are a helpful assistant.");
        let mut config = AppConfig::default();
        config.apply_env_overrides();
        assert_eq!(
            config.system_prompt.as_deref(),
            Some("You are a helpful assistant.")
        );
        std::env::remove_var("CODE_BUDDY_SYSTEM_PROMPT");
    }

    #[test]
    fn env_override_streaming() {
        std::env::set_var("CODE_BUDDY_STREAMING", "false");
        let mut config = AppConfig::default();
        config.apply_env_overrides();
        assert!(!config.streaming);
        std::env::remove_var("CODE_BUDDY_STREAMING");
    }

    #[test]
    fn env_override_invalid_timeout_ignored() {
        let mut config = AppConfig::default();
        let original_timeout = config.timeout_seconds;
        std::env::set_var("CODE_BUDDY_TIMEOUT_SECONDS", "not-a-number");
        config.apply_env_overrides();
        assert_eq!(config.timeout_seconds, original_timeout);
        std::env::remove_var("CODE_BUDDY_TIMEOUT_SECONDS");
    }

    #[test]
    fn save_and_reload_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut original = AppConfig::default();
        original.provider = "openai".to_string();
        original.model = Some("gpt-4o".to_string());
        original.timeout_seconds = 45;
        original.save_to(&path).unwrap();

        let loaded = AppConfig::load_from(&path).unwrap();
        assert_eq!(loaded.provider, "openai");
        assert_eq!(loaded.model.as_deref(), Some("gpt-4o"));
        assert_eq!(loaded.timeout_seconds, 45);
    }
}
