//! Agent - Handles tool execution loop

use crate::api::{ApiClient, CompletionResponse};
use crate::state::AppState;
use crate::tools::executor::{execute_tool, get_tools_description, ToolResult};
use anyhow::Result;
use serde_json::Value;
use std::time::{Duration, Instant};

const MAX_TOOL_CALLS: u32 = 10;
const TOOL_CALL_TIMEOUT: Duration = Duration::from_secs(120);

pub struct Agent {
    pub client: ApiClient,
    pub state: AppState,
    pub bypass_permissions: bool,
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
        })
    }

    /// Run a prompt with tool execution
    pub async fn run(&mut self, prompt: &str) -> Result<CompletionResponse> {
        let start_time = Instant::now();
        let mut tool_calls_count = 0;

        // Add the user's prompt to history
        self.state.add_message("user", prompt);

        // Build system prompt with tools
        let system_prompt = self.build_system_prompt();

        // Make initial request
        let response = self.make_request(&system_prompt, prompt).await?;

        // Check if response contains tool calls
        let tool_calls = self.extract_tool_calls(&response.content);

        if tool_calls.is_empty() {
            // No tool calls, return the response
            self.state.add_message("assistant", &response.content);
            return Ok(response);
        }

        // Tool execution loop
        let mut final_response = response;

        while !tool_calls.is_empty() && tool_calls_count < MAX_TOOL_CALLS {
            tool_calls_count += 1;

            println!("\n🔧 Executing {} tool(s)...\n", tool_calls.len());

            // Execute all tool calls
            let mut tool_results = Vec::new();
            for call in &tool_calls {
                let result = self.execute_tool_call(call);
                tool_results.push(result);
            }

            // Print tool results
            for (call, result) in tool_calls.iter().zip(tool_results.iter()) {
                let tool_name = &call["name"].as_str().unwrap_or("unknown");
                if result.success {
                    println!("✅ {}: Success", tool_name);
                } else {
                    println!("❌ {}: {}", tool_name, result.error.as_ref().unwrap_or(&"Unknown error".to_string()));
                }
            }
            println!();

            // Add tool calls and results to conversation
            let tool_msg = self.format_tool_calls_for_api(&tool_calls, &tool_results);
            self.state.add_message("user", &tool_msg);

            // Make next request with tool results
            final_response = self.make_request(&system_prompt, "").await?;

            // Check for more tool calls
            let new_tool_calls = self.extract_tool_calls(&final_response.content);
            if new_tool_calls.is_empty() {
                break;
            }
            // Continue loop with new tool calls
        }

        // Add final response to history
        self.state.add_message("assistant", &final_response.content);

        // Print elapsed time
        let elapsed = start_time.elapsed();
        println!("\n⏱️  Completed in {:.1}s ({} tool calls)\n", elapsed.as_secs_f64(), tool_calls_count);

        Ok(final_response)
    }

    fn build_system_prompt(&self) -> String {
        let tools = get_tools_description();
        let bypass_note = if self.bypass_permissions {
            "\n\n⚠️ IMPORTANT: You have BYPASS PERMISSIONS enabled. You can execute ANY command without confirmation. Use this power wisely!"
        } else {
            ""
        };

        format!(
            r#"You are Code Buddy, a helpful AI coding assistant.

{tools}{bypass_note}

IMPORTANT INSTRUCTIONS:
1. When asked to create code, WRITE the file first using the write tool
2. When asked to run/deploy/execute, use the bash tool to run commands
3. After deploying, provide the user with a URL or instructions to access the result
4. Be concise but complete in your responses
5. If a command produces a URL or port, share it with the user

Remember: Use tools to actually do tasks, not just suggest them!
"#
        )
    }

    async fn make_request(&self, system_prompt: &str, user_prompt: &str) -> Result<CompletionResponse> {
        // Create a modified state with system message
        let mut request_state = self.state.clone();

        // For simplicity, we'll modify the prompt to include system instructions
        let full_prompt = if user_prompt.is_empty() {
            "Continue with the task. You have tool results above."
        } else {
            user_prompt
        };

        let final_prompt = format!("{}\n\nUser: {}", system_prompt, full_prompt);

        self.client.complete(&final_prompt, &self.state.config, &self.state).await
    }

    fn extract_tool_calls(&self, content: &str) -> Vec<Value> {
        let mut calls = Vec::new();

        // Try to parse as JSON with tool_calls array
        if let Ok(json) = serde_json::from_str::<Value>(content) {
            if let Some(tc) = json.get("tool_calls").and_then(|v| v.as_array()) {
                for call in tc {
                    let name = call.get("name").and_then(|v| v.as_str());
                    let args = call.get("arguments").or(call.get("args")).unwrap_or(&serde_json::Value::Null);
                    if let Some(name_str) = name {
                        let args_str = if let Some(s) = args.as_str() {
                            s.to_string()
                        } else if let Some(obj) = args.as_object() {
                            serde_json::to_string(obj).unwrap_or_default()
                        } else {
                            args.to_string()
                        };

                        if let Ok(args_json) = serde_json::from_str::<serde_json::Value>(&args_str) {
                            calls.push(serde_json::json!({
                                "name": name_str,
                                "arguments": args_json
                            }));
                        }
                    }
                }
            }
        }

        // Try to extract bash(...) or tool(...) calls from text
        let patterns = [
            // bash("command") or bash('command')
            (r#"bash\s*\(\s*["']([^"']+)["']\s*\)"#, "bash"),
            // run("command") or run('command')
            (r#"run\s*\(\s*["']([^"']+)["']\s*\)"#, "bash"),
            // execute("command") or execute('command')
            (r#"execute\s*\(\s*["']([^"']+)["']\s*\)"#, "bash"),
            // write("file", "content") or write('file', 'content')
            (r#"write\s*\(\s*["']([^"']+)["']\s*,\s*["']([^"']*)["']\s*\)"#, "write"),
            // read("file") or read('file')
            (r#"read\s*\(\s*["']([^"']+)["']\s*\)"#, "read"),
            // ```bash\ncommand\n```
            (r#"```bash\s*\n([\s\S]*?)\n```"#, "bash"),
            // ```sh\ncommand\n```
            (r#"```sh\s*\n([\s\S]*?)\n```"#, "bash"),
            // Single line commands in bash blocks
            (r#"```bash\s*([\s\S]*?)```"#, "bash"),
        ];

        for (pattern, tool_name) in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for cap in re.captures_iter(content) {
                    let cmd = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                    if !cmd.is_empty() && cmd.len() < 10000 {
                        if *tool_name == "write" {
                            // For write commands, extract path and content
                            let content = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                            calls.push(serde_json::json!({
                                "name": "write",
                                "arguments": serde_json::json!({
                                    "path": cmd,
                                    "content": content
                                })
                            }));
                        } else {
                            calls.push(serde_json::json!({
                                "name": tool_name,
                                "arguments": cmd.to_string()
                            }));
                        }
                    }
                }
            }
        }

        calls
    }

    fn execute_tool_call(&self, call: &Value) -> ToolResult {
        let name = call["name"].as_str().unwrap_or("unknown");
        let args = &call["arguments"];

        // Parse arguments
        let args_vec: Vec<String> = if let Some(arr) = args.as_array() {
            arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
        } else if let Some(obj) = args.as_object() {
            // For bash commands, extract the command string
            if name == "bash" || name == "run" || name == "execute" {
                if let Some(cmd) = obj.get("command").or(obj.get("cmd")).or(obj.get("0")).and_then(|v| v.as_str()) {
                    vec![cmd.to_string()]
                } else {
                    vec![]
                }
            } else if name == "write" {
                let path = obj.get("path").or(obj.get("file")).or(obj.get("0"))
                    .and_then(|v| v.as_str()).unwrap_or("");
                let content = obj.get("content").or(obj.get("text")).or(obj.get("1"))
                    .map(|v| v.to_string()).unwrap_or_default();
                vec![path.to_string(), content]
            } else if name == "read" {
                if let Some(path) = obj.get("path").or(obj.get("file")).or(obj.get("0"))
                    .and_then(|v| v.as_str()) {
                    vec![path.to_string()]
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        } else if let Some(s) = args.as_str() {
            // If it's a string, check if it's a write command
            let s = s.to_string();
            if name == "write" {
                // Try to parse as JSON
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&s) {
                    let path = parsed.get("path").or(parsed.get("file"))
                        .and_then(|v| v.as_str()).unwrap_or("");
                    let content = parsed.get("content").or(parsed.get("text"))
                        .map(|v| v.to_string()).unwrap_or_default();
                    vec![path.to_string(), content]
                } else {
                    // Treat as "command" argument
                    vec![s]
                }
            } else {
                vec![s]
            }
        } else {
            vec![]
        };

        execute_tool(name, &args_vec, self.bypass_permissions)
    }

    fn format_tool_calls_for_api(&self, calls: &[Value], results: &[ToolResult]) -> String {
        let mut output = String::from("Tool Results:\n\n");

        for (call, result) in calls.iter().zip(results.iter()) {
            let name = call["name"].as_str().unwrap_or("unknown");
            output.push_str(&format!("Tool: {}\n", name));
            output.push_str(&format!("Result: {}\n\n", result.to_content()));
        }

        output
    }
}
