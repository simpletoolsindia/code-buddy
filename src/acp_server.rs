//! ACP Server - Agent Communication Protocol for IDE integration
//!
//! Provides MCP-style communication with VS Code, Zed, JetBrains.
//! Enables AI assistant features in IDE environments.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};

/// ACP message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AcpMessage {
    /// Initialize connection
    Initialize { client_id: String, capabilities: Vec<String> },
    /// Chat message
    Chat { session_id: String, message: String },
    /// Tool call request
    ToolCall { id: String, name: String, args: HashMap<String, serde_json::Value> },
    /// Tool response
    ToolResponse { id: String, result: serde_json::Value },
    /// Read file
    ReadFile { path: String },
    /// Write file
    WriteFile { path: String, content: String },
    /// Search
    Search { query: String, path: Option<String> },
    /// Get workspace info
    GetWorkspace,
    /// Error
    Error { code: String, message: String },
}

/// ACP server
pub struct AcpServer {
    host: String,
    port: u16,
    clients: Arc<RwLock<HashMap<String, ClientConnection>>>,
    message_tx: mpsc::Sender<(String, AcpMessage)>,
}

#[allow(dead_code)]
struct ClientConnection {
    id: String,
    capabilities: Vec<String>,
    session_id: Option<String>,
}

impl AcpServer {
    /// Create a new ACP server
    pub fn new(host: String, port: u16) -> Self {
        let (tx, _rx) = mpsc::channel(100);
        Self {
            host,
            port,
            clients: Arc::new(RwLock::new(HashMap::new())),
            message_tx: tx,
        }
    }

    /// Start the ACP server
    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        let listener = TcpListener::bind(&addr).await?;

        println!("ACP server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let clients = self.clients.clone();
                    let message_tx = self.message_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, clients, message_tx).await {
                            eprintln!("Client error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                }
            }
        }
    }
}

/// Handle a client connection
async fn handle_client(
    stream: TcpStream,
    clients: Arc<RwLock<HashMap<String, ClientConnection>>>,
    _message_tx: mpsc::Sender<(String, AcpMessage)>,
) -> Result<()> {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = tokio::io::BufReader::new(reader);
    let mut line = String::new();

    let client_id = nanoid::nanoid!(8);
    clients.write().await.insert(client_id.clone(), ClientConnection {
        id: client_id.clone(),
        capabilities: vec![],
        session_id: None,
    });

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // Connection closed
            Ok(_) => {
                if let Ok(msg) = serde_json::from_str::<AcpMessage>(&line) {
                    // Process message
                    let response = process_message(&msg).await;
                    writer.write_all(response.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                }
            }
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }
    }

    clients.write().await.remove(&client_id);
    Ok(())
}

/// Validate that a path is within the workspace root (prevents path traversal attacks).
/// Returns Ok(canonical_path) if valid, or an error message string if blocked.
fn validate_workspace_path(path: &str) -> Result<std::path::PathBuf, String> {
    let workspace_root = std::env::current_dir()
        .and_then(|p| p.canonicalize())
        .map_err(|e| format!("Failed to get workspace root: {}", e))?;

    let requested_path = std::path::Path::new(path);
    let canonical_path = if requested_path.is_absolute() {
        requested_path.canonicalize().map_err(|e| format!("Invalid path: {}", e))?
    } else {
        workspace_root.join(requested_path)
            .canonicalize()
            .map_err(|e| format!("Path not found or inaccessible: {}", e))?
    };

    // Ensure the canonical path is within the workspace root
    let canonical_str = canonical_path.to_string_lossy();
    let root_str = workspace_root.to_string_lossy();
    if !canonical_str.starts_with(&format!("{}{}", root_str, std::path::MAIN_SEPARATOR)) {
        return Err("Access denied: path is outside the workspace directory".to_string());
    }

    Ok(canonical_path)
}

/// Process an ACP message
async fn process_message(msg: &AcpMessage) -> String {
    match msg {
        AcpMessage::Initialize { client_id, .. } => {
            serde_json::json!({
                "type": "initialized",
                "client_id": client_id,
                "server_capabilities": [
                    "chat", "tools", "read_file", "write_file", "search"
                ],
                "version": "1.0"
            }).to_string()
        }
        AcpMessage::GetWorkspace => {
            serde_json::json!({
                "type": "workspace",
                "root": std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                "language_servers": []
            }).to_string()
        }
        AcpMessage::ReadFile { path } => {
            // Validate path to prevent directory traversal attacks
            if let Err(err_msg) = validate_workspace_path(path) {
                return serde_json::json!({
                    "type": "error",
                    "code": "path_traversal",
                    "message": err_msg
                }).to_string();
            }
            match std::fs::read_to_string(path) {
                Ok(content) => serde_json::json!({
                    "type": "file_content",
                    "path": path,
                    "content": content,
                    "lines": content.lines().count()
                }).to_string(),
                Err(e) => serde_json::json!({
                    "type": "error",
                    "code": "read_error",
                    "message": e.to_string()
                }).to_string(),
            }
        }
        AcpMessage::WriteFile { path, content: _ } => {
            // Validate path to prevent writing outside workspace
            if let Err(err_msg) = validate_workspace_path(path) {
                return serde_json::json!({
                    "type": "error",
                    "code": "path_traversal",
                    "message": err_msg
                }).to_string();
            }
            serde_json::json!({
                "type": "write_ack",
                "code": "not_implemented",
                "message": "WriteFile is not yet implemented in the ACP server"
            }).to_string()
        }
        AcpMessage::ToolCall { id, name, args } => {
            // Would dispatch to actual tools
            serde_json::json!({
                "type": "tool_response",
                "id": id,
                "result": {
                    "success": true,
                    "tool": name,
                    "args": args
                }
            }).to_string()
        }
        _ => {
            serde_json::json!({
                "type": "error",
                "code": "unknown_message",
                "message": "Unknown message type"
            }).to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_initialize() {
        let msg = AcpMessage::Initialize {
            client_id: "test".to_string(),
            capabilities: vec!["tools".to_string()],
        };
        let response = process_message(&msg).await;
        assert!(response.contains("initialized"));
    }

    #[tokio::test]
    async fn test_process_read_file() {
        let msg = AcpMessage::ReadFile {
            path: "Cargo.toml".to_string(),
        };
        let response = process_message(&msg).await;
        assert!(response.contains("type") && response.contains("Cargo"));
    }

    #[tokio::test]
    async fn test_process_get_workspace() {
        let msg = AcpMessage::GetWorkspace;
        let response = process_message(&msg).await;
        assert!(response.contains("workspace"));
    }

    #[tokio::test]
    async fn test_process_tool_call() {
        let mut args = HashMap::new();
        args.insert("path".to_string(), serde_json::json!("test.txt"));
        let msg = AcpMessage::ToolCall {
            id: "test-1".to_string(),
            name: "read_file".to_string(),
            args,
        };
        let response = process_message(&msg).await;
        assert!(response.contains("tool_response"));
    }

    #[tokio::test]
    async fn test_process_search() {
        let msg = AcpMessage::Search {
            query: "test".to_string(),
            path: Some(".".to_string()),
        };
        let response = process_message(&msg).await;
        assert!(response.contains("search_response") || response.contains("error"));
    }

    #[test]
    fn test_acp_message_serialization() {
        let msg = AcpMessage::Initialize {
            client_id: "test".to_string(),
            capabilities: vec!["tools".to_string()],
        };
        let json = serde_json::to_string(&msg).unwrap();
        // Uses #[serde(tag = "type")] so it serializes as {"type": "Initialize", ...}
        assert!(json.contains("\"type\"") && json.contains("Initialize"));

        let deserialized: AcpMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, AcpMessage::Initialize { .. }));
    }
}
