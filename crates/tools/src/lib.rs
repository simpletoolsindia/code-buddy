//! Tool implementations and registry for Code Buddy.
//!
//! Each tool implements the [`Tool`] trait and is registered in [`ToolRegistry`].
//! The registry exposes all tool definitions to the provider, executes calls by
//! name, validates input against each tool's declared JSON Schema, and enforces
//! per-call timeouts.

pub mod bash;
pub mod fs;
pub mod parser;
pub mod path_utils;
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
/// It enforces a per-call timeout, validates inputs against declared JSON Schema,
/// and handles unknown-tool errors cleanly.
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

    /// Execute a named tool with schema validation and a per-call timeout.
    ///
    /// Steps:
    /// 1. Look up the tool by name → [`ToolError::UnknownTool`] if missing.
    /// 2. Validate `input` against the tool's `input_schema()` →
    ///    [`ToolError::SchemaValidation`] on mismatch.
    /// 3. Run with the configured timeout → [`ToolError::Timeout`] if exceeded.
    ///
    /// # Errors
    /// - [`ToolError::UnknownTool`] if the name is not registered.
    /// - [`ToolError::SchemaValidation`] if the input fails schema validation.
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

        let schema = tool.input_schema();
        validate_against_schema(name, &schema, &input)?;

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

// ── JSON Schema validation ─────────────────────────────────────────────────────

/// Validate `input` against a JSON Schema `schema`.
///
/// Covers the subset of JSON Schema used by all built-in tool schemas:
/// - `type: "object"` — input must be a JSON object.
/// - `required: [...]` — each listed field must be present.
/// - `properties.*.type` — if a field is present and its declared type is
///   one of `string | number | integer | boolean | array | object | null`,
///   its runtime type must match.
///
/// Other JSON Schema keywords (e.g. `minimum`, `pattern`, `enum`) are silently
/// ignored; per-tool `execute()` implementations validate those constraints
/// themselves with richer error messages.
pub(crate) fn validate_against_schema(
    tool: &str,
    schema: &Value,
    input: &Value,
) -> Result<(), ToolError> {
    // Step 1: input must be an object (or null/empty if schema says so).
    // All built-in tool schemas declare `type: "object"`.
    if schema.get("type").and_then(Value::as_str) == Some("object") {
        if !input.is_object() {
            return Err(ToolError::SchemaValidation {
                tool: tool.to_string(),
                reason: format!(
                    "input must be a JSON object, got {}",
                    json_type_name(input)
                ),
            });
        }
    }

    // Step 2: check required fields.
    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for field_val in required {
            if let Some(field) = field_val.as_str() {
                if input.get(field).is_none() {
                    return Err(ToolError::SchemaValidation {
                        tool: tool.to_string(),
                        reason: format!("missing required field '{field}'"),
                    });
                }
            }
        }
    }

    // Step 3: type-check declared properties that are present in input.
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        for (field, prop_schema) in properties {
            if let Some(value) = input.get(field) {
                if let Some(declared_type) = prop_schema.get("type").and_then(Value::as_str) {
                    let matches = json_value_matches_type(value, declared_type);
                    if !matches {
                        return Err(ToolError::SchemaValidation {
                            tool: tool.to_string(),
                            reason: format!(
                                "field '{field}' must be {declared_type}, got {}",
                                json_type_name(value)
                            ),
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

/// Returns `true` if `value` matches the JSON Schema primitive `type` string.
fn json_value_matches_type(value: &Value, type_str: &str) -> bool {
    match type_str {
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => value.is_i64() || value.is_u64(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "null" => value.is_null(),
        _ => true, // Unknown type keywords are not rejected.
    }
}

/// Human-readable name for the JSON type of `value`.
fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── Helpers ───────────────────────────────────────────────────────────────

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
            json!({
                "type": "object",
                "properties": {
                    "msg": { "type": "string" },
                    "count": { "type": "integer" }
                },
                "required": ["msg"]
            })
        }
        async fn execute(&self, input: Value) -> Result<String, ToolError> {
            Ok(input["msg"].as_str().unwrap_or("").to_string())
        }
    }

    // ── ToolRegistry ──────────────────────────────────────────────────────────

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
                json!({ "type": "object" })
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

    // ── JSON Schema validation ─────────────────────────────────────────────────

    #[test]
    fn schema_validation_passes_valid_input() {
        let schema = json!({
            "type": "object",
            "properties": { "msg": { "type": "string" } },
            "required": ["msg"]
        });
        assert!(validate_against_schema("echo", &schema, &json!({ "msg": "hi" })).is_ok());
    }

    #[test]
    fn schema_validation_rejects_missing_required_field() {
        let schema = json!({
            "type": "object",
            "properties": { "msg": { "type": "string" } },
            "required": ["msg"]
        });
        let err = validate_against_schema("echo", &schema, &json!({})).unwrap_err();
        assert!(
            matches!(err, ToolError::SchemaValidation { ref reason, .. } if reason.contains("msg")),
            "expected missing-field error, got {err:?}"
        );
    }

    #[test]
    fn schema_validation_rejects_non_object_input() {
        let schema = json!({ "type": "object" });
        let err = validate_against_schema("echo", &schema, &json!("a string")).unwrap_err();
        assert!(matches!(err, ToolError::SchemaValidation { .. }));
    }

    #[test]
    fn schema_validation_rejects_wrong_field_type() {
        let schema = json!({
            "type": "object",
            "properties": { "count": { "type": "integer" } },
            "required": ["count"]
        });
        let err =
            validate_against_schema("echo", &schema, &json!({ "count": "not-a-number" }))
                .unwrap_err();
        assert!(
            matches!(err, ToolError::SchemaValidation { ref reason, .. } if reason.contains("count")),
            "expected type error for count, got {err:?}"
        );
    }

    #[test]
    fn schema_validation_allows_extra_fields() {
        let schema = json!({
            "type": "object",
            "properties": { "msg": { "type": "string" } },
            "required": ["msg"]
        });
        // Extra field "extra" is allowed (additionalProperties not enforced).
        let result =
            validate_against_schema("echo", &schema, &json!({ "msg": "hi", "extra": 42 }));
        assert!(result.is_ok());
    }

    /// Schema validation happens BEFORE tool execution; the registry must reject
    /// missing required fields before ever calling `execute()`.
    #[tokio::test]
    async fn registry_rejects_missing_required_field_before_execution() {
        let mut reg = ToolRegistry::new();
        reg.register(EchoTool);
        let err = reg.execute("echo", json!({})).await.unwrap_err();
        assert!(
            matches!(err, ToolError::SchemaValidation { .. }),
            "expected SchemaValidation, got {err:?}"
        );
    }

    /// Wrong type for a declared field is caught by schema validation.
    #[tokio::test]
    async fn registry_rejects_wrong_field_type() {
        let mut reg = ToolRegistry::new();
        reg.register(EchoTool);
        // `count` is declared as `integer`, passing a string should fail validation.
        let err = reg
            .execute("echo", json!({ "msg": "ok", "count": "not-an-int" }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::SchemaValidation { .. }));
    }
}
