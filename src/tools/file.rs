//! File tools - Read, Write, Edit files

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use super::Tool;

/// Read file tool
pub struct FileRead;

impl FileRead {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileRead {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for FileRead {
    fn name(&self) -> &str {
        "Read"
    }

    fn description(&self) -> &str {
        "Read files from the filesystem"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            anyhow::bail!("Read tool requires a file path");
        }

        let path = Path::new(&args[0]);
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", args[0]))?;

        // Apply line limit if specified
        let limit = args.get(1).and_then(|l| l.parse::<usize>().ok());
        let offset = args.get(2).and_then(|o| o.parse::<usize>().ok()).unwrap_or(0);

        let lines: Vec<&str> = content.lines().skip(offset).collect();
        let lines = if let Some(limit) = limit {
            lines.into_iter().take(limit).collect::<Vec<_>>()
        } else {
            lines
        };

        Ok(lines.join("\n"))
    }
}

/// Write file tool
pub struct FileWrite;

impl FileWrite {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileWrite {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for FileWrite {
    fn name(&self) -> &str {
        "Write"
    }

    fn description(&self) -> &str {
        "Write/overwrite files"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.len() < 2 {
            anyhow::bail!("Write tool requires: <path> <content>");
        }

        let path = Path::new(&args[0]);
        let content = &args[1];

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        fs::write(path, content)
            .with_context(|| format!("Failed to write file: {}", args[0]))?;

        Ok(format!("Written {} bytes to {}", content.len(), args[0]))
    }
}

/// Edit file tool
pub struct FileEdit;

impl FileEdit {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileEdit {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for FileEdit {
    fn name(&self) -> &str {
        "Edit"
    }

    fn description(&self) -> &str {
        "Edit files with line-based changes"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.len() < 3 {
            anyhow::bail!("Edit tool requires: <path> <old_text> <new_text>");
        }

        let path = Path::new(&args[0]);
        let old_text = &args[1];
        let new_text = &args[2];

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", args[0]))?;

        if !content.contains(old_text) {
            anyhow::bail!("Text not found in file: {}", old_text);
        }

        let new_content = content.replace(old_text, new_text);

        fs::write(path, &new_content)
            .with_context(|| format!("Failed to write file: {}", args[0]))?;

        Ok(format!("Edited {}", args[0]))
    }
}
