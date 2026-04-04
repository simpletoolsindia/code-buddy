//! Sandbox Tool - Execute code safely in isolated environments
//!
//! Supports: python, python3, javascript, node, bash, ruby, php, rust, go
//! Use for validating generated code, testing patches, running snippets.

use anyhow::Result;
use super::Tool;

/// Sandbox tool for safe code execution
pub struct SandboxTool;

impl SandboxTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SandboxTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for SandboxTool {
    fn name(&self) -> &str {
        "Sandbox"
    }

    fn description(&self) -> &str {
        "Execute code in a sandboxed environment with timeout and resource limits. \
Supports: python, python3, javascript, node, bash, ruby, php, rust, go. \
Use for validating generated code, testing snippets, running untrusted code safely. \
Args: <language> <code> [--timeout <seconds>] [--output-limit <chars>]
Example: Sandbox('python', 'print(1 + 2)')
Example: Sandbox('bash', 'echo hello')
Example: Sandbox('python3', '...', '60')"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("Sandbox tool usage:\n\
  Sandbox(<language>, <code>, [timeout_secs], [output_limit])\n\
  Languages: python, python3, javascript, node, bash, ruby, php, rust, go\n\
  Default timeout: 30s, max output: 50000 chars".to_string());
        }

        let language_str = args.first().map(|s| s.as_str()).unwrap_or("bash");
        let code = args.get(1).map(|s| s.as_str()).unwrap_or("");
        let timeout_secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
        let output_limit: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(50000);

        let language = code_buddy::sandbox::Language::from_language_str(language_str)
            .unwrap_or(code_buddy::sandbox::Language::Bash);

        let mut config = code_buddy::sandbox::SandboxConfig::default();
        config.timeout_secs = timeout_secs;
        config.max_output_chars = output_limit;

        let result = code_buddy::sandbox::execute_code_sync(code, &language, &config);

        let output = serde_json::to_string_pretty(&serde_json::json!({
            "success": result.success,
            "language": result.language,
            "exit_code": result.exit_code,
            "duration_ms": result.duration_ms,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "truncated": result.truncated,
            "error": result.error,
        }))?;

        Ok(output)
    }
}
