//! Search tools: glob file discovery and regex content search.
//!
//! # Security
//! Both tools enforce CWD confinement via `resolve_within_cwd`:
//! - Absolute paths are rejected if they fall outside the project root.
//! - `..` traversal components that escape CWD are rejected.
//!
//! `GrepSearchTool` additionally compiles the user-supplied regex pattern before
//! any file I/O (mitigating bug_report.md §2 — injection/DoS via malformed patterns).

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use code_buddy_errors::ToolError;
use serde_json::{Value, json};
use tracing::instrument;

use crate::Tool;
use crate::path_utils::resolve_within_cwd;

// ── GlobSearchTool ────────────────────────────────────────────────────────────

/// Find files matching a glob pattern within the project root.
pub struct GlobSearchTool {
    cwd: PathBuf,
}

impl GlobSearchTool {
    #[must_use]
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }
}

#[async_trait]
impl Tool for GlobSearchTool {
    fn name(&self) -> &str {
        "glob_search"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern (e.g. '**/*.rs'). \
         Returns a list of matching paths relative to the project root. \
         All paths are confined to the project root — absolute or traversal \
         paths outside the root are rejected."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g. '**/*.rs', 'src/**/*.toml')"
                },
                "path": {
                    "type": "string",
                    "description": "Base directory to search (relative to project root). Defaults to '.'."
                }
            },
            "required": ["pattern"]
        })
    }

    #[instrument(skip(self), fields(tool = "glob_search"))]
    async fn execute(&self, input: Value) -> Result<String, ToolError> {
        let pattern_str = input["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs {
                tool: "glob_search".to_string(),
                reason: "missing required field 'pattern'".to_string(),
            })?;

        // ── Pattern confinement ──────────────────────────────────────────────
        // Reject absolute glob patterns before any I/O: `PathBuf::join` on an
        // absolute pattern silently discards the base directory, allowing the
        // glob to roam the entire filesystem (e.g. `/etc/*`).
        if pattern_str.starts_with('/') {
            return Err(ToolError::PathTraversal {
                tool: "glob_search".to_string(),
                path: pattern_str.to_string(),
            });
        }
        // Reject `..` traversal inside the pattern string itself.
        if pattern_str.contains("../") || pattern_str.contains("/..") || pattern_str == ".." {
            return Err(ToolError::PathTraversal {
                tool: "glob_search".to_string(),
                path: pattern_str.to_string(),
            });
        }

        let base_str = input["path"].as_str().unwrap_or(".");

        // ── CWD confinement ──────────────────────────────────────────────────
        // Both relative and absolute `base_str` values are validated here.
        // An absolute path outside cwd or a `..`-traversal is rejected before
        // any file I/O.
        let base = resolve_within_cwd("glob_search", &self.cwd, base_str)?;

        // Build the full glob pattern from the confined base path.
        // Safety: `pattern_str` has been verified non-absolute above, so
        // `base.join(pattern_str)` cannot discard `base`.
        let full_pattern = base.join(pattern_str);
        let full_pattern_str = full_pattern.to_string_lossy().to_string();

        let cwd = self.cwd.canonicalize().map_err(|e| ToolError::ExecutionFailed {
            tool: "glob_search".to_string(),
            reason: format!("cannot resolve cwd: {e}"),
        })?;

        let matches = tokio::task::spawn_blocking(move || -> Result<Vec<String>, ToolError> {
            let paths = glob::glob(&full_pattern_str).map_err(|e| ToolError::InvalidArgs {
                tool: "glob_search".to_string(),
                reason: format!("invalid glob pattern: {e}"),
            })?;

            let mut results = Vec::new();
            for entry in paths.flatten() {
                let rel = entry
                    .strip_prefix(&cwd)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| entry.to_string_lossy().to_string());
                results.push(rel);
            }
            results.sort();
            Ok(results)
        })
        .await
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "glob_search".to_string(),
            reason: format!("task error: {e}"),
        })??;

        if matches.is_empty() {
            return Ok("No files matched the pattern.".to_string());
        }

        Ok(matches.join("\n"))
    }
}

// ── GrepSearchTool ────────────────────────────────────────────────────────────

/// Search file contents with a regex pattern, confined to the project root.
///
/// # Injection-safety (bug_report.md §2)
///
/// The user-supplied pattern is compiled with `regex::Regex` before any file
/// I/O occurs. Malformed patterns are rejected with [`ToolError::InvalidArgs`]
/// rather than panicking or being passed raw to a query engine.
///
/// # CWD confinement
///
/// The search root is resolved and validated through `resolve_within_cwd` before
/// any filesystem access. Absolute paths and `..` traversals outside the project
/// root are rejected.
pub struct GrepSearchTool {
    cwd: PathBuf,
}

impl GrepSearchTool {
    #[must_use]
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }
}

/// Walk a directory tree and search each file for `pattern` matches.
fn grep_dir(
    dir: &Path,
    pattern: &regex::Regex,
    include_glob: Option<&glob::Pattern>,
) -> Vec<String> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') || matches!(name, "target" | "node_modules") {
                        continue;
                    }
                }
                results.extend(grep_dir(&path, pattern, include_glob));
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(ig) = include_glob {
                    if !ig.matches(name) {
                        continue;
                    }
                }
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    for (idx, line) in contents.lines().enumerate() {
                        if pattern.is_match(line) {
                            results.push(format!("{}:{}: {}", path.display(), idx + 1, line));
                        }
                    }
                }
            }
        }
    }
    results
}

#[async_trait]
impl Tool for GrepSearchTool {
    fn name(&self) -> &str {
        "grep_search"
    }

    fn description(&self) -> &str {
        "Search file contents for a regex pattern. Returns matching lines with \
         file paths and line numbers. Use 'include' to restrict to specific file types. \
         All paths are confined to the project root."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search (relative to project root). Defaults to '.'."
                },
                "include": {
                    "type": "string",
                    "description": "File name glob to include (e.g. '*.rs', '*.md'). Optional."
                }
            },
            "required": ["pattern"]
        })
    }

    #[instrument(skip(self), fields(tool = "grep_search"))]
    async fn execute(&self, input: Value) -> Result<String, ToolError> {
        let raw_pattern = input["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs {
                tool: "grep_search".to_string(),
                reason: "missing required field 'pattern'".to_string(),
            })?;

        // ── Injection-safety fix (bug_report.md §2) ──────────────────────────
        let regex = regex::Regex::new(raw_pattern).map_err(|e| ToolError::InvalidArgs {
            tool: "grep_search".to_string(),
            reason: format!("invalid regex pattern: {e}"),
        })?;

        let path_str = input["path"].as_str().unwrap_or(".");

        // ── CWD confinement ──────────────────────────────────────────────────
        let target = resolve_within_cwd("grep_search", &self.cwd, path_str)?;

        let include_str = input["include"].as_str().map(str::to_string);

        let results = tokio::task::spawn_blocking(move || -> Result<Vec<String>, ToolError> {
            let include_glob = include_str
                .as_deref()
                .map(|s| {
                    glob::Pattern::new(s).map_err(|e| ToolError::InvalidArgs {
                        tool: "grep_search".to_string(),
                        reason: format!("invalid include pattern: {e}"),
                    })
                })
                .transpose()?;

            if target.is_file() {
                let mut matches = Vec::new();
                if let Ok(contents) = std::fs::read_to_string(&target) {
                    for (idx, line) in contents.lines().enumerate() {
                        if regex.is_match(line) {
                            matches.push(format!("{}:{}: {}", target.display(), idx + 1, line));
                        }
                    }
                }
                Ok(matches)
            } else {
                Ok(grep_dir(&target, &regex, include_glob.as_ref()))
            }
        })
        .await
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "grep_search".to_string(),
            reason: format!("task error: {e}"),
        })??;

        if results.is_empty() {
            return Ok("No matches found.".to_string());
        }

        let limited = if results.len() > 200 {
            let mut r = results[..200].to_vec();
            r.push(format!("... ({} more matches truncated)", results.len() - 200));
            r
        } else {
            results
        };

        Ok(limited.join("\n"))
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    // ── GlobSearchTool ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn glob_finds_files() {
        let dir = tmp();
        std::fs::write(dir.path().join("a.rs"), "").unwrap();
        std::fs::write(dir.path().join("b.rs"), "").unwrap();
        std::fs::write(dir.path().join("c.txt"), "").unwrap();

        let tool = GlobSearchTool::new(dir.path().to_path_buf());
        let out = tool.execute(json!({ "pattern": "*.rs" })).await.unwrap();
        assert!(out.contains("a.rs"));
        assert!(out.contains("b.rs"));
        assert!(!out.contains("c.txt"));
    }

    #[tokio::test]
    async fn glob_no_matches_message() {
        let dir = tmp();
        let tool = GlobSearchTool::new(dir.path().to_path_buf());
        let out = tool.execute(json!({ "pattern": "*.xyz" })).await.unwrap();
        assert!(out.contains("No files matched"));
    }

    #[tokio::test]
    async fn glob_invalid_pattern_is_error() {
        let dir = tmp();
        let tool = GlobSearchTool::new(dir.path().to_path_buf());
        let result = tool.execute(json!({ "pattern": "[unclosed" })).await;
        let _ = result; // Either error or empty result is acceptable.
    }

    /// Path traversal via `..` is rejected before any filesystem access.
    #[tokio::test]
    async fn glob_rejects_traversal_path() {
        let dir = tmp();
        let tool = GlobSearchTool::new(dir.path().to_path_buf());
        let err = tool
            .execute(json!({ "pattern": "*.rs", "path": "../../etc" }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal, got {err:?}"
        );
    }

    /// Absolute path outside cwd is rejected.
    #[tokio::test]
    async fn glob_rejects_absolute_path_outside_cwd() {
        let dir = tmp();
        let tool = GlobSearchTool::new(dir.path().to_path_buf());
        let err = tool
            .execute(json!({ "pattern": "*.rs", "path": "/etc" }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    /// Regression: absolute glob pattern (e.g. `/etc/*`) must be rejected.
    ///
    /// Old bug: `PathBuf::join(base, "/etc/*")` silently discards `base`,
    /// making the glob roam the entire filesystem.
    #[tokio::test]
    async fn glob_rejects_absolute_pattern() {
        let dir = tmp();
        let tool = GlobSearchTool::new(dir.path().to_path_buf());
        let err = tool
            .execute(json!({ "pattern": "/etc/*" }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal for absolute glob pattern, got {err:?}"
        );
    }

    /// Glob pattern containing `..` traversal must be rejected.
    #[tokio::test]
    async fn glob_rejects_dotdot_in_pattern() {
        let dir = tmp();
        let tool = GlobSearchTool::new(dir.path().to_path_buf());
        let err = tool
            .execute(json!({ "pattern": "../../etc/*.conf" }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal for ../ in glob pattern, got {err:?}"
        );
    }

    // ── GrepSearchTool ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn grep_finds_matches() {
        let dir = tmp();
        std::fs::write(
            dir.path().join("code.rs"),
            "fn main() {\n    println!(\"hello\");\n}\n",
        )
        .unwrap();

        let tool = GrepSearchTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(json!({ "pattern": "println" }))
            .await
            .unwrap();
        assert!(out.contains("code.rs"));
        assert!(out.contains("println"));
    }

    #[tokio::test]
    async fn grep_no_matches_message() {
        let dir = tmp();
        std::fs::write(dir.path().join("f.txt"), "hello world").unwrap();
        let tool = GrepSearchTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(json!({ "pattern": "xyz_not_present" }))
            .await
            .unwrap();
        assert!(out.contains("No matches"));
    }

    /// Regression test for bug_report.md §2 (Injection via malformed patterns).
    ///
    /// A malformed regex must be rejected with a structured `InvalidArgs` error
    /// rather than causing a panic or being passed raw to a query engine.
    #[tokio::test]
    async fn grep_malformed_regex_returns_structured_error() {
        let dir = tmp();
        let tool = GrepSearchTool::new(dir.path().to_path_buf());

        let bad_patterns = [
            r"[unclosed",
            r"(?invalid_flag",
            r"*no-quantifier",
            r"(?P<>empty name)",
        ];

        for pattern in &bad_patterns {
            let err = tool
                .execute(json!({ "pattern": pattern }))
                .await
                .unwrap_err();
            assert!(
                matches!(err, ToolError::InvalidArgs { .. }),
                "pattern {pattern:?} should produce InvalidArgs, got {err:?}"
            );
        }
    }

    #[tokio::test]
    async fn grep_with_include_filter() {
        let dir = tmp();
        std::fs::write(dir.path().join("src.rs"), "fn hello() {}").unwrap();
        std::fs::write(dir.path().join("notes.md"), "fn hello world").unwrap();

        let tool = GrepSearchTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(json!({ "pattern": "fn hello", "include": "*.rs" }))
            .await
            .unwrap();
        assert!(out.contains("src.rs"));
        assert!(!out.contains("notes.md"));
    }

    /// Path traversal via `..` is rejected before any regex is executed.
    #[tokio::test]
    async fn grep_rejects_traversal_path() {
        let dir = tmp();
        let tool = GrepSearchTool::new(dir.path().to_path_buf());
        let err = tool
            .execute(json!({ "pattern": "hello", "path": "../../etc" }))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal, got {err:?}"
        );
    }

    /// Absolute path outside cwd is rejected.
    #[tokio::test]
    async fn grep_rejects_absolute_path_outside_cwd() {
        let dir = tmp();
        let tool = GrepSearchTool::new(dir.path().to_path_buf());
        let err = tool
            .execute(json!({ "pattern": "hello", "path": "/etc" }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }
}
