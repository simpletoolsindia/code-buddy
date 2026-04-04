//! Tool executor - Runs tools and returns results (Claude Code style)

use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use tracing::{debug, warn, instrument};
use crate::tools::Tool;

/// Get the current working directory, with fallbacks
fn get_cwd() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Normalize a path: resolve relative paths and try to find the file
fn resolve_path(path_str: &str) -> Option<PathBuf> {
    let cwd = get_cwd();
    let path = Path::new(path_str);

    // Already absolute
    if path.is_absolute() && path.exists() {
        return Some(path.to_path_buf());
    }

    // Try as-is relative to cwd
    let abs_path = cwd.join(path_str);
    if abs_path.exists() {
        return Some(abs_path);
    }

    // Try with normalized path (resolve .., etc.)
    let normalized = cwd.join(path).canonicalize().ok();
    if normalized.is_some() {
        return normalized;
    }

    // Try parent directories (useful for src/file.rs when in src/)
    let mut search_path = cwd.clone();
    loop {
        let trial = search_path.join(path_str);
        if trial.exists() {
            return Some(trial);
        }
        if !search_path.pop() {
            break;
        }
    }

    // Try from common project roots
    for root in ["..", "../..", "../../.."] {
        let trial = cwd.join(root).join(path_str);
        if trial.exists() {
            return Some(trial);
        }
    }

    // Return original path anyway - let the actual read fail with clear error
    Some(PathBuf::from(path_str))
}

/// Validate that a path stays within the current working directory.
/// Prevents directory traversal attacks (e.g., "../../../etc").
fn validate_path_within_cwd(path_str: &str) -> anyhow::Result<PathBuf> {
    let cwd = get_cwd();
    let cwd_canonical = cwd.canonicalize()
        .unwrap_or_else(|_| cwd.clone());

    // Join with cwd to handle relative paths
    let joined = cwd.join(path_str);

    // Canonicalize to resolve .. and symlinks
    let resolved = joined.canonicalize()
        .unwrap_or_else(|_| {
            // If canonicalize fails (path doesn't exist yet), normalize manually
            // by resolving .. components from the joined path
            let mut components = joined.components();
            let mut normalized = PathBuf::new();
            for component in components {
                match component {
                    std::path::Component::ParentDir => {
                        normalized.pop();
                    }
                    std::path::Component::Normal(s) => {
                        normalized.push(s);
                    }
                    std::path::Component::RootDir => {
                        normalized = PathBuf::from(std::path::MAIN_SEPARATOR_STR);
                    }
                    _ => {}
                }
            }
            normalized
        });

    // Check the resolved path is within cwd
    let resolved_str = resolved.to_string_lossy();
    let cwd_str = cwd_canonical.to_string_lossy();

    if !resolved_str.starts_with(cwd_str.as_ref()) {
        anyhow::bail!(
            "Path '{}' resolves to '{}' which is outside the current directory. \
             Directory traversal is not allowed.",
            path_str, resolved_str
        );
    }

    Ok(resolved)
}

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
            format!("Error: {}", self.error.as_deref().unwrap_or("Unknown error"))
        }
    }
}

/// Execute a tool by name with arguments
#[instrument(skip(args), fields(tool = %name))]
pub fn execute_tool(name: &str, args: &[String], _bypass_permissions: bool) -> ToolResult {
    debug!("Executing tool: {} with args: {:?}", name, args);
    match name {
        // File/shell tools
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
        // Advanced tools
        "cron" | "crontab" | "schedule" => {
            let result: anyhow::Result<String> = crate::tools::CronTool::new().execute(args);
            match result {
                Ok(r) => ToolResult::success(r),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "sandbox" | "execute_code" | "run_code" => {
            let result: anyhow::Result<String> = crate::tools::SandboxTool::new().execute(args);
            match result {
                Ok(r) => ToolResult::success(r),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "container" | "docker" | "ssh" | "remote_exec" => {
            let result: anyhow::Result<String> = crate::tools::ContainerTool::new().execute(args);
            match result {
                Ok(r) => ToolResult::success(r),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "batch" | "batch_run" | "parallel" => {
            let result: anyhow::Result<String> = crate::tools::BatchTool::new().execute(args);
            match result {
                Ok(r) => ToolResult::success(r),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "mixtureofagents" | "moa" | "ensemble" => {
            let result: anyhow::Result<String> = crate::tools::MoATool::new().execute(args);
            match result {
                Ok(r) => ToolResult::success(r),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "imagegenerate" | "image_gen" | "generate_image" => {
            let result: anyhow::Result<String> = crate::tools::ImageTool::new().execute(args);
            match result {
                Ok(r) => ToolResult::success(r),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "skin" | "theme" | "appearance" => {
            let result: anyhow::Result<String> = crate::tools::SkinTool::new().execute(args);
            match result {
                Ok(r) => ToolResult::success(r),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "profile" | "profiles" => {
            let result: anyhow::Result<String> = crate::tools::ProfileTool::new().execute(args);
            match result {
                Ok(r) => ToolResult::success(r),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "acpserver" | "acp" | "acp-server" | "ide_bridge" => {
            let result: anyhow::Result<String> = crate::tools::AcpServerTool::new().execute(args);
            match result {
                Ok(r) => ToolResult::success(r),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        _ => ToolResult::error(format!("Unknown tool: {}", name)),
    }
}

/// Parse a shell command string into program and arguments, respecting quotes and escapes.
/// This avoids shell injection by not using `sh -c`.
fn parse_command_args(input: &str) -> (String, Vec<String>) {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];

        if escaped {
            // Handle escape sequences
            match c {
                'n' => current.push('\n'),
                't' => current.push('\t'),
                'r' => current.push('\r'),
                _ => current.push(c),
            }
            escaped = false;
            i += 1;
            continue;
        }

        if c == '\\' && !in_single_quote {
            escaped = true;
            i += 1;
            continue;
        }

        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            i += 1;
            continue;
        }

        if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            i += 1;
            continue;
        }

        if c.is_whitespace() && !in_single_quote && !in_double_quote {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
            i += 1;
            continue;
        }

        current.push(c);
        i += 1;
    }

    if !current.is_empty() {
        args.push(current);
    }

    if args.is_empty() {
        return (String::new(), vec![]);
    }

    let program = args.remove(0);
    (program, args)
}

pub(crate) fn execute_bash(args: &[String]) -> ToolResult {
    if args.is_empty() {
        return ToolResult::error("bash: command argument required".to_string());
    }

    // Parse command: if single arg, parse it as shell command string.
    // If multiple args, first is program, rest are arguments.
    let (program, program_args) = if args.len() == 1 {
        let input = args[0].trim();
        if input.is_empty() {
            return ToolResult::error("bash: command argument required".to_string());
        }
        let parsed = parse_command_args(input);
        // Block obviously dangerous patterns even after parsing
        if parsed.0.is_empty() {
            return ToolResult::error("bash: could not parse command".to_string());
        }
        parsed
    } else {
        // Multiple args: first is program, rest are arguments (safe - no shell interpretation)
        let program = args[0].clone();
        let program_args = args[1..].to_vec();
        (program, program_args)
    };

    // Additional safety: block commands that try to invoke shells recursively
    // Check for null bytes (truncation attack via Path::file_name)
    if program.contains('\0') {
        warn!("Blocked null byte in program: {}", program);
        return ToolResult::error("Blocked: Null byte in command not allowed".to_string());
    }

    // Use basename to prevent bypass via paths like /bin/bash
    // Use case-insensitive matching to prevent BA\"SH bypasses
    let dangerous = ["sh", "bash", "zsh", "dash", "fish", "ash"];
    let program_basename = std::path::Path::new(&program)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_lowercase())
        .unwrap_or_else(|| program.to_lowercase());

    if dangerous.iter().any(|d| program_basename == *d || program_basename.starts_with(format!("{}-c", d).as_str())) {
        warn!("Blocked shell invocation: {}", program);
        return ToolResult::error("Blocked: Recursive shell invocation not allowed".to_string());
    }

    // Block dangerous command patterns
    let full_cmd = format!("{} {}", program, program_args.join(" "));
    let dangerous_patterns = [
        "rm -rf /",
        "rm -rf /*",
        "rm -rf .",
        "dd if=",
        ":(){:|:&};:",  // fork bomb
        "mkfs",
        "fdisk",
        "badblocks -w",
    ];
    for pattern in dangerous_patterns {
        if full_cmd.contains(pattern) {
            warn!("Blocked dangerous command: {}", full_cmd);
            return ToolResult::error(format!("Blocked: Dangerous command pattern detected ({})", pattern));
        }
    }

    let output = Command::new(&program)
        .args(&program_args)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if output.status.success() {
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
                let exit_code = output.status.code().unwrap_or(-1);
                let error_msg = if stdout.is_empty() && !stderr.is_empty() {
                    stderr.to_string()
                } else if !stdout.is_empty() {
                    format!("Exit code {}: {}", exit_code, stdout)
                } else {
                    format!("Exit code {}", exit_code)
                };
                ToolResult::error(error_msg)
            }
        }
        Err(e) => ToolResult::error(format!("Failed to execute '{}': {}", program, e)),
    }
}

pub(crate) fn execute_read(args: &[String]) -> ToolResult {
    if args.is_empty() {
        return ToolResult::error("read: file path required".to_string());
    }

    let path_str = &args[0];

    // Validate path stays within current working directory (prevents path traversal)
    let path = match validate_path_within_cwd(path_str) {
        Ok(p) => p,
        Err(e) => {
            return ToolResult::error(format!("read: {}", e));
        }
    };

    // Check if file exists
    if !path.exists() {
        // Try to provide helpful suggestions
        let cwd = get_cwd();
        let suggestion = if path_str.starts_with('/') {
            format!(
                "File not found: {}\n\
                Hint: This absolute path doesn't exist. Current directory: {}\n\
                Try using a relative path or check with 'ls -la'",
                path_str, cwd.display()
            )
        } else {
            // Check if parent exists
            let parent = path.parent();
            if let Some(p) = parent {
                if !p.exists() {
                    format!(
                        "File not found: {}\n\
                        Hint: Parent directory '{}' doesn't exist.\n\
                        Current directory: {}",
                        path_str, p.display(), cwd.display()
                    )
                } else {
                    format!(
                        "File not found: {}\n\
                        Hint: File doesn't exist. Check the filename spelling.\n\
                        Current directory: {}\n\
                        Try: (1) glob('*{}') to find files, (2) bash('find . -name \"{}\"')",
                        path_str, path_str, path_str, cwd.display()
                    )
                }
            } else {
                format!("File not found: {}", path_str)
            }
        };
        return ToolResult::error(suggestion);
    }

    match fs::read_to_string(&path) {
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
        Err(e) => {
            // Provide clearer error for permission issues and try fallback
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                // Try using bash cat as fallback (path is already validated)
                let path_for_cat = path.to_string_lossy().into_owned();
                let cat_output = Command::new("cat")
                    .arg(&path_for_cat)
                    .output();

                if let Ok(cat_result) = cat_output {
                    if cat_result.status.success() {
                        let content = String::from_utf8_lossy(&cat_result.stdout);
                        let max_len = 10000;
                        if content.len() > max_len {
                            return ToolResult::success(format!(
                                "{}...\n\n[Truncated - {} chars total]",
                                &content[..max_len],
                                content.len()
                            ));
                        }
                        return ToolResult::success(content.to_string());
                    }
                }

                ToolResult::error(format!(
                    "Permission denied reading: {}\n\
                    Hint: Check file permissions with 'ls -la {}'",
                    path_str, path_str
                ))
            } else {
                // Try bash cat as fallback for other errors too (path is already validated)
                let path_for_cat = path.to_string_lossy().into_owned();
                let cat_output = Command::new("cat")
                    .arg(&path_for_cat)
                    .output();

                if let Ok(cat_result) = cat_output {
                    if cat_result.status.success() {
                        let content = String::from_utf8_lossy(&cat_result.stdout);
                        let max_len = 10000;
                        if content.len() > max_len {
                            return ToolResult::success(format!(
                                "{}...\n\n[Truncated - {} chars total]",
                                &content[..max_len],
                                content.len()
                            ));
                        }
                        return ToolResult::success(content.to_string());
                    }
                }

                ToolResult::error(format!("Failed to read {}: {}", path_str, e))
            }
        }
    }
}

pub(crate) fn execute_write(args: &[String]) -> ToolResult {
    if args.len() < 2 {
        return ToolResult::error("write: requires <file_path> <content>".to_string());
    }

    // Validate path stays within current working directory
    let validated = match validate_path_within_cwd(&args[0]) {
        Ok(p) => p,
        Err(e) => return ToolResult::error(format!("write: {}", e)),
    };

    let content = args[1..].join(" ");

    // Create parent directories if needed (using validated path)
    if let Some(parent) = validated.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return ToolResult::error(format!("Failed to create directory: {}", e));
            }
        }
    }

    match fs::write(&validated, &content) {
        Ok(_) => ToolResult::success(format!("Wrote {} bytes to {}", content.len(), args[0])),
        Err(e) => ToolResult::error(format!("Failed to write: {}", e)),
    }
}

pub(crate) fn execute_edit(args: &[String]) -> ToolResult {
    if args.len() < 3 {
        return ToolResult::error("edit: requires <file_path> <old_text> <new_text>".to_string());
    }

    let path_str = &args[0];

    // Resolve the path
    let resolved_path = resolve_path(path_str);
    let path = match &resolved_path {
        Some(p) => p,
        None => {
            return ToolResult::error(format!(
                "File not found: {}\nHint: Use glob to find the file first.",
                path_str
            ));
        }
    };

    if !path.exists() {
        let cwd = get_cwd();
        return ToolResult::error(format!(
            "File not found: {}\nCurrent directory: {}\nHint: Try: glob('*{}*') to find the file.",
            path_str, cwd.display(), path_str
        ));
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            // Try bash cat as fallback
            let cat_output = Command::new("cat")
                .arg(path_str)
                .output();

            if let Ok(cat_result) = cat_output {
                if cat_result.status.success() {
                    let content = String::from_utf8_lossy(&cat_result.stdout);
                    let new_content = edit_content(&content, &args[1], &args[2..].join(" "));
                    match new_content {
                        Some(final_content) => {
                            match fs::write(path, &final_content) {
                                Ok(_) => return ToolResult::success(format!("Edited {}", path_str)),
                                Err(e2) => return ToolResult::error(format!("Failed to write: {}", e2)),
                            }
                        }
                        None => return ToolResult::error(format!("Text not found in file: {}", args[1])),
                    }
                }
            }
            return ToolResult::error(format!("Failed to read: {}", e));
        }
    };

    let new_content = edit_content(&content, &args[1], &args[2..].join(" "));

    match new_content {
        Some(final_content) => {
            match fs::write(path, &final_content) {
                Ok(_) => ToolResult::success(format!("Edited {}", path_str)),
                Err(e) => ToolResult::error(format!("Failed to write: {}", e)),
            }
        }
        None => {
            // Provide helpful context about what text looks like in the file
            let preview = content.lines()
                .take(10)
                .enumerate()
                .map(|(i, l)| format!("{:3}: {}", i + 1, l))
                .collect::<Vec<_>>()
                .join("\n");

            ToolResult::error(format!(
                "Text not found in file: {}\n\nFile preview (first 10 lines):\n{}\n\n\
                Hint: Ensure the old_text EXACTLY matches content in file, including whitespace.",
                args[1], preview
            ))
        }
    }
}

/// Helper to perform edit: replaces only the first occurrence of old_text with new_text.
fn edit_content(content: &str, old_text: &str, new_text: &str) -> Option<String> {
    if let Some(pos) = content.find(old_text) {
        let mut result = content.to_string();
        result.replace_range(pos..pos + old_text.len(), new_text);
        Some(result)
    } else {
        None
    }
}

pub(crate) fn execute_glob(args: &[String]) -> ToolResult {
    let pattern = args.first().map(|s| s.as_str()).unwrap_or("*");

    // Handle patterns with directory prefix
    let (search_dir, file_pattern) = if let Some(slash_pos) = pattern.rfind('/') {
        let dir = &pattern[..slash_pos];
        let file = &pattern[slash_pos + 1..];
        // Prevent directory traversal attacks
        if dir.contains("..") {
            return ToolResult::error(format!(
                "Blocked: Directory traversal not allowed in glob pattern '{}'",
                pattern
            ));
        }
        (dir, file)
    } else {
        (".", pattern)
    };

    // Build find command with depth limit and exclude common build/test directories
    // Maxdepth of 10 prevents excessive filesystem traversal
    let output = Command::new("find")
        .arg(search_dir)
        .args(["-maxdepth", "10", "-name", file_pattern])
        .arg("!")
        .arg("-path")
        .arg("*/target/*")
        .arg("!")
        .arg("-path")
        .arg("*/.git/*")
        .arg("!")
        .arg("-path")
        .arg("*/node_modules/*")
        .arg("!")
        .arg("-path")
        .arg("*/.venv/*")
        .arg("!")
        .arg("-path")
        .arg("*/__pycache__/*")
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.is_empty() {
                // Try a broader search
                let broader = Command::new("find")
                    .arg(".")
                    .args(["-name", &format!("*{}*", file_pattern)])
                    .arg("!")
                    .arg("-path")
                    .arg("*/target/*")
                    .output();

                if let Ok(broader_out) = broader {
                    let broader_stdout = String::from_utf8_lossy(&broader_out.stdout);
                    if broader_stdout.is_empty() {
                        ToolResult::success(format!(
                            "No files found matching '{}'\n\
                            Hint: (1) Check spelling, (2) Try a simpler pattern, (3) Use bash('find . -type f')",
                            pattern
                        ))
                    } else {
                        let files: Vec<&str> = broader_stdout.lines()
                            .filter(|l| !l.is_empty())
                            .take(50) // Limit results
                            .collect();
                        ToolResult::success(format!("Found {} files:\n{}", files.len(), files.join("\n")))
                    }
                } else {
                    ToolResult::success(format!("No files found matching '{}'", pattern))
                }
            } else {
                // Clean up output
                let files: Vec<&str> = stdout.lines()
                    .filter(|l| !l.is_empty())
                    .take(50) // Limit results to avoid token waste
                    .collect();
                let count = files.len();
                let listing = files.join("\n");
                if count == 50 {
                    ToolResult::success(format!("Found 50+ files (showing first 50):\n{}\n...", listing))
                } else if count == 1 {
                    ToolResult::success(format!("Found 1 file:\n{}", listing))
                } else {
                    ToolResult::success(format!("Found {} files:\n{}", count, listing))
                }
            }
        }
        Err(e) => {
            // Provide helpful fallback
            let ls_output = Command::new("ls")
                .arg("-la")
                .output();

            if let Ok(ls_result) = ls_output {
                if ls_result.status.success() {
                    ToolResult::error(format!(
                        "Glob failed: {}\n\nCurrent directory contents:\n{}\n\n\
                        Hint: Check current directory with 'ls'",
                        e, String::from_utf8_lossy(&ls_result.stdout)
                    ))
                } else {
                    ToolResult::error(format!("Glob failed: {}", e))
                }
            } else {
                ToolResult::error(format!("Glob failed: {}", e))
            }
        }
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

    // SECURITY: Validate URL to prevent SSRF attacks
    // Only allow http:// and https:// protocols
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return ToolResult::error(
            "webfetch: Only HTTP/HTTPS URLs are allowed. \
             Blocked protocols (file://, ftp://, etc.) are not permitted.".to_string()
        );
    }

    // Block localhost and private IP ranges to prevent SSRF
    // First URL-decode the URL to catch encoded bypasses (e.g. %6c%6f%63%61lhost)
    let decoded_url = urlencoding::decode(url).map(|d| d.to_lowercase()).unwrap_or_else(|_| url.to_lowercase());

    // Block private/internal hostnames and IPs
    if decoded_url.contains("localhost")
        || decoded_url.contains("127.0.0.1")
        || decoded_url.contains("0.0.0.0")
        || decoded_url.contains("[::1]")
        || decoded_url.contains("[::ffff:127.0.0.1]")
        || decoded_url.contains("metadata.google.internal")
        || decoded_url.contains("169.254.169.254")
        || decoded_url.contains("metadata.internal")
        || decoded_url.contains(".internal.")
        || decoded_url.contains(".corp.")
        || decoded_url.contains(".localdomain")
        || decoded_url.contains("metadata.google.internal")
        // Block IPv6 link-local and unique local addresses
        || decoded_url.contains("[fe80:")
        || decoded_url.contains("[fc00:")
        || decoded_url.contains("[fd00:")
        || decoded_url.contains("[fd")
        || decoded_url.contains("[::ffff:127.0.0.1]")
        || decoded_url.contains("[::ffff:0:0]")
    {
        return ToolResult::error(
            "webfetch: Access to localhost and cloud metadata endpoints is blocked for security.".to_string()
        );
    }

    // Also check original URL for already-decoded bypasses
    let url_lower = url.to_lowercase();
    if url_lower.contains("localhost")
        || url_lower.contains("127.0.0.1")
        || url_lower.contains("0.0.0.0")
        || url_lower.contains("[::1]")
        || url_lower.contains("[::ffff:127.0.0.1]")
        || url_lower.contains("metadata.google.internal")
        || url_lower.contains("169.254.169.254")
        || url_lower.contains("metadata.internal")
        || url_lower.contains(".internal.")
        || url_lower.contains(".corp.")
        || url_lower.contains(".localdomain")
        || url_lower.contains("[fe80:")
        || url_lower.contains("[fc00:")
        || url_lower.contains("[fd00:")
        || url_lower.contains("[fd")
    {
        return ToolResult::error(
            "webfetch: Access to localhost and cloud metadata endpoints is blocked for security.".to_string()
        );
    }

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

    // Validate path stays within current working directory
    let validated = match validate_path_within_cwd(&args[0]) {
        Ok(p) => p,
        Err(e) => return ToolResult::error(format!("mkdir: {}", e)),
    };

    match fs::create_dir_all(&validated) {
        Ok(_) => ToolResult::success(format!("Created directory: {}", args[0])),
        Err(e) => ToolResult::error(format!("Failed to create directory: {}", e)),
    }
}

pub(crate) fn execute_rm(args: &[String]) -> ToolResult {
    if args.is_empty() {
        return ToolResult::error("rm: file path required".to_string());
    }

    // Validate path stays within current working directory
    let validated = match validate_path_within_cwd(&args[0]) {
        Ok(p) => p,
        Err(e) => return ToolResult::error(format!("rm: {}", e)),
    };

    let recursive = args.contains(&"-r".to_string()) || args.contains(&"-rf".to_string());

    let result = if validated.is_dir() && recursive {
        fs::remove_dir_all(&validated)
    } else if validated.is_dir() {
        fs::remove_dir(&validated)
    } else {
        fs::remove_file(&validated)
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

    // Validate source path stays within current working directory
    let src_validated = match validate_path_within_cwd(&args[0]) {
        Ok(p) => p,
        Err(e) => return ToolResult::error(format!("cp: {}", e)),
    };

    // Validate destination path stays within current working directory
    let dst_validated = match validate_path_within_cwd(&args[1]) {
        Ok(p) => p,
        Err(e) => return ToolResult::error(format!("cp: {}", e)),
    };

    match fs::copy(&src_validated, &dst_validated) {
        Ok(_) => ToolResult::success(format!("Copied {} to {}", args[0], args[1])),
        Err(e) => ToolResult::error(format!("Failed to copy: {}", e)),
    }
}

pub(crate) fn execute_mv(args: &[String]) -> ToolResult {
    if args.len() < 2 {
        return ToolResult::error("mv: requires <source> <destination>".to_string());
    }

    // Validate source path stays within current working directory
    let src_validated = match validate_path_within_cwd(&args[0]) {
        Ok(p) => p,
        Err(e) => return ToolResult::error(format!("mv: {}", e)),
    };

    // Validate destination path stays within current working directory
    let dst_validated = match validate_path_within_cwd(&args[1]) {
        Ok(p) => p,
        Err(e) => return ToolResult::error(format!("mv: {}", e)),
    };

    match fs::rename(&src_validated, &dst_validated) {
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

Cron(action: string, ...args) - Manage scheduled/recurring tasks
  Actions: list, create <schedule> <prompt>, delete <id>, pause <id>, resume <id>, trigger <id>
  Schedules: 30m (every 30 min), 2h (every 2 hours), 1d (daily), +1h (1 hour from now)
  Cron expressions: "0 9 * * *" (every day at 9am)
  Example: Cron("list")
  Example: Cron("create", "30m", "Check system status")
  Example: Cron("create", "0 9 * * *", "Morning report")

Sandbox(language: string, code: string, timeout?: number) - Execute code safely
  Languages: python, python3, javascript, node, bash, ruby, php, rust, go
  Default timeout: 30s
  Example: Sandbox("python", "print(1 + 2)")
  Example: Sandbox("bash", "echo hello", "60")

Container(backend: string, command: string, ...) - Execute in Docker, SSH, or remote
  Backends: docker, ssh, modal, local
  Flags: --image <img>, --host <addr>, --timeout <secs>
  Example: Container("docker", "cargo build", "--image rust:1.75")
  Example: Container("ssh", "ls", "--host user@server")
  Example: Container("local", "make build")

Batch(task1: string, task2: string, ..., --concurrency N) - Run tasks in parallel
  Default concurrency: 4
  Example: Batch("lint module1.rs", "lint module2.rs", "lint module3.rs")
  Example: Batch("analyze file1", "analyze file2", "--concurrency 8")

MixtureOfAgents(prompt: string, ...) - Ensemble reasoning with multiple AI perspectives
  Uses architect, security, and pragmatist agents
  Example: MixtureOfAgents("Review this architecture for scalability")
  Example: MoA("Debug why API returns 500 on POST /users", "--agents 3")

ImageGenerate(prompt: string, ...) - Generate AI images
  Flags: --provider openai|stable|replicate, --width <px>, --height <px>
  Example: ImageGenerate("A futuristic city at sunset")
  Example: ImageGenerate("Architecture diagram", "--width 1024 --height 512")

Skin(action: string, ...) - Theme and appearance management
  Actions: list, apply <name>, create <name> <desc>, current
  Built-in: default (gold/kawaii), ares (red/sci-fi), mono, slate
  Example: Skin("list")
  Example: Skin("apply", "dracula")

Profile(action: string, ...) - Isolated multi-instance environments
  Actions: list, create <name>, switch <name>, delete <name>, current
  Example: Profile("list")
  Example: Profile("create", "work-project-x")
  Example: Profile("switch", "personal")

AcpServer(action: string, ...) - IDE integration server (VS Code, JetBrains, Zed)
  Actions: start [--host <addr>] [--port <n>], stop, status, info
  Example: AcpServer("start", "--port 8080")
  Example: AcpServer("status")

IMPORTANT:
- Always use tools to DO tasks, not just describe them
- After starting a server, report the URL
- Check if files exist before reading
"#.to_string()
}
