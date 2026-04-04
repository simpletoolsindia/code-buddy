//! Init Command - Project initialization and onboarding
//!
//! Provides project initialization with smart defaults.

use anyhow::Result;
use glob::glob;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Project type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectType {
    Rust,
    Python,
    Node,
    Go,
    Java,
    CSharp,
    TypeScript,
    Generic,
}

impl ProjectType {
    pub fn detect() -> Self {
        let path = std::env::current_dir().unwrap_or_default();

        if path.join("Cargo.toml").exists() {
            return ProjectType::Rust;
        }
        if path.join("pyproject.toml").exists() || path.join("setup.py").exists() || path.join("requirements.txt").exists() {
            return ProjectType::Python;
        }
        if path.join("package.json").exists() {
            if path.join("tsconfig.json").exists() {
                return ProjectType::TypeScript;
            }
            return ProjectType::Node;
        }
        if path.join("go.mod").exists() {
            return ProjectType::Go;
        }
        if path.join("pom.xml").exists() || path.join("build.gradle").exists() {
            return ProjectType::Java;
        }
        if glob(&path.join("*.csproj").to_string_lossy()).ok().is_some_and(|mut g| g.next().is_some()) {
            return ProjectType::CSharp;
        }

        ProjectType::Generic
    }

    pub fn config_file(&self) -> Option<&'static str> {
        match self {
            ProjectType::Rust => Some("claude.json"),
            ProjectType::Python => Some("claude.json"),
            ProjectType::Node => Some("claude.json"),
            ProjectType::Go => Some("claude.json"),
            ProjectType::Java => Some("claude.json"),
            ProjectType::TypeScript => Some("claude.json"),
            ProjectType::CSharp => Some("claude.json"),
            ProjectType::Generic => Some("claude.json"),
        }
    }
}

/// Project initialization options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitOptions {
    pub project_type: Option<ProjectType>,
    pub project_name: Option<String>,
    pub skip_git: bool,
    pub skip_readme: bool,
    pub config_path: PathBuf,
}

/// Initialize a project
pub fn init(options: &InitOptions) -> Result<InitResult> {
    let project_type = options.project_type.clone().unwrap_or_else(ProjectType::detect);
    let project_name = options.project_name.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "my-project".to_string())
    });

    let mut result = InitResult::new(&project_name, project_type.clone());

    // Create CLAUDE.md if it doesn't exist
    let claude_md = PathBuf::from("CLAUDE.md");
    if !claude_md.exists() {
        let content = generate_claude_md(&project_name, &project_type);
        std::fs::write(&claude_md, content)?;
        result.files_created.push("CLAUDE.md".to_string());
    }

    // Create .claude.json if it doesn't exist
    if let Some(config_file) = project_type.config_file() {
        let config_path = options.config_path.join(config_file);
        if !config_path.exists() {
            let content = generate_claude_json(&project_name, &project_type);
            std::fs::write(&config_path, content)?;
            result.files_created.push(config_file.to_string());
        }
    }

    // Initialize git if not skip_git and not already a git repo
    if !options.skip_git {
        let is_git = Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !is_git {
            Command::new("git")
                .args(["init"])
                .output()?;
            result.git_initialized = true;
        }
    }

    // Create README if not skip_readme and doesn't exist
    if !options.skip_readme {
        let readme_md = PathBuf::from("README.md");
        if !readme_md.exists() {
            let content = generate_readme(&project_name, &project_type);
            std::fs::write(&readme_md, content)?;
            result.files_created.push("README.md".to_string());
        }
    }

    Ok(result)
}

/// Initialize result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitResult {
    pub project_name: String,
    pub project_type: ProjectType,
    pub files_created: Vec<String>,
    pub git_initialized: bool,
    pub success: bool,
}

impl InitResult {
    pub fn new(project_name: &str, project_type: ProjectType) -> Self {
        Self {
            project_name: project_name.to_string(),
            project_type,
            files_created: Vec::new(),
            git_initialized: false,
            success: true,
        }
    }
}

use std::process::Command;

fn generate_claude_md(project_name: &str, project_type: &ProjectType) -> String {
    let lang_hints = match project_type {
        ProjectType::Rust => vec!["Rust", "Cargo", "Cargo.toml"],
        ProjectType::Python => vec!["Python", "pyproject.toml", "requirements.txt"],
        ProjectType::Node => vec!["Node.js", "package.json", "npm"],
        ProjectType::TypeScript => vec!["TypeScript", "tsconfig.json", "npm"],
        ProjectType::Go => vec!["Go", "go.mod"],
        _ => vec!["Code", "Project"],
    };

    format!(
        r#"# {project_name}

## Project Overview

## Commands

## Architecture

## Key Files

## Notes
"#
    )
}

fn generate_claude_json(project_name: &str, project_type: &ProjectType) -> String {
    let settings = match project_type {
        ProjectType::Rust => serde_json::json!({
            "projectType": "rust",
            "tools": ["Read", "Write", "Edit", "Bash", "Grep", "Glob"],
        }),
        ProjectType::Python => serde_json::json!({
            "projectType": "python",
            "tools": ["Read", "Write", "Edit", "Bash", "Grep", "Glob"],
        }),
        ProjectType::Node | ProjectType::TypeScript => serde_json::json!({
            "projectType": "node",
            "tools": ["Read", "Write", "Edit", "Bash", "Grep", "Glob"],
        }),
        _ => serde_json::json!({
            "projectType": "generic",
            "tools": ["Read", "Write", "Edit", "Bash", "Grep", "Glob"],
        }),
    };

    serde_json::to_string_pretty(&settings).unwrap_or_default()
}

fn generate_readme(project_name: &str, project_type: &ProjectType) -> String {
    format!(
        r#"# {project_name}

## Getting Started

## Installation

## Usage

## License
"#
    )
}

/// Run init command
pub fn run(options: Option<InitOptions>) -> Result<String> {
    let opts = options.unwrap_or_else(|| InitOptions {
        project_type: None,
        project_name: None,
        skip_git: false,
        skip_readme: false,
        config_path: std::env::current_dir().unwrap_or_default(),
    });

    let result = init(&opts)?;

    let mut output = format!("Initialized {} project: {}\n\n", format!("{:?}", result.project_type), result.project_name);

    if !result.files_created.is_empty() {
        output.push_str("Created files:\n");
        for file in &result.files_created {
            output.push_str(&format!("  - {}\n", file));
        }
        output.push('\n');
    }

    if result.git_initialized {
        output.push_str("Initialized git repository\n\n");
    }

    output.push_str("Project is ready! You can now use Code Buddy to help with your project.\n");

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_type_display() {
        assert_eq!(format!("{:?}", ProjectType::Rust), "Rust");
    }

    #[test]
    fn test_init_result() {
        let result = InitResult::new("test-project", ProjectType::Generic);
        assert_eq!(result.project_name, "test-project");
        assert!(result.success);
    }
}
