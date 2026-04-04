//! Concrete, named provider adapter types.
//!
//! Each type is a thin newtype wrapper around [`OpenAiCompatAdapter`] that gives
//! each provider a distinct named type in the public API. This satisfies Rust's
//! type-level distinction (e.g. `LmStudioProvider` vs `OpenRouterProvider`) while
//! reusing all HTTP, retry, streaming, and normalization logic from the shared adapter.
//!
//! # Construction
//! Use the associated `new` constructor on each type. The
//! [`ProviderRegistry`](crate::ProviderRegistry) calls these internally — you
//! rarely need to construct them directly.

use async_trait::async_trait;
use code_buddy_errors::TransportError;
use code_buddy_transport::{MessageRequest, MessageResponse, Provider, StreamSource};
use std::time::Duration;

use crate::openai_compat::{AdapterConfig, OpenAiCompatAdapter};

// ── LM Studio ─────────────────────────────────────────────────────────────────

/// Provider adapter for [LM Studio](https://lmstudio.ai/).
///
/// Connects to a locally-running LM Studio server (default: `http://localhost:1234/v1`).
/// No API key is required.
pub struct LmStudioProvider(OpenAiCompatAdapter);

impl LmStudioProvider {
    /// Create a new LM Studio adapter.
    ///
    /// `base_url` defaults to `http://localhost:1234/v1` when `None`.
    /// `max_retries` is clamped to 3 per spec.
    #[must_use]
    pub fn new(base_url: Option<String>, timeout: Duration, max_retries: u32) -> Self {
        let cfg = AdapterConfig::lm_studio()
            .with_timeout(timeout)
            .with_max_retries(max_retries)
            .with_base_url_override(
                base_url.unwrap_or_else(|| "http://localhost:1234/v1".to_string()),
            );
        Self(OpenAiCompatAdapter::new(cfg))
    }
}

#[async_trait]
impl Provider for LmStudioProvider {
    fn name(&self) -> &str {
        self.0.name()
    }

    async fn send(&self, request: &MessageRequest) -> Result<MessageResponse, TransportError> {
        self.0.send(request).await
    }

    async fn stream(
        &self,
        request: &MessageRequest,
    ) -> Result<Box<dyn StreamSource>, TransportError> {
        self.0.stream(request).await
    }
}

// ── OpenRouter ─────────────────────────────────────────────────────────────────

/// Provider adapter for [OpenRouter](https://openrouter.ai/).
///
/// Requires an `OPENROUTER_API_KEY` (or equivalent config value).
pub struct OpenRouterProvider(OpenAiCompatAdapter);

impl OpenRouterProvider {
    /// Create a new `OpenRouter` adapter. `max_retries` is clamped to 3.
    #[must_use]
    pub fn new(api_key: impl Into<String>, timeout: Duration, max_retries: u32) -> Self {
        let cfg = AdapterConfig::openrouter(api_key)
            .with_timeout(timeout)
            .with_max_retries(max_retries);
        Self(OpenAiCompatAdapter::new(cfg))
    }

    /// Create with a custom endpoint override (for testing or proxies).
    #[must_use]
    pub fn with_endpoint(
        api_key: impl Into<String>,
        endpoint: impl Into<String>,
        timeout: Duration,
        max_retries: u32,
    ) -> Self {
        let cfg = AdapterConfig::openrouter(api_key)
            .with_timeout(timeout)
            .with_max_retries(max_retries)
            .with_base_url_override(endpoint.into());
        Self(OpenAiCompatAdapter::new(cfg))
    }
}

#[async_trait]
impl Provider for OpenRouterProvider {
    fn name(&self) -> &str {
        self.0.name()
    }

    async fn send(&self, request: &MessageRequest) -> Result<MessageResponse, TransportError> {
        self.0.send(request).await
    }

    async fn stream(
        &self,
        request: &MessageRequest,
    ) -> Result<Box<dyn StreamSource>, TransportError> {
        self.0.stream(request).await
    }
}

// ── NVIDIA NIM ─────────────────────────────────────────────────────────────────

/// Provider adapter for [NVIDIA NIM](https://build.nvidia.com/).
///
/// Requires an `NVIDIA_API_KEY` (or equivalent config value).
pub struct NvidiaProvider(OpenAiCompatAdapter);

impl NvidiaProvider {
    /// Create a new NVIDIA NIM adapter. `max_retries` is clamped to 3.
    #[must_use]
    pub fn new(api_key: impl Into<String>, timeout: Duration, max_retries: u32) -> Self {
        let cfg = AdapterConfig::nvidia(api_key)
            .with_timeout(timeout)
            .with_max_retries(max_retries);
        Self(OpenAiCompatAdapter::new(cfg))
    }

    /// Create with a custom endpoint override.
    #[must_use]
    pub fn with_endpoint(
        api_key: impl Into<String>,
        endpoint: impl Into<String>,
        timeout: Duration,
        max_retries: u32,
    ) -> Self {
        let cfg = AdapterConfig::nvidia(api_key)
            .with_timeout(timeout)
            .with_max_retries(max_retries)
            .with_base_url_override(endpoint.into());
        Self(OpenAiCompatAdapter::new(cfg))
    }
}

#[async_trait]
impl Provider for NvidiaProvider {
    fn name(&self) -> &str {
        self.0.name()
    }

    async fn send(&self, request: &MessageRequest) -> Result<MessageResponse, TransportError> {
        self.0.send(request).await
    }

    async fn stream(
        &self,
        request: &MessageRequest,
    ) -> Result<Box<dyn StreamSource>, TransportError> {
        self.0.stream(request).await
    }
}

// ── OpenAI ─────────────────────────────────────────────────────────────────────

/// Provider adapter for the [OpenAI API](https://platform.openai.com/).
///
/// Requires an `OPENAI_API_KEY` (or equivalent config value).
pub struct OpenAiCompatProvider(OpenAiCompatAdapter);

impl OpenAiCompatProvider {
    /// Create a new `OpenAI` adapter. `max_retries` is clamped to 3.
    #[must_use]
    pub fn new(api_key: impl Into<String>, timeout: Duration, max_retries: u32) -> Self {
        let cfg = AdapterConfig::openai(api_key)
            .with_timeout(timeout)
            .with_max_retries(max_retries);
        Self(OpenAiCompatAdapter::new(cfg))
    }

    /// Create with a custom base URL (e.g. for Azure or proxy).
    #[must_use]
    pub fn with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        timeout: Duration,
        max_retries: u32,
    ) -> Self {
        let cfg = AdapterConfig::openai(api_key)
            .with_timeout(timeout)
            .with_max_retries(max_retries)
            .with_base_url_override(base_url.into());
        Self(OpenAiCompatAdapter::new(cfg))
    }
}

#[async_trait]
impl Provider for OpenAiCompatProvider {
    fn name(&self) -> &str {
        self.0.name()
    }

    async fn send(&self, request: &MessageRequest) -> Result<MessageResponse, TransportError> {
        self.0.send(request).await
    }

    async fn stream(
        &self,
        request: &MessageRequest,
    ) -> Result<Box<dyn StreamSource>, TransportError> {
        self.0.stream(request).await
    }
}

// ── Custom local ────────────────────────────────────────────────────────────────

/// Provider adapter for any custom OpenAI-compatible endpoint.
///
/// Useful for Ollama, `LocalAI`, or self-hosted models with any base URL.
/// The API key is optional; pass an empty string if not required.
pub struct CustomLocalProvider(OpenAiCompatAdapter);

impl CustomLocalProvider {
    /// Create a new custom local adapter.
    ///
    /// `api_key` may be empty if the endpoint does not require authentication.
    /// `max_retries` is clamped to 3.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        timeout: Duration,
        max_retries: u32,
    ) -> Self {
        let cfg = AdapterConfig::custom(name, base_url, api_key)
            .with_timeout(timeout)
            .with_max_retries(max_retries);
        Self(OpenAiCompatAdapter::new(cfg))
    }
}

#[async_trait]
impl Provider for CustomLocalProvider {
    fn name(&self) -> &str {
        self.0.name()
    }

    async fn send(&self, request: &MessageRequest) -> Result<MessageResponse, TransportError> {
        self.0.send(request).await
    }

    async fn stream(
        &self,
        request: &MessageRequest,
    ) -> Result<Box<dyn StreamSource>, TransportError> {
        self.0.stream(request).await
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lm_studio_provider_name() {
        let p = LmStudioProvider::new(None, Duration::from_secs(30), 3);
        assert_eq!(p.name(), "LM Studio");
    }

    #[test]
    fn lm_studio_provider_custom_url() {
        let p = LmStudioProvider::new(
            Some("http://localhost:5555/v1".to_string()),
            Duration::from_secs(30),
            3,
        );
        assert_eq!(p.name(), "LM Studio");
    }

    #[test]
    fn openrouter_provider_name() {
        let p = OpenRouterProvider::new("sk-test", Duration::from_secs(30), 3);
        assert_eq!(p.name(), "OpenRouter");
    }

    #[test]
    fn nvidia_provider_name() {
        let p = NvidiaProvider::new("nvapi-test", Duration::from_secs(30), 3);
        assert_eq!(p.name(), "NVIDIA");
    }

    #[test]
    fn openai_compat_provider_name() {
        let p = OpenAiCompatProvider::new("sk-test", Duration::from_secs(30), 3);
        assert_eq!(p.name(), "OpenAI");
    }

    #[test]
    fn custom_local_provider_name() {
        let p = CustomLocalProvider::new(
            "Ollama",
            "http://localhost:11434/v1",
            "",
            Duration::from_secs(30),
            3,
        );
        assert_eq!(p.name(), "Ollama");
    }
}
