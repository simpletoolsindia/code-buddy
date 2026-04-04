//! Provider registry: selects and constructs the correct adapter from config.
//!
//! The registry is the single point where `provider` config values are mapped to
//! concrete [`Provider`] trait objects. No provider-specific branching should
//! appear outside this module.

use std::time::Duration;

use code_buddy_config::AppConfig;
use code_buddy_errors::TransportError;
use code_buddy_transport::Provider;

use crate::adapters::{
    CustomLocalProvider, LmStudioProvider, NvidiaProvider, OpenAiCompatProvider,
    OpenRouterProvider,
};

/// Constructs provider adapters from configuration.
pub struct ProviderRegistry;

impl ProviderRegistry {
    /// Build the correct adapter from the given [`AppConfig`].
    ///
    /// # Errors
    /// Returns [`TransportError::MissingCredentials`] if a required API key is absent.
    pub fn from_config(config: &AppConfig) -> Result<Box<dyn Provider>, TransportError> {
        let timeout = Duration::from_secs(config.timeout_seconds);
        let endpoint = config.endpoint.as_deref();

        let max_retries = config.max_retries;

        let provider: Box<dyn Provider> = match config.provider.as_str() {
            "lm-studio" => {
                let base_url = endpoint.map(str::to_string);
                Box::new(LmStudioProvider::new(base_url, timeout, max_retries))
            }
            "ollama" => {
                let base_url = endpoint
                    .map_or_else(|| "http://localhost:11434/v1".to_string(), |u| format!("{u}/v1"));
                Box::new(CustomLocalProvider::new("Ollama", base_url, String::new(), timeout, max_retries))
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
                if let Some(url) = endpoint {
                    Box::new(OpenRouterProvider::with_endpoint(api_key, url, timeout, max_retries))
                } else {
                    Box::new(OpenRouterProvider::new(api_key, timeout, max_retries))
                }
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
                if let Some(url) = endpoint {
                    Box::new(NvidiaProvider::with_endpoint(api_key, url, timeout, max_retries))
                } else {
                    Box::new(NvidiaProvider::new(api_key, timeout, max_retries))
                }
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
                if let Some(url) = endpoint {
                    Box::new(OpenAiCompatProvider::with_base_url(api_key, url, timeout, max_retries))
                } else {
                    Box::new(OpenAiCompatProvider::new(api_key, timeout, max_retries))
                }
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
                Box::new(CustomLocalProvider::new("Custom", base_url, api_key, timeout, max_retries))
            }
            other => {
                return Err(TransportError::Config {
                    detail: format!(
                        "unknown provider '{other}'. \
                         Valid: lm-studio, ollama, openrouter, nvidia, openai, custom"
                    ),
                });
            }
        };

        Ok(provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_for(provider: &str) -> AppConfig {
        AppConfig { provider: provider.to_string(), ..Default::default() }
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

    // ── LM Studio integration tests ────────────────────────────────────────────
    //
    // These tests require a locally-running LM Studio instance on port 1234 with
    // at least one model loaded. They are marked `#[ignore]` so they are skipped
    // in CI unless explicitly opted-in with `cargo test -- --ignored`.
    //
    // Run with: cargo test -p code-buddy-providers lm_studio -- --ignored --nocapture

    /// Smoke test: non-streaming send to LM Studio.
    ///
    /// Requires LM Studio running at `http://localhost:1234/v1` with a model loaded.
    #[tokio::test]
    #[ignore = "requires LM Studio running at localhost:1234"]
    async fn lm_studio_send_smoke() {
        use code_buddy_transport::MessageRequest;

        let cfg = config_for("lm-studio");
        let provider = ProviderRegistry::from_config(&cfg).expect("build provider");
        assert_eq!(provider.name(), "LM Studio");

        let req = MessageRequest::simple("local-model", "Say exactly the word PONG and nothing else.");
        let resp = provider.send(&req).await.expect("send request");
        let text = resp.text_content();
        assert!(!text.is_empty(), "expected non-empty response from LM Studio");
        eprintln!("LM Studio send response: {text:?}");
    }

    /// Streaming smoke test: SSE events from LM Studio.
    ///
    /// Requires LM Studio running at `http://localhost:1234/v1` with a model loaded.
    #[tokio::test]
    #[ignore = "requires LM Studio running at localhost:1234"]
    async fn lm_studio_stream_smoke() {
        use code_buddy_transport::{MessageRequest, StreamEvent};

        let cfg = config_for("lm-studio");
        let provider = ProviderRegistry::from_config(&cfg).expect("build provider");

        let req = MessageRequest::simple(
            "local-model",
            "Count from 1 to 5, one number per line.",
        )
        .with_streaming();

        let mut source = provider.stream(&req).await.expect("start stream");

        let mut collected = String::new();
        let mut saw_stop = false;
        loop {
            match source.next_event().await.expect("next event") {
                None => break,
                Some(StreamEvent::TextDelta(t)) => {
                    eprint!("{t}");
                    collected.push_str(&t);
                }
                Some(StreamEvent::MessageStop) => {
                    eprintln!();
                    saw_stop = true;
                    break;
                }
                Some(StreamEvent::Usage(u)) => {
                    eprintln!("\n[tokens: in={} out={}]", u.input_tokens, u.output_tokens);
                }
                Some(_) => {}
            }
        }

        assert!(!collected.is_empty(), "expected streamed text from LM Studio");
        assert!(saw_stop, "expected MessageStop event from LM Studio stream");
    }
}
