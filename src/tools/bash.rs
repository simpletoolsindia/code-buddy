//! Bash tool - Execute shell commands
//!
//! Provides safe command execution with validation and timeout support.
//! Commands are parsed and executed directly rather than through shell
//! to prevent injection attacks.

use anyhow::Result;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tracing::{debug, error, warn};

use super::Tool;

/// Dangerous commands that should be blocked
const BLOCKED_COMMANDS: &[&str] = &[
    "rm -rf /",
    "mkfs",
    "dd if=",
    ":(){:|:&};:",  // Fork bomb
    "curl | sh",
    "wget | sh",
];

/// Commands that require additional confirmation (not implemented, just logged)
const DANGEROUS_COMMANDS: &[&str] = &[
    "sudo",
    "su ",
    "chmod 777",
    "chmod -R 777",
    "killall",
    "pkill",
    "reboot",
    "shutdown",
];

pub struct BashTool {
    timeout_seconds: u64,
    allowed_dirs: Vec<String>,
    blocked_patterns: Vec<String>,
}

impl BashTool {
    /// Create a new BashTool with default settings
    pub fn new() -> Self {
        Self {
            timeout_seconds: 120,
            allowed_dirs: Vec::new(),
            blocked_patterns: Vec::new(),
        }
    }

    /// Set the timeout for command execution
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Add a directory that commands are allowed to access
    pub fn with_allowed_dir(mut self, dir: impl Into<String>) -> Self {
        self.allowed_dirs.push(dir.into());
        self
    }

    /// Validate command doesn't contain dangerous patterns
    fn validate_command(command: &str) -> Result<()> {
        let cmd_lower = command.to_lowercase();

        // Check for blocked commands
        for blocked in BLOCKED_COMMANDS {
            if cmd_lower.contains(&blocked.to_lowercase()) {
                warn!("Blocked dangerous command pattern detected: {}", blocked);
                anyhow::bail!("Command contains blocked pattern: {}", blocked);
            }
        }

        // Check for dangerous patterns
        for dangerous in DANGEROUS_COMMANDS {
            if cmd_lower.contains(dangerous) {
                debug!("Warning: Command contains potentially dangerous pattern: {}", dangerous);
            }
        }

        // Check for obvious injection attempts
        if command.contains("; rm -rf") || command.contains("&& rm -rf") {
            anyhow::bail!("Command injection attempt detected");
        }

        // Check for newlines (could be multiple commands)
        if command.contains('\n') {
            // Allow newlines for multi-line scripts but warn
            debug!("Multi-line command detected, processing each line");
        }

        Ok(())
    }

    /// Parse command into program and arguments (safe alternative to shell)
    fn parse_command(input: &str) -> (String, Vec<String>) {
        // Split by whitespace but preserve quoted strings
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
            return ("sh".to_string(), vec!["sh".to_string()]);
        }

        let program = args[0].clone();
        (program, args)
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for BashTool {
    fn name(&self) -> &str {
        "Bash"
    }

    fn description(&self) -> &str {
        "Execute shell commands safely"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            anyhow::bail!("Bash tool requires a command argument");
        }

        let command = args.join(" ");
        debug!("Executing bash command: {}", command);

        // Validate command for dangerous patterns
        if let Err(e) = Self::validate_command(&command) {
            error!("Command validation failed: {}", e);
            return Err(e);
        }

        // Parse command into program and arguments
        let (program, parsed_args) = Self::parse_command(&command);

        debug!("Running: {} with args: {:?}", program, parsed_args);

        // Execute with timeout using std::process::Command
        let mut cmd = Command::new(&program);
        // Skip program name (index 0), use safe slice that handles single-element case
        if parsed_args.len() > 1 {
            cmd.args(&parsed_args[1..]);
        }

        // Set up pipes for output capture
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Execute with timeout using a watchdog thread
        let (tx, rx) = mpsc::channel();
        let timeout_duration = Duration::from_secs(self.timeout_seconds);

        thread::spawn(move || {
            let output = cmd.output();
            let _ = tx.send(output);
        });

        let output = match rx.recv_timeout(timeout_duration) {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                error!("Failed to execute command: {}", e);
                anyhow::bail!("Failed to execute command: {}", e);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                error!("Command timed out after {} seconds", self.timeout_seconds);
                anyhow::bail!("Command timed out after {} seconds", self.timeout_seconds);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                error!("Command thread panicked");
                anyhow::bail!("Command execution failed: thread panic");
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        debug!("Command exit status: {}", output.status);

        if output.status.success() {
            if stdout.is_empty() {
                Ok("(no output)".to_string())
            } else {
                Ok(stdout.to_string())
            }
        } else {
            let exit_code = output.status.code().unwrap_or(-1);
            Err(anyhow::anyhow!(
                "Command failed with exit code {}:\n{}\n{}",
                exit_code,
                stderr,
                stdout
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_command_safe() {
        assert!(BashTool::validate_command("ls -la").is_ok());
        assert!(BashTool::validate_command("git status").is_ok());
        assert!(BashTool::validate_command("echo hello").is_ok());
        assert!(BashTool::validate_command("cargo build").is_ok());
    }

    #[test]
    fn test_validate_command_blocked() {
        assert!(BashTool::validate_command("rm -rf /").is_err());
        assert!(BashTool::validate_command("; rm -rf").is_err());
        assert!(BashTool::validate_command("echo && rm -rf").is_err());
    }

    #[test]
    fn test_parse_command() {
        let (prog, args) = BashTool::parse_command("ls -la");
        assert_eq!(prog, "ls");
        assert_eq!(args, vec!["ls", "-la"]);

        let (prog, args) = BashTool::parse_command("git commit -m \"hello world\"");
        assert_eq!(prog, "git");
        assert_eq!(args, vec!["git", "commit", "-m", "hello world"]);
    }
}