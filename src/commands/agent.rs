//! Agent - Handles tool execution loop (Claude Code style)

use crate::api::{ApiClient, CompletionResponse};
use crate::state::AppState;
use crate::tools::executor::{execute_tool, get_tools_description, ToolResult};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{info, instrument};

const MAX_TOOL_CALLS: u32 = 10;

pub struct Agent {
    pub client: ApiClient,
    pub state: AppState,
    pub bypass_permissions: bool,
    pub verbose: bool,
    pub tool_results: HashMap<String, String>,
}

impl Agent {
    pub fn new(state: &mut AppState) -> Result<Self> {
        let client = ApiClient::new(state)?;
        let bypass_permissions = state.config.permission_mode.as_deref()
            .map(|m| m == "bypass")
            .unwrap_or(false);

        Ok(Self {
            client,
            state: state.clone(),
            bypass_permissions,
            verbose: false,
            tool_results: HashMap::new(),
        })
    }

    /// Run a prompt with tool execution - Claude Code style
    #[instrument(skip(self), fields(prompt_len = %prompt.len()))]
    pub async fn run(&mut self, prompt: &str) -> Result<CompletionResponse> {
        let start_time = Instant::now();
        let mut tool_calls_count = 0u32;

        info!("Starting agent run with prompt ({} chars)", prompt.len());

        // Add the user's prompt to history
        self.state.add_message("user", prompt);

        // Build system prompt with tools
        let system_prompt = self.build_system_prompt();

        // Show thinking indicator
        println!();
        self.show_thinking();

        // Make initial request
        let response = self.make_request(&system_prompt, prompt).await?;

        // Hide thinking, show response
        self.hide_thinking();

        // Check if response contains tool calls
        let tool_calls = self.extract_tool_calls(&response.content);

        if tool_calls.is_empty() {
            // No tool calls, print response and return
            self.print_response(&response.content);
            self.state.add_message("assistant", &response.content);
            return Ok(response);
        }

        // Tool execution loop
        let mut final_response = response;
        let mut all_tool_results = String::new();

        while !tool_calls.is_empty() && tool_calls_count < MAX_TOOL_CALLS {
            tool_calls_count += 1;

            println!();
            self.show_progress(&format!("Running {} tool(s)...", tool_calls.len()));

            // Execute all tool calls
            let mut results: Vec<ToolResult> = Vec::new();
            let mut result_summaries: Vec<String> = Vec::new();

            for call in tool_calls.iter() {
                let tool_name = call["name"].as_str().unwrap_or("unknown");
                let summary = self.get_tool_summary(call);

                print!("\r  {} ", self.get_tool_icon(tool_name));
                print!("{}", summary);
                if summary.len() < 50 {
                    print!("{}", " ".repeat(50 - summary.len()));
                }
                std::io::Write::flush(&mut std::io::stdout()).ok();

                let result = self.execute_tool_call(call);

                if result.success {
                    let content = if result.output.len() > 100 {
                        format!("{}... ({} chars)", &result.output[..100], result.output.len())
                    } else {
                        result.output.clone()
                    };
                    result_summaries.push(content);
                    println!("\r  {} {} ", self.get_checkmark(), summary);
                } else {
                    let error_msg = result.error.as_deref().unwrap_or("Unknown error");
                    println!("\r  {} {} Error: {}", self.get_x_mark(), summary, error_msg);

                    // Mark this tool call as failed - don't retry same path
                    // The model will see the error and adjust its approach
                }
                results.push(result);
            }

            // Add tool results to conversation for next turn
            let tool_msg = self.format_tool_calls_for_api(&tool_calls, &results);
            all_tool_results.push_str(&tool_msg);
            self.state.add_message("user", &tool_msg);

            // Show thinking for next API call
            self.show_thinking();

            // Make next request with tool results
            final_response = self.make_request_with_history(&system_prompt, prompt).await?;

            self.hide_thinking();

            // Check for more tool calls
            let new_tool_calls = self.extract_tool_calls(&final_response.content);

            if new_tool_calls.is_empty() {
                // No more tool calls, we're done
                break;
            }

            // Continue loop with new tool calls
        }

        // Print final response
        self.print_response(&final_response.content);

        // Add final response to history
        self.state.add_message("assistant", &final_response.content);

        // Print elapsed time
        let elapsed = start_time.elapsed();
        if elapsed.as_secs() > 2 {
            println!("\n  Completed in {:.1}s ({} tool calls)", elapsed.as_secs_f64(), tool_calls_count);
        }
        println!();

        Ok(final_response)
    }

    fn build_system_prompt(&self) -> String {
        let tools = get_tools_description();
        let bypass_note = if self.bypass_permissions {
            "\n\n⚠️ BYPASS MODE: You have permission to execute any command without confirmation."
        } else {
            ""
        };

        // Get current working directory for context
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        format!(
            r#"You are Code Buddy, a CLI coding assistant similar to Claude Code.

Current working directory: {cwd}

You have access to tools to complete tasks. When a user asks you to:
- Create files: Use the write tool
- Run commands: Use the bash tool
- Read files: Use the read tool
- Edit files: Use the edit tool

{tools}{bypass_note}

Guidelines:
1. Execute tools to actually DO tasks, not just describe them
2. After running a server/deploy command, report the URL
3. Be concise but complete
4. When you run a command, show the user what you're doing
5. Use RELATIVE paths from the current directory (e.g., "src/main.rs" not "/path/to/main.rs")
6. If a tool fails, read the error message carefully and try a different approach
7. Do NOT retry the exact same tool call that just failed - adjust your approach
8. If read fails, try: (a) different path variations, (b) use bash with 'cat' or 'ls', (c) use glob to find the file
"#
        )
    }

    async fn make_request(&self, system_prompt: &str, user_prompt: &str) -> Result<CompletionResponse> {
        let full_prompt = format!("{}\n\nUser: {}", system_prompt, user_prompt);
        self.client.complete(&full_prompt, &self.state.config, &self.state).await
    }

    async fn make_request_with_history(&self, system_prompt: &str, _original_prompt: &str) -> Result<CompletionResponse> {
        // Build conversation with history
        let mut messages = String::new();

        // Add history
        for msg in &self.state.conversation_history {
            let role = if msg.role == "user" { "User" } else { "Assistant" };
            messages.push_str(&format!("\n\n{}: {}\n", role, msg.content));
        }

        let full_prompt = format!("{}\n\nConversation so far:{}",
            system_prompt,
            if messages.is_empty() { " (new conversation)".to_string() } else { messages }
        );

        self.client.complete(&full_prompt, &self.state.config, &self.state).await
    }

    fn extract_tool_calls(&self, content: &str) -> Vec<Value> {
        let mut calls = Vec::new();
        let mut seen_calls: HashMap<String, bool> = HashMap::new();

        // Extract JSON from code blocks first
        let code_block_re = regex::Regex::new(r#"```(?:json)?\s*([\s\S]*?)```"#).unwrap();
        let mut json_content = content.to_string();

        for cap in code_block_re.captures_iter(content) {
            if let Some(block) = cap.get(1) {
                json_content = block.as_str().to_string();
                break; // Use first JSON block found
            }
        }

        // Try to parse as JSON
        if let Ok(json) = serde_json::from_str::<Value>(&json_content) {
            // Handle array of tool calls: [ { "tool": "name", ... }, ... ]
            if let Some(arr) = json.as_array() {
                for item in arr {
                    if let Some(tool_name) = item.get("tool").and_then(|v| v.as_str()) {
                        let args = item.get("parameters")
                            .or(item.get("arguments"))
                            .or(item.get("args"))
                            .unwrap_or(&serde_json::Value::Null);
                        let call_id = format!("{}-{:?}", tool_name, args);

                        if let std::collections::hash_map::Entry::Vacant(e) = seen_calls.entry(call_id) {
                            e.insert(true);
                            calls.push(serde_json::json!({
                                "name": tool_name,
                                "arguments": args
                            }));
                        }
                    } else if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        // Handle { "name": "tool", "arguments": ... } format
                        let args = item.get("arguments")
                            .or(item.get("args"))
                            .or(item.get("parameters"))
                            .unwrap_or(&serde_json::Value::Null);
                        let call_id = format!("{}-{:?}", name, args);

                        if let std::collections::hash_map::Entry::Vacant(e) = seen_calls.entry(call_id) {
                            e.insert(true);
                            calls.push(serde_json::json!({
                                "name": name,
                                "arguments": args
                            }));
                        }
                    }
                }
            }

            // Handle tool_calls array: { "tool_calls": [ { "name": "tool", ... }, ... ] }
            if let Some(tc) = json.get("tool_calls").and_then(|v| v.as_array()) {
                for call in tc {
                    if let Some(name) = call.get("name").and_then(|v| v.as_str()) {
                        let args = call.get("arguments")
                            .or(call.get("args"))
                            .or(call.get("parameters"))
                            .unwrap_or(&serde_json::Value::Null);
                        let call_id = format!("{}-{:?}", name, args);

                        if let std::collections::hash_map::Entry::Vacant(e) = seen_calls.entry(call_id) {
                            e.insert(true);
                            calls.push(serde_json::json!({
                                "name": name,
                                "arguments": args
                            }));
                        }
                    }
                }
            }

            // Handle single tool call: { "tool": "name", "parameters": {...} }
            if let Some(tool_name) = json.get("tool").and_then(|v| v.as_str()) {
                let args = json.get("parameters")
                    .or(json.get("arguments"))
                    .or(json.get("args"))
                    .unwrap_or(&serde_json::Value::Null);
                let call_id = format!("{}-{:?}", tool_name, args);

                if let std::collections::hash_map::Entry::Vacant(e) = seen_calls.entry(call_id) {
                    e.insert(true);
                    calls.push(serde_json::json!({
                        "name": tool_name,
                        "arguments": args
                    }));
                }
            }
        }

        // Extract from text patterns - Claude Code style
        let text_calls = self.extract_from_text(content);
        for call in text_calls {
            let name = call["name"].as_str().unwrap_or("unknown");
            let args_str = serde_json::to_string(&call["arguments"]).unwrap_or_default();
            let call_id = format!("{}-{}", name, args_str);

            if let std::collections::hash_map::Entry::Vacant(e) = seen_calls.entry(call_id) {
                e.insert(true);
                calls.push(call);
            }
        }

        calls
    }

    fn extract_from_text(&self, content: &str) -> Vec<Value> {
        let mut calls: Vec<Value> = Vec::new();
        let mut seen: HashMap<String, bool> = HashMap::new();

        // First, look for standalone bash code blocks that contain actual commands
        // These take priority as they're most reliable
        let code_block_re = regex::Regex::new(r#"```(?:bash|sh)\s*\n([\s\S]*?)```"#).unwrap();
        for cap in code_block_re.captures_iter(content) {
            let block = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let lines: Vec<&str> = block.lines()
                .filter(|l| {
                    let trimmed = l.trim();
                    // Skip empty lines and comments
                    !trimmed.is_empty() && !trimmed.starts_with('#')
                })
                .collect();

            if !lines.is_empty() {
                let cmd = lines.join("; ");
                // Check if it's a valid command (not path-like or documentation)
                if !cmd.starts_with('/') && !cmd.contains("example") && cmd.len() < 2000 {
                    let key = format!("bash:{}", cmd);
                    if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(key) {
                        e.insert(true);
                        calls.push(serde_json::json!({
                            "name": "bash",
                            "arguments": cmd
                        }));
                    }
                }
            }
        }

        // Look for write tool calls with proper syntax
        // write("path", "content") or write('path', 'content')
        let write_re = regex::Regex::new(r#"write\s*\(\s*["']([^"']+)["']\s*,\s*["']([^"']*)["']\s*\)"#).unwrap();
        for cap in write_re.captures_iter(content) {
            let path = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            let file_content = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Validate path - should not be absolute with /path/to or contain weird chars
            if !path.is_empty()
                && !path.starts_with("/path/")
                && !path.contains("/to/")
                && path.len() < 300
                && !file_content.is_empty()
            {
                let key = format!("write:{}:{}", path, &file_content[..file_content.len().min(50)]);
                if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(key) {
                    e.insert(true);
                    calls.push(serde_json::json!({
                        "name": "write",
                        "arguments": {
                            "path": path,
                            "content": file_content
                        }
                    }));
                }
            }
        }

        // Look for bash("command") style calls
        let bash_re = regex::Regex::new(r#"bash\s*\(\s*["']([^"']+)["']\s*\)"#).unwrap();
        for cap in bash_re.captures_iter(content) {
            let cmd = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");

            // Filter out paths and documentation
            if !cmd.is_empty()
                && !cmd.starts_with('/')
                && !cmd.contains("/path/")
                && !cmd.contains("example")
                && !cmd.contains("...")
                && cmd.len() < 2000
            {
                let key = format!("bash:{}", cmd);
                if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(key) {
                    e.insert(true);
                    calls.push(serde_json::json!({
                        "name": "bash",
                        "arguments": cmd
                    }));
                }
            }
        }

        // Look for simple bash commands at the start of lines
        let simple_bash_re = regex::Regex::new(r#"^\s*(?:bash|sh)\s+([^\n]+)"#).unwrap();
        for cap in simple_bash_re.captures_iter(content) {
            let cmd = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");

            if !cmd.is_empty()
                && !cmd.starts_with('-')
                && !cmd.contains("example")
                && cmd.len() < 1000
            {
                let key = format!("bash:{}", cmd);
                if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(key) {
                    e.insert(true);
                    calls.push(serde_json::json!({
                        "name": "bash",
                        "arguments": cmd
                    }));
                }
            }
        }

        // Look for call:tool{key: "value"} style (Gemma/NVIDIA format)
        // Examples: call:bash{command: "ls"} or call:read{path: "file.txt"}
        let gemma_re = regex::Regex::new(r#"call:(\w+)\s*\{([^}]+)\}"#).unwrap();
        for cap in gemma_re.captures_iter(content) {
            let tool_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let args_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Parse the arguments from key: "value" format
            let key_re = regex::Regex::new(r#"(\w+)\s*:\s*["']([^"']*)["']"#).unwrap();
            let mut args_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

            for arg_cap in key_re.captures_iter(args_str) {
                if let (Some(key), Some(val)) = (arg_cap.get(1), arg_cap.get(2)) {
                    args_map.insert(key.as_str().to_string(), val.as_str().to_string());
                }
            }

            if !args_map.is_empty() {
                let key = format!("{}:{:?}", tool_name, args_map);
                if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(key.clone()) {
                    e.insert(true);

                    // Convert to our standard format
                    let arguments = if tool_name == "bash" || tool_name == "read" || tool_name == "write" || tool_name == "edit" || tool_name == "glob" || tool_name == "grep" {
                        if let Some(v) = args_map.get("command").or(args_map.get("path")).or(args_map.get("pattern")).or(args_map.get("file")) {
                            serde_json::json!(v)
                        } else {
                            serde_json::json!(args_map)
                        }
                    } else {
                        serde_json::json!(args_map)
                    };

                    calls.push(serde_json::json!({
                        "name": tool_name,
                        "arguments": arguments
                    }));
                }
            }
        }

        calls
    }

    fn execute_tool_call(&self, call: &Value) -> ToolResult {
        let name = call["name"].as_str().unwrap_or("unknown");
        let args = &call["arguments"];

        let args_vec = self.parse_tool_args(name, args);

        execute_tool(name, &args_vec, self.bypass_permissions)
    }

    fn parse_tool_args(&self, tool_name: &str, args: &Value) -> Vec<String> {
        match args {
            Value::String(s) => vec![s.clone()],
            Value::Array(arr) => arr.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
            Value::Object(obj) => {
                match tool_name {
                    "bash" | "run" | "execute" => {
                        obj.get("command")
                            .or(obj.get("cmd"))
                            .or(obj.get("0"))
                            .and_then(|v| v.as_str())
                            .map(|s| vec![s.to_string()])
                            .unwrap_or_default()
                    }
                    "write" => {
                        let path = obj.get("path").or(obj.get("file")).or(obj.get("0"))
                            .and_then(|v| v.as_str()).unwrap_or("");
                        let content = obj.get("content").or(obj.get("text")).or(obj.get("1"))
                            .map(|v| v.to_string()).unwrap_or_default();
                        vec![path.to_string(), content]
                    }
                    "read" | "glob" | "grep" => {
                        obj.get("path").or(obj.get("file")).or(obj.get("pattern")).or(obj.get("0"))
                            .and_then(|v| v.as_str())
                            .map(|s| vec![s.to_string()])
                            .unwrap_or_default()
                    }
                    "edit" => {
                        let path = obj.get("path").or(obj.get("file")).or(obj.get("0"))
                            .and_then(|v| v.as_str()).unwrap_or("");
                        let old_text = obj.get("old_text").or(obj.get("old")).or(obj.get("1"))
                            .and_then(|v| v.as_str()).unwrap_or("");
                        let new_text = obj.get("new_text").or(obj.get("new")).or(obj.get("2"))
                            .map(|v| v.to_string()).unwrap_or_default();
                        vec![path.to_string(), old_text.to_string(), new_text]
                    }
                    _ => vec![]
                }
            }
            _ => vec![]
        }
    }

    fn get_tool_summary(&self, call: &Value) -> String {
        let name = call["name"].as_str().unwrap_or("unknown");
        let args = &call["arguments"];

        match name {
            "bash" => {
                if let Some(cmd) = args.as_str() {
                    let cmd = cmd.trim();
                    if cmd.len() > 50 {
                        format!("{}...", &cmd[..50])
                    } else {
                        cmd.to_string()
                    }
                } else if let Some(obj) = args.as_object() {
                    obj.get("command").or(obj.get("cmd"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "running command".to_string())
                } else {
                    "running command".to_string()
                }
            }
            "write" => {
                if let Some(obj) = args.as_object() {
                    obj.get("path").or(obj.get("file"))
                        .and_then(|v| v.as_str())
                        .map(|s| format!("Writing {}", s))
                        .unwrap_or_else(|| "Writing file".to_string())
                } else {
                    "Writing file".to_string()
                }
            }
            "read" => {
                if let Some(obj) = args.as_object() {
                    obj.get("path").or(obj.get("file"))
                        .and_then(|v| v.as_str())
                        .map(|s| format!("Reading {}", s))
                        .unwrap_or_else(|| "Reading file".to_string())
                } else if let Some(path) = args.as_str() {
                    format!("Reading {}", path)
                } else {
                    "Reading file".to_string()
                }
            }
            "edit" => {
                if let Some(obj) = args.as_object() {
                    obj.get("path").or(obj.get("file"))
                        .and_then(|v| v.as_str())
                        .map(|s| format!("Editing {}", s))
                        .unwrap_or_else(|| "Editing file".to_string())
                } else {
                    "Editing file".to_string()
                }
            }
            "glob" => {
                if let Some(pattern) = args.as_str() {
                    format!("Finding {}", pattern)
                } else {
                    "Finding files".to_string()
                }
            }
            "grep" => {
                if let Some(pattern) = args.as_str() {
                    format!("Searching for {}", pattern)
                } else {
                    "Searching".to_string()
                }
            }
            _ => format!("Running {}", name),
        }
    }

    fn get_tool_icon(&self, tool_name: &str) -> &'static str {
        match tool_name {
            "bash" | "run" | "execute" => "⚡",
            "write" => "📝",
            "read" => "📖",
            "edit" => "✏️",
            "glob" => "🔍",
            "grep" => "🔎",
            "webfetch" | "web_fetch" => "🌐",
            "websearch" | "web_search" => "🌐",
            _ => "🔧",
        }
    }

    fn get_checkmark(&self) -> &'static str {
        "✅"
    }

    fn get_x_mark(&self) -> &'static str {
        "❌"
    }

    fn show_thinking(&self) {
        print!("  ⠿ thinking...");
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }

    fn hide_thinking(&self) {
        print!("\r{:width$}\r", "", width = 20);
    }

    fn show_progress(&self, msg: &str) {
        println!("  {}", msg);
    }

    fn print_response(&self, content: &str) {
        // Print response, stripping any tool call artifacts
        let cleaned = self.clean_response(content);
        if !cleaned.trim().is_empty() {
            println!();
            println!("{}", cleaned);
        }
    }

    fn clean_response(&self, content: &str) -> String {
        // Remove tool call artifacts from response
        let mut lines: Vec<&str> = Vec::new();
        let mut in_code_block = false;
        let mut in_tool_call = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip lines that are just tool call artifacts
            if trimmed.starts_with("bash(") || trimmed.starts_with("write(") ||
               trimmed.starts_with("read(") || trimmed.starts_with("edit(") ||
               trimmed.starts_with("run(") || trimmed.starts_with("execute(") {
                in_tool_call = true;
                continue;
            }

            if trimmed == "```" && in_code_block {
                in_code_block = false;
                continue;
            }

            if trimmed.starts_with("```bash") || trimmed.starts_with("```sh") ||
               trimmed.starts_with("```") {
                in_code_block = true;
                // Check if this code block contains a single tool call
                continue;
            }

            if !in_tool_call && !in_code_block {
                lines.push(line);
            }

            if in_tool_call && trimmed.is_empty() {
                in_tool_call = false;
            }
        }

        lines.join("\n")
    }

    fn format_tool_calls_for_api(&self, calls: &[Value], results: &[ToolResult]) -> String {
        let mut output = String::from("\n\n[Tool Results]\n");

        for (call, result) in calls.iter().zip(results.iter()) {
            let name = call["name"].as_str().unwrap_or("unknown");
            output.push_str(&format!("- {}: ", name));

            if result.success {
                let content = &result.output;
                if content.len() > 500 {
                    output.push_str(&format!("{}\n... ({} chars total)\n", &content[..500], content.len()));
                } else {
                    output.push_str(&format!("{}\n", content));
                }
            } else {
                output.push_str(&format!("Error: {}\n", result.error.as_deref().unwrap_or("Unknown")));
            }
            output.push('\n');
        }

        output.push_str("Continue with the task.\n");
        output
    }
}
