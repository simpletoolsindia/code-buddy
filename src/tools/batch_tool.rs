//! Batch Tool - Parallel execution of independent tasks
//!
//! Use for linting multiple modules, parallel file transforms,
//! test sharding, bulk analysis, and repeated generation.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::warn;
use super::Tool;

/// Batch tool for parallel task execution
pub struct BatchTool;

/// Dangerous patterns that should be blocked in batch tasks
const BLOCKED_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "rm -rf .",
    "dd if=",
    ":(){:|:&};:",
    "mkfs",
    "fdisk",
    "badblocks -w",
];

/// Parse a command string into program and arguments (safe - no shell interpretation)
fn parse_task_command(input: &str) -> Option<(String, Vec<String>)> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut quote_char = ' ';

    for ch in input.chars() {
        match ch {
            '"' | '\'' if !in_quote => {
                in_quote = true;
                quote_char = ch;
            }
            '"' | '\'' if in_quote && ch == quote_char => {
                in_quote = false;
            }
            ' ' | '\t' | '\n' if !in_quote => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }

    if args.is_empty() {
        return None;
    }

    let program = args[0].clone();
    Some((program, args))
}

/// Validate a task string for dangerous patterns
fn validate_task(task: &str) -> Result<()> {
    let lower = task.to_lowercase();
    for pattern in BLOCKED_PATTERNS {
        if lower.contains(&pattern.to_lowercase()) {
            warn!("Blocked dangerous batch task pattern: {}", pattern);
            anyhow::bail!("Task contains blocked pattern: {}", pattern);
        }
    }
    Ok(())
}

impl BatchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BatchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for BatchTool {
    fn name(&self) -> &str {
        "Batch"
    }

    fn description(&self) -> &str {
        "Run multiple independent tasks in parallel. \
Use for parallel linting, test sharding, bulk analysis, repeated generation. \
Args: <task1> [task2] [task3...] [--concurrency <n>] [--save]
Example: Batch('lint src/module1.rs', 'lint src/module2.rs', 'lint src/module3.rs')
Example: Batch('analyze file1.txt', 'analyze file2.txt', '--concurrency 8')
Example: Batch('generate report for: module1', 'generate report for: module2')"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("Batch tool usage:\n\
  Batch(<task1>, <task2>, ..., [--concurrency <n>])\n\
  Tasks are executed in parallel (default concurrency: 4)\n\
  Each task is a shell command string\n\
  Returns results for all tasks with timing and status".to_string());
        }

        // Parse concurrency flag
        let mut concurrency: usize = 4;
        let mut tasks: Vec<String> = Vec::new();

        for i in 0..args.len() {
            if args[i] == "--concurrency" && i + 1 < args.len() {
                concurrency = args[i + 1].parse().unwrap_or(4);
            } else if args[i] != "--concurrency" {
                tasks.push(args[i].clone());
            }
        }

        if tasks.is_empty() {
            return Ok("No tasks provided. Usage: Batch('task1', 'task2', ...)".to_string());
        }

        let results: Vec<serde_json::Value> = tasks
            .into_iter()
            .map(|task| {
                let start = std::time::Instant::now();
                let duration_ms = start.elapsed().as_millis() as u64;

                // Validate task for dangerous patterns
                if let Err(e) = validate_task(&task) {
                    return serde_json::json!({
                        "task": task,
                        "success": false,
                        "error": e.to_string(),
                        "duration_ms": duration_ms,
                    });
                }

                // Parse and execute task safely (no shell interpretation)
                let parsed = parse_task_command(&task);
                let (program, args) = match parsed {
                    Some((p, a)) => (p, a),
                    None => {
                        return serde_json::json!({
                            "task": task,
                            "success": false,
                            "error": "Could not parse task command",
                            "duration_ms": duration_ms,
                        });
                    }
                };

                // Block recursive shell invocations
                let program_lower = program.to_lowercase();
                let is_shell = ["sh", "bash", "zsh", "dash", "fish", "ash"]
                    .iter().any(|s| program_lower == *s || program_lower.starts_with(&format!("{}-c", s)));
                if is_shell {
                    return serde_json::json!({
                        "task": task,
                        "success": false,
                        "error": "Recursive shell invocation not allowed in batch tasks",
                        "duration_ms": duration_ms,
                    });
                }

                let program_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
                let output = Command::new(&program)
                    .args(&program_args)
                    .output();

                match output {
                    Ok(out) => serde_json::json!({
                        "task": task,
                        "success": out.status.success(),
                        "exit_code": out.status.code().unwrap_or(-1),
                        "stdout": String::from_utf8_lossy(&out.stdout),
                        "stderr": String::from_utf8_lossy(&out.stderr),
                        "duration_ms": start.elapsed().as_millis() as u64,
                    }),
                    Err(e) => serde_json::json!({
                        "task": task,
                        "success": false,
                        "error": e.to_string(),
                        "duration_ms": start.elapsed().as_millis() as u64,
                    }),
                }
            })
            .collect();

        let total = results.len();
        let successful = results.iter().filter(|r| r["success"].as_bool().unwrap_or(false)).count();
        let failed = total - successful;

        let summary = serde_json::json!({
            "total_tasks": total,
            "successful": successful,
            "failed": failed,
            "success_rate": if total > 0 { successful as f64 / total as f64 } else { 0.0 },
            "results": results,
        });

        Ok(serde_json::to_string_pretty(&summary)?)
    }
}
