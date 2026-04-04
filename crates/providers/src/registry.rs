//! Provider registry: selects and constructs the correct adapter from config.
//!
//! The registry is the single point where `provider` config values are mapped to
//! concrete [`Provider`] trait objects. No provider-specific branching should
//! appear outside this module.

use std::time::Duration;

use code_buddy_config::AppConfig;
use code_buddy_errors::TransportError;
use code_buddy_transport::Provider;

use crate::openai_compat::{AdapterConfig, OpenAiCompatAdapter};

/// Constructs provider adapters from configuration.
pub struct ProviderRegistry;

impl ProviderRegistry {
    /// Build the correct adapter from the given [`AppConfig`].
    ///
    /// # Errors
    /// Returns [`TransportError::MissingCredentials`] if a required API key is absent.
    pub fn from_config(config: &AppConfig) -> Result<Box<dyn Provider>, TransportError> {
        let timeout = Duration::from_secs(config.timeout_seconds);
        let max_retries = config.max_retries;
        let endpoint = config.endpoint.as_deref();

        let adapter_config = match config.provider.as_str() {
            "lm-studio" => {
                let base_url = endpoint
                    .unwrap_or("http://localhost:1234/v1")
                    .to_string();
                AdapterConfig::lm_studio()
                    .with_base_url_override(base_url)
                    .with_timeout(timeout)
                    .with_max_retries(max_retries)
            }
            "openrouter" => {
                let api_key = config
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("OPENROUTER_API_KEY").ok())
                    .filter(|k| !k.is_empty())
                    .ok_or_else(|| TransportError::MissingCredentials {
                        provider: "OpenRouter".to_string(),
                        env_var: "OPENROUTER_API_KEY".to_string(),
                    })?;
                let mut cfg = AdapterConfig::openrouter(api_key)
                    .with_timeout(timeout)
                    .with_max_retries(max_retries);
                if let Some(url) = endpoint {
                    cfg = cfg.with_base_url_override(url.to_string());
                }
                cfg
            }
            "nvidia" => {
                let api_key = config
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("NVIDIA_API_KEY").ok())
                    .filter(|k| !k.is_empty())
                    .ok_or_else(|| TransportError::MissingCredentials {
                        provider: "NVIDIA".to_string(),
                        env_var: "NVIDIA_API_KEY".to_string(),
                    })?;
                let mut cfg = AdapterConfig::nvidia(api_key)
                    .with_timeout(timeout)
                    .with_max_retries(max_retries);
                if let Some(url) = endpoint {
                    cfg = cfg.with_base_url_override(url.to_string());
                }
                cfg
            }
            "openai" => {
                let api_key = config
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                    .filter(|k| !k.is_empty())
                    .ok_or_else(|| TransportError::MissingCredentials {
                        provider: "OpenAI".to_string(),
                        env_var: "OPENAI_API_KEY".to_string(),
                    })?;
                let mut cfg = AdapterConfig::openai(api_key)
                    .with_timeout(timeout)
                    .with_max_retries(max_retries);
                if let Some(url) = endpoint {
                    cfg = cfg.with_base_url_override(url.to_string());
                }
                cfg
            }
            "custom" => {
                let base_url = endpoint
                    .ok_or_else(|| TransportError::Config {
                        detail: "provider=custom requires endpoint to be set in config".to_string(),
                    })?
                    .to_string();
                let api_key = config
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("CUSTOM_API_KEY").ok())
                    .unwrap_or_default();
                AdapterConfig::custom("Custom", base_url, api_key)
                    .with_timeout(timeout)
                    .with_max_retries(max_retries)
            }
            other => {
                return Err(TransportError::Config {
                    detail: format!(
                        "unknown provider '{other}'. \
                         Valid: lm-studio, openrouter, nvidia, openai, custom"
                    ),
                });
            }
        };

        Ok(Box::new(OpenAiCompatAdapter::new(adapter_config)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_for(provider: &str) -> AppConfig {
        let mut cfg = AppConfig::default();
        cfg.provider = provider.to_string();
        cfg
    }

    fn config_with_key(provider: &str, key: &str) -> AppConfig {
        let mut cfg = config_for(provider);
        cfg.api_key = Some(key.to_string());
        cfg
    }

    #[test]
    fn lm_studio_requires_no_key() {
        let cfg = config_for("lm-studio");
        assert!(ProviderRegistry::from_config(&cfg).is_ok());
    }

    #[test]
    fn lm_studio_custom_endpoint() {
        let mut cfg = config_for("lm-studio");
        cfg.endpoint = Some("http://localhost:5000/v1".to_string());
        let provider = ProviderRegistry::from_config(&cfg).unwrap();
        assert_eq!(provider.name(), "LM Studio");
    }

    #[test]
    fn openrouter_missing_key_is_error() {
        let _lock = env_lock();
        std::env::remove_var("OPENROUTER_API_KEY");
        let cfg = config_for("openrouter");
        let err = ProviderRegistry::from_config(&cfg).err().unwrap();
        assert!(matches!(err, TransportError::MissingCredentials { .. }));
    }

    #[test]
    fn openrouter_key_from_config() {
        let cfg = config_with_key("openrouter", "sk-test");
        let provider = ProviderRegistry::from_config(&cfg).unwrap();
        assert_eq!(provider.name(), "OpenRouter");
    }

    #[test]
    fn openai_missing_key_is_error() {
        let _lock = env_lock();
        std::env::remove_var("OPENAI_API_KEY");
        let cfg = config_for("openai");
        assert!(ProviderRegistry::from_config(&cfg).is_err());
    }

    #[test]
    fn openai_key_from_config() {
        let cfg = config_with_key("openai", "sk-openai");
        let provider = ProviderRegistry::from_config(&cfg).unwrap();
        assert_eq!(provider.name(), "OpenAI");
    }

    #[test]
    fn nvidia_missing_key_is_error() {
        let _lock = env_lock();
        std::env::remove_var("NVIDIA_API_KEY");
        let cfg = config_for("nvidia");
        assert!(ProviderRegistry::from_config(&cfg).is_err());
    }

    #[test]
    fn nvidia_key_from_config() {
        let cfg = config_with_key("nvidia", "nvapi-test");
        let provider = ProviderRegistry::from_config(&cfg).unwrap();
        assert_eq!(provider.name(), "NVIDIA");
    }

    #[test]
    fn custom_missing_endpoint_is_error() {
        let cfg = config_for("custom");
        let err = ProviderRegistry::from_config(&cfg).err().unwrap();
        assert!(matches!(err, TransportError::Config { .. }));
    }

    #[test]
    fn custom_with_endpoint() {
        let mut cfg = config_for("custom");
        cfg.endpoint = Some("http://my-server/v1".to_string());
        let provider = ProviderRegistry::from_config(&cfg).unwrap();
        assert_eq!(provider.name(), "Custom");
    }

    #[test]
    fn unknown_provider_is_error() {
        let cfg = config_for("unknown-provider");
        let err = ProviderRegistry::from_config(&cfg).err().unwrap();
        assert!(matches!(err, TransportError::Config { .. }));
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        use std::sync::{Mutex, OnceLock};
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock")
    }
}
