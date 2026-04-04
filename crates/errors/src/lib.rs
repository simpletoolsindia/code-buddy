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

/// Transport errors — network and HTTP-level failures.
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("Connection timed out after {seconds}s to {url}")]
    Timeout { seconds: u64, url: String },

    #[error("Connection refused to {url}. Is the server running?")]
    ConnectionRefused { url: String },

    #[error("TLS error connecting to {url}: {reason}")]
    Tls { url: String, reason: String },

    #[error("Failed to parse response from {url}: {reason}")]
    ResponseParse { url: String, reason: String },

    #[error("Server error {status} from {url}: {body}")]
    ServerError {
        status: u16,
        url: String,
        body: String,
    },

    #[error("SSE stream error: {0}")]
    SseStream(String),
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

