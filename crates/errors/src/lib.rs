//! Structured error types for Code Buddy.
//!
//! All error types use `thiserror` for clean, user-visible messages.
//! No raw `unwrap()` or `expect()` calls should propagate through these types.

use thiserror::Error;

/// Configuration errors — problems loading, parsing, or validating the config file.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Config file not found at {path}")]
    NotFound { path: String },

    #[error("Failed to read config file at {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse config file at {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to write config file at {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid config value for '{field}': {reason}")]
    Validation { field: String, reason: String },

    #[error("Missing required config value for '{field}'. Set it via config file or environment variable {env_var}")]
    MissingRequired { field: String, env_var: String },

    #[error("Failed to determine config directory. Set HOME or XDG_CONFIG_HOME.")]
    NoConfigDir,
}

/// Transport errors — network, HTTP, parse, and authentication failures.
///
/// These are the primary error type returned by provider adapters and the
/// streaming transport. Each variant carries the provider name for clear
/// user-facing messages.
#[derive(Debug, Clone, Error)]
pub enum TransportError {
    /// A network-level error (DNS, connection, etc.) that is retryable.
    #[error("[{provider}] Network error: {detail}")]
    Network { provider: String, detail: String },

    /// A TCP connection failure (server not running, port closed).
    #[error("[{provider}] Connection failed: {detail}")]
    Connection { provider: String, detail: String },

    /// Request timed out.
    #[error("[{provider}] Request timed out after {timeout_secs}s")]
    Timeout {
        provider: String,
        timeout_secs: u64,
    },

    /// HTTP 4xx/5xx error from the provider API.
    #[error("[{provider}] API error {status}: {message}")]
    ApiError {
        provider: String,
        status: u16,
        message: String,
    },

    /// Authentication failure (missing or invalid API key).
    #[error("[{provider}] Missing credentials — set {env_var} or configure api_key")]
    MissingCredentials { provider: String, env_var: String },

    /// Response body parse failure.
    #[error("[{provider}] Failed to parse response: {detail}")]
    Parse { provider: String, detail: String },

    /// SSE stream parse error.
    #[error("[{provider}] SSE stream error: {detail}")]
    Sse { provider: String, detail: String },

    /// Retries exhausted after transient errors.
    #[error("[{provider}] All {attempts} attempts failed: {last_error}")]
    RetriesExhausted {
        provider: String,
        attempts: u32,
        last_error: String,
    },

    /// Configuration error preventing provider construction.
    #[error("Provider config error: {detail}")]
    Config { detail: String },
}

impl TransportError {
    /// Whether this error is likely transient and worth retrying.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Network { .. } | Self::Timeout { .. } | Self::Connection { .. })
    }
}

/// Provider errors — problems with LLM provider communication or authentication.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Authentication failed for provider '{provider}'. Check your API key.")]
    AuthFailed { provider: String },

    #[error("API key missing for provider '{provider}'. Set it via config or environment variable {env_var}")]
    MissingApiKey { provider: String, env_var: String },

    #[error("Unknown provider '{provider}'. Valid providers: {valid}")]
    UnknownProvider { provider: String, valid: String },

    #[error("Provider '{provider}' returned an error: {message}")]
    ApiError { provider: String, message: String },

    #[error("Model '{model}' not found on provider '{provider}'")]
    ModelNotFound { model: String, provider: String },

    #[error("Rate limit exceeded for provider '{provider}'. Retry after {retry_after:?}s")]
    RateLimit {
        provider: String,
        retry_after: Option<u64>,
    },

    #[error("Provider '{provider}' is unreachable at {url}")]
    Unreachable { provider: String, url: String },

    #[error("Transport error from provider '{provider}': {source}")]
    Transport {
        provider: String,
        #[source]
        source: TransportError,
    },
}

/// Tool errors — problems during tool execution.
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Unknown tool '{name}'. Available tools: {available}")]
    UnknownTool { name: String, available: String },

    #[error("Invalid arguments for tool '{tool}': {reason}")]
    InvalidArgs { tool: String, reason: String },

    #[error("Tool '{tool}' execution failed: {reason}")]
    ExecutionFailed { tool: String, reason: String },

    #[error("Tool '{tool}' timed out after {seconds}s")]
    Timeout { tool: String, seconds: u64 },

    #[error("Tool '{tool}' attempted to access a path outside the working directory: {path}")]
    PathTraversal { tool: String, path: String },

    #[error("Failed to parse tool call JSON: {reason}")]
    ParseFailed { reason: String },

    #[error("Tool call schema validation failed for '{tool}': {reason}")]
    SchemaValidation { tool: String, reason: String },
}

/// Runtime errors — orchestration and conversation loop failures.
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Maximum tool iterations ({max}) exceeded. Breaking to prevent infinite loop.")]
    MaxIterationsExceeded { max: usize },

    #[error("Session context too large ({tokens} tokens). Compact the conversation first.")]
    ContextTooLarge { tokens: u32 },

    #[error("Failed to serialize request: {0}")]
    Serialization(String),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
