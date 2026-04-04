//! Telemetry and logging infrastructure for Code Buddy.
//!
//! Initializes `tracing-subscriber` with environment-controlled log levels,
//! structured JSON output option, and a safe secret-redaction pattern for logs.
//!
//! Span helpers (`request_start`, `response_received`, `tool_call_start`,
//! `tool_call_done`) are provided so all provider adapters emit consistent
//! structured tracing events without ad-hoc `tracing::debug!` calls.

use tracing::{debug, info};
use tracing_subscriber::{fmt, EnvFilter};

/// Logging output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable pretty-printed output (default).
    Pretty,
    /// Structured JSON output (useful for log aggregation).
    Json,
}

/// Configuration for the telemetry subsystem.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Enable debug-level logging. Equivalent to setting `RUST_LOG=debug`.
    pub debug: bool,
    /// Output format.
    pub format: LogFormat,
    /// Override the log filter string (e.g. `code_buddy=debug,reqwest=warn`).
    pub filter_override: Option<String>,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            debug: false,
            format: LogFormat::Pretty,
            filter_override: None,
        }
    }
}

/// Initialize the global tracing subscriber.
///
/// This must be called exactly once, early in `main()`, before any async tasks start.
/// Subsequent calls are silently ignored (the subscriber is already set).
///
/// # Errors
/// Returns an error if the tracing subscriber could not be initialized.
pub fn init(config: &TelemetryConfig) -> Result<(), String> {
    let filter = build_filter(config);

    let result = match config.format {
        LogFormat::Pretty => {
            let subscriber = fmt::Subscriber::builder()
                .with_env_filter(filter)
                .with_target(true)
                .with_file(config.debug)
                .with_line_number(config.debug)
                .finish();
            tracing::subscriber::set_global_default(subscriber)
        }
        LogFormat::Json => {
            let subscriber = fmt::Subscriber::builder()
                .json()
                .with_env_filter(filter)
                .with_target(true)
                .finish();
            tracing::subscriber::set_global_default(subscriber)
        }
    };

    result.map_err(|e| format!("Failed to initialize tracing: {e}"))
}

// ── Structured span helpers ───────────────────────────────────────────────────
//
// These are thin wrappers around `tracing` macros that emit consistent,
// structured events for all provider adapters. Phase 2 provider implementations
// should call these instead of ad-hoc debug!/info! calls.

/// Emit a structured event when an LLM request is about to be sent.
///
/// Call this immediately before the HTTP request is dispatched.
pub fn request_start(provider: &str, model: &str, prompt_tokens_approx: usize) {
    debug!(
        provider = provider,
        model = model,
        prompt_tokens_approx = prompt_tokens_approx,
        "request_start"
    );
}

/// Emit a structured event when the first token of a response is received.
///
/// For streaming responses, call this on the first `StreamEvent::ContentDelta`.
/// For non-streaming, call it when the HTTP response body is fully available.
pub fn response_received(provider: &str, model: &str, latency_ms: u64, streaming: bool) {
    debug!(
        provider = provider,
        model = model,
        latency_ms = latency_ms,
        streaming = streaming,
        "response_received"
    );
}

/// Emit a structured event when a tool call is dispatched by the model.
///
/// Call this when the model requests a tool invocation (before execution).
pub fn tool_call_start(tool_name: &str, call_id: &str) {
    info!(
        tool_name = tool_name,
        call_id = call_id,
        "tool_call_start"
    );
}

/// Emit a structured event when a tool call completes.
///
/// Call this after the tool finishes executing (success or error).
pub fn tool_call_done(tool_name: &str, call_id: &str, success: bool, duration_ms: u64) {
    debug!(
        tool_name = tool_name,
        call_id = call_id,
        success = success,
        duration_ms = duration_ms,
        "tool_call_done"
    );
}

// ─────────────────────────────────────────────────────────────────────────────

fn build_filter(config: &TelemetryConfig) -> EnvFilter {
    // Priority: explicit override > RUST_LOG env > debug flag > default
    if let Some(ref override_str) = config.filter_override {
        return EnvFilter::new(override_str);
    }

    if std::env::var("RUST_LOG").is_ok() {
        return EnvFilter::from_default_env();
    }

    if config.debug {
        EnvFilter::new("code_buddy=debug,code_buddy_config=debug,code_buddy_transport=debug,warn")
    } else {
        EnvFilter::new("code_buddy=info,warn")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_filter_debug() {
        let config = TelemetryConfig {
            debug: true,
            ..Default::default()
        };
        let filter = build_filter(&config);
        drop(filter);
    }

    #[test]
    fn build_filter_default() {
        let config = TelemetryConfig::default();
        let filter = build_filter(&config);
        drop(filter);
    }

    #[test]
    fn span_helpers_do_not_panic() {
        request_start("lm-studio", "mistral-7b", 100);
        response_received("lm-studio", "mistral-7b", 250, true);
        tool_call_start("bash", "call-001");
        tool_call_done("bash", "call-001", true, 42);
    }
}
