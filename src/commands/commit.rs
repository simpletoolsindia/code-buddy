//! Commit Command - Smart git commit handling
//!
//! Provides smart commit message generation and git workflow.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Commit options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct CommitOptions {
    pub message: Option<String>,
    pub all: bool,
    pub amend: bool,
    pub no_verify: bool,
    pub dry_run: bool,
    pub sign_off: bool,
}


/// Commit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResult {
    pub success: bool,
    pub commit_hash: Option<String>,
    pub message: String,
    pub files_changed: usize,
}

impl CommitResult {
    pub fn success(hash: &str, files: usize, msg: &str) -> Self {
        Self {
            success: true,
            commit_hash: Some(hash.to_string()),
            message: msg.to_string(),
            files_changed: files,
        }
    }

    pub fn failure(msg: &str) -> Self {
        Self {
            success: false,
            commit_hash: None,
            message: msg.to_string(),
            files_changed: 0,
        }
    }
}

/// Check if we're in a git repository
pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get git status summary
pub fn git_status() -> Result<GitStatus> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()?;

    let content = String::from_utf8_lossy(&output.stdout);
    let mut staged = Vec::new();
    let mut modified = Vec::new();
    let mut untracked = Vec::new();

    for line in content.lines() {
        if line.len() < 3 {
            continue;
        }
        let status = &line[..2];
        let file = line[3..].to_string();

        match status {
            "M" | "A" | "R" | "C" => staged.push(file),
            " D" | "MM" => modified.push(file),
            "??" => untracked.push(file),
            _ => modified.push(file),
        }
    }

    Ok(GitStatus {
        staged,
        modified,
        untracked,
    })
}

/// Git status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub staged: Vec<String>,
    pub modified: Vec<String>,
    pub untracked: Vec<String>,
}

impl GitStatus {
    pub fn is_clean(&self) -> bool {
        self.staged.is_empty() && self.modified.is_empty() && self.untracked.is_empty()
    }

    pub fn summary(&self) -> String {
        let parts: Vec<String> = vec![
            if !self.staged.is_empty() {
                format!("{} staged", self.staged.len())
            } else {
                String::new()
            },
            if !self.modified.is_empty() {
                format!("{} modified", self.modified.len())
            } else {
                String::new()
            },
            if !self.untracked.is_empty() {
                format!("{} untracked", self.untracked.len())
            } else {
                String::new()
            },
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();

        if parts.is_empty() {
            "Working tree clean".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// Get diff summary
pub fn get_diff_summary() -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "--stat"])
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Get staged diff summary
pub fn get_staged_diff_summary() -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--stat"])
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Stage all changes
pub fn stage_all() -> Result<()> {
    Command::new("git")
        .args(["add", "-A"])
        .status()?;
    Ok(())
}

/// Create a git commit
pub fn commit(options: &CommitOptions) -> Result<CommitResult> {
    let status = git_status()?;

    if status.staged.is_empty() && !options.all {
        return Ok(CommitResult::failure(
            "No changes staged. Use 'git add' first or --all flag.",
        ));
    }

    // Build git commit command
    let mut cmd = Command::new("git");
    cmd.arg("commit");

    if options.amend {
        cmd.arg("--amend");
    }

    if options.no_verify {
        cmd.arg("--no-verify");
    }

    if options.sign_off {
        cmd.arg("--signoff");
    }

    if options.dry_run {
        cmd.arg("--dry-run");
    }

    if let Some(ref msg) = options.message {
        cmd.arg("-m").arg(msg);
    } else if options.dry_run {
        // Dry run without message just shows what would be committed
        return Ok(CommitResult {
            success: true,
            commit_hash: None,
            message: "Dry run - no commit made".to_string(),
            files_changed: status.staged.len(),
        });
    } else {
        return Ok(CommitResult::failure("No commit message provided"));
    }

    let output = cmd.output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let hash = extract_commit_hash(&stdout);
        Ok(CommitResult::success(
            &hash,
            status.staged.len(),
            &stdout,
        ))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(CommitResult::failure(&stderr))
    }
}

/// Extract commit hash from output
fn extract_commit_hash(output: &str) -> String {
    for line in output.lines() {
        if line.contains("[master") || line.contains("[main") || line.contains("[HEAD") {
            if let Some(start) = line.find('[') {
                if let Some(end) = line[start..].find(']') {
                    return line[start + 1..start + end].to_string();
                }
            }
        }
    }
    String::from("unknown")
}

/// Get recent commits
pub fn recent_commits(count: usize) -> Result<Vec<CommitInfo>> {
    let output = Command::new("git")
        .args(["log", &format!("-{}", count), "--pretty=format:%H|%s|%an|%ad", "--date=short"])
        .output()?;

    let content = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            commits.push(CommitInfo {
                hash: parts[0].to_string(),
                message: parts[1].to_string(),
                author: parts[2].to_string(),
                date: parts[3].to_string(),
            });
        }
    }

    Ok(commits)
}

/// Commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

impl CommitInfo {
    pub fn short_hash(&self) -> String {
        self.hash[..7].to_string()
    }
}

/// Format commit as markdown
pub fn format_commit(commit: &CommitInfo) -> String {
    format!(
        "- `{}` **{}** - {}\n",
        commit.short_hash(),
        commit.author,
        commit.message
    )
}

/// Run the commit command
pub fn run(options: CommitOptions) -> Result<CommitResult> {
    if !is_git_repo() {
        return Ok(CommitResult::failure("Not in a git repository"));
    }

    let status = git_status()?;
    if status.is_clean() {
        return Ok(CommitResult::failure("Nothing to commit, working tree clean"));
    }

    // Stage all if requested
    if options.all {
        stage_all()?;
    }

    commit(&options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_status_struct() {
        let status = GitStatus {
            staged: vec!["file1.rs".to_string()],
            modified: vec!["file2.rs".to_string()],
            untracked: vec![],
        };
        assert!(!status.is_clean());
        assert_eq!(status.summary(), "1 staged, 1 modified");
    }

    #[test]
    fn test_clean_status() {
        let status = GitStatus {
            staged: vec![],
            modified: vec![],
            untracked: vec![],
        };
        assert!(status.is_clean());
        assert_eq!(status.summary(), "Working tree clean");
    }

    #[test]
    fn test_commit_info() {
        let commit = CommitInfo {
            hash: "abc123def456".to_string(),
            message: "Fix bug".to_string(),
            author: "Test".to_string(),
            date: "2024-01-01".to_string(),
        };
        assert_eq!(commit.short_hash(), "abc123d");
    }
}
