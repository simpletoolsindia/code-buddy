//! Container Tool - Execute code in Docker, SSH, Modal, or other backends
//!
//! Use for system dependencies, OS-level isolation, reproducible builds,
//! language toolchains, and remote execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use super::Tool;

/// Container tool for remote/sandboxed execution
pub struct ContainerTool;

/// Validate command to prevent injection attacks
fn validate_command(command: &str) -> anyhow::Result<()> {
    let dangerous_patterns = [
        "; rm -rf",
        "&& rm -rf",
        "|| rm -rf",
        "| rm -rf",
        "&& shutdown",
        "; shutdown",
        "&& halt",
        "; halt",
        "&& poweroff",
        "; poweroff",
        "&& init 0",
        "; init 0",
    ];
    let cmd_lower = command.to_lowercase();
    for pattern in dangerous_patterns {
        if cmd_lower.contains(&pattern.to_lowercase()) {
            anyhow::bail!("Command contains potentially dangerous pattern: {}", pattern);
        }
    }

    // Block shell metacharacters that could enable injection in sh -c contexts
    let shell_metacharacters = ['`', '$', '|', '&', ';', '>', '<', '\n', '\r'];
    for mc in shell_metacharacters {
        if command.contains(mc) {
            anyhow::bail!(
                "Command contains shell metacharacter '{}' which is not allowed for security reasons",
                mc
            );
        }
    }

    Ok(())
}

impl ContainerTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ContainerTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ContainerTool {
    fn name(&self) -> &str {
        "Container"
    }

    fn description(&self) -> &str {
        "Execute commands in isolated containers or remote environments. \
Backends: docker, ssh, modal, daytona, singularity, local. \
Use for builds, integration tests, dependency-heavy workloads, remote machine tasks. \
Args: <backend> <command> [--image <image>] [--host <host>] [--timeout <secs>]
Example: Container('docker', 'cargo build', '--image rust:1.75')
Example: Container('docker', 'npm install && npm test', '--image node:20')
Example: Container('ssh', 'ls -la', '--host user@server')
Example: Container('local', 'make build')"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("Container tool usage:\n\
  Container(<backend>, <command>, [--image <img>], [--host <addr>], [--timeout <secs>])\n\
  Backends: docker, ssh, modal, daytona, singularity, local\n\
  Docker: specify --image (e.g., rust:1.75, node:20, python:3.12)\n\
  SSH: specify --host (e.g., user@server)\n\
  Local: default backend, no extra args needed".to_string());
        }

        let backend = args.first().map(|s| s.to_lowercase()).unwrap_or_default();
        let command = args.get(1).map(|s| s.as_str()).unwrap_or("");

        // Parse optional flags
        let mut image = "ubuntu:latest".to_string();
        let mut host = "localhost".to_string();
        let mut timeout_secs: u64 = 300;

        for i in 2..args.len() {
            match args[i].as_str() {
                "--image" if i + 1 < args.len() => { image = args[i + 1].clone(); }
                "--host" if i + 1 < args.len() => { host = args[i + 1].clone(); }
                "--timeout" if i + 1 < args.len() => {
                    timeout_secs = args[i + 1].parse().unwrap_or(300);
                }
                _ => {}
            }
        }

        // Validate command to prevent injection
        validate_command(command)?;

        let result = match backend.as_str() {
            "local" => {
                let start = std::time::Instant::now();
                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .output()?;
                let duration_ms = start.elapsed().as_millis() as u64;
                serde_json::to_string_pretty(&serde_json::json!({
                    "backend": "local",
                    "success": output.status.success(),
                    "command": command,
                    "exit_code": output.status.code().unwrap_or(-1),
                    "stdout": String::from_utf8_lossy(&output.stdout),
                    "stderr": String::from_utf8_lossy(&output.stderr),
                    "duration_ms": duration_ms,
                }))?
            }
            "docker" => {
                let start = std::time::Instant::now();
                let output = std::process::Command::new("docker")
                    .args(["run", "--rm", "--network=none", &image, "sh", "-c", command])
                    .output()?;
                let duration_ms = start.elapsed().as_millis() as u64;
                serde_json::to_string_pretty(&serde_json::json!({
                    "backend": "docker",
                    "image": image,
                    "success": output.status.success(),
                    "command": command,
                    "exit_code": output.status.code().unwrap_or(-1),
                    "stdout": String::from_utf8_lossy(&output.stdout),
                    "stderr": String::from_utf8_lossy(&output.stderr),
                    "duration_ms": duration_ms,
                }))?
            }
            "ssh" => {
                let start = std::time::Instant::now();
                let output = std::process::Command::new("ssh")
                    .args(["-o", "StrictHostKeyChecking=no", "-o", &format!("ConnectTimeout={}", timeout_secs as u32), &host, command])
                    .output()?;
                let duration_ms = start.elapsed().as_millis() as u64;
                serde_json::to_string_pretty(&serde_json::json!({
                    "backend": "ssh",
                    "host": host,
                    "success": output.status.success(),
                    "command": command,
                    "exit_code": output.status.code().unwrap_or(-1),
                    "stdout": String::from_utf8_lossy(&output.stdout),
                    "stderr": String::from_utf8_lossy(&output.stderr),
                    "duration_ms": duration_ms,
                }))?
            }
            "modal" => {
                serde_json::to_string_pretty(&serde_json::json!({
                    "backend": "modal",
                    "message": "Modal backend requires setup. Install modal Python package: pip install modal",
                    "hint": "Use Container('docker', ...) for local container execution instead"
                }))?
            }
            "daytona" | "singularity" => {
                serde_json::to_string_pretty(&serde_json::json!({
                    "backend": backend,
                    "message": format!("{} backend not yet configured", backend),
                    "hint": "Use Container('docker', ...) or Container('local', ...)"
                }))?
            }
            _ => {
                return Ok(format!("Unknown backend: {}\nBackends: docker, ssh, modal, daytona, singularity, local", backend));
            }
        };

        Ok(result)
    }
}
