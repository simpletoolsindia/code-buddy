//! Tool executor - Runs tools and returns results

use anyhow::Result;
use std::process::Command;
use std::path::Path;
use std::fs;

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

impl ToolResult {
    pub fn success(output: String) -> Self {
        Self {
            success: true,
            output,
            error: None,
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(msg),
        }
    }

    pub fn to_content(&self) -> String {
        if self.success {
            self.output.clone()
        } else {
            format!("Error: {}", self.error.as_ref().unwrap_or(&"Unknown error".to_string()))
        }
    }
}

/// Execute a tool by name with arguments
pub fn execute_tool(name: &str, args: &[String], _bypass_permissions: bool) -> ToolResult {
    match name {
        "bash" | "shell" | "run" => execute_bash(args),
        "read" | "cat" => execute_read(args),
        "write" | "create" => execute_write(args),
        "edit" => execute_edit(args),
        "glob" => execute_glob(args),
        _ => ToolResult::error(format!("Unknown tool: {}", name)),
    }
}

fn execute_bash(args: &[String]) -> ToolResult {
    if args.is_empty() {
        return ToolResult::error("Bash tool requires a command argument".to_string());
    }

    let command = args.join(" ");
    let output = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if output.status.success() {
                if stdout.is_empty() && !stderr.is_empty() {
                    ToolResult::success(stderr.to_string())
                } else {
                    ToolResult::success(stdout.to_string())
                }
            } else {
                let exit_code = output.status.code().unwrap_or(-1);
                ToolResult::success(format!(
                    "Exit code {}:\n{}\n{}",
                    exit_code,
                    stderr,
                    stdout
                ))
            }
        }
        Err(e) => ToolResult::error(format!("Failed to execute command: {}", e)),
    }
}

fn execute_read(args: &[String]) -> ToolResult {
    if args.is_empty() {
        return ToolResult::error("Read tool requires a file path argument".to_string());
    }

    let path = Path::new(&args[0]);
    if !path.exists() {
        return ToolResult::error(format!("File not found: {}", args[0]));
    }

    match fs::read_to_string(path) {
        Ok(content) => {
            let max_len = 10000;
            if content.len() > max_len {
                ToolResult::success(format!(
                    "{}...\n\n[Content truncated - {} chars total]",
                    &content[..max_len],
                    content.len()
                ))
            } else {
                ToolResult::success(content)
            }
        }
        Err(e) => ToolResult::error(format!("Failed to read file: {}", e)),
    }
}

fn execute_write(args: &[String]) -> ToolResult {
    if args.len() < 2 {
        return ToolResult::error("Write tool requires: <file_path> <content>".to_string());
    }

    let path = Path::new(&args[0]);
    let content = args[1..].join(" ");

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = fs::create_dir_all(parent) {
                return ToolResult::error(format!("Failed to create directory: {}", e));
            }
        }
    }

    match fs::write(path, &content) {
        Ok(_) => ToolResult::success(format!("File written: {}", args[0])),
        Err(e) => ToolResult::error(format!("Failed to write file: {}", e)),
    }
}

fn execute_edit(args: &[String]) -> ToolResult {
    if args.len() < 3 {
        return ToolResult::error("Edit tool requires: <file_path> <old_text> <new_text>".to_string());
    }

    let path = Path::new(&args[0]);
    if !path.exists() {
        return ToolResult::error(format!("File not found: {}", args[0]));
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return ToolResult::error(format!("Failed to read file: {}", e)),
    };

    let old_text = &args[1];
    let new_text = &args[2..].join(" ");

    if !content.contains(old_text) {
        return ToolResult::error(format!("Text not found in file: {}", old_text));
    }

    let new_content = content.replace(old_text, &new_text);

    match fs::write(path, &new_content) {
        Ok(_) => ToolResult::success(format!("File edited: {}", args[0])),
        Err(e) => ToolResult::error(format!("Failed to write file: {}", e)),
    }
}

fn execute_glob(args: &[String]) -> ToolResult {
    if args.is_empty() {
        return ToolResult::error("Glob tool requires a pattern argument".to_string());
    }

    let pattern = &args[0];
    let output = Command::new("find")
        .arg(".")
        .arg("-name")
        .arg(pattern)
        .arg("!")
        .arg("-path")
        .arg("./target/*")
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.is_empty() {
                ToolResult::success("No files found".to_string())
            } else {
                ToolResult::success(stdout.to_string())
            }
        }
        Err(e) => ToolResult::error(format!("Failed to glob: {}", e)),
    }
}

/// Get available tools as a description string for the LLM
pub fn get_tools_description() -> String {
    r#"You have access to the following tools:

bash: Execute shell commands
  Usage: bash("command here")
  Example: bash("ls -la")

read: Read file contents
  Usage: read("/path/to/file")

write: Create or overwrite a file
  Usage: write("/path/to/file", "file content here")

edit: Edit specific text in a file
  Usage: edit("/path/to/file", "old text", "new text")

glob: Find files by pattern
  Usage: glob("*.rs")
"#.to_string()
}
