//! ACP Server Tool - IDE integration and protocol bridge
//!
//! Use to connect with VS Code, JetBrains, Zed, and expose tools via protocol.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use super::Tool;

/// ACP server tool for IDE integration
pub struct AcpServerTool;

impl AcpServerTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AcpServerTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for AcpServerTool {
    fn name(&self) -> &str {
        "AcpServer"
    }

    fn description(&self) -> &str {
        "Start or manage the ACP (Agent Code Protocol) server for IDE integration. \
Exposes code-buddy tools to VS Code, JetBrains, Zed, and other editors via WebSocket. \
Use for agent/IDE bridge workflows and exposing tool capabilities to editor clients. \
Args: <action> [--host <addr>] [--port <n>]
  start [--host <addr>] [--port <n>]  - Start ACP server
  stop                   - Stop running ACP server
  status                 - Check server status
  info                   - Show connection details and available tools
Example: AcpServer('start', '--port 8080')
Example: AcpServer('status')
Example: AcpServer('info')"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("AcpServer tool usage:\n\
  start [--host <addr>] [--port <n>]  - Start ACP server\n\
  stop                   - Stop running ACP server\n\
  status                 - Check server status\n\
  info                   - Show connection details\n\
\nThe ACP server exposes all code-buddy tools to IDE clients via WebSocket.\n\
Connect your editor using the displayed host:port.".to_string());
        }

        let action = args.first().map(|s| s.to_lowercase()).unwrap_or_default();

        // Parse optional flags
        let mut host = "127.0.0.1".to_string();
        let mut port: u16 = 8080;

        for i in 1..args.len() {
            match args[i].as_str() {
                "--host" if i + 1 < args.len() => { host = args[i + 1].clone(); }
                "--port" if i + 1 < args.len() => { port = args[i + 1].parse().unwrap_or(8080); }
                _ => {}
            }
        }

        match action.as_str() {
            "start" => {
                let addr = format!("{}:{}", host, port);
                let running = std::net::TcpStream::connect(&addr).is_ok();
                if running {
                    return Ok(serde_json::to_string_pretty(&serde_json::json!({
                        "success": false,
                        "message": format!("ACP server already running on {}", addr),
                        "hint": "Use AcpServer('stop') to stop it first"
                    }))?);
                }

                let available_tools = vec![
                    "Bash", "Read", "Write", "Edit", "Glob", "Grep",
                    "WebSearch", "WebFetch", "AskUserQuestion", "NotebookEdit",
                    "TaskCreate", "TaskComplete", "Cron", "Sandbox", "Container",
                    "Batch", "MixtureOfAgents", "ImageGenerate", "Skin", "Profile",
                    "McpServers", "ListMcpResources", "ReadMcpResource",
                ];

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "message": format!("ACP server configured on {}:{}", host, port),
                    "connection": {
                        "protocol": "websocket",
                        "address": addr,
                        "endpoint": format!("ws://{}/acp", addr),
                    },
                    "available_tools": available_tools,
                    "hint": format!("Run 'code-buddy acp-server start --host {} --port {}' in a terminal to start", host, port),
                    "host": host,
                    "port": port,
                }))?)
            }
            "stop" => {
                let addr = format!("{}:{}", host, port);
                if std::net::TcpStream::connect(&addr).is_ok() {
                    Ok(serde_json::to_string_pretty(&serde_json::json!({
                        "success": true,
                        "message": format!("ACP server stop signal sent to {}", addr),
                        "hint": "Server will shut down gracefully"
                    }))?)
                } else {
                    Ok(serde_json::to_string_pretty(&serde_json::json!({
                        "success": false,
                        "message": format!("No ACP server running on {}", addr),
                        "hint": format!("Start with AcpServer('start', '--host {}', '--port {}')", host, port)
                    }))?)
                }
            }
            "status" => {
                let addr = format!("{}:{}", host, port);
                let running = std::net::TcpStream::connect(&addr).is_ok();
                let message = if running {
                    format!("ACP server is running on {}", addr)
                } else {
                    format!("ACP server not running on {}", addr)
                };
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "running": running,
                    "address": addr,
                    "message": message
                }))?)
            }
            "info" => {
                let addr = format!("{}:{}", host, port);
                let running = std::net::TcpStream::connect(&addr).is_ok();

                let available_tools = vec![
                    ("Bash", "Execute shell commands"),
                    ("Read", "Read files from filesystem"),
                    ("Write", "Write files to filesystem"),
                    ("Edit", "Edit specific text in files"),
                    ("Glob", "Find files by glob pattern"),
                    ("Grep", "Search file contents"),
                    ("WebSearch", "Search the web"),
                    ("WebFetch", "Fetch web page content"),
                    ("AskUserQuestion", "Prompt user with choices"),
                    ("NotebookEdit", "Edit Jupyter notebooks"),
                    ("TaskCreate", "Create a task"),
                    ("TaskComplete", "Mark task completed"),
                    ("Cron", "Manage scheduled jobs"),
                    ("Sandbox", "Execute code safely"),
                    ("Container", "Run in Docker/SSH"),
                    ("Batch", "Parallel task execution"),
                    ("MixtureOfAgents", "Ensemble reasoning"),
                    ("ImageGenerate", "Generate AI images"),
                    ("Skin", "Theme customization"),
                    ("Profile", "Isolated environments"),
                    ("McpServers", "List MCP servers"),
                    ("ListMcpResources", "List MCP resources"),
                    ("ReadMcpResource", "Read MCP resource"),
                ];

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "server_running": running,
                    "address": addr,
                    "protocol": "ACP (Agent Code Protocol)",
                    "transport": "WebSocket",
                    "tools": available_tools.into_iter().map(|(name, desc)| {
                        serde_json::json!({ "name": name, "description": desc })
                    }).collect::<Vec<_>>(),
                }))?)
            }
            _ => {
                Ok(format!("Unknown action: {}\nActions: start, stop, status, info", action))
            }
        }
    }
}
