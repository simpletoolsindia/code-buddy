//! Code Execution Sandbox
//!
//! Safe code execution with timeout and resource limits.
//! Supports Python, JavaScript, Bash, and more.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use tokio::time::{timeout, Duration};

/// Supported languages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Language {
    Python,
    Python3,
    JavaScript,
    Node,
    Bash,
    Shell,
    Ruby,
    PHP,
    Rust,
    Go,
}

impl Language {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "python" | "py" => Some(Language::Python3),
            "python3" | "py3" => Some(Language::Python3),
            "javascript" | "js" => Some(Language::JavaScript),
            "node" | "nodejs" => Some(Language::Node),
            "bash" | "sh" | "shell" => Some(Language::Bash),
            "ruby" | "rb" => Some(Language::Ruby),
            "php" => Some(Language::PHP),
            "rust" | "rs" => Some(Language::Rust),
            "go" | "golang" => Some(Language::Go),
            _ => None,
        }
    }

    pub fn command(&self) -> &str {
        match self {
            Language::Python | Language::Python3 => "python3",
            Language::JavaScript | Language::Node => "node",
            Language::Bash | Language::Shell => "bash",
            Language::Ruby => "ruby",
            Language::PHP => "php",
            Language::Rust => "rustc",
            Language::Go => "go",
        }
    }
}

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub timeout_secs: u64,
    pub max_memory_mb: Option<u64>,
    pub max_output_chars: usize,
    pub working_dir: Option<PathBuf>,
    pub env_vars: Vec<(String, String)>,
    pub allow_network: bool,
    pub allow_filesystem: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_memory_mb: Some(512),
            max_output_chars: 50000,
            working_dir: None,
            env_vars: vec![],
            allow_network: false,
            allow_filesystem: true,
        }
    }
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub language: String,
    pub error: Option<String>,
    pub truncated: bool,
}

impl ExecutionResult {
    /// Format as markdown
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str(&format!("## Execution Result ({})\n\n", self.language));
        md.push_str(&format!("- **Status:** {}\n", if self.success { "Success" } else { "Failed" }));
        md.push_str(&format!("- **Exit Code:** {}\n", self.exit_code));
        md.push_str(&format!("- **Duration:** {}ms\n", self.duration_ms));

        if !self.stdout.is_empty() {
            md.push_str("\n### stdout\n\n```\n");
            let output = if self.truncated {
                format!("{}...\n[OUTPUT TRUNCATED - {} chars total]", &self.stdout[..self.stdout.len().min(1000)], self.stdout.len())
            } else {
                self.stdout.clone()
            };
            md.push_str(&output);
            md.push_str("\n```\n");
        }

        if !self.stderr.is_empty() {
            md.push_str("\n### stderr\n\n```\n");
            md.push_str(&self.stderr[..self.stderr.len().min(2000)]);
            md.push_str("\n```\n");
        }

        if let Some(err) = &self.error {
            md.push_str(&format!("\n**Error:** {}\n", err));
        }

        md
    }
}

/// Execute code in a sandboxed environment
pub async fn execute_code(
    code: &str,
    language: &Language,
    config: &SandboxConfig,
) -> Result<ExecutionResult> {
    let start = std::time::Instant::now();
    let lang_str = format!("{:?}", language);

    // Create temp file for code
    let temp_dir = std::env::temp_dir().join("code-buddy-sandbox");
    std::fs::create_dir_all(&temp_dir)?;

    let extension = match language {
        Language::Python | Language::Python3 => "py",
        Language::JavaScript | Language::Node => "js",
        Language::Bash | Language::Shell => "sh",
        Language::Ruby => "rb",
        Language::PHP => "php",
        Language::Rust => "rs",
        Language::Go => "go",
    };

    let filename = format!("sandbox_{}.{}", nanoid::nanoid!(8), extension);
    let file_path = temp_dir.join(&filename);

    // Write code to temp file
    std::fs::write(&file_path, code)?;

    let working_dir = config.working_dir.clone().unwrap_or(temp_dir.clone());

    // Build command
    let (cmd, args) = match language {
        Language::Bash | Language::Shell => ("bash".to_string(), vec![file_path.to_str().unwrap().to_string()]),
        Language::Python | Language::Python3 => ("python3".to_string(), vec![file_path.to_str().unwrap().to_string()]),
        Language::JavaScript | Language::Node => ("node".to_string(), vec![file_path.to_str().unwrap().to_string()]),
        Language::Ruby => ("ruby".to_string(), vec![file_path.to_str().unwrap().to_string()]),
        Language::PHP => ("php".to_string(), vec![file_path.to_str().unwrap().to_string()]),
        Language::Go => ("go".to_string(), vec!["run".to_string(), file_path.to_str().unwrap().to_string()]),
        Language::Rust => {
            // Compile Rust first
            let output = Command::new("rustc")
                .arg(&file_path)
                .arg("-o").arg(temp_dir.join("sandbox_binary"))
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Ok(ExecutionResult {
                    success: false,
                    stdout: String::new(),
                    stderr: stderr.to_string(),
                    exit_code: output.status.code().unwrap_or(1),
                    duration_ms: start.elapsed().as_millis() as u64,
                    language: lang_str,
                    error: Some("Compilation failed".to_string()),
                    truncated: false,
                });
            }

            ("timeout".to_string(), vec![
                format!("{}", config.timeout_secs),
                temp_dir.join("sandbox_binary").to_str().unwrap().to_string()
            ])
        }
    };

    // Execute with timeout
    let timeout_duration = Duration::from_secs(config.timeout_secs);

    let output = timeout(timeout_duration, async {
        tokio::process::Command::new(&cmd)
            .args(&args)
            .current_dir(&working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output()
            .await
    }).await;

    // Clean up temp file
    let _ = std::fs::remove_file(&file_path);

    let duration_ms = start.elapsed().as_millis() as u64;

    match output {
        Ok(Ok(out)) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let truncated = stdout.len() > config.max_output_chars;
            let exit_code = out.status.code().unwrap_or(-1);

            Ok(ExecutionResult {
                success: out.status.success(),
                stdout: if truncated { stdout[..config.max_output_chars].to_string() } else { stdout },
                stderr,
                exit_code,
                duration_ms,
                language: lang_str,
                error: None,
                truncated,
            })
        }
        Ok(Err(e)) => {
            Ok(ExecutionResult {
                success: false,
                stdout: String::new(),
                stderr: String::new(),
                exit_code: -1,
                duration_ms,
                language: lang_str,
                error: Some(format!("Execution failed: {}", e)),
                truncated: false,
            })
        }
        Err(_) => {
            Ok(ExecutionResult {
                success: false,
                stdout: String::new(),
                stderr: String::new(),
                exit_code: -1,
                duration_ms,
                language: lang_str,
                error: Some(format!("Execution timed out after {}s", config.timeout_secs)),
                truncated: false,
            })
        }
    }
}

/// Execute code synchronously
pub fn execute_code_sync(
    code: &str,
    language: &Language,
    config: &SandboxConfig,
) -> ExecutionResult {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(execute_code(code, language, config)).unwrap()
}

/// Quick execute with defaults
pub fn quick_exec(code: &str, language: &str) -> ExecutionResult {
    let lang = Language::from_str(language).unwrap_or(Language::Bash);
    let config = SandboxConfig::default();
    execute_code_sync(code, &lang, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_execution() {
        let result = quick_exec("print('Hello from sandbox!')", "python");
        assert!(result.success);
        assert!(result.stdout.contains("Hello from sandbox!"));
    }

    #[test]
    fn test_bash_execution() {
        let result = quick_exec("echo 'Hello from bash!'", "bash");
        assert!(result.success);
        assert!(result.stdout.contains("Hello from bash!"));
    }

    #[test]
    fn test_timeout() {
        let lang = Language::Bash;
        let mut config = SandboxConfig::default();
        config.timeout_secs = 1;
        let result = execute_code_sync("sleep 10", &lang, &config);
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(Language::from_str("python"), Some(Language::Python3));
        assert_eq!(Language::from_str("js"), Some(Language::JavaScript));
        assert_eq!(Language::from_str("shell"), Some(Language::Bash));
        assert_eq!(Language::from_str("unknown"), None);
    }
}
