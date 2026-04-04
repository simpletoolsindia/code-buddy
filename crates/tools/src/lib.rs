//! Tool implementations and registry for Code Buddy.
//!
//! Each tool implements the [`Tool`] trait and is registered in [`ToolRegistry`].
//! The registry exposes all tool definitions to the provider, executes calls by
//! name, and enforces per-call timeouts.

pub mod bash;
pub mod fs;
pub mod parser;
pub mod search;

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use code_buddy_errors::ToolError;
use code_buddy_transport::ToolDefinition;
use serde_json::Value;
use tracing::instrument;

/// A single tool that the model can call.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Stable tool name (used in requests to the model).
    fn name(&self) -> &str;

    /// Short, human-readable description for the model.
    fn description(&self) -> &str;

    /// JSON Schema for the input object.
    fn input_schema(&self) -> Value;

    /// Execute the tool with the given input.
    ///
    /// # Errors
    /// Returns [`ToolError`] on invalid args, execution failure, or timeout.
    async fn execute(&self, input: Value) -> Result<String, ToolError>;
}

/// Registry of all available tools.
///
/// The registry owns a map of named tools and is the sole execution path.
/// It enforces a per-call timeout and validates tool names.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    tool_timeout: Duration,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// Create an empty registry with a 30-second default timeout.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            tool_timeout: Duration::from_secs(30),
        }
    }

    /// Override the per-call tool execution timeout.
    #[must_use]
    pub fn with_timeout(mut self, d: Duration) -> Self {
        self.tool_timeout = d;
        self
    }

    /// Register a tool, replacing any existing tool with the same name.
    pub fn register(&mut self, tool: impl Tool + 'static) {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }

    /// Register all six built-in tools scoped to `cwd`.
    pub fn register_builtin(&mut self, cwd: std::path::PathBuf) {
        self.register(bash::BashTool::new(cwd.clone()));
        self.register(fs::ReadFileTool::new(cwd.clone()));
        self.register(fs::WriteFileTool::new(cwd.clone()));
        self.register(fs::EditFileTool::new(cwd.clone()));
        self.register(search::GlobSearchTool::new(cwd.clone()));
        self.register(search::GrepSearchTool::new(cwd));
    }

    /// Returns `true` if no tools are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Tool definitions to include in a provider request.
    #[must_use]
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        let mut defs: Vec<ToolDefinition> = self
            .tools
            .values()
            .map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: Some(t.description().to_string()),
                input_schema: t.input_schema(),
            })
            .collect();
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
    }

    /// Execute a named tool with a per-call timeout.
    ///
    /// # Errors
    /// - [`ToolError::UnknownTool`] if the name is not registered.
    /// - [`ToolError::Timeout`] if execution exceeds the configured timeout.
    /// - Any error returned by the tool itself.
    #[instrument(skip(self), fields(tool = name))]
    pub async fn execute(&self, name: &str, input: Value) -> Result<String, ToolError> {
        let tool = self.tools.get(name).ok_or_else(|| ToolError::UnknownTool {
            name: name.to_string(),
            available: self
                .tools
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        })?;

        let timeout = self.tool_timeout;
        let seconds = timeout.as_secs();

        tokio::time::timeout(timeout, tool.execute(input))
            .await
            .map_err(|_| ToolError::Timeout {
                tool: name.to_string(),
                seconds,
            })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echo input"
        }
        fn input_schema(&self) -> Value {
            json!({ "type": "object", "properties": { "msg": { "type": "string" } } })
        }
        async fn execute(&self, input: Value) -> Result<String, ToolError> {
            Ok(input["msg"].as_str().unwrap_or("").to_string())
        }
    }

    #[tokio::test]
    async fn registry_executes_registered_tool() {
        let mut reg = ToolRegistry::new();
        reg.register(EchoTool);
        let result = reg.execute("echo", json!({ "msg": "hello" })).await.unwrap();
        assert_eq!(result, "hello");
    }

    #[tokio::test]
    async fn registry_unknown_tool_returns_error() {
        let reg = ToolRegistry::new();
        let err = reg.execute("missing", json!({})).await.unwrap_err();
        assert!(matches!(err, ToolError::UnknownTool { .. }));
    }

    #[tokio::test]
    async fn registry_timeout_returns_error() {
        struct SlowTool;
        #[async_trait]
        impl Tool for SlowTool {
            fn name(&self) -> &str {
                "slow"
            }
            fn description(&self) -> &str {
                "slow"
            }
            fn input_schema(&self) -> Value {
                json!({})
            }
            async fn execute(&self, _input: Value) -> Result<String, ToolError> {
                tokio::time::sleep(Duration::from_secs(10)).await;
                Ok("never".to_string())
            }
        }

        let mut reg = ToolRegistry::new().with_timeout(Duration::from_millis(50));
        reg.register(SlowTool);
        let err = reg.execute("slow", json!({})).await.unwrap_err();
        assert!(matches!(err, ToolError::Timeout { .. }));
    }

    #[test]
    fn definitions_sorted_by_name() {
        let mut reg = ToolRegistry::new();
        reg.register(EchoTool);
        let defs = reg.definitions();
        assert_eq!(defs[0].name, "echo");
        assert_eq!(defs[0].description.as_deref(), Some("Echo input"));
    }
}
