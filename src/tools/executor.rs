//! Tool executor - Runs tools and returns results (Claude Code style)

use std::process::Command;
use std::path::Path;
use std::fs;
use tracing::{debug, warn, instrument};

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
#[instrument(skip(args), fields(tool = %name))]
pub fn execute_tool(name: &str, args: &[String], _bypass_permissions: bool) -> ToolResult {
    debug!("Executing tool: {} with args: {:?}", name, args);
    match name {
        "bash" | "run" | "execute" | "shell" => execute_bash(args),
        "read" | "cat" | "file_read" => execute_read(args),
        "write" | "file_write" | "create" => execute_write(args),
        "edit" | "file_edit" | "patch" => execute_edit(args),
        "glob" | "find" | "ls" => execute_glob(args),
        "grep" | "rg" => execute_grep(args),
        "webfetch" | "web_fetch" | "fetch" | "curl" | "wget" => execute_webfetch(args),
        "websearch" | "web_search" => execute_websearch(args),
        "mkdir" | "directory" => execute_mkdir(args),
        "rm" | "delete" | "remove" => execute_rm(args),
        "cp" | "copy" => execute_cp(args),
        "mv" | "move" | "rename" => execute_mv(args),
        _ => ToolResult::error(format!("Unknown tool: {}", name)),
    }
}

pub(crate) fn execute_bash(args: &[String]) -> ToolResult {
    if args.is_empty() {
        return ToolResult::error("bash: command argument required".to_string());
    }

    let command = args.join(" ");

    // Basic security check - block dangerous patterns
    let cmd_lower = command.to_lowercase();
    if cmd_lower.contains("rm -rf /") || cmd_lower.contains("; rm -rf") || cmd_lower.contains("mkfs") {
        warn!("Blocked dangerous command: {}", command);
        return ToolResult::error("Blocked dangerous command pattern".to_string());
    }

    let output = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if output.status.success() {
                // Command succeeded
                let result = if stdout.is_empty() {
                    if stderr.is_empty() {
                        "Done".to_string()
                    } else {
                        stderr.to_string()
                    }
                } else {
                    stdout.to_string()
                };
                ToolResult::success(result)
            } else {
                // Command failed
                let exit_code = output.status.code().unwrap_or(-1);
                let error_msg = if stdout.is_empty() && !stderr.is_empty() {
                    stderr.to_string()
                } else {
                    format!("Exit code {}: {}", exit_code, stdout)
                };
                ToolResult::success(error_msg)
            }
        }
        Err(e) => ToolResult::error(format!("Failed to execute: {}", e)),
    }
}

pub(crate) fn execute_read(args: &[String]) -> ToolResult {
    if args.is_empty() {
        return ToolResult::error("read: file path required".to_string());
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
                    "{}...\n\n[Truncated - {} chars total]",
                    &content[..max_len],
                    content.len()
                ))
            } else {
                ToolResult::success(content)
            }
        }
        Err(e) => ToolResult::error(format!("Failed to read: {}", e)),
    }
}

pub(crate) fn execute_write(args: &[String]) -> ToolResult {
    if args.len() < 2 {
        return ToolResult::error("write: requires <file_path> <content>".to_string());
    }

    let path = Path::new(&args[0]);
    let content = args[1..].join(" ");

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return ToolResult::error(format!("Failed to create directory: {}", e));
            }
        }
    }

    match fs::write(path, &content) {
        Ok(_) => ToolResult::success(format!("Wrote {} bytes to {}", content.len(), args[0])),
        Err(e) => ToolResult::error(format!("Failed to write: {}", e)),
    }
}

pub(crate) fn execute_edit(args: &[String]) -> ToolResult {
    if args.len() < 3 {
        return ToolResult::error("edit: requires <file_path> <old_text> <new_text>".to_string());
    }

    let path = Path::new(&args[0]);
    if !path.exists() {
        return ToolResult::error(format!("File not found: {}", args[0]));
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return ToolResult::error(format!("Failed to read: {}", e)),
    };

    let old_text = &args[1];
    let new_text = &args[2..].join(" ");

    if !content.contains(old_text) {
        return ToolResult::error(format!("Text not found in file: {}", old_text));
    }

    let new_content = content.replace(old_text, new_text);

    match fs::write(path, &new_content) {
        Ok(_) => ToolResult::success(format!("Edited {}", args[0])),
        Err(e) => ToolResult::error(format!("Failed to write: {}", e)),
    }
}

pub(crate) fn execute_glob(args: &[String]) -> ToolResult {
    let pattern = args.first().map(|s| s.as_str()).unwrap_or("*");

    // Use find command for glob-like behavior
    let output = Command::new("find")
        .args([".", "-name", pattern])
        .arg("!")
        .arg("-path")
        .arg("./target")
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.is_empty() {
                ToolResult::success("No files found".to_string())
            } else {
                // Clean up output
                let files: Vec<&str> = stdout.lines()
                    .filter(|l| !l.is_empty())
                    .collect();
                ToolResult::success(files.join("\n"))
            }
        }
        Err(e) => ToolResult::error(format!("Glob failed: {}", e)),
    }
}

pub(crate) fn execute_grep(args: &[String]) -> ToolResult {
    if args.is_empty() {
        return ToolResult::error("grep: pattern required".to_string());
    }

    let pattern = &args[0];
    let path = args.get(1).map(|s| s.as_str()).unwrap_or(".");

    // Use ripgrep if available, fallback to grep
    let (cmd, grep_args) = if Command::new("rg").arg("--version").output().is_ok() {
        ("rg", vec!["-n", pattern, path])
    } else {
        ("grep", vec!["-n", "-r", pattern, path])
    };

    let output = Command::new(cmd)
        .args(grep_args)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if stdout.is_empty() {
                if stderr.is_empty() {
                    ToolResult::success("No matches found".to_string())
                } else {
                    ToolResult::success(stderr.to_string())
                }
            } else {
                ToolResult::success(stdout.to_string())
            }
        }
        Err(e) => ToolResult::error(format!("Grep failed: {}", e)),
    }
}

pub(crate) fn execute_webfetch(args: &[String]) -> ToolResult {
    if args.is_empty() {
        return ToolResult::error("webfetch: URL required".to_string());
    }

    let url = &args[0];

    let output = Command::new("curl")
        .args(["-s", "-L", "--max-time", "30", url])
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let max_len = 8000;
            if stdout.len() > max_len {
                ToolResult::success(format!("{}\n\n[Truncated - {} chars total]",
                    &stdout[..max_len], stdout.len()))
            } else {
                ToolResult::success(stdout.to_string())
            }
        }
        Err(e) => ToolResult::error(format!("Web fetch failed: {}", e)),
    }
}

pub(crate) fn execute_websearch(args: &[String]) -> ToolResult {
    if args.is_empty() {
        return ToolResult::error("websearch: query required".to_string());
    }

    // Use ddg (duckduckgo) or fallback to curl
    let query = args.join(" ");

    // Try using curl to search
    let url = format!("https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(&query));

    let output = Command::new("curl")
        .args(["-s", "-L", "--max-time", "30", &url])
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse basic HTML results
            let results: Vec<String> = stdout.lines()
                .filter(|l| l.contains("<a class=\"result__a\""))
                .take(5)
                .map(|l| {
                    // Extract title from HTML
                    l.lines()
                        .find(|s| s.contains("<a class=\"result__a\""))
                        .map(|s| s.replace("<a class=\"result__a\" href=\"", "")
                               .replace("\">", ": ")
                               .replace("</a>", "")
                               .replace("&amp;", "&")
                               .replace("&lt;", "<")
                               .replace("&gt;", ">"))
                        .unwrap_or_default()
                })
                .filter(|s| !s.is_empty())
                .collect();

            if results.is_empty() {
                ToolResult::success("No search results found".to_string())
            } else {
                ToolResult::success(results.join("\n"))
            }
        }
        Err(e) => ToolResult::error(format!("Web search failed: {}", e)),
    }
}

pub(crate) fn execute_mkdir(args: &[String]) -> ToolResult {
    if args.is_empty() {
        return ToolResult::error("mkdir: directory path required".to_string());
    }

    let path = Path::new(&args[0]);

    match fs::create_dir_all(path) {
        Ok(_) => ToolResult::success(format!("Created directory: {}", args[0])),
        Err(e) => ToolResult::error(format!("Failed to create directory: {}", e)),
    }
}

pub(crate) fn execute_rm(args: &[String]) -> ToolResult {
    if args.is_empty() {
        return ToolResult::error("rm: file path required".to_string());
    }

    let path = Path::new(&args[0]);
    let recursive = args.contains(&"-r".to_string()) || args.contains(&"-rf".to_string());

    let result = if path.is_dir() && recursive {
        fs::remove_dir_all(path)
    } else if path.is_dir() {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    };

    match result {
        Ok(_) => ToolResult::success(format!("Removed: {}", args[0])),
        Err(e) => ToolResult::error(format!("Failed to remove: {}", e)),
    }
}

pub(crate) fn execute_cp(args: &[String]) -> ToolResult {
    if args.len() < 2 {
        return ToolResult::error("cp: requires <source> <destination>".to_string());
    }

    let src = Path::new(&args[0]);
    let dst = Path::new(&args[1]);

    match fs::copy(src, dst) {
        Ok(_) => ToolResult::success(format!("Copied {} to {}", args[0], args[1])),
        Err(e) => ToolResult::error(format!("Failed to copy: {}", e)),
    }
}

pub(crate) fn execute_mv(args: &[String]) -> ToolResult {
    if args.len() < 2 {
        return ToolResult::error("mv: requires <source> <destination>".to_string());
    }

    let src = Path::new(&args[0]);
    let dst = Path::new(&args[1]);

    match fs::rename(src, dst) {
        Ok(_) => ToolResult::success(format!("Moved {} to {}", args[0], args[1])),
        Err(e) => ToolResult::error(format!("Failed to move: {}", e)),
    }
}

/// Get available tools as a description string for the LLM
pub fn get_tools_description() -> String {
    r#"You have access to the following tools:

bash(command: string) - Execute shell commands
  Example: bash("ls -la")
  Example: bash("python -m http.server 8080")

write(path: string, content: string) - Create or overwrite a file
  Example: write("/path/to/file.html", "<html><body>Hello</body></html>")

read(path: string) - Read file contents
  Example: read("/path/to/file.txt")

edit(path: string, old_text: string, new_text: string) - Edit specific text in a file
  Example: edit("file.txt", "old text", "new text")

glob(pattern: string) - Find files matching pattern
  Example: glob("*.js")
  Example: glob("src/**/*.rs")

grep(pattern: string, path?: string) - Search file contents
  Example: grep("TODO", "src/")
  Example: grep("function")

webfetch(url: string) - Fetch web page content
  Example: webfetch("https://example.com")

websearch(query: string) - Search the web
  Example: websearch("rust async tutorial")

mkdir(path: string) - Create directory
  Example: mkdir("/path/to/new/dir")

rm(path: string) - Remove file or directory
  Example: rm("/path/to/file.txt")
  Example: rm("-r", "/path/to/dir")

cp(source: string, dest: string) - Copy file
  Example: cp("/src/file.txt", "/dst/file.txt")

mv(source: string, dest: string) - Move/rename file
  Example: mv("/old/path.txt", "/new/path.txt")

IMPORTANT:
- Always use tools to DO tasks, not just describe them
- After starting a server, report the URL
- Check if files exist before reading
"#.to_string()
}
