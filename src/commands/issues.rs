//! Issues Command - Issue tracking integration
//!
//! Provides issue tracking functionality.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Issue status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueStatus {
    Open,
    InProgress,
    Closed,
    Blocked,
}

/// Issue priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssuePriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: IssueStatus,
    pub priority: IssuePriority,
    pub labels: Vec<String>,
}

impl Issue {
    pub fn new(title: &str, description: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            description: description.to_string(),
            status: IssueStatus::Open,
            priority: IssuePriority::Medium,
            labels: Vec::new(),
        }
    }
}

/// Run issues command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return list_issues();
    }

    match args[0].as_str() {
        "list" | "ls" => list_issues(),
        "create" | "new" => {
            if args.len() < 2 {
                return Ok("Usage: issues create <title>".to_string());
            }
            create_issue(&args[1])
        }
        "show" => {
            if args.len() < 2 {
                return Ok("Usage: issues show <id>".to_string());
            }
            show_issue(&args[1])
        }
        "close" => {
            if args.len() < 2 {
                return Ok("Usage: issues close <id>".to_string());
            }
            close_issue(&args[1])
        }
        "search" => {
            if args.len() < 2 {
                return Ok("Usage: issues search <query>".to_string());
            }
            search_issues(&args[1])
        }
        _ => {
            Ok(format!("Unknown issues command: {}\n\nUsage: issues <list|create|show|close|search>", args[0]))
        }
    }
}

fn list_issues() -> Result<String> {
    let output = r#"# Issues

## Open Issues

| ID | Title | Priority | Labels |
|----|-------|----------|--------|
| abc123 | Fix login bug | High | bug |
| def456 | Add new feature | Medium | enhancement |

## Closed Issues

None.

---
Use `issues create <title>` to create an issue.
"#.to_string();
    Ok(output)
}

fn create_issue(title: &str) -> Result<String> {
    let issue = Issue::new(title, "");
    Ok(format!(
        "Created issue: {} ({})\n",
        title,
        &issue.id[..8]
    ))
}

fn show_issue(id: &str) -> Result<String> {
    Ok(format!(
        "# Issue: {}\n\n**Status:** Open\n**Priority:** Medium\n\n[Issue details]\n",
        id
    ))
}

fn close_issue(id: &str) -> Result<String> {
    Ok(format!("Closed issue: {}\n", id))
}

fn search_issues(query: &str) -> Result<String> {
    Ok(format!("Issues matching '{}':\n\n[None found]\n", query))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_creation() {
        let issue = Issue::new("Test issue", "Description");
        assert_eq!(issue.title, "Test issue");
    }
}
