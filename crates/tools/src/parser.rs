//! Tool call parser: extracts tool calls from model responses and streaming events.
//!
//! Handles three cases:
//! 1. Complete responses — extract `OutputContentBlock::ToolUse` blocks directly.
//! 2. Streaming events — accumulate `ToolUseDelta` fragments, parse at `MessageStop`.
//! 3. Malformed JSON — attempt repair; fall back to `ToolError::ParseFailed`.

use code_buddy_errors::ToolError;
use code_buddy_transport::{MessageResponse, OutputContentBlock, StreamEvent};
use serde_json::Value;

/// A fully parsed tool call ready for execution.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedToolCall {
    /// Opaque call identifier (used to correlate results back to the model).
    pub id: String,
    /// Tool name (must match a registered tool).
    pub name: String,
    /// Parsed JSON input object.
    pub input: Value,
}

// ── From complete MessageResponse ─────────────────────────────────────────────

/// Extract all tool calls from a complete `MessageResponse`.
///
/// Returns an empty vec if no `ToolUse` blocks are present.
///
/// # Errors
/// Returns [`ToolError::ParseFailed`] if any tool use block contains
/// non-object input JSON.
pub fn extract_tool_calls(
    response: &MessageResponse,
) -> Result<Vec<ParsedToolCall>, ToolError> {
    let mut calls = Vec::new();
    for block in &response.content {
        if let OutputContentBlock::ToolUse { id, name, input } = block {
            validate_tool_call_input(name, input)?;
            calls.push(ParsedToolCall {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            });
        }
    }
    Ok(calls)
}

/// Validate that a tool call input is a JSON object.
fn validate_tool_call_input(name: &str, input: &Value) -> Result<(), ToolError> {
    if !input.is_object() && !input.is_null() {
        return Err(ToolError::SchemaValidation {
            tool: name.to_string(),
            reason: format!(
                "input must be a JSON object, got {}",
                json_type_name(input)
            ),
        });
    }
    Ok(())
}

fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ── Streaming accumulator ─────────────────────────────────────────────────────

/// Accumulates `ToolUseDelta` streaming events and produces `ParsedToolCall`s
/// when the stream ends.
///
/// Models may emit several `ToolUseDelta` events for a single tool call
/// (each carrying a fragment of the JSON input). This accumulator collects
/// those fragments in order and parses the complete JSON at `MessageStop`.
#[derive(Debug, Default)]
pub struct StreamingToolCallAccumulator {
    /// Ordered list of (id, name, accumulated_input_json).
    calls: Vec<(String, String, String)>,
}

impl StreamingToolCallAccumulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a single `ToolUseDelta` event.
    pub fn feed_delta(&mut self, id: &str, name: &str, input_json: &str) {
        // Find existing accumulator for this id, or create a new one.
        if let Some(entry) = self.calls.iter_mut().find(|(eid, _, _)| eid == id) {
            entry.2.push_str(input_json);
        } else {
            self.calls.push((id.to_string(), name.to_string(), input_json.to_string()));
        }
    }

    /// Feed a `StreamEvent`, returning the text delta if present.
    ///
    /// Returns `Some(text)` for `TextDelta` events, `None` otherwise.
    pub fn process_event(&mut self, event: &StreamEvent) -> Option<String> {
        match event {
            StreamEvent::TextDelta(text) => Some(text.clone()),
            StreamEvent::ToolUseDelta { id, name, input_json } => {
                self.feed_delta(id, name, input_json);
                None
            }
            _ => None,
        }
    }

    /// Consume the accumulator and parse all collected tool calls.
    ///
    /// For each accumulated call:
    /// - If the input JSON is empty or `{}`, uses an empty object.
    /// - If the JSON is valid, parses it.
    /// - If the JSON is malformed, attempts best-effort repair; on failure
    ///   returns [`ToolError::ParseFailed`] for that call.
    pub fn finish(self) -> Result<Vec<ParsedToolCall>, ToolError> {
        let mut results = Vec::new();
        for (id, name, raw_json) in self.calls {
            let input = parse_tool_input_json(&name, &raw_json)?;
            results.push(ParsedToolCall { id, name, input });
        }
        Ok(results)
    }
}

/// Parse tool call input JSON with best-effort repair for weak local models.
///
/// Attempts:
/// 1. Direct parse.
/// 2. Append a closing `}` if it looks like a truncated object.
/// 3. Wrap in `{}` if the string is empty.
///
/// If all attempts fail, returns [`ToolError::ParseFailed`].
pub fn parse_tool_input_json(tool_name: &str, raw: &str) -> Result<Value, ToolError> {
    let trimmed = raw.trim();

    // Fast path: empty input → empty object.
    if trimmed.is_empty() {
        return Ok(Value::Object(serde_json::Map::new()));
    }

    // Attempt 1: direct parse.
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        if v.is_object() || v.is_null() {
            return Ok(v);
        }
        // Non-object value — wrap it if possible.
        return Err(ToolError::ParseFailed {
            reason: format!(
                "tool '{tool_name}' input must be an object, got: {trimmed}"
            ),
        });
    }

    // Attempt 2: append closing brace (handles truncated streaming JSON).
    let repaired = format!("{trimmed}}}");
    if let Ok(v) = serde_json::from_str::<Value>(&repaired) {
        if v.is_object() {
            return Ok(v);
        }
    }

    // Attempt 3: wrap the whole thing in an object under `"_raw"`.
    // This is a last resort so the model's intent is preserved.
    // The runtime may choose to surface this as a warning.
    Err(ToolError::ParseFailed {
        reason: format!("could not parse tool '{tool_name}' input as JSON: {trimmed}"),
    })
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use code_buddy_transport::{MessageResponse, OutputContentBlock, Usage};
    use serde_json::json;

    fn make_response(blocks: Vec<OutputContentBlock>) -> MessageResponse {
        MessageResponse {
            id: "msg_1".to_string(),
            model: "test".to_string(),
            content: blocks,
            stop_reason: Some("tool_use".to_string()),
            usage: Usage::default(),
        }
    }

    // ── extract_tool_calls ────────────────────────────────────────────────────

    #[test]
    fn extract_single_tool_call() {
        let resp = make_response(vec![OutputContentBlock::ToolUse {
            id: "call_1".to_string(),
            name: "read_file".to_string(),
            input: json!({ "path": "src/main.rs" }),
        }]);

        let calls = extract_tool_calls(&resp).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].input["path"], "src/main.rs");
    }

    #[test]
    fn extract_multiple_tool_calls() {
        let resp = make_response(vec![
            OutputContentBlock::ToolUse {
                id: "c1".to_string(),
                name: "read_file".to_string(),
                input: json!({ "path": "a.rs" }),
            },
            OutputContentBlock::ToolUse {
                id: "c2".to_string(),
                name: "bash".to_string(),
                input: json!({ "command": "ls" }),
            },
        ]);

        let calls = extract_tool_calls(&resp).unwrap();
        assert_eq!(calls.len(), 2);
    }

    #[test]
    fn extract_no_tool_calls_returns_empty() {
        let resp = make_response(vec![OutputContentBlock::Text {
            text: "hello".to_string(),
        }]);
        let calls = extract_tool_calls(&resp).unwrap();
        assert!(calls.is_empty());
    }

    #[test]
    fn extract_non_object_input_is_schema_error() {
        let resp = make_response(vec![OutputContentBlock::ToolUse {
            id: "c1".to_string(),
            name: "bash".to_string(),
            input: json!("not an object"),
        }]);
        let err = extract_tool_calls(&resp).unwrap_err();
        assert!(matches!(err, ToolError::SchemaValidation { .. }));
    }

    // ── parse_tool_input_json ─────────────────────────────────────────────────

    #[test]
    fn parse_valid_json_object() {
        let v = parse_tool_input_json("bash", r#"{"command": "ls"}"#).unwrap();
        assert_eq!(v["command"], "ls");
    }

    #[test]
    fn parse_empty_string_becomes_empty_object() {
        let v = parse_tool_input_json("echo", "").unwrap();
        assert!(v.is_object());
        assert_eq!(v.as_object().unwrap().len(), 0);
    }

    #[test]
    fn parse_truncated_json_repaired() {
        // Missing closing brace — should be repaired.
        let v = parse_tool_input_json("bash", r#"{"command": "ls""#).unwrap();
        assert_eq!(v["command"], "ls");
    }

    /// Malformed JSON (neither parseable nor repairable) returns ParseFailed.
    #[test]
    fn parse_malformed_json_returns_parse_failed() {
        let err = parse_tool_input_json("bash", "not json at all {{{{").unwrap_err();
        assert!(matches!(err, ToolError::ParseFailed { .. }));
    }

    // ── StreamingToolCallAccumulator ──────────────────────────────────────────

    #[test]
    fn accumulator_single_delta() {
        let mut acc = StreamingToolCallAccumulator::new();
        acc.feed_delta("id1", "bash", r#"{"command": "ls"}"#);
        let calls = acc.finish().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "bash");
        assert_eq!(calls[0].input["command"], "ls");
    }

    #[test]
    fn accumulator_fragmented_deltas() {
        let mut acc = StreamingToolCallAccumulator::new();
        // JSON arrives in three fragments.
        acc.feed_delta("id1", "bash", r#"{"comm"#);
        acc.feed_delta("id1", "bash", r#"and": "#);
        acc.feed_delta("id1", "bash", r#""ls"}"#);
        let calls = acc.finish().unwrap();
        assert_eq!(calls[0].input["command"], "ls");
    }

    #[test]
    fn accumulator_multiple_distinct_calls() {
        let mut acc = StreamingToolCallAccumulator::new();
        acc.feed_delta("id1", "read_file", r#"{"path": "a.rs"}"#);
        acc.feed_delta("id2", "bash", r#"{"command": "cargo build"}"#);
        let calls = acc.finish().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].id, "id1");
        assert_eq!(calls[1].id, "id2");
    }

    #[test]
    fn accumulator_malformed_input_returns_error() {
        let mut acc = StreamingToolCallAccumulator::new();
        acc.feed_delta("id1", "bash", "!!! definitely not json");
        let err = acc.finish().unwrap_err();
        assert!(matches!(err, ToolError::ParseFailed { .. }));
    }

    #[test]
    fn accumulator_text_delta_returned_by_process_event() {
        let mut acc = StreamingToolCallAccumulator::new();
        let text = acc.process_event(&StreamEvent::TextDelta("hello".to_string()));
        assert_eq!(text, Some("hello".to_string()));
    }

    #[test]
    fn accumulator_tool_delta_processed_not_returned() {
        let mut acc = StreamingToolCallAccumulator::new();
        let result = acc.process_event(&StreamEvent::ToolUseDelta {
            id: "id1".to_string(),
            name: "bash".to_string(),
            input_json: r#"{"command":"ls"}"#.to_string(),
        });
        assert_eq!(result, None);
        // But the call should be accumulated.
        let calls = acc.finish().unwrap();
        assert_eq!(calls.len(), 1);
    }
}
