//! Tool implementations
//!
//! This module contains all available tools for Code Buddy.

pub mod acp_tool;
pub mod bash;
pub mod batch_tool;
pub mod container_tool;
pub mod cron_tool;
pub mod executor;
pub mod file;
pub mod glob;
pub mod grep;
pub mod image_tool;
pub mod interactive;
pub mod mcp;
pub mod moa_tool;
pub mod profile_tool;
pub mod sandbox_tool;
pub mod skin_tool;
pub mod task;
pub mod web;

#[cfg(test)]
mod tests;

use anyhow::Result;

/// Tool trait for implementing Claude Code tools
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: &[String]) -> Result<String>;
}

/// Async tool trait
pub trait AsyncTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: &[String]) -> impl std::future::Future<Output = Result<String>> + Send;
}

// Re-export commonly used types
pub use task::{Task, TaskList, TaskStatus, Priority, TaskStats, format_task_list};
pub use web::{
    WebSearchTool, WebFetchTool,
    WebFetchRequest, WebFetchResult,
    WebSearchRequest, WebSearchResult, WebSearchResponse,
    fetch_web, search_web, format_search_results, format_fetch_result,
};
pub use interactive::{
    AskUserQuestionTool, AskUserQuestionRequest, AskUserQuestionResponse,
    NotebookEditTool, Notebook, NotebookCell, CellType,
};
pub use mcp::{
    McpResource, McpServerInfo, McpClient,
    ListMcpResourcesTool, ReadMcpResourceTool, McpServersTool,
    format_mcp_resources,
};

// Import tool implementations
pub use acp_tool::AcpServerTool;
pub use batch_tool::BatchTool;
pub use bash::BashTool;
pub use container_tool::ContainerTool;
pub use cron_tool::CronTool;
pub use file::FileRead as ReadTool;
pub use file::FileWrite as WriteTool;
pub use file::FileEdit as EditTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use image_tool::ImageTool;
pub use moa_tool::MoATool;
pub use profile_tool::ProfileTool;
pub use sandbox_tool::SandboxTool;
pub use skin_tool::SkinTool;

/// Get all available tool names
pub fn all_tool_names() -> Vec<&'static str> {
    vec![
        "Bash",
        "Read",
        "Write",
        "Edit",
        "Glob",
        "Grep",
        "WebSearch",
        "WebFetch",
        "AskUserQuestion",
        "NotebookEdit",
        "ListMcpResources",
        "ReadMcpResource",
        "McpServers",
        "TaskCreate",
        "TaskComplete",
        "Cron",
        "Sandbox",
        "Container",
        "Batch",
        "MixtureOfAgents",
        "ImageGenerate",
        "Skin",
        "Profile",
        "AcpServer",
    ]
}

/// Get tool description
pub fn get_tool_description(name: &str) -> Option<&'static str> {
    match name.to_lowercase().as_str() {
        "bash" => Some("Execute shell commands"),
        "read" => Some("Read files from the filesystem"),
        "write" => Some("Write files to the filesystem"),
        "edit" => Some("Edit files with line-based changes"),
        "glob" => Some("Find files by glob pattern"),
        "grep" => Some("Search file contents"),
        "websearch" => Some("Search the web for information"),
        "webfetch" => Some("Fetch web page content"),
        "askuserquestion" => Some("Ask the user a question with optional choices"),
        "notebookedit" => Some("Edit Jupyter notebook cells"),
        "listmcpresources" => Some("List available MCP server resources"),
        "readmcpresource" => Some("Read a specific MCP resource by URI"),
        "mcpservers" => Some("List and manage MCP server connections"),
        "taskcreate" => Some("Create a new task"),
        "taskcomplete" => Some("Mark a task as completed"),
        "cron" => Some("Manage scheduled/recurring tasks (30m, 2h, cron expr)"),
        "sandbox" => Some("Execute code safely in sandboxed environment (python, bash, rust, etc.)"),
        "container" => Some("Execute in Docker, SSH, Modal or other backends"),
        "batch" => Some("Run multiple independent tasks in parallel"),
        "mixtureofagents" => Some("Use multiple AI agents for ensemble reasoning"),
        "imagegenerate" => Some("Generate AI images (DALL-E, Stable Diffusion)"),
        "skin" => Some("Manage CLI themes and appearance customization"),
        "profile" => Some("Manage isolated multi-instance environments"),
        "acpserver" => Some("Start IDE integration server (VS Code, JetBrains, Zed)"),
        _ => None,
    }
}
