//! File tools - Read, Write, Edit files

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::Tool;

/// Validate path stays within current working directory to prevent path traversal attacks
fn validate_path(path_str: &str) -> anyhow::Result<PathBuf> {
    let cwd = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let requested = Path::new(path_str);

    // Handle absolute paths
    let abs_path = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        cwd.join(requested)
    };

    // Canonicalize to resolve .. and symlinks
    let canonical = abs_path.canonicalize()
        .unwrap_or(abs_path);

    // Verify it's within cwd
    let cwd_canonical = cwd.canonicalize()
        .unwrap_or(cwd);

    if !canonical.starts_with(&cwd_canonical) {
        anyhow::bail!("Access denied: path '{}' is outside the current directory", path_str);
    }

    Ok(canonical)
}

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

        // Validate path to prevent path traversal
        let validated = validate_path(&args[0])?;
        let content = fs::read_to_string(&validated)
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

        // Validate path to prevent path traversal
        let validated = validate_path(&args[0])?;
        let content = &args[1];

        // Ensure parent directory exists
        if let Some(parent) = validated.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        fs::write(&validated, content)
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

        // Validate path to prevent path traversal
        let validated = validate_path(&args[0])?;
        let old_text = &args[1];
        let new_text = &args[2];

        let content = fs::read_to_string(&validated)
            .with_context(|| format!("Failed to read file: {}", args[0]))?;

        if !content.contains(old_text) {
            anyhow::bail!("Text not found in file: {}", old_text);
        }

        // Replace only the first occurrence to avoid unintended multi-replacements
        let new_content = if let Some(pos) = content.find(old_text) {
            let mut result = content.clone();
            result.replace_range(pos..pos + old_text.len(), new_text);
            result
        } else {
            anyhow::bail!("Text not found in file: {}", old_text);
        };

        fs::write(&validated, &new_content)
            .with_context(|| format!("Failed to write file: {}", args[0]))?;

        Ok(format!("Edited {}", args[0]))
    }
}
