//! `BashTool` — execute shell commands in a sandboxed working directory.
//!
//! # Confinement policy
//!
//! Before any command is executed, `validate_bash_command` performs best-effort
//! static analysis on the command string to block path escapes.
//!
//! ## Rules
//!
//! 1. **Relative traversal** (`../`) — rejected before any disk access.
//!
//! 2. **Absolute path confinement** — all absolute-path tokens found in the
//!    command string (sequences starting with `/` at a word boundary) must be
//!    prefixed by the canonicalized project root.  This covers `/etc/passwd`,
//!    `/home/other/secrets`, `/var/log/syslog`, `/tmp/attacker/file`, etc.
//!    without relying on an ever-incomplete denylist.
//!
//! 3. **Working directory pinning** — the child process is started with
//!    `current_dir` set to `cwd`, so every relative path resolves within the
//!    project tree.
//!
//! ## Known limitations
//!
//! Shell is Turing-complete.  A command that constructs a path dynamically
//! (e.g. `cat "/et"+"c/passwd"` or via variable expansion) bypasses string-level
//! analysis.  For fully trusted isolation, OS-level sandboxing (seccomp,
//! namespaces, containers) is required.  This implementation blocks the common
//! patterns produced by LLMs operating on the filesystem.

use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use code_buddy_errors::ToolError;
use serde_json::{Value, json};
use tracing::instrument;

use crate::Tool;

// ── confinement ───────────────────────────────────────────────────────────────

/// Bytes that may legally appear immediately before or after an absolute path
/// token in a shell command.
const WORD_DELIMITERS: &[u8] = b" \t\"'();|&=`\n";

/// Scan `command` and return all substrings that look like absolute path tokens.
///
/// An absolute path token is a `/`-leading sequence that starts at a
/// whitespace/shell-delimiter boundary.  Examples that are found:
/// - `cat /etc/passwd` → `["/etc/passwd"]`
/// - `echo "hello" && cat /var/log/file` → `["/var/log/file"]`
/// - `cargo build` → `[]`  (no absolute paths)
///
/// This is deliberately conservative: it may miss dynamically constructed paths
/// (those bypasses require OS-level sandboxing).
fn absolute_path_tokens(command: &str) -> Vec<&str> {
    let bytes = command.as_bytes();
    let len = bytes.len();
    let mut tokens = Vec::new();
    let mut i = 0usize;

    while i < len {
        if bytes[i] == b'/' {
            let at_boundary = i == 0 || WORD_DELIMITERS.contains(&bytes[i - 1]);
            if at_boundary {
                // Find the end of this token (next delimiter or end of string).
                let start = i;
                i += 1;
                while i < len && !WORD_DELIMITERS.contains(&bytes[i]) {
                    i += 1;
                }
                // Only keep tokens longer than just "/" (bare slash is not a useful check).
                if i > start + 1 {
                    tokens.push(&command[start..i]);
                }
                continue;
            }
        }
        i += 1;
    }

    tokens
}

/// Validate a bash command string before execution.
///
/// Returns `Ok(())` if the command passes confinement checks, or a
/// [`ToolError::PathTraversal`] if it contains a path that escapes `cwd`.
///
/// See the module-level documentation for the full policy description.
pub(crate) fn validate_bash_command(
    tool: &str,
    command: &str,
    cwd: &Path,
) -> Result<(), ToolError> {
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

    // Rule 2 — positive absolute-path confinement.
    //
    // Canonicalize cwd first so we compare against the true filesystem path
    // (e.g. /private/var/... on macOS rather than /var/...).
    let canon_cwd = match cwd.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // If cwd cannot be canonicalized (e.g. in unit tests with a
            // non-existent directory) fall back to the provided path.
            cwd.to_path_buf()
        }
    };

    for token in absolute_path_tokens(command) {
        // Use Path::starts_with for component-aware comparison:
        // "/home/user/proj-evil" must NOT pass when cwd is "/home/user/proj".
        // String prefix matching would incorrectly allow it; Path::starts_with
        // only matches on full path components.
        if !Path::new(token).starts_with(&canon_cwd) {
            return Err(ToolError::PathTraversal {
                tool: tool.to_string(),
                path: token.to_string(),
            });
        }
    }

    Ok(())
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
         Absolute paths must be within the project root; relative traversal (../) \
         is rejected."
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
        // Two concurrent executions can never write to the same path —
        // each sees its own $TMPDIR.
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
        assert!(out.contains('1') || out.is_empty() || out.contains("status"));
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

    /// An absolute path to `/etc/passwd` must be rejected.
    #[tokio::test]
    async fn bash_absolute_etc_passwd_is_rejected() {
        let (t, _dir) = tmp_tool();
        let err = t
            .execute(json!({ "command": "cat /etc/passwd" }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal, got {err:?}"
        );
    }

    /// Access to `/proc` must be rejected.
    #[tokio::test]
    async fn bash_proc_access_is_rejected() {
        let (t, _dir) = tmp_tool();
        let err = t
            .execute(json!({ "command": "cat /proc/self/environ" }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    /// Absolute path to `/home/other` (not under cwd) must be rejected.
    #[tokio::test]
    async fn bash_home_outside_cwd_is_rejected() {
        let (t, _dir) = tmp_tool();
        let err = t
            .execute(json!({ "command": "cat /home/attacker/secrets.txt" }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal for /home outside cwd, got {err:?}"
        );
    }

    /// Absolute path to `/var/log/syslog` (not under cwd) must be rejected.
    #[tokio::test]
    async fn bash_var_log_is_rejected() {
        let (t, _dir) = tmp_tool();
        let err = t
            .execute(json!({ "command": "tail /var/log/syslog" }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal for /var outside cwd, got {err:?}"
        );
    }

    /// Absolute path to `/tmp/outside/file` (not under cwd) must be rejected.
    #[tokio::test]
    async fn bash_tmp_outside_cwd_is_rejected() {
        let (t, _dir) = tmp_tool();
        let err = t
            .execute(json!({ "command": "cat /tmp/attacker/secrets" }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal for /tmp outside cwd, got {err:?}"
        );
    }

    /// Normal project commands (no absolute paths) must NOT be blocked.
    #[tokio::test]
    async fn bash_normal_commands_are_allowed() {
        let (t, _dir) = tmp_tool();
        t.execute(json!({ "command": "echo ok" })).await.unwrap();
        t.execute(json!({ "command": "pwd" })).await.unwrap();
        t.execute(json!({ "command": "ls ." })).await.unwrap();
    }

    // ── confinement — unit tests on validate_bash_command ────────────────────

    #[test]
    fn validate_rejects_dotdot_slash() {
        let cwd = std::env::current_dir().unwrap();
        let err = validate_bash_command("bash", "cat ../secret.txt", &cwd).unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    #[test]
    fn validate_rejects_slash_dotdot() {
        let cwd = std::env::current_dir().unwrap();
        let err =
            validate_bash_command("bash", "ls /some/path/..", &cwd).unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    #[test]
    fn validate_rejects_absolute_etc() {
        let dir = tempfile::tempdir().unwrap();
        let err =
            validate_bash_command("bash", "cat /etc/shadow", dir.path()).unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    #[test]
    fn validate_rejects_absolute_home_outside_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let err =
            validate_bash_command("bash", "cat /home/other/file", dir.path()).unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    #[test]
    fn validate_rejects_absolute_var() {
        let dir = tempfile::tempdir().unwrap();
        let err =
            validate_bash_command("bash", "tail /var/log/syslog", dir.path()).unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    #[test]
    fn validate_allows_normal_commands() {
        let cwd = std::env::current_dir().unwrap();
        validate_bash_command("bash", "cargo build", &cwd).unwrap();
        validate_bash_command("bash", "echo hello", &cwd).unwrap();
        validate_bash_command("bash", "cat src/main.rs", &cwd).unwrap();
        validate_bash_command("bash", "grep -r TODO .", &cwd).unwrap();
    }

    #[test]
    fn validate_allows_absolute_path_within_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let file_inside = dir.path().join("src").join("main.rs");
        // The absolute path to a file inside cwd should be allowed.
        let cmd = format!("cat {}", file_inside.display());
        validate_bash_command("bash", &cmd, dir.path()).unwrap();
    }

    /// Regression: string-prefix matching would allow `/home/user/proj-evil`
    /// when CWD is `/home/user/proj` because the string starts with the prefix.
    /// `Path::starts_with` is component-aware and must reject this.
    #[test]
    fn validate_rejects_prefix_collision_path() {
        let dir = tempfile::tempdir().unwrap();
        // Build a sibling directory name that shares the prefix of dir but is different.
        let parent = dir.path().parent().unwrap();
        let sibling_name = format!("{}-evil", dir.path().file_name().unwrap().to_str().unwrap());
        let sibling = parent.join(&sibling_name);
        let cmd = format!("cat {}/secret.txt", sibling.display());
        let err = validate_bash_command("bash", &cmd, dir.path()).unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal for prefix-collision path, got {err:?}"
        );
    }

    // ── Regression §1: unique tmp dirs prevent race condition ─────────────────

    /// Regression test for bug_report.md §1 (Race Condition in Sandbox).
    ///
    /// Two concurrent `BashTool` executions that both write to `$TMPDIR/output`
    /// must NOT conflict. Each invocation sees its own isolated `$TMPDIR`.
    #[tokio::test]
    async fn concurrent_bash_tools_get_unique_tmp_dirs() {
        let cwd = Arc::new(std::env::current_dir().unwrap());
        let mut set = JoinSet::new();

        for i in 0..8u32 {
            let cwd = cwd.clone();
            set.spawn(async move {
                let t = BashTool::new((*cwd).clone());
                let shell_cmd = format!(
                    "echo {i} > \"$TMPDIR/output\" && cat \"$TMPDIR/output\""
                );
                let out = t.execute(json!({ "command": shell_cmd })).await.unwrap();
                let n: u32 = out.trim().parse().expect("should parse number");
                assert_eq!(n, i, "output must match the value we wrote");
            });
        }

        while let Some(res) = set.join_next().await {
            res.unwrap();
        }
    }
}
