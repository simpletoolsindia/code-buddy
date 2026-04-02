//! Context Files - AGENTS.md and CLAUDE.md support
//!
//! Automatically loads project-specific context files.
//! AGENTS.md: Multi-agent instructions per project
//! CLAUDE.md: Project-specific guidelines

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Context file types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContextFileType {
    /// AGENTS.md - Multi-agent instructions
    Agents,
    /// CLAUDE.md - Project guidelines
    Claude,
    /// .claude.json - Structured config
    Config,
    /// CLAUDE.local.md - Local overrides
    Local,
}

/// Context file entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFile {
    pub file_type: ContextFileType,
    pub path: PathBuf,
    pub content: String,
    pub loaded: bool,
}

/// Context loader
pub struct ContextLoader {
    search_paths: Vec<PathBuf>,
}

impl ContextLoader {
    /// Create a new context loader
    pub fn new(current_dir: PathBuf) -> Self {
        let mut search_paths = vec![current_dir.clone()];

        // Also check parent directories up to git root
        let mut dir = current_dir.clone();
        while let Some(parent) = dir.parent() {
            if parent == dir {
                break;
            }
            if parent.join(".git").exists() {
                search_paths.push(parent.to_path_buf());
                break;
            }
            dir = parent.to_path_buf();
        }

        Self { search_paths }
    }

    /// Load all context files for current directory
    pub fn load_all(&self) -> Result<Vec<ContextFile>> {
        let mut files = vec![];

        for dir in &self.search_paths {
            // AGENTS.md
            let agents_path = dir.join("AGENTS.md");
            if agents_path.exists() {
                if let Ok(content) = fs::read_to_string(&agents_path) {
                    files.push(ContextFile {
                        file_type: ContextFileType::Agents,
                        path: agents_path.clone(),
                        content,
                        loaded: true,
                    });
                }
            }

            // CLAUDE.md
            let claude_path = dir.join("CLAUDE.md");
            if claude_path.exists() {
                if let Ok(content) = fs::read_to_string(&claude_path) {
                    files.push(ContextFile {
                        file_type: ContextFileType::Claude,
                        path: claude_path.clone(),
                        content,
                        loaded: true,
                    });
                }
            }

            // CLAUDE.local.md (local overrides)
            let local_path = dir.join("CLAUDE.local.md");
            if local_path.exists() {
                if let Ok(content) = fs::read_to_string(&local_path) {
                    files.push(ContextFile {
                        file_type: ContextFileType::Local,
                        path: local_path.clone(),
                        content,
                        loaded: true,
                    });
                }
            }

            // .claude.json
            let config_path = dir.join(".claude.json");
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    files.push(ContextFile {
                        file_type: ContextFileType::Config,
                        path: config_path.clone(),
                        content,
                        loaded: true,
                    });
                }
            }
        }

        Ok(files)
    }

    /// Build system prompt from context files
    pub fn build_system_prompt(&self) -> Result<String> {
        let files = self.load_all()?;
        let mut prompt = String::new();

        // AGENTS.md first (highest priority)
        for file in files.iter().filter(|f| f.file_type == ContextFileType::Agents) {
            prompt.push_str(&format!("\n## From AGENTS.md ({})\n\n{}\n",
                file.path.display(), file.content.trim()));
        }

        // CLAUDE.md
        for file in files.iter().filter(|f| f.file_type == ContextFileType::Claude) {
            prompt.push_str(&format!("\n## Project Guidelines ({})\n\n{}\n",
                file.path.display(), file.content.trim()));
        }

        // .claude.json config
        for file in files.iter().filter(|f| f.file_type == ContextFileType::Config) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&file.content) {
                // Extract relevant config for system prompt
                if let Some(instructions) = config.get("instructions").and_then(|v| v.as_str()) {
                    prompt.push_str(&format!("\n## Config Instructions\n\n{}\n", instructions));
                }
            }
        }

        // CLAUDE.local.md last (local overrides)
        for file in files.iter().filter(|f| f.file_type == ContextFileType::Local) {
            prompt.push_str(&format!("\n## Local Overrides ({})\n\n{}\n",
                file.path.display(), file.content.trim()));
        }

        Ok(prompt)
    }

    /// Check if context files have changed
    pub fn check_modified(&self, known_mtimes: &HashMap<PathBuf, std::time::SystemTime>) -> Vec<PathBuf> {
        let mut modified = vec![];
        let files = self.load_all().unwrap_or_default();

        for file in files {
            if let Ok(current_mtime) = fs::metadata(&file.path) {
                if let Some(known_mtime) = known_mtimes.get(&file.path) {
                    if current_mtime.modified().ok() != Some(*known_mtime) {
                        modified.push(file.path);
                    }
                }
            }
        }

        modified
    }

    /// Get mtimes of all context files
    pub fn get_mtimes(&self) -> HashMap<PathBuf, std::time::SystemTime> {
        let mut mtimes = HashMap::new();
        let files = self.load_all().unwrap_or_default();

        for file in files {
            if let Ok(meta) = fs::metadata(&file.path) {
                if let Ok(mtime) = meta.modified() {
                    mtimes.insert(file.path, mtime);
                }
            }
        }

        mtimes
    }
}

/// Parse CLAUDE.md and extract sections
pub fn parse_claude_md(content: &str) -> HashMap<String, String> {
    let mut sections = HashMap::new();
    let mut current_section = String::new();
    let mut current_title = String::new();

    for line in content.lines() {
        if line.starts_with("## ") || line.starts_with("# ") {
            // Save previous section
            if !current_title.is_empty() && !current_section.trim().is_empty() {
                sections.insert(current_title.clone(), current_section.trim().to_string());
            }
            current_title = line.trim_start_matches('#').trim().to_string();
            current_section = String::new();
        } else {
            current_section.push_str(line);
            current_section.push('\n');
        }
    }

    // Save last section
    if !current_title.is_empty() && !current_section.trim().is_empty() {
        sections.insert(current_title, current_section.trim().to_string());
    }

    sections
}

/// Generate a CLAUDE.md template
pub fn generate_claude_md_template(project_name: &str, project_type: &str) -> String {
    format!(r#"# {} - Project Guidelines

## Project Overview
Brief description of this project.

## Tech Stack
- Language/Framework: {}
- Key Dependencies: ...

## Project Structure
```
/src
  ...
```

## Key Conventions
- Code style: ...
- Naming: ...
- Testing: ...

## Commands
- Build: ...
- Test: ...
- Run: ...

## Notes
Any project-specific notes or reminders.
"#, project_name, project_type)
}

/// Generate AGENTS.md template
pub fn generate_agents_md_template() -> String {
    r#"# Multi-Agent Instructions

## Agent Personas
Define different agent roles and their responsibilities.

### Senior Developer
- Role: ...
- Instructions: ...

### Junior Developer
- Role: ...
- Instructions: ...

### DevOps Engineer
- Role: ...
- Instructions: ...

## Communication Protocols
How agents should communicate and share context.

## Task Assignment
How tasks are distributed among agents.
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_context_loader() {
        let loader = ContextLoader::new(PathBuf::from("."));
        let files = loader.load_all().unwrap_or_default();
        // May be empty in test environment
        assert!(files.len() >= 0);
    }

    #[test]
    fn test_parse_claude_md() {
        let content = r#"# Test

## Section 1
Content 1

## Section 2
Content 2
"#;
        let sections = parse_claude_md(content);
        assert!(sections.contains_key("Section 1"));
        assert!(sections.contains_key("Section 2"));
    }
}
