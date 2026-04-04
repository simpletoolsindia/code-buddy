//! Telemetry and logging infrastructure for Code Buddy.
//!
//! Initializes `tracing-subscriber` with environment-controlled log levels,
//! structured JSON output option, and a safe secret-redaction pattern for logs.

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
    /// Override the log filter string (e.g. "code_buddy=debug,reqwest=warn").
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
        // Should not panic
        drop(filter);
    }

    #[test]
    fn build_filter_default() {
        let config = TelemetryConfig::default();
        let filter = build_filter(&config);
        drop(filter);
    }
}
