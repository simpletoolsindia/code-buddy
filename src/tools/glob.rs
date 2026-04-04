//! Glob tool - Find files by pattern

use anyhow::{Context, Result};
use walkdir::WalkDir;

use super::Tool;

/// Glob tool for finding files
pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn description(&self) -> &str {
        "Find files by glob pattern"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            anyhow::bail!("Glob tool requires a pattern argument");
        }

        let pattern = &args[0];
        let base_path = args.get(1).map(|p| p.as_str()).unwrap_or(".");

        // Parse glob pattern
        let glob_pattern = glob::Pattern::new(pattern)
            .context("Invalid glob pattern")?;

        let mut matches = Vec::new();

        for entry in WalkDir::new(base_path)
            .max_depth(10)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    if glob_pattern.matches(name) {
                        matches.push(entry.path().display().to_string());
                    }
                }
            }
        }

        matches.sort();
        Ok(matches.join("\n"))
    }
}
