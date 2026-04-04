//! Core message types for LLM API communication.
//!
//! These types follow the Anthropic message format as the canonical internal
//! representation. Provider adapters translate to/from provider-specific formats.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A request to send to a language model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageRequest {
    /// Model identifier (e.g. "mistralai/mistral-7b-instruct").
    pub model: String,

    /// Maximum tokens to generate.
    pub max_tokens: u32,

    /// Conversation messages.
    pub messages: Vec<InputMessage>,

    /// Optional system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Tools available to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,

    /// Tool choice behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Enable streaming.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stream: bool,

    /// Sampling temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

impl MessageRequest {
    /// Create a simple single-turn user request.
    #[must_use]
    pub fn simple(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            max_tokens: 4096,
            messages: vec![InputMessage::user_text(prompt)],
            system: None,
            tools: None,
            tool_choice: None,
            stream: false,
            temperature: None,
        }
    }

    /// Enable streaming on this request.
    #[must_use]
    pub fn with_streaming(mut self) -> Self {
        self.stream = true;
        self
    }
}

/// A message in the conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputMessage {
    /// Role: "user" or "assistant".
    pub role: String,
    /// Content blocks.
    pub content: Vec<InputContentBlock>,
}

impl InputMessage {
    /// Create a user text message.
    #[must_use]
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: vec![InputContentBlock::Text { text: text.into() }],
        }
    }

    /// Create an assistant text message.
    #[must_use]
    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: vec![InputContentBlock::Text { text: text.into() }],
        }
    }

    /// Create a user message containing a tool result.
    #[must_use]
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self {
            role: "user".to_string(),
            content: vec![InputContentBlock::ToolResult {
                tool_use_id: tool_use_id.into(),
                content: content.into(),
                is_error,
            }],
        }
    }
}

/// A content block in an input message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
    ToolResult { tool_use_id: String, content: String, #[serde(default)] is_error: bool },
}

/// A content block in a model output message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
}

/// Full response from the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageResponse {
    /// Response ID.
    pub id: String,
    /// Model that responded.
    pub model: String,
    /// Output content blocks.
    pub content: Vec<OutputContentBlock>,
    /// Stop reason.
    #[serde(default)]
    pub stop_reason: Option<String>,
    /// Token usage.
    pub usage: Usage,
}

impl MessageResponse {
    /// Extract all text content concatenated.
    #[must_use]
    pub fn text_content(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| {
                if let OutputContentBlock::Text { text } = b {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Extract all tool use blocks.
    #[must_use]
    pub fn tool_calls(&self) -> Vec<(&str, &str, &Value)> {
        self.content
            .iter()
            .filter_map(|b| {
                if let OutputContentBlock::ToolUse { id, name, input } = b {
                    Some((id.as_str(), name.as_str(), input))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Whether the model stopped due to a tool call.
    #[must_use]
    pub fn stopped_for_tool(&self) -> bool {
        self.stop_reason.as_deref() == Some("tool_use")
    }
}

/// Token usage statistics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// A single content block during streaming.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
}

/// An event emitted by a streaming provider.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    /// A text delta fragment.
    TextDelta(String),
    /// A tool call has started (id, name, partial_input).
    ToolUseDelta { id: String, name: String, input_json: String },
    /// Token usage for this turn.
    Usage(Usage),
    /// The model has stopped.
    MessageStop,
}

/// Tool definition sent to the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema object describing the tool's input.
    pub input_schema: Value,
}

/// How the model should use tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    Any,
    Tool { name: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn simple_request_roundtrip() {
        let req = MessageRequest::simple("test-model", "Hello");
        let json = serde_json::to_string(&req).unwrap();
        let decoded: MessageRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn user_text_message() {
        let msg = InputMessage::user_text("Hello, world!");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content.len(), 1);
        if let InputContentBlock::Text { text } = &msg.content[0] {
            assert_eq!(text, "Hello, world!");
        } else {
            panic!("expected text content block");
        }
    }

    #[test]
    fn tool_result_message() {
        let msg = InputMessage::tool_result("call_1", "file contents here", false);
        assert_eq!(msg.role, "user");
        if let InputContentBlock::ToolResult { tool_use_id, content, is_error } = &msg.content[0] {
            assert_eq!(tool_use_id, "call_1");
            assert_eq!(content, "file contents here");
            assert!(!is_error);
        } else {
            panic!("expected tool result block");
        }
    }

    #[test]
    fn response_text_content() {
        let resp = MessageResponse {
            id: "msg_1".to_string(),
            model: "test".to_string(),
            content: vec![
                OutputContentBlock::Text { text: "Hello ".to_string() },
                OutputContentBlock::Text { text: "world".to_string() },
            ],
            stop_reason: Some("end_turn".to_string()),
            usage: Usage { input_tokens: 10, output_tokens: 20 },
        };
        assert_eq!(resp.text_content(), "Hello world");
    }

    #[test]
    fn response_tool_calls() {
        let input = json!({"path": "/tmp/test.txt"});
        let resp = MessageResponse {
            id: "msg_2".to_string(),
            model: "test".to_string(),
            content: vec![
                OutputContentBlock::ToolUse {
                    id: "call_1".to_string(),
                    name: "read_file".to_string(),
                    input: input.clone(),
                },
            ],
            stop_reason: Some("tool_use".to_string()),
            usage: Usage::default(),
        };
        assert!(resp.stopped_for_tool());
        let calls = resp.tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, "read_file");
    }
}
