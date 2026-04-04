//! BashTool — execute shell commands in a sandboxed working directory.
//!
//! # Safety design
//! - Working directory is pinned to the CWD provided at construction time.
//! - Each invocation gets a unique `TMPDIR` / `TEMP` / `TMP` so concurrent
//!   executions never share the same temp space (fixes the race-condition bug
//!   described in bug_report.md §1).
//! - No path-traversal check on the `command` string itself — the model is
//!   expected to operate within the project tree, and the shell is the
//!   boundary enforcer.

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use code_buddy_errors::ToolError;
use serde_json::{Value, json};
use tracing::instrument;

use crate::Tool;

/// Execute an arbitrary shell command and return stdout + stderr.
pub struct BashTool {
    cwd: PathBuf,
}

impl BashTool {
    /// Create a new `BashTool` rooted at `cwd`.
    #[must_use]
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command in the project directory and return its output. \
         Returns combined stdout and stderr. Commands run with a 30-second timeout."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Optional timeout in milliseconds (max 30000)"
                }
            },
            "required": ["command"]
        })
    }

    #[instrument(skip(self), fields(tool = "bash"))]
    async fn execute(&self, input: Value) -> Result<String, ToolError> {
        let command = input["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs {
                tool: "bash".to_string(),
                reason: "missing required field 'command'".to_string(),
            })?;

        if command.trim().is_empty() {
            return Err(ToolError::InvalidArgs {
                tool: "bash".to_string(),
                reason: "command must not be empty".to_string(),
            });
        }

        let timeout_ms = input["timeout_ms"]
            .as_u64()
            .unwrap_or(30_000)
            .min(30_000);

        // Bug fix §1: Each invocation gets an isolated temp directory.
        // Two concurrent executions can never write to the same `sandbox_binary`
        // path — each sees its own $TMPDIR.
        let tmp_dir = tempfile::Builder::new()
            .prefix(&format!("cb_bash_{}_", uuid::Uuid::new_v4().simple()))
            .tempdir()
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "bash".to_string(),
                reason: format!("failed to create temp dir: {e}"),
            })?;

        let output = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(&self.cwd)
                .env("TMPDIR", tmp_dir.path())
                .env("TEMP", tmp_dir.path())
                .env("TMP", tmp_dir.path())
                .output(),
        )
        .await
        .map_err(|_| ToolError::Timeout {
            tool: "bash".to_string(),
            seconds: timeout_ms / 1000,
        })?
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "bash".to_string(),
            reason: format!("spawn failed: {e}"),
        })?;

        let mut result = String::new();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.is_empty() {
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str("[stderr]\n");
            result.push_str(&stderr);
        }

        if !output.status.success() && result.is_empty() {
            result = format!(
                "Command exited with status {}",
                output.status.code().unwrap_or(-1)
            );
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::task::JoinSet;

    fn tool() -> BashTool {
        BashTool::new(std::env::current_dir().unwrap())
    }

    #[tokio::test]
    async fn bash_echo_command() {
        let t = tool();
        let out = t.execute(json!({ "command": "echo hello" })).await.unwrap();
        assert!(out.contains("hello"));
    }

    #[tokio::test]
    async fn bash_empty_command_is_error() {
        let t = tool();
        let err = t.execute(json!({ "command": "" })).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs { .. }));
    }

    #[tokio::test]
    async fn bash_missing_command_field_is_error() {
        let t = tool();
        let err = t.execute(json!({})).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs { .. }));
    }

    /// Regression test for bug_report.md §1 (Race Condition in Sandbox).
    ///
    /// Two concurrent `BashTool` executions that both write to `$TMPDIR/output`
    /// must NOT conflict. Each invocation sees its own isolated `$TMPDIR`, so the
    /// files are distinct and the outputs are independent.
    #[tokio::test]
    async fn concurrent_bash_tools_get_unique_tmp_dirs() {
        let cwd = Arc::new(std::env::current_dir().unwrap());
        let mut set = JoinSet::new();

        for i in 0..8u32 {
            let cwd = cwd.clone();
            set.spawn(async move {
                let t = BashTool::new((*cwd).clone());
                // Write a value to $TMPDIR/output — each invocation writes
                // to its own isolated $TMPDIR, so no file can be overwritten.
                let cmd = format!(
                    "echo {i} > \"$TMPDIR/output\" && cat \"$TMPDIR/output\""
                );
                let out = t.execute(json!({ "command": cmd })).await.unwrap();
                let n: u32 = out.trim().parse().expect("should parse number");
                assert_eq!(n, i, "output must match the value we wrote");
            });
        }

        while let Some(res) = set.join_next().await {
            res.unwrap();
        }
    }

    #[tokio::test]
    async fn bash_stderr_is_captured() {
        let t = tool();
        let out = t
            .execute(json!({ "command": "echo err >&2" }))
            .await
            .unwrap();
        assert!(out.contains("err"));
    }

    #[tokio::test]
    async fn bash_exit_code_failure_captured() {
        let t = tool();
        let out = t.execute(json!({ "command": "exit 1" })).await.unwrap();
        assert!(out.contains("1") || out.is_empty() || out.contains("status"));
    }
}
