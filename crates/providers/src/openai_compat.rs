//! Generic OpenAI chat-completions adapter.
//!
//! All supported providers (LM Studio, OpenRouter, NVIDIA, OpenAI, custom) speak the
//! OpenAI `/v1/chat/completions` wire format. This module implements the shared
//! HTTP logic once, parameterised by [`AdapterConfig`] for per-provider differences.
//!
//! # Retry policy
//! Transient errors (5xx responses, network timeouts, connection failures) are retried
//! with exponential backoff: 200 ms → 400 ms → 800 ms, capped at `max_retries` (default 3).
//! Non-retryable errors (4xx, auth failures, parse errors) are returned immediately.
//!
//! # Streaming
//! Streaming uses the SSE [`SseParser`] from the transport crate. Each `data:` frame is
//! deserialized as a `ChatCompletionChunk` and translated to a [`StreamEvent`].

use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use async_trait::async_trait;
use code_buddy_errors::TransportError;
use code_buddy_transport::{
    MessageRequest, MessageResponse, OutputContentBlock, Provider, SseParser, StreamEvent,
    StreamSource, Usage,
};
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{debug, warn};

// ── Adapter configuration ─────────────────────────────────────────────────────

/// Per-provider configuration for the OpenAI-compat adapter.
#[derive(Debug, Clone)]
pub struct AdapterConfig {
    /// Human-readable provider name, used in error messages.
    pub provider_name: String,
    /// Base URL for the chat completions API (e.g. `http://localhost:1234/v1`).
    pub base_url: String,
    /// API key, if required. Empty string for unauthenticated local providers.
    pub api_key: String,
    /// Request timeout.
    pub timeout: Duration,
    /// Max retries on transient errors.
    pub max_retries: u32,
    /// Whether this provider is local (affects error messaging).
    pub is_local: bool,
}

impl AdapterConfig {
    /// LM Studio default config (no auth, localhost).
    #[must_use]
    pub fn lm_studio() -> Self {
        Self {
            provider_name: "LM Studio".to_string(),
            base_url: "http://localhost:1234/v1".to_string(),
            api_key: String::new(),
            timeout: Duration::from_secs(120),
            max_retries: 3,
            is_local: true,
        }
    }

    /// OpenRouter config.
    #[must_use]
    pub fn openrouter(api_key: impl Into<String>) -> Self {
        Self {
            provider_name: "OpenRouter".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            api_key: api_key.into(),
            timeout: Duration::from_secs(120),
            max_retries: 3,
            is_local: false,
        }
    }

    /// NVIDIA NIM config.
    #[must_use]
    pub fn nvidia(api_key: impl Into<String>) -> Self {
        Self {
            provider_name: "NVIDIA".to_string(),
            base_url: "https://integrate.api.nvidia.com/v1".to_string(),
            api_key: api_key.into(),
            timeout: Duration::from_secs(120),
            max_retries: 3,
            is_local: false,
        }
    }

    /// OpenAI config.
    #[must_use]
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self {
            provider_name: "OpenAI".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: api_key.into(),
            timeout: Duration::from_secs(120),
            max_retries: 3,
            is_local: false,
        }
    }

    /// Custom endpoint config.
    #[must_use]
    pub fn custom(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            provider_name: name.into(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            timeout: Duration::from_secs(120),
            max_retries: 3,
            is_local: true,
        }
    }

    /// Override the base URL.
    #[must_use]
    pub fn with_base_url_override(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    /// Override timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Override max retries.
    #[must_use]
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

// ── Wire types ────────────────────────────────────────────────────────────────

/// Chat completions request body (OpenAI wire format).
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<Value>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    tool_calls: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

/// Non-streaming chat completion response.
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    id: String,
    #[serde(default)]
    model: String,
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    #[serde(default)]
    #[allow(dead_code)]
    role: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCallObject>,
}

#[derive(Debug, Deserialize)]
struct ToolCallObject {
    id: String,
    function: ToolCallFunction,
}

#[derive(Debug, Deserialize)]
struct ToolCallFunction {
    name: String,
    #[serde(default)]
    arguments: String,
}

#[derive(Debug, Deserialize, Default)]
struct ChatUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

/// Streaming chunk response.
#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
    #[serde(default)]
    #[allow(dead_code)]
    id: String,
    #[serde(default)]
    #[allow(dead_code)]
    model: String,
    choices: Vec<ChunkChoice>,
    #[serde(default)]
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    delta: ChunkDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ChunkDelta {
    #[serde(default)]
    #[allow(dead_code)]
    role: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCallChunk>,
}

#[derive(Debug, Deserialize)]
struct ToolCallChunk {
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    function: Option<ToolCallChunkFunction>,
}

#[derive(Debug, Deserialize)]
struct ToolCallChunkFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

// ── Tool call accumulator ─────────────────────────────────────────────────────

/// Accumulates streaming tool call deltas into complete tool call events.
#[derive(Debug, Default)]
struct ToolCallAccumulator {
    calls: HashMap<usize, AccumulatedToolCall>,
}

#[derive(Debug, Default)]
struct AccumulatedToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl ToolCallAccumulator {
    fn ingest(&mut self, chunk: &ToolCallChunk) {
        let entry = self.calls.entry(chunk.index).or_default();
        if let Some(ref id) = chunk.id {
            entry.id.clone_from(id);
        }
        if let Some(ref func) = chunk.function {
            if let Some(ref name) = func.name {
                entry.name.clone_from(name);
            }
            if let Some(ref args) = func.arguments {
                entry.arguments.push_str(args);
            }
        }
    }

    fn drain_complete(&mut self) -> Vec<StreamEvent> {
        self.calls
            .drain()
            .map(|(_, acc)| StreamEvent::ToolUseDelta {
                id: acc.id,
                name: acc.name,
                input_json: acc.arguments,
            })
            .collect()
    }
}

// ── Parse a single SSE data frame into StreamEvents ───────────────────────────

/// Parse one SSE data frame (`data:` value) into [`StreamEvent`]s.
///
/// Returns `(events, is_done)`. `is_done=true` when `[DONE]` is seen.
fn parse_chunk_events(
    data: &str,
    provider_name: &str,
    tool_accumulator: &mut ToolCallAccumulator,
) -> (Vec<StreamEvent>, bool) {
    if data.trim() == "[DONE]" {
        let mut events = tool_accumulator.drain_complete();
        events.push(StreamEvent::MessageStop);
        return (events, true);
    }

    let chunk: ChatCompletionChunk = match serde_json::from_str(data) {
        Ok(c) => c,
        Err(e) => {
            debug!(
                provider = %provider_name,
                data = %data,
                error = %e,
                "failed to parse SSE chunk, skipping"
            );
            return (vec![], false);
        }
    };

    let mut events: Vec<StreamEvent> = Vec::new();

    if let Some(usage) = chunk.usage {
        events.push(StreamEvent::Usage(Usage {
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
        }));
    }

    for choice in chunk.choices {
        if let Some(text) = choice.delta.content {
            if !text.is_empty() {
                events.push(StreamEvent::TextDelta(text));
            }
        }
        for tc in &choice.delta.tool_calls {
            tool_accumulator.ingest(tc);
        }
        if choice.finish_reason.as_deref() == Some("tool_calls") {
            events.extend(tool_accumulator.drain_complete());
        }
    }

    (events, false)
}

// ── SSE streaming source ──────────────────────────────────────────────────────

/// A streaming source backed by an SSE HTTP response body.
pub struct SseStreamSource {
    inner: Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send + Unpin>,
    parser: SseParser,
    provider_name: String,
    pending: VecDeque<StreamEvent>,
    done: bool,
    tool_accumulator: ToolCallAccumulator,
}

impl SseStreamSource {
    #[must_use]
    pub fn new(
        stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + Unpin + 'static,
        provider_name: String,
    ) -> Self {
        Self {
            inner: Box::new(stream),
            parser: SseParser::new(),
            provider_name,
            pending: VecDeque::new(),
            done: false,
            tool_accumulator: ToolCallAccumulator::default(),
        }
    }
}

#[async_trait]
impl StreamSource for SseStreamSource {
    async fn next_event(&mut self) -> Result<Option<StreamEvent>, TransportError> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Ok(Some(event));
            }

            if self.done {
                return Ok(None);
            }

            let chunk_bytes = match self.inner.next().await {
                Some(Ok(bytes)) => bytes,
                Some(Err(e)) => {
                    return Err(TransportError::Network {
                        provider: self.provider_name.clone(),
                        detail: e.to_string(),
                    });
                }
                None => {
                    // Stream ended — flush parser and emit stop
                    let finish_frames = self.parser.finish();
                    for frame in &finish_frames {
                        let (events, done) = parse_chunk_events(
                            &frame.data,
                            &self.provider_name,
                            &mut self.tool_accumulator,
                        );
                        self.pending.extend(events);
                        if done {
                            self.done = true;
                        }
                    }
                    if !self.done {
                        // Emit any accumulated tool calls + stop
                        self.pending
                            .extend(self.tool_accumulator.drain_complete());
                        self.pending.push_back(StreamEvent::MessageStop);
                        self.done = true;
                    }
                    continue;
                }
            };

            let frames = self.parser.push(&chunk_bytes);
            for frame in &frames {
                let (events, done) = parse_chunk_events(
                    &frame.data,
                    &self.provider_name,
                    &mut self.tool_accumulator,
                );
                self.pending.extend(events);
                if done {
                    self.done = true;
                }
            }
        }
    }
}

// ── Adapter ───────────────────────────────────────────────────────────────────

/// Generic OpenAI-compatible provider adapter.
///
/// Handles request building, auth, retry/timeout logic, response normalization,
/// and SSE streaming for all OpenAI-format endpoints.
#[derive(Debug, Clone)]
pub struct OpenAiCompatAdapter {
    http: reqwest::Client,
    config: AdapterConfig,
}

impl OpenAiCompatAdapter {
    /// Create a new adapter with the given config.
    #[must_use]
    pub fn new(config: AdapterConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_default();
        Self { http, config }
    }

    fn endpoint(&self) -> String {
        completions_endpoint(&self.config.base_url)
    }

    fn request_builder(&self) -> reqwest::RequestBuilder {
        let builder = self.http.post(self.endpoint());
        if self.config.api_key.is_empty() {
            builder
        } else {
            builder.bearer_auth(&self.config.api_key)
        }
    }

    fn build_chat_request(&self, req: &MessageRequest, stream: bool) -> ChatRequest {
        let mut messages: Vec<ChatMessage> = Vec::new();

        if let Some(ref sys) = req.system {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(sys.clone()),
                tool_calls: vec![],
                tool_call_id: None,
                name: None,
            });
        }

        for msg in &req.messages {
            use code_buddy_transport::InputContentBlock;

            let text_parts: Vec<&str> = msg
                .content
                .iter()
                .filter_map(|b| {
                    if let InputContentBlock::Text { text } = b {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect();

            let tool_calls: Vec<Value> = msg
                .content
                .iter()
                .filter_map(|b| {
                    if let InputContentBlock::ToolUse { id, name, input } = b {
                        Some(json!({
                            "id": id,
                            "type": "function",
                            "function": {
                                "name": name,
                                "arguments": serde_json::to_string(input).unwrap_or_default()
                            }
                        }))
                    } else {
                        None
                    }
                })
                .collect();

            let tool_results: Vec<(String, String)> = msg
                .content
                .iter()
                .filter_map(|b| {
                    if let InputContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } = b
                    {
                        Some((tool_use_id.clone(), content.clone()))
                    } else {
                        None
                    }
                })
                .collect();

            if !tool_results.is_empty() {
                for (tool_use_id, content) in tool_results {
                    messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: Some(content),
                        tool_calls: vec![],
                        tool_call_id: Some(tool_use_id),
                        name: None,
                    });
                }
            } else if !tool_calls.is_empty() {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: if text_parts.is_empty() {
                        None
                    } else {
                        Some(text_parts.join(""))
                    },
                    tool_calls,
                    tool_call_id: None,
                    name: None,
                });
            } else {
                messages.push(ChatMessage {
                    role: msg.role.clone(),
                    content: Some(text_parts.join("")),
                    tool_calls: vec![],
                    tool_call_id: None,
                    name: None,
                });
            }
        }

        let tools = req.tools.as_ref().map(|tools| {
            tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema,
                        }
                    })
                })
                .collect::<Vec<_>>()
        });

        let tool_choice = req.tool_choice.as_ref().map(|tc| {
            use code_buddy_transport::ToolChoice;
            match tc {
                ToolChoice::Auto => Value::String("auto".to_string()),
                ToolChoice::Any => Value::String("required".to_string()),
                ToolChoice::Tool { name } => json!({
                    "type": "function",
                    "function": { "name": name }
                }),
            }
        });

        ChatRequest {
            model: req.model.clone(),
            messages,
            max_tokens: Some(req.max_tokens),
            temperature: req.temperature,
            stream,
            tools,
            tool_choice,
        }
    }

    async fn send_with_retry(
        &self,
        req: &MessageRequest,
        stream: bool,
    ) -> Result<reqwest::Response, TransportError> {
        let body = self.build_chat_request(req, stream);
        let initial_backoff = Duration::from_millis(200);
        let max_backoff = Duration::from_secs(2);

        let mut attempt = 0u32;
        loop {
            attempt += 1;
            debug!(
                provider = %self.config.provider_name,
                model = %req.model,
                attempt = attempt,
                "sending request"
            );

            let result = self
                .request_builder()
                .json(&body)
                .send()
                .await
                .map_err(|e| self.map_reqwest_error(e));

            match result {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        return Ok(response);
                    }
                    let status_code = status.as_u16();
                    let is_retryable = status_code >= 500;
                    if is_retryable && attempt <= self.config.max_retries {
                        let backoff = backoff_duration(attempt, initial_backoff, max_backoff);
                        warn!(
                            provider = %self.config.provider_name,
                            status = status_code,
                            attempt = attempt,
                            ?backoff,
                            "retryable server error, backing off"
                        );
                        tokio::time::sleep(backoff).await;
                        continue;
                    }
                    let body_text = response.text().await.unwrap_or_default();
                    return Err(TransportError::ApiError {
                        provider: self.config.provider_name.clone(),
                        status: status_code,
                        message: body_text,
                    });
                }
                Err(e) => {
                    let is_retryable = matches!(
                        e,
                        TransportError::Timeout { .. } | TransportError::Network { .. }
                    );
                    if is_retryable && attempt <= self.config.max_retries {
                        let backoff = backoff_duration(attempt, initial_backoff, max_backoff);
                        warn!(
                            provider = %self.config.provider_name,
                            error = %e,
                            attempt = attempt,
                            ?backoff,
                            "retryable network error, backing off"
                        );
                        tokio::time::sleep(backoff).await;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }

    fn map_reqwest_error(&self, e: reqwest::Error) -> TransportError {
        if e.is_timeout() {
            TransportError::Timeout {
                provider: self.config.provider_name.clone(),
                timeout_secs: self.config.timeout.as_secs(),
            }
        } else if e.is_connect() {
            let hint = if self.config.is_local {
                "Is the local server running?".to_string()
            } else {
                "Check your network connection.".to_string()
            };
            TransportError::Connection {
                provider: self.config.provider_name.clone(),
                detail: format!("{e}. {hint}"),
            }
        } else {
            TransportError::Network {
                provider: self.config.provider_name.clone(),
                detail: e.to_string(),
            }
        }
    }
}

#[async_trait]
impl Provider for OpenAiCompatAdapter {
    fn name(&self) -> &str {
        &self.config.provider_name
    }

    async fn send(&self, request: &MessageRequest) -> Result<MessageResponse, TransportError> {
        let response = self.send_with_retry(request, false).await?;
        let payload: ChatCompletionResponse = response.json().await.map_err(|e| {
            TransportError::Parse {
                provider: self.config.provider_name.clone(),
                detail: e.to_string(),
            }
        })?;
        normalize_response(&self.config.provider_name, &request.model, payload)
    }

    async fn stream(
        &self,
        request: &MessageRequest,
    ) -> Result<Box<dyn StreamSource>, TransportError> {
        let response = self.send_with_retry(request, true).await?;
        let byte_stream = response.bytes_stream();
        Ok(Box::new(SseStreamSource::new(
            byte_stream,
            self.config.provider_name.clone(),
        )))
    }
}

// ── Response normalization ────────────────────────────────────────────────────

fn normalize_response(
    provider_name: &str,
    model: &str,
    payload: ChatCompletionResponse,
) -> Result<MessageResponse, TransportError> {
    let choice = payload.choices.into_iter().next().ok_or_else(|| {
        TransportError::Parse {
            provider: provider_name.to_string(),
            detail: "chat completion response has no choices".to_string(),
        }
    })?;

    let mut content: Vec<OutputContentBlock> = Vec::new();

    if let Some(text) = choice.message.content.filter(|t| !t.is_empty()) {
        content.push(OutputContentBlock::Text { text });
    }

    for tc in choice.message.tool_calls {
        let input = parse_tool_arguments(&tc.function.arguments);
        content.push(OutputContentBlock::ToolUse {
            id: tc.id,
            name: tc.function.name,
            input,
        });
    }

    let stop_reason = choice.finish_reason.map(|r| normalize_finish_reason(&r));
    let usage = payload.usage.unwrap_or_default();

    Ok(MessageResponse {
        id: payload.id,
        model: if payload.model.is_empty() {
            model.to_string()
        } else {
            payload.model
        },
        content,
        stop_reason,
        usage: Usage {
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
        },
    })
}

fn normalize_finish_reason(reason: &str) -> String {
    match reason {
        "stop" | "end_turn" => "end_turn".to_string(),
        "tool_calls" | "tool_use" => "tool_use".to_string(),
        "length" | "max_tokens" => "max_tokens".to_string(),
        other => other.to_string(),
    }
}

fn parse_tool_arguments(arguments: &str) -> Value {
    serde_json::from_str(arguments).unwrap_or_else(|_| json!({ "raw": arguments }))
}

pub(crate) fn completions_endpoint(base_url: &str) -> String {
    let suffix = "/chat/completions";
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with(suffix) {
        trimmed.to_string()
    } else {
        format!("{trimmed}{suffix}")
    }
}

fn backoff_duration(attempt: u32, initial: Duration, max: Duration) -> Duration {
    let millis = initial.as_millis() as u64 * 2u64.pow(attempt.saturating_sub(1));
    Duration::from_millis(millis).min(max)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use code_buddy_transport::{InputMessage, MessageRequest};

    fn make_adapter() -> OpenAiCompatAdapter {
        OpenAiCompatAdapter::new(AdapterConfig::lm_studio())
    }

    #[test]
    fn completions_endpoint_appends_path() {
        assert_eq!(
            completions_endpoint("http://localhost:1234/v1"),
            "http://localhost:1234/v1/chat/completions"
        );
    }

    #[test]
    fn completions_endpoint_strips_trailing_slash() {
        assert_eq!(
            completions_endpoint("http://localhost:1234/v1/"),
            "http://localhost:1234/v1/chat/completions"
        );
    }

    #[test]
    fn completions_endpoint_does_not_double_append() {
        assert_eq!(
            completions_endpoint("http://localhost:1234/v1/chat/completions"),
            "http://localhost:1234/v1/chat/completions"
        );
    }

    #[test]
    fn backoff_grows_exponentially() {
        let init = Duration::from_millis(200);
        let max = Duration::from_secs(2);
        assert_eq!(backoff_duration(1, init, max), Duration::from_millis(200));
        assert_eq!(backoff_duration(2, init, max), Duration::from_millis(400));
        assert_eq!(backoff_duration(3, init, max), Duration::from_millis(800));
        assert_eq!(backoff_duration(4, init, max), Duration::from_millis(1600));
        assert_eq!(backoff_duration(5, init, max), Duration::from_secs(2));
        assert_eq!(backoff_duration(6, init, max), Duration::from_secs(2));
    }

    #[test]
    fn normalize_stop_reasons() {
        assert_eq!(normalize_finish_reason("stop"), "end_turn");
        assert_eq!(normalize_finish_reason("end_turn"), "end_turn");
        assert_eq!(normalize_finish_reason("tool_calls"), "tool_use");
        assert_eq!(normalize_finish_reason("tool_use"), "tool_use");
        assert_eq!(normalize_finish_reason("length"), "max_tokens");
        assert_eq!(normalize_finish_reason("unknown"), "unknown");
    }

    #[test]
    fn parse_tool_arguments_valid_json() {
        let val = parse_tool_arguments(r#"{"city":"Paris"}"#);
        assert_eq!(val["city"], "Paris");
    }

    #[test]
    fn parse_tool_arguments_invalid_json_falls_back() {
        let val = parse_tool_arguments("not-json");
        assert_eq!(val["raw"], "not-json");
    }

    #[test]
    fn build_chat_request_includes_system() {
        let adapter = make_adapter();
        let req = MessageRequest {
            model: "test".to_string(),
            max_tokens: 100,
            messages: vec![InputMessage::user_text("hello")],
            system: Some("You are a robot.".to_string()),
            tools: None,
            tool_choice: None,
            stream: false,
            temperature: None,
        };
        let chat = adapter.build_chat_request(&req, false);
        assert_eq!(chat.messages[0].role, "system");
        assert_eq!(
            chat.messages[0].content.as_deref(),
            Some("You are a robot.")
        );
        assert_eq!(chat.messages[1].role, "user");
    }

    #[test]
    fn build_chat_request_no_system() {
        let adapter = make_adapter();
        let req = MessageRequest::simple("model", "Hi");
        let chat = adapter.build_chat_request(&req, false);
        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].role, "user");
    }

    #[test]
    fn build_chat_request_stream_flag() {
        let adapter = make_adapter();
        let req = MessageRequest::simple("model", "Hi");
        assert!(!adapter.build_chat_request(&req, false).stream);
        assert!(adapter.build_chat_request(&req, true).stream);
    }

    #[test]
    fn normalize_response_text() {
        let payload = ChatCompletionResponse {
            id: "chatcmpl-1".to_string(),
            model: "gpt-4o".to_string(),
            choices: vec![ChatChoice {
                message: ChatChoiceMessage {
                    role: "assistant".to_string(),
                    content: Some("Hello!".to_string()),
                    tool_calls: vec![],
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(ChatUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
            }),
        };
        let resp = normalize_response("OpenAI", "gpt-4o", payload).unwrap();
        assert_eq!(resp.id, "chatcmpl-1");
        assert_eq!(resp.text_content(), "Hello!");
        assert_eq!(resp.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(resp.usage.input_tokens, 10);
        assert_eq!(resp.usage.output_tokens, 5);
    }

    #[test]
    fn normalize_response_tool_call() {
        let payload = ChatCompletionResponse {
            id: "chatcmpl-2".to_string(),
            model: "gpt-4o".to_string(),
            choices: vec![ChatChoice {
                message: ChatChoiceMessage {
                    role: "assistant".to_string(),
                    content: None,
                    tool_calls: vec![ToolCallObject {
                        id: "call_abc".to_string(),
                        function: ToolCallFunction {
                            name: "read_file".to_string(),
                            arguments: r#"{"path":"/etc/hosts"}"#.to_string(),
                        },
                    }],
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            usage: None,
        };
        let resp = normalize_response("OpenAI", "gpt-4o", payload).unwrap();
        assert!(resp.stopped_for_tool());
        let calls = resp.tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, "read_file");
        assert_eq!(calls[0].2["path"], "/etc/hosts");
    }

    #[test]
    fn normalize_response_no_choices_is_error() {
        let payload = ChatCompletionResponse {
            id: "chatcmpl-3".to_string(),
            model: "".to_string(),
            choices: vec![],
            usage: None,
        };
        assert!(normalize_response("Test", "model", payload).is_err());
    }

    #[test]
    fn normalize_response_uses_request_model_when_empty() {
        let payload = ChatCompletionResponse {
            id: "id".to_string(),
            model: String::new(),
            choices: vec![ChatChoice {
                message: ChatChoiceMessage {
                    role: "assistant".to_string(),
                    content: Some("hi".to_string()),
                    tool_calls: vec![],
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };
        let resp = normalize_response("LM Studio", "my-model", payload).unwrap();
        assert_eq!(resp.model, "my-model");
    }

    #[test]
    fn adapter_config_lm_studio_no_key() {
        let config = AdapterConfig::lm_studio();
        assert!(config.api_key.is_empty());
        assert!(config.is_local);
    }

    #[test]
    fn adapter_config_openrouter() {
        let config = AdapterConfig::openrouter("sk-test");
        assert_eq!(config.api_key, "sk-test");
        assert!(!config.is_local);
        assert!(config.base_url.contains("openrouter.ai"));
    }

    #[test]
    fn adapter_config_nvidia() {
        let config = AdapterConfig::nvidia("nvapi-key");
        assert!(config.base_url.contains("nvidia.com"));
    }

    #[test]
    fn adapter_config_with_timeout_override() {
        let config = AdapterConfig::lm_studio().with_timeout(Duration::from_secs(30));
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn adapter_config_with_base_url_override() {
        let config = AdapterConfig::lm_studio()
            .with_base_url_override("http://localhost:5000/v1".to_string());
        assert_eq!(config.base_url, "http://localhost:5000/v1");
    }

    #[test]
    fn tool_call_accumulator_builds_from_deltas() {
        let mut acc = ToolCallAccumulator::default();
        acc.ingest(&ToolCallChunk {
            index: 0,
            id: Some("call_1".to_string()),
            function: Some(ToolCallChunkFunction {
                name: Some("bash".to_string()),
                arguments: Some(r#"{"cmd":"#.to_string()),
            }),
        });
        acc.ingest(&ToolCallChunk {
            index: 0,
            id: None,
            function: Some(ToolCallChunkFunction {
                name: None,
                arguments: Some(r#""ls"}"#.to_string()),
            }),
        });
        let events = acc.drain_complete();
        assert_eq!(events.len(), 1);
        if let StreamEvent::ToolUseDelta {
            id,
            name,
            input_json,
        } = &events[0]
        {
            assert_eq!(id, "call_1");
            assert_eq!(name, "bash");
            assert_eq!(input_json, r#"{"cmd":"ls"}"#);
        } else {
            panic!("expected ToolUseDelta");
        }
    }

    #[test]
    fn parse_chunk_events_text_delta() {
        let mut acc = ToolCallAccumulator::default();
        let data = r#"{"id":"c1","model":"m","choices":[{"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let (events, done) = parse_chunk_events(data, "test", &mut acc);
        assert!(!done);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StreamEvent::TextDelta(t) if t == "Hello"));
    }

    #[test]
    fn parse_chunk_events_done_sentinel() {
        let mut acc = ToolCallAccumulator::default();
        let (events, done) = parse_chunk_events("[DONE]", "test", &mut acc);
        assert!(done);
        assert!(events.iter().any(|e| matches!(e, StreamEvent::MessageStop)));
    }

    #[test]
    fn parse_chunk_events_malformed_skipped() {
        let mut acc = ToolCallAccumulator::default();
        let (events, done) = parse_chunk_events("not valid json{{", "test", &mut acc);
        assert!(!done);
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn sse_stream_source_yields_text_then_stop() {
        use bytes::Bytes;
        use futures_util::stream;

        let chunks: Vec<Result<Bytes, reqwest::Error>> = vec![
            Ok(Bytes::from(
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}\n\n",
            )),
            Ok(Bytes::from("data: [DONE]\n\n")),
        ];
        let byte_stream = stream::iter(chunks);
        let mut src = SseStreamSource::new(byte_stream, "test".to_string());

        let e1 = src.next_event().await.unwrap();
        assert!(matches!(e1, Some(StreamEvent::TextDelta(ref t)) if t == "Hi"));

        let e2 = src.next_event().await.unwrap();
        assert!(matches!(e2, Some(StreamEvent::MessageStop)));

        let e3 = src.next_event().await.unwrap();
        assert!(e3.is_none());
    }

    #[tokio::test]
    async fn sse_stream_source_partial_chunks() {
        use bytes::Bytes;
        use futures_util::stream;

        // Split the SSE frame across two network chunks
        let chunks: Vec<Result<Bytes, reqwest::Error>> = vec![
            Ok(Bytes::from(
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hel",
            )),
            Ok(Bytes::from(
                "lo\"},\"finish_reason\":null}]}\n\ndata: [DONE]\n\n",
            )),
        ];
        let byte_stream = stream::iter(chunks);
        let mut src = SseStreamSource::new(byte_stream, "test".to_string());

        let e1 = src.next_event().await.unwrap();
        assert!(matches!(e1, Some(StreamEvent::TextDelta(ref t)) if t == "Hello"));

        let stop = src.next_event().await.unwrap();
        assert!(matches!(stop, Some(StreamEvent::MessageStop)));
    }
}
