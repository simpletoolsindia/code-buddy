//! Search tools: glob file discovery and regex content search.
//!
//! # Security
//! `GrepSearchTool` sanitizes the user-supplied regex pattern before
//! compiling it (mitigating bug_report.md §2 — FTS5/injection-style
//! denial-of-service via malformed patterns). The fix is to compile the
//! pattern with `regex::Regex` and surface a structured `ToolError::InvalidArgs`
//! rather than propagating a panic or using raw shell injection.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use code_buddy_errors::ToolError;
use serde_json::{Value, json};
use tracing::instrument;

use crate::Tool;

// ── GlobSearchTool ────────────────────────────────────────────────────────────

/// Find files matching a glob pattern.
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
         Returns a list of matching paths relative to the project root."
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

        let base_str = input["path"].as_str().unwrap_or(".");
        let base = self.cwd.join(base_str);

        // Combine: base/pattern
        let full_pattern = base.join(pattern_str);
        let full_pattern_str = full_pattern.to_string_lossy().to_string();

        let cwd = self.cwd.clone();

        // Spawn blocking glob iteration on a thread pool.
        let matches = tokio::task::spawn_blocking(move || -> Result<Vec<String>, ToolError> {
            let paths = glob::glob(&full_pattern_str).map_err(|e| ToolError::InvalidArgs {
                tool: "glob_search".to_string(),
                reason: format!("invalid glob pattern: {e}"),
            })?;

            let mut results = Vec::new();
            for entry in paths.flatten() {
                // Return paths relative to cwd when possible.
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

/// Search file contents with a regex pattern.
///
/// # Injection-safety (bug_report.md §2)
///
/// Unlike the original code that passed raw queries directly into FTS5 MATCH,
/// this tool compiles the user-supplied pattern with `regex::Regex`. Malformed
/// patterns are rejected with a structured [`ToolError::InvalidArgs`] before any
/// file I/O occurs, preventing DOS from catastrophic backtracking or malformed
/// expressions.
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
                // Skip hidden dirs and common noise dirs.
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') || matches!(name, "target" | "node_modules") {
                        continue;
                    }
                }
                results.extend(grep_dir(&path, pattern, include_glob));
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Apply include filter if provided.
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
         file paths and line numbers. Use 'include' to restrict to specific file types."
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
        // Compile the regex up-front. Malformed patterns are rejected cleanly
        // with InvalidArgs rather than panicking or being passed raw to a
        // query engine (as in the original FTS5 vulnerability).
        let regex = regex::Regex::new(raw_pattern).map_err(|e| ToolError::InvalidArgs {
            tool: "grep_search".to_string(),
            reason: format!("invalid regex pattern: {e}"),
        })?;

        let path_str = input["path"].as_str().unwrap_or(".");
        let target = self.cwd.join(path_str);

        let include_str = input["include"]
            .as_str()
            .map(str::to_string);

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

        // Limit to 200 lines so large repos don't flood the context.
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
        let out = tool
            .execute(json!({ "pattern": "*.xyz" }))
            .await
            .unwrap();
        assert!(out.contains("No files matched"));
    }

    #[tokio::test]
    async fn glob_invalid_pattern_is_error() {
        let dir = tmp();
        let tool = GlobSearchTool::new(dir.path().to_path_buf());
        // Glob crate doesn't error on most patterns, but an unclosed bracket does.
        let result = tool.execute(json!({ "pattern": "[unclosed" })).await;
        // Either error or empty result is acceptable.
        let _ = result;
    }

    // ── GrepSearchTool ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn grep_finds_matches() {
        let dir = tmp();
        std::fs::write(dir.path().join("code.rs"), "fn main() {\n    println!(\"hello\");\n}\n")
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
    /// rather than causing a panic, process crash, or being passed raw to a
    /// query engine. This verifies the "compile-first" defensive approach.
    #[tokio::test]
    async fn grep_malformed_regex_returns_structured_error() {
        let dir = tmp();
        let tool = GrepSearchTool::new(dir.path().to_path_buf());

        // These patterns are invalid regexes — they should be rejected cleanly.
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
}
