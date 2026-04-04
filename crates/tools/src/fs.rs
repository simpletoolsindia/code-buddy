//! File system tools: read, write, and edit files within the project directory.
//!
//! All tools validate that the target path stays within the configured CWD,
//! preventing path-traversal attacks.

use std::path::PathBuf;

use async_trait::async_trait;
use code_buddy_errors::ToolError;
use serde_json::{Value, json};
use tracing::instrument;

use crate::Tool;
use crate::path_utils::resolve_within_cwd;

// ── ReadFileTool ─────────────────────────────────────────────────────────────

/// Read the contents of a file within the project directory.
pub struct ReadFileTool {
    cwd: PathBuf,
}

impl ReadFileTool {
    #[must_use]
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Optionally specify start_line and end_line \
         (1-based, inclusive) to read a range of lines."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to project root)"
                },
                "start_line": {
                    "type": "integer",
                    "description": "First line to return (1-based). Optional."
                },
                "end_line": {
                    "type": "integer",
                    "description": "Last line to return (1-based, inclusive). Optional."
                }
            },
            "required": ["path"]
        })
    }

    #[instrument(skip(self), fields(tool = "read_file"))]
    async fn execute(&self, input: Value) -> Result<String, ToolError> {
        let path_str = input["path"].as_str().ok_or_else(|| ToolError::InvalidArgs {
            tool: "read_file".to_string(),
            reason: "missing required field 'path'".to_string(),
        })?;

        let target = resolve_within_cwd("read_file", &self.cwd, path_str)?;

        if !target.exists() {
            return Err(ToolError::ExecutionFailed {
                tool: "read_file".to_string(),
                reason: format!("file not found: {path_str}"),
            });
        }

        let contents = tokio::fs::read_to_string(&target)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "read_file".to_string(),
                reason: format!("read error: {e}"),
            })?;

        let start = input["start_line"].as_u64().map(|n| n as usize);
        let end = input["end_line"].as_u64().map(|n| n as usize);

        if start.is_none() && end.is_none() {
            return Ok(contents);
        }

        let lines: Vec<&str> = contents.lines().collect();
        let total = lines.len();
        let lo = start.unwrap_or(1).saturating_sub(1);
        let hi = end.unwrap_or(total).min(total);

        if lo >= hi {
            return Ok(String::new());
        }

        Ok(lines[lo..hi].join("\n"))
    }
}

// ── WriteFileTool ─────────────────────────────────────────────────────────────

/// Write content to a file, creating parent directories if needed.
pub struct WriteFileTool {
    cwd: PathBuf,
}

impl WriteFileTool {
    #[must_use]
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file (creates or overwrites). Parent directories are \
         created automatically."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to write (relative to project root)"
                },
                "content": {
                    "type": "string",
                    "description": "File content to write"
                }
            },
            "required": ["path", "content"]
        })
    }

    #[instrument(skip(self), fields(tool = "write_file"))]
    async fn execute(&self, input: Value) -> Result<String, ToolError> {
        let path_str = input["path"].as_str().ok_or_else(|| ToolError::InvalidArgs {
            tool: "write_file".to_string(),
            reason: "missing required field 'path'".to_string(),
        })?;

        let content = input["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs {
                tool: "write_file".to_string(),
                reason: "missing required field 'content'".to_string(),
            })?;

        let target = resolve_within_cwd("write_file", &self.cwd, path_str)?;

        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "write_file".to_string(),
                    reason: format!("cannot create parent dirs: {e}"),
                })?;
        }

        let byte_count = content.len();
        tokio::fs::write(&target, content)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "write_file".to_string(),
                reason: format!("write error: {e}"),
            })?;

        Ok(format!("Wrote {byte_count} bytes to {path_str}"))
    }
}

// ── EditFileTool ─────────────────────────────────────────────────────────────

/// Find-and-replace a unique string within a file.
pub struct EditFileTool {
    cwd: PathBuf,
}

impl EditFileTool {
    #[must_use]
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Replace the first occurrence of `old_string` with `new_string` in a file. \
         Returns an error if `old_string` is not found or appears more than once."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to project root)"
                },
                "old_string": {
                    "type": "string",
                    "description": "Exact text to find (must be unique in the file)"
                },
                "new_string": {
                    "type": "string",
                    "description": "Replacement text"
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    #[instrument(skip(self), fields(tool = "edit_file"))]
    async fn execute(&self, input: Value) -> Result<String, ToolError> {
        let path_str = input["path"].as_str().ok_or_else(|| ToolError::InvalidArgs {
            tool: "edit_file".to_string(),
            reason: "missing required field 'path'".to_string(),
        })?;

        let old = input["old_string"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs {
                tool: "edit_file".to_string(),
                reason: "missing required field 'old_string'".to_string(),
            })?;

        let new = input["new_string"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs {
                tool: "edit_file".to_string(),
                reason: "missing required field 'new_string'".to_string(),
            })?;

        let target = resolve_within_cwd("edit_file", &self.cwd, path_str)?;

        if !target.exists() {
            return Err(ToolError::ExecutionFailed {
                tool: "edit_file".to_string(),
                reason: format!("file not found: {path_str}"),
            });
        }

        let contents = tokio::fs::read_to_string(&target)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "edit_file".to_string(),
                reason: format!("read error: {e}"),
            })?;

        let count = contents.matches(old).count();
        if count == 0 {
            return Err(ToolError::ExecutionFailed {
                tool: "edit_file".to_string(),
                reason: format!("old_string not found in {path_str}"),
            });
        }
        if count > 1 {
            return Err(ToolError::ExecutionFailed {
                tool: "edit_file".to_string(),
                reason: format!(
                    "old_string appears {count} times in {path_str}; it must be unique"
                ),
            });
        }

        let updated = contents.replacen(old, new, 1);
        tokio::fs::write(&target, &updated)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "edit_file".to_string(),
                reason: format!("write error: {e}"),
            })?;

        Ok(format!("Edited {path_str}"))
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

    // ── ReadFileTool ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn read_file_reads_content() {
        let dir = tmp();
        let path = dir.path().join("hello.txt");
        std::fs::write(&path, "line1\nline2\nline3\n").unwrap();

        let tool = ReadFileTool::new(dir.path().to_path_buf());
        let out = tool.execute(json!({ "path": "hello.txt" })).await.unwrap();
        assert_eq!(out, "line1\nline2\nline3\n");
    }

    #[tokio::test]
    async fn read_file_line_range() {
        let dir = tmp();
        std::fs::write(dir.path().join("f.txt"), "a\nb\nc\nd\n").unwrap();

        let tool = ReadFileTool::new(dir.path().to_path_buf());
        let out = tool
            .execute(json!({ "path": "f.txt", "start_line": 2, "end_line": 3 }))
            .await
            .unwrap();
        assert_eq!(out, "b\nc");
    }

    #[tokio::test]
    async fn read_file_not_found() {
        let dir = tmp();
        let tool = ReadFileTool::new(dir.path().to_path_buf());
        let err = tool
            .execute(json!({ "path": "missing.txt" }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed { .. }));
    }

    #[tokio::test]
    async fn read_file_path_traversal_blocked() {
        let dir = tmp();
        let tool = ReadFileTool::new(dir.path().to_path_buf());
        let err = tool
            .execute(json!({ "path": "../../etc/passwd" }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    // ── WriteFileTool ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn write_file_creates_file() {
        let dir = tmp();
        let tool = WriteFileTool::new(dir.path().to_path_buf());
        tool.execute(json!({ "path": "out.txt", "content": "hello world" }))
            .await
            .unwrap();

        let content = std::fs::read_to_string(dir.path().join("out.txt")).unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn write_file_creates_parent_dirs() {
        let dir = tmp();
        let tool = WriteFileTool::new(dir.path().to_path_buf());
        tool.execute(json!({ "path": "a/b/c.txt", "content": "deep" }))
            .await
            .unwrap();

        let content = std::fs::read_to_string(dir.path().join("a/b/c.txt")).unwrap();
        assert_eq!(content, "deep");
    }

    #[tokio::test]
    async fn write_file_path_traversal_blocked() {
        let dir = tmp();
        let tool = WriteFileTool::new(dir.path().to_path_buf());
        let err = tool
            .execute(json!({ "path": "../../evil.txt", "content": "x" }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    // ── EditFileTool ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn edit_file_replaces_unique_string() {
        let dir = tmp();
        let path = dir.path().join("src.txt");
        std::fs::write(&path, "hello world\n").unwrap();

        let tool = EditFileTool::new(dir.path().to_path_buf());
        tool.execute(json!({
            "path": "src.txt",
            "old_string": "world",
            "new_string": "rust"
        }))
        .await
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello rust\n");
    }

    #[tokio::test]
    async fn edit_file_not_found_is_error() {
        let dir = tmp();
        let tool = EditFileTool::new(dir.path().to_path_buf());
        let err = tool
            .execute(json!({
                "path": "nope.txt",
                "old_string": "x",
                "new_string": "y"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed { .. }));
    }

    #[tokio::test]
    async fn edit_file_string_not_found_is_error() {
        let dir = tmp();
        std::fs::write(dir.path().join("f.txt"), "abc").unwrap();
        let tool = EditFileTool::new(dir.path().to_path_buf());
        let err = tool
            .execute(json!({
                "path": "f.txt",
                "old_string": "xyz",
                "new_string": "def"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed { .. }));
    }

    #[tokio::test]
    async fn edit_file_duplicate_string_is_error() {
        let dir = tmp();
        std::fs::write(dir.path().join("f.txt"), "abc abc").unwrap();
        let tool = EditFileTool::new(dir.path().to_path_buf());
        let err = tool
            .execute(json!({
                "path": "f.txt",
                "old_string": "abc",
                "new_string": "xyz"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed { .. }));
    }
}
