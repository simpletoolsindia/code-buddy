//! Hooks System
//!
//! Hooks allow you to run custom scripts at specific points in Code Buddy's lifecycle.
//! Similar to Claude Code's hooks system.
//!
//! Hook Types:
//! - `before_write` - Run before modifying files
//! - `after_write` - Run after modifying files
//! - `before_submit` - Run before submitting tool results
//! - `after_submit` - Run after submitting tool results
//! - `on_error` - Run when an error occurs
//! - `on_compact` - Run when context is compacted
//!
//! Configuration:
//! Add hooks to your config.json:
//! ```json
//! {
//!   "hooks": [
//!     {
//!       "event": "before_write",
//!       "command": "echo 'About to write: {path}'",
//!       "enabled": true
//!     }
//!   ]
//! }
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Hook event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HookEvent {
    /// Before writing/modifying files
    #[serde(rename = "before_write")]
    BeforeWrite,
    /// After writing/modifying files
    #[serde(rename = "after_write")]
    AfterWrite,
    /// Before submitting tool results
    #[serde(rename = "before_submit")]
    BeforeSubmit,
    /// After submitting tool results
    #[serde(rename = "after_submit")]
    AfterSubmit,
    /// When an error occurs
    #[serde(rename = "on_error")]
    OnError,
    /// When context is compacted
    #[serde(rename = "on_compact")]
    OnCompact,
    /// Before running a command
    #[serde(rename = "before_command")]
    BeforeCommand,
    /// After running a command
    #[serde(rename = "after_command")]
    AfterCommand,
    /// On session start
    #[serde(rename = "on_start")]
    OnStart,
    /// On session end
    #[serde(rename = "on_exit")]
    OnExit,
}

impl HookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookEvent::BeforeWrite => "before_write",
            HookEvent::AfterWrite => "after_write",
            HookEvent::BeforeSubmit => "before_submit",
            HookEvent::AfterSubmit => "after_submit",
            HookEvent::OnError => "on_error",
            HookEvent::OnCompact => "on_compact",
            HookEvent::BeforeCommand => "before_command",
            HookEvent::AfterCommand => "after_command",
            HookEvent::OnStart => "on_start",
            HookEvent::OnExit => "on_exit",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "before_write" => Some(HookEvent::BeforeWrite),
            "after_write" => Some(HookEvent::AfterWrite),
            "before_submit" => Some(HookEvent::BeforeSubmit),
            "after_submit" => Some(HookEvent::AfterSubmit),
            "on_error" => Some(HookEvent::OnError),
            "on_compact" => Some(HookEvent::OnCompact),
            "before_command" => Some(HookEvent::BeforeCommand),
            "after_command" => Some(HookEvent::AfterCommand),
            "on_start" => Some(HookEvent::OnStart),
            "on_exit" => Some(HookEvent::OnExit),
            _ => None,
        }
    }
}

/// A hook definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    /// Unique identifier
    pub id: Option<String>,
    /// Event that triggers this hook
    pub event: HookEvent,
    /// Command to execute (shell command or path to script)
    pub command: String,
    /// Whether the hook is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Working directory for the command
    pub working_dir: Option<String>,
    /// Environment variables to pass
    pub env: Option<HashMap<String, String>>,
    /// Timeout in seconds (0 = no timeout)
    #[serde(default)]
    pub timeout: u64,
    /// Continue on hook failure
    #[serde(default = "default_continue_on_failure")]
    pub continue_on_failure: bool,
    /// Description for the hook
    pub description: Option<String>,
}

fn default_enabled() -> bool {
    true
}

fn default_continue_on_failure() -> bool {
    false
}

/// Result of running a hook
#[derive(Debug, Clone)]
pub struct HookResult {
    /// Whether the hook succeeded
    pub success: bool,
    /// stdout from the hook
    pub stdout: String,
    /// stderr from the hook
    pub stderr: String,
    /// Exit code
    pub exit_code: i32,
    /// Execution time in milliseconds
    pub duration_ms: u64,
}

/// Hooks manager
#[derive(Debug, Default)]
pub struct HooksManager {
    /// All registered hooks
    hooks: Vec<Hook>,
}

impl HooksManager {
    /// Create a new hooks manager
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// Load hooks from a config file
    pub fn load_from_file(path: &std::path::Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)?;
        let hooks: Vec<Hook> = serde_json::from_str(&content)?;
        Ok(Self { hooks })
    }

    /// Load hooks from a JSON string
    pub fn load_from_json(json: &str) -> Result<Self> {
        let hooks: Vec<Hook> = serde_json::from_str(json)?;
        Ok(Self { hooks })
    }

    /// Add a hook
    pub fn add(&mut self, hook: Hook) {
        self.hooks.push(hook);
    }

    /// Remove a hook by ID
    pub fn remove(&mut self, id: &str) -> bool {
        let initial_len = self.hooks.len();
        self.hooks.retain(|h| h.id.as_deref() != Some(id));
        self.hooks.len() < initial_len
    }

    /// Get hooks for a specific event
    pub fn get_hooks(&self, event: &HookEvent) -> Vec<&Hook> {
        self.hooks
            .iter()
            .filter(|h| h.event == *event && h.enabled)
            .collect()
    }

    /// Get all hooks
    pub fn get_all_hooks(&self) -> &[Hook] {
        &self.hooks
    }

    /// Check if there are any hooks for an event
    pub fn has_hooks(&self, event: &HookEvent) -> bool {
        self.get_hooks(event).iter().any(|_| true)
    }

    /// Run all hooks for an event
    pub async fn run_hooks(
        &self,
        event: &HookEvent,
        context: &HookContext,
    ) -> Vec<HookResult> {
        let hooks = self.get_hooks(event);
        let mut results = Vec::new();

        for hook in hooks {
            let result = run_single_hook(hook, context).await;
            let success = result.success;
            results.push(result);

            // Stop if a hook fails and continue_on_failure is false
            if !success && !hook.continue_on_failure {
                break;
            }
        }

        results
    }

    /// Run hooks synchronously
    pub fn run_hooks_sync(&self, event: &HookEvent, context: &HookContext) -> Vec<HookResult> {
        let hooks = self.get_hooks(event);
        let mut results = Vec::new();

        for hook in hooks {
            let result = run_single_hook_sync(hook, context);
            results.push(result);

            if !results.last().map(|r| r.success).unwrap_or(true) && !hook.continue_on_failure {
                break;
            }
        }

        results
    }
}

/// Context passed to hooks
#[derive(Debug, Clone, Serialize)]
pub struct HookContext {
    /// Event type
    pub event: String,
    /// File path (for write hooks)
    pub path: Option<String>,
    /// Tool name (for tool hooks)
    pub tool: Option<String>,
    /// Command (for command hooks)
    pub command: Option<String>,
    /// Error message (for error hooks)
    pub error: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Current working directory
    pub cwd: Option<String>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl HookContext {
    /// Create a new context
    pub fn new(event: &str) -> Self {
        Self {
            event: event.to_string(),
            path: None,
            tool: None,
            command: None,
            error: None,
            session_id: None,
            cwd: std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(String::from)),
            metadata: None,
        }
    }

    /// Set file path
    pub fn with_path(mut self, path: &str) -> Self {
        self.path = Some(path.to_string());
        self
    }

    /// Set tool name
    pub fn with_tool(mut self, tool: &str) -> Self {
        self.tool = Some(tool.to_string());
        self
    }

    /// Set command
    pub fn with_command(mut self, cmd: &str) -> Self {
        self.command = Some(cmd.to_string());
        self
    }

    /// Set error
    pub fn with_error(mut self, err: &str) -> Self {
        self.error = Some(err.to_string());
        self
    }

    /// Set session ID
    pub fn with_session(mut self, session: &str) -> Self {
        self.session_id = Some(session.to_string());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Expand variables in a string
    pub fn expand(&self, template: &str) -> String {
        let mut result = template.to_string();

        // Expand {event}
        result = result.replace("{event}", &self.event);

        // Expand {path}
        if let Some(ref path) = self.path {
            result = result.replace("{path}", path);
        }

        // Expand {tool}
        if let Some(ref tool) = self.tool {
            result = result.replace("{tool}", tool);
        }

        // Expand {command}
        if let Some(ref cmd) = self.command {
            result = result.replace("{command}", cmd);
        }

        // Expand {error}
        if let Some(ref error) = self.error {
            result = result.replace("{error}", error);
        }

        // Expand {cwd}
        if let Some(ref cwd) = self.cwd {
            result = result.replace("{cwd}", cwd);
        }

        result
    }
}

/// Run a single hook asynchronously
async fn run_single_hook(hook: &Hook, context: &HookContext) -> HookResult {
    let expanded_command = context.expand(&hook.command);
    let start = std::time::Instant::now();

    // Determine working directory
    let working_dir = hook
        .working_dir
        .as_ref()
        .map(|d| context.expand(d))
        .or_else(|| context.cwd.clone());

    // Build the command
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", &expanded_command]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", &expanded_command]);
        c
    };

    if let Some(ref dir) = working_dir {
        cmd.current_dir(dir);
    }

    // Add environment variables
    if let Some(ref env) = hook.env {
        for (key, value) in env {
            cmd.env(key, context.expand(value));
        }
    }

    // Execute with optional timeout
    let output = match tokio::process::Command::from(cmd).output().await {
        Ok(o) => o,
        Err(e) => {
            return HookResult {
                success: false,
                stdout: String::new(),
                stderr: format!("Hook execution failed: {}", e),
                exit_code: -1,
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }
    };

    let duration = start.elapsed().as_millis() as u64;

    HookResult {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        duration_ms: duration,
    }
}

/// Run a single hook synchronously
fn run_single_hook_sync(hook: &Hook, context: &HookContext) -> HookResult {
    let expanded_command = context.expand(&hook.command);
    let start = std::time::Instant::now();

    // Determine working directory
    let working_dir = hook
        .working_dir
        .as_ref()
        .map(|d| context.expand(d))
        .or_else(|| context.cwd.clone());

    // Build the command
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", &expanded_command]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", &expanded_command]);
        c
    };

    if let Some(ref dir) = working_dir {
        cmd.current_dir(dir);
    }

    // Add environment variables
    if let Some(ref env) = hook.env {
        for (key, value) in env {
            cmd.env(key, context.expand(value));
        }
    }

    // Execute with optional timeout
    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            return HookResult {
                success: false,
                stdout: String::new(),
                stderr: format!("Hook execution failed: {}", e),
                exit_code: -1,
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }
    };

    let duration = start.elapsed().as_millis() as u64;

    HookResult {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        duration_ms: duration,
    }
}

/// Example hook configurations
pub mod examples {
    use super::*;

    pub fn git_auto_commit() -> Hook {
        Hook {
            id: Some("git-auto-commit".to_string()),
            event: HookEvent::AfterWrite,
            command: "git add -A && git commit -m 'Auto-commit from Code Buddy'".to_string(),
            enabled: true,
            working_dir: None,
            env: None,
            timeout: 30,
            continue_on_failure: true,
            description: Some("Auto-commit changes after writing files".to_string()),
        }
    }

    pub fn lint_check() -> Hook {
        Hook {
            id: Some("lint-check".to_string()),
            event: HookEvent::BeforeSubmit,
            command: "cargo clippy --quiet".to_string(),
            enabled: true,
            working_dir: None,
            env: None,
            timeout: 60,
            continue_on_failure: true,
            description: Some("Run clippy before submitting changes".to_string()),
        }
    }

    pub fn log_commands() -> Hook {
        Hook {
            id: Some("log-commands".to_string()),
            event: HookEvent::AfterCommand,
            command: r#"echo "[{event}] {command} completed in {duration}ms""#.to_string(),
            enabled: true,
            working_dir: None,
            env: None,
            timeout: 5,
            continue_on_failure: true,
            description: Some("Log executed commands".to_string()),
        }
    }

    pub fn error_notifier() -> Hook {
        Hook {
            id: Some("error-notifier".to_string()),
            event: HookEvent::OnError,
            command: r#"echo "Error occurred: {error}""#.to_string(),
            enabled: true,
            working_dir: None,
            env: None,
            timeout: 5,
            continue_on_failure: true,
            description: Some("Notify on errors".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_parsing() {
        assert_eq!(HookEvent::from_str("before_write"), Some(HookEvent::BeforeWrite));
        assert_eq!(HookEvent::from_str("on_error"), Some(HookEvent::OnError));
        assert_eq!(HookEvent::from_str("unknown"), None);
    }

    #[test]
    fn test_hook_event_str() {
        assert_eq!(HookEvent::BeforeWrite.as_str(), "before_write");
        assert_eq!(HookEvent::OnError.as_str(), "on_error");
    }

    #[test]
    fn test_context_expand() {
        let ctx = HookContext::new("before_write")
            .with_path("/path/to/file.txt")
            .with_tool("Write");

        assert_eq!(ctx.expand("{event}"), "before_write");
        assert_eq!(ctx.expand("{path}"), "/path/to/file.txt");
        assert_eq!(ctx.expand("{tool}"), "Write");
        assert_eq!(ctx.expand("{cwd}"), std::env::current_dir().unwrap().to_str().unwrap());
    }

    #[test]
    fn test_hooks_manager() {
        let mut manager = HooksManager::new();

        manager.add(Hook {
            id: Some("test1".to_string()),
            event: HookEvent::BeforeWrite,
            command: "echo test".to_string(),
            enabled: true,
            working_dir: None,
            env: None,
            timeout: 0,
            continue_on_failure: false,
            description: None,
        });

        assert!(manager.has_hooks(&HookEvent::BeforeWrite));
        assert!(!manager.has_hooks(&HookEvent::AfterWrite));
        assert_eq!(manager.get_hooks(&HookEvent::BeforeWrite).len(), 1);
    }

    #[test]
    fn test_load_from_json() {
        let json = r#"[
            {
                "event": "before_write",
                "command": "echo test",
                "enabled": true
            }
        ]"#;

        let manager = HooksManager::load_from_json(json).unwrap();
        assert!(manager.has_hooks(&HookEvent::BeforeWrite));
    }
}
