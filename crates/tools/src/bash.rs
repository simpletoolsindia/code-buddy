//! BashTool — execute shell commands in a sandboxed working directory.
//!
//! # Confinement policy
//!
//! Before any command is executed, `validate_bash_command` performs best-effort
//! static analysis on the command string to block obvious path escapes:
//!
//! 1. **Relative traversal** — any command containing `../` or `/..` is
//!    rejected.  Normal project work (build, test, lint) never needs to
//!    navigate above the project root.
//!
//! 2. **Absolute system-path access** — tokens that reference well-known
//!    system directories (`/etc/`, `/proc/`, `/sys/`, `/dev/`, `/root/`,
//!    `/boot/`) are rejected even when spelled with a trailing slash omitted.
//!
//! 3. **Working directory pinning** — the process is started with `current_dir`
//!    set to `cwd`, so every relative path resolves within the project tree.
//!
//! ## Known limitations
//!
//! Shell is Turing-complete; a sufficiently creative command can construct a
//! path dynamically and bypass any string-level check.  For fully trusted
//! isolation, OS-level sandboxing (seccomp, namespaces, containers) is
//! required.  This implementation is designed to block the common LLM mistake
//! patterns seen in practice, not to replace an OS sandbox.

use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use code_buddy_errors::ToolError;
use serde_json::{Value, json};
use tracing::instrument;

use crate::Tool;

// ── confinement ───────────────────────────────────────────────────────────────

/// Well-known system path prefixes that must never appear as path arguments.
const BLOCKED_PREFIXES: &[&str] = &[
    "/etc/", "/proc/", "/sys/", "/dev/", "/root/", "/boot/",
];

/// Bare system directory names (without trailing slash).
const BLOCKED_DIRS: &[&str] = &["/etc", "/proc", "/sys", "/dev", "/root", "/boot"];

/// Validate a bash command string before execution.
///
/// Returns `Ok(())` if the command passes confinement checks, or a
/// `ToolError::PathTraversal` if it contains a known escape pattern.
///
/// See the module-level documentation for the full policy description.
pub(crate) fn validate_bash_command(
    tool: &str,
    command: &str,
    cwd: &Path,
) -> Result<(), ToolError> {
    let _cwd = cwd; // reserved for future absolute-path prefix checks

    // Rule 1 — relative traversal.
    if command.contains("../")
        || command.contains("/..")
        || command == ".."
        || command.ends_with("/..")
    {
        return Err(ToolError::PathTraversal {
            tool: tool.to_string(),
            path: "(../ traversal in command)".to_string(),
        });
    }

    // Rule 2a — absolute prefix match (includes trailing slash).
    for prefix in BLOCKED_PREFIXES {
        if command.contains(prefix) {
            return Err(ToolError::PathTraversal {
                tool: tool.to_string(),
                path: format!("(system path {prefix} in command)"),
            });
        }
    }

    // Rule 2b — bare directory names as path-like tokens.
    for dir in BLOCKED_DIRS {
        if token_present(command, dir) {
            return Err(ToolError::PathTraversal {
                tool: tool.to_string(),
                path: dir.to_string(),
            });
        }
    }

    Ok(())
}

/// Return `true` when `needle` appears in `haystack` as a whitespace- or
/// quote-delimited token (i.e., not as an interior substring of a longer path).
///
/// For example, `token_present("ls /etc", "/etc")` is `true`, but
/// `token_present("cat /etcetera/file", "/etc")` is `false` (the token is
/// followed by `e`, not a delimiter).
fn token_present(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    let nlen = needle_bytes.len();

    let mut i = 0usize;
    while i + nlen <= bytes.len() {
        if bytes[i..i + nlen] == *needle_bytes {
            let before = if i == 0 { b' ' } else { bytes[i - 1] };
            let after = if i + nlen >= bytes.len() {
                b' '
            } else {
                bytes[i + nlen]
            };
            let before_ok =
                matches!(before, b' ' | b'\t' | b'"' | b'\'' | b'(' | b'`' | b';' | b'|' | b'&');
            let after_ok =
                matches!(after, b' ' | b'\t' | b'"' | b'\'' | b')' | b'`' | b';' | b'|' | b'&');
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

// ── tool ──────────────────────────────────────────────────────────────────────

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
         Returns combined stdout and stderr. Commands run with a 30-second timeout. \
         Commands that access paths outside the project root (via ../ or system \
         directories) are rejected."
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

        // CWD confinement check — reject before spawning any process.
        validate_bash_command("bash", command, &self.cwd)?;

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

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::task::JoinSet;

    fn cwd() -> PathBuf {
        std::env::current_dir().unwrap()
    }

    fn tool() -> BashTool {
        BashTool::new(cwd())
    }

    fn tmp_tool() -> (BashTool, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let t = BashTool::new(dir.path().to_path_buf());
        (t, dir)
    }

    // ── basic execution ───────────────────────────────────────────────────────

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

    // ── CWD confinement — rejection tests ─────────────────────────────────────

    /// `../` traversal must be rejected before any process is spawned.
    #[tokio::test]
    async fn bash_dotdot_traversal_is_rejected() {
        let t = tool();
        let err = t
            .execute(json!({ "command": "cat ../../etc/passwd" }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal, got {err:?}"
        );
    }

    /// An absolute path directly to `/etc/passwd` must be rejected.
    #[tokio::test]
    async fn bash_absolute_etc_passwd_is_rejected() {
        let t = tool();
        let err = t
            .execute(json!({ "command": "cat /etc/passwd" }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal, got {err:?}"
        );
    }

    /// Access to `/proc` (system info) must be rejected.
    #[tokio::test]
    async fn bash_proc_access_is_rejected() {
        let t = tool();
        let err = t
            .execute(json!({ "command": "cat /proc/self/environ" }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    /// `ls /etc` (without trailing slash) must also be rejected.
    #[tokio::test]
    async fn bash_bare_etc_token_is_rejected() {
        let t = tool();
        let err = t
            .execute(json!({ "command": "ls /etc" }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal for bare /etc token, got {err:?}"
        );
    }

    /// Normal project commands must NOT be blocked.
    #[tokio::test]
    async fn bash_normal_commands_are_allowed() {
        let (t, _dir) = tmp_tool();
        t.execute(json!({ "command": "echo ok" })).await.unwrap();
        t.execute(json!({ "command": "pwd" })).await.unwrap();
        t.execute(json!({ "command": "ls ." })).await.unwrap();
    }

    /// A path that contains `/etc` as a substring of a longer component must
    /// NOT be blocked (e.g. `src/etcetera/file.txt`).
    #[test]
    fn bash_etcetera_subpath_is_not_blocked() {
        let cwd = PathBuf::from("/tmp");
        // "etcetera" contains "etc" but is not the /etc token.
        validate_bash_command("bash", "cat src/etcetera/notes.txt", &cwd).unwrap();
    }

    // ── confinement — unit tests ──────────────────────────────────────────────

    #[test]
    fn validate_rejects_dotdot_slash() {
        let cwd = PathBuf::from("/tmp/project");
        let err =
            validate_bash_command("bash", "cat ../secret.txt", &cwd).unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    #[test]
    fn validate_rejects_slash_dotdot() {
        let cwd = PathBuf::from("/tmp/project");
        let err =
            validate_bash_command("bash", "ls /tmp/project/..", &cwd).unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    #[test]
    fn validate_rejects_etc_with_trailing_slash() {
        let cwd = PathBuf::from("/tmp/project");
        let err =
            validate_bash_command("bash", "cat /etc/shadow", &cwd).unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    #[test]
    fn validate_rejects_proc_with_trailing_slash() {
        let cwd = PathBuf::from("/tmp/project");
        let err =
            validate_bash_command("bash", "cat /proc/cpuinfo", &cwd).unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    #[test]
    fn validate_rejects_bare_etc_token() {
        let cwd = PathBuf::from("/tmp/project");
        let err = validate_bash_command("bash", "ls /etc", &cwd).unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    #[test]
    fn validate_allows_normal_commands() {
        let cwd = PathBuf::from("/tmp/project");
        validate_bash_command("bash", "cargo build", &cwd).unwrap();
        validate_bash_command("bash", "echo hello", &cwd).unwrap();
        validate_bash_command("bash", "cat src/main.rs", &cwd).unwrap();
        validate_bash_command("bash", "grep -r TODO .", &cwd).unwrap();
    }

    #[test]
    fn validate_allows_subpath_containing_etc() {
        let cwd = PathBuf::from("/tmp/project");
        // "etcetera" should NOT be flagged as "/etc"
        validate_bash_command("bash", "ls src/etcetera", &cwd).unwrap();
    }

    // ── Regression §1: unique tmp dirs prevent race condition ─────────────────

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
}
