//! MCP Resource Tools - ReadMcpResourceTool, ListMcpResourcesTool
//!
//! Provides tools for reading and listing MCP server resources.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use super::Tool;

/// MCP resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

impl McpResource {
    pub fn new(uri: &str, name: &str) -> Self {
        Self {
            uri: uri.to_string(),
            name: name.to_string(),
            description: None,
            mime_type: None,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn with_mime_type(mut self, mime: &str) -> Self {
        self.mime_type = Some(mime.to_string());
        self
    }
}

/// MCP server connection info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: Option<String>,
    pub resources: Vec<McpResource>,
}

/// MCP client for interacting with MCP servers
pub struct McpClient {
    servers: Vec<McpServerInfo>,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
        }
    }

    /// Add a server to track
    pub fn add_server(&mut self, server: McpServerInfo) {
        self.servers.push(server);
    }

    /// List all servers
    pub fn list_servers(&self) -> Vec<&McpServerInfo> {
        self.servers.iter().collect()
    }

    /// Get server by name
    pub fn get_server(&self, name: &str) -> Option<&McpServerInfo> {
        self.servers.iter().find(|s| s.name == name)
    }

    /// List all resources across all servers
    pub fn list_all_resources(&self) -> Vec<&McpResource> {
        self.servers
            .iter()
            .flat_map(|s| s.resources.iter())
            .collect()
    }

    /// Find resource by URI
    pub fn find_resource(&self, uri: &str) -> Option<&McpResource> {
        self.list_all_resources()
            .into_iter()
            .find(|r| r.uri == uri)
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// ListMcpResourcesTool - List available MCP resources
pub struct ListMcpResourcesTool;

impl ListMcpResourcesTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ListMcpResourcesTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ListMcpResourcesTool {
    fn name(&self) -> &str {
        "ListMcpResources"
    }

    fn description(&self) -> &str {
        "List available MCP server resources"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        let server_filter = args.first().map(|s| s.as_str());
        Ok(format!(
            "ListMcpResources: {}",
            server_filter.unwrap_or("(all servers)")
        ))
    }
}

/// ReadMcpResourceTool - Read an MCP resource by URI
pub struct ReadMcpResourceTool;

impl ReadMcpResourceTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReadMcpResourceTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ReadMcpResourceTool {
    fn name(&self) -> &str {
        "ReadMcpResource"
    }

    fn description(&self) -> &str {
        "Read a specific MCP resource by URI"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("Usage: ReadMcpResource <uri>".to_string());
        }
        let uri = &args[0];
        Ok(format!("ReadMcpResource: {}", uri))
    }
}

/// McpServersTool - Manage MCP server connections
pub struct McpServersTool;

impl McpServersTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for McpServersTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for McpServersTool {
    fn name(&self) -> &str {
        "McpServers"
    }

    fn description(&self) -> &str {
        "List and manage MCP server connections"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        let action = args.first().map(|s| s.as_str()).unwrap_or("list");
        match action {
            "list" => Ok("McpServers: listing available servers".to_string()),
            "add" => Ok("McpServers: adding new server".to_string()),
            "remove" => Ok("McpServers: removing server".to_string()),
            _ => Ok(format!("McpServers: unknown action '{}'", action)),
        }
    }
}

/// Format MCP resources as markdown
pub fn format_mcp_resources(resources: &[&McpResource], servers: &[&McpServerInfo]) -> String {
    let mut md = String::from("# MCP Resources\n\n");

    if servers.is_empty() {
        md.push_str("No MCP servers connected.\n");
        return md;
    }

    for server in servers {
        md.push_str(&format!("## Server: {}\n", server.name));
        if let Some(ref version) = server.version {
            md.push_str(&format!("Version: {}\n\n", version));
        }
        if server.resources.is_empty() {
            md.push_str("No resources available.\n");
        } else {
            for resource in &server.resources {
                md.push_str(&format!("### {}\n", resource.name));
                md.push_str(&format!("URI: `{}`\n", resource.uri));
                if let Some(ref desc) = resource.description {
                    md.push_str(&format!("{}\n", desc));
                }
                if let Some(ref mime) = resource.mime_type {
                    md.push_str(&format!("Type: {}\n", mime));
                }
                md.push('\n');
            }
        }
        md.push('\n');
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_resource() {
        let resource = McpResource::new("file:///test", "Test Resource")
            .with_description("A test resource")
            .with_mime_type("text/plain");
        assert_eq!(resource.uri, "file:///test");
        assert!(resource.description.is_some());
    }

    #[test]
    fn test_mcp_client() {
        let mut client = McpClient::new();
        client.add_server(McpServerInfo {
            name: "test-server".to_string(),
            version: Some("1.0.0".to_string()),
            resources: vec![McpResource::new("file:///test", "Test")],
        });
        assert_eq!(client.list_servers().len(), 1);
    }

    #[test]
    fn test_list_mcp_resources_tool() {
        let tool = ListMcpResourcesTool::new();
        assert_eq!(tool.name(), "ListMcpResources");
    }

    #[test]
    fn test_read_mcp_resource_tool() {
        let tool = ReadMcpResourceTool::new();
        assert_eq!(tool.name(), "ReadMcpResource");
    }
}
