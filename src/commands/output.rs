//! Output Display Module - Claude Code-style terminal output
//!
//! Provides clean, consistent output display for tool results and status messages.
//! All output is centralized here to prevent duplicate rendering.


/// Output state for managing single-place rendering
pub struct OutputState {
    /// Current status message being displayed
    current_status: Option<String>,
    /// Tool execution results buffer
    pending_results: Vec<ToolResult>,
    /// Whether we're in quiet mode
    quiet: bool,
}

impl Default for OutputState {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputState {
    pub fn new() -> Self {
        Self {
            current_status: None,
            pending_results: Vec::new(),
            quiet: false,
        }
    }

    /// Set quiet mode
    pub fn set_quiet(&mut self, quiet: bool) {
        self.quiet = quiet;
    }

    /// Clear the current status line
    pub fn clear_status(&mut self) {
        if self.current_status.is_some() {
            // Clear the line and move cursor back
            print!("\r\x1b[K");
            std::io::Write::flush(&mut std::io::stdout()).ok();
            self.current_status = None;
        }
    }

    /// Update the status message (replaces previous status)
    pub fn show_status(&mut self, message: &str) {
        self.clear_status();
        print!("\r{}", message);
        std::io::Write::flush(&mut std::io::stdout()).ok();
        self.current_status = Some(message.to_string());
    }

    /// Show tool execution header
    pub fn show_tool_start(&mut self, tool_name: &str, args: &[String]) {
        if self.quiet { return; }

        self.clear_status();

        let args_preview = if args.is_empty() {
            String::new()
        } else {
            let preview = args.join(" ");
            if preview.len() > 50 {
                format!("...{}", &preview[preview.len()-47..])
            } else {
                preview
            }
        };

        println!();
        print!("\x1b[90m{} \x1b[36m{}\x1b[0m", tool_name.to_uppercase(), args_preview);
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }

    /// Show tool completion
    pub fn show_tool_result(&mut self, result: &ToolResult) {
        if self.quiet { return; }

        self.clear_status();
        println!();

        if result.success {
            // Show success message
            let lines: Vec<&str> = result.output.lines().collect();
            for line in lines.iter().take(10) {
                println!("  \x1b[32m{}\x1b[0m", line);
            }
            if lines.len() > 10 {
                println!("  \x1b[90m... ({} more lines)\x1b[0m", lines.len() - 10);
            }
        } else if let Some(ref error) = result.error {
            // Show error message
            println!("  \x1b[31mError:\x1b[0m {}", error);
        }
    }

    /// Show file changes in git-like style
    pub fn show_file_changes(&mut self, changes: &[FileChange]) {
        if self.quiet { return; }

        println!();
        for change in changes {
            match change {
                FileChange::Created(path) => {
                    println!("  \x1b[32m+ {}\x1b[0m", path);
                }
                FileChange::Modified(path) => {
                    println!("  \x1b[33mM {}\x1b[0m", path);
                }
                FileChange::Deleted(path) => {
                    println!("  \x1b[31mD {}\x1b[0m", path);
                }
                FileChange::Renamed { from, to } => {
                    println!("  \x1b[33mR {}\x1b[0m -> \x1b[33m{}\x1b[0m", from, to);
                }
            }
        }
    }

    /// Show diff output
    pub fn show_diff(&mut self, diff: &DiffOutput) {
        if self.quiet { return; }

        println!();
        for line in &diff.lines {
            let colored = match line.line_type {
                DiffLineType::Addition => format!("\x1b[32m{}\x1b[0m", line.content),
                DiffLineType::Deletion => format!("\x1b[31m{}\x1b[0m", line.content),
                DiffLineType::Context => line.content.clone(),
                DiffLineType::Header => format!("\x1b[90m{}\x1b[0m", line.content),
            };
            println!("{}", colored);
        }
    }

    /// Show thinking/loading status with quotes
    pub fn show_thinking(&mut self) {
        if self.quiet { return; }

        let quotes = [
            "Thinking...",
            "Analyzing...",
            "Processing...",
            "Consulting the docs...",
            "Planning...",
        ];

        let quote = quotes[std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as usize)
            .unwrap_or(0) % quotes.len()];

        self.show_status(&format!("\x1b[90m{} \x1b[33m▋\x1b[0m", quote));
    }

    /// Show token usage
    pub fn show_token_usage(&mut self, input: u32, output: u32) {
        if self.quiet { return; }

        println!();
        println!("  \x1b[90mTokens: {} in, {} out ({} total)\x1b[0m", input, output, input + output);
    }

    /// Show completion message
    pub fn show_complete(&mut self) {
        self.clear_status();
        println!();
    }
}

/// Tool result from executor
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// File change
#[derive(Debug, Clone)]
pub enum FileChange {
    Created(String),
    Modified(String),
    Deleted(String),
    Renamed { from: String, to: String },
}

/// Diff line
#[derive(Debug, Clone)]
pub struct DiffOutput {
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub content: String,
    pub line_type: DiffLineType,
}

#[derive(Debug, Clone)]
pub enum DiffLineType {
    Addition,
    Deletion,
    Context,
    Header,
}

/// Format a diff from old and new content
pub fn format_diff(old_content: &str, new_content: &str, file_path: &str) -> DiffOutput {
    let mut lines = Vec::new();

    // Header
    lines.push(DiffLine {
        content: format!("diff --git a/{} b/{}", file_path, file_path),
        line_type: DiffLineType::Header,
    });
    lines.push(DiffLine {
        content: format!("--- a/{}", file_path),
        line_type: DiffLineType::Header,
    });
    lines.push(DiffLine {
        content: format!("+++ b/{}", file_path),
        line_type: DiffLineType::Header,
    });

    // Simple line-by-line diff
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    let max_lines = old_lines.len().max(new_lines.len());
    for i in 0..max_lines {
        let old_line = old_lines.get(i);
        let new_line = new_lines.get(i);

        match (old_line, new_line) {
            (Some(old), Some(new)) if old == new => {
                lines.push(DiffLine {
                    content: format!(" {}", new),
                    line_type: DiffLineType::Context,
                });
            }
            (Some(old), None) => {
                lines.push(DiffLine {
                    content: format!("-{}", old),
                    line_type: DiffLineType::Deletion,
                });
            }
            (None, Some(new)) => {
                lines.push(DiffLine {
                    content: format!("+{}", new),
                    line_type: DiffLineType::Addition,
                });
            }
            (Some(old), Some(new)) => {
                lines.push(DiffLine {
                    content: format!("-{}", old),
                    line_type: DiffLineType::Deletion,
                });
                lines.push(DiffLine {
                    content: format!("+{}", new),
                    line_type: DiffLineType::Addition,
                });
            }
            _ => {}
        }
    }

    DiffOutput { lines }
}
