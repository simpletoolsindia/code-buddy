//! `ConversationRuntime` — the tool-calling loop and session manager.
//!
//! # Loop design
//!
//! 1. Append user message to history.
//! 2. Build a `MessageRequest` with tool definitions.
//! 3. **First turn** (iteration 0): if streaming is configured and a text callback
//!    is provided, stream and accumulate both text deltas and tool-use deltas
//!    simultaneously. This lets the REPL display tokens live while still detecting
//!    tool calls.
//! 4. **Subsequent turns** (tool results have been injected): always use `send()`.
//! 5. If tool calls are present: execute each via `ToolRegistry`, append results,
//!    increment the iteration counter, and loop.
//! 6. If no tool calls: store the assistant message and return `TurnSummary`.
//! 7. If `max_iterations` is exceeded: pop the user message and return
//!    `RuntimeError::MaxIterationsExceeded`.
//!
//! # Regression notes
//! - Bug §3 (Tokio panic on static runtime init): this runtime is `async` throughout.
//!   There is no `OnceLock<tokio::runtime::Runtime>` with `.expect()`.
//! - Bug §4 (silent API key failure): `ProviderRegistry::from_config()` returns
//!   `MissingCredentials` before this runtime is entered. No `unwrap_or_default()`
//!   on API keys anywhere in this path.

use std::time::Duration;

use code_buddy_errors::{ProviderError, RuntimeError, ToolError, TransportError};
use code_buddy_tools::{
    ToolRegistry,
    parser::{
        ParsedToolCall, StreamingToolCallAccumulator, extract_tool_calls,
    },
};
use code_buddy_transport::{
    InputMessage, MessageRequest, Provider, StreamEvent, ToolChoice,
};
use tracing::{debug, instrument, warn};

// ── Config ────────────────────────────────────────────────────────────────────

/// Runtime configuration for a conversation session.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Model identifier passed to the provider.
    pub model: String,
    /// Maximum tokens to request from the provider.
    pub max_tokens: u32,
    /// Sampling temperature.
    pub temperature: Option<f32>,
    /// System prompt prepended to every request.
    pub system_prompt: Option<String>,
    /// Whether to stream the first response turn to the terminal.
    pub streaming: bool,
    /// Maximum tool-call iterations before aborting. Default: 10.
    pub max_iterations: usize,
    /// Per-tool execution timeout. Default: 30s.
    pub tool_timeout: Duration,
    /// Print debug info (token counts, etc.).
    pub debug: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            model: "local-model".to_string(),
            max_tokens: 4096,
            temperature: None,
            system_prompt: None,
            streaming: false,
            max_iterations: 10,
            tool_timeout: Duration::from_secs(30),
            debug: false,
        }
    }
}

// ── TurnSummary ───────────────────────────────────────────────────────────────

/// Summary of a completed conversation turn.
#[derive(Debug, Clone)]
pub struct TurnSummary {
    /// The final text response from the assistant (after all tool calls).
    pub response_text: String,
    /// Number of tool calls executed in this turn.
    pub tool_calls_made: u32,
    /// Number of provider round-trips (1 + tool iterations).
    pub iterations: u32,
    /// Cumulative input tokens across all iterations.
    pub input_tokens: u32,
    /// Cumulative output tokens across all iterations.
    pub output_tokens: u32,
}

// ── TextSink ──────────────────────────────────────────────────────────────────

/// An optional text callback passed to `run_turn`.
///
/// Using a concrete wrapper type (rather than `Option<&mut dyn FnMut>`) avoids
/// Rust's lifetime-invariance issue with mutable trait object references inside
/// async loops. Callers box their closure once; the runtime reborrows it as
/// `Option<&mut dyn FnMut(&str)>` within each loop iteration.
pub struct TextSink(Option<Box<dyn FnMut(&str) + Send>>);

impl TextSink {
    /// No-op sink.
    pub fn none() -> Self {
        Self(None)
    }

    /// Wrap a boxed closure.
    pub fn new(f: Box<dyn FnMut(&str) + Send>) -> Self {
        Self(Some(f))
    }

    fn is_some(&self) -> bool {
        self.0.is_some()
    }

    fn call(&mut self, text: &str) {
        if let Some(f) = &mut self.0 {
            f(text);
        }
    }
}

// ── ConversationRuntime ───────────────────────────────────────────────────────

/// Manages a multi-turn conversation with tool-calling support.
pub struct ConversationRuntime {
    provider: Box<dyn Provider>,
    tools: ToolRegistry,
    history: Vec<InputMessage>,
    config: RuntimeConfig,
}

impl ConversationRuntime {
    /// Create a new runtime.
    #[must_use]
    pub fn new(
        provider: Box<dyn Provider>,
        tools: ToolRegistry,
        config: RuntimeConfig,
    ) -> Self {
        let timeout = config.tool_timeout;
        let mut tools = tools;
        tools = tools.with_timeout(timeout);
        Self {
            provider,
            tools,
            history: Vec::new(),
            config,
        }
    }

    /// Immutable view of the conversation history.
    #[must_use]
    pub fn history(&self) -> &[InputMessage] {
        &self.history
    }

    /// Clear all conversation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Run one conversation turn with an optional text callback.
    ///
    /// Pass `TextSink::none()` when no streaming output is needed.
    /// Pass `TextSink::new(Box::new(|text| ...))` to receive text deltas.
    ///
    /// On any error, the user message is popped from history so state remains
    /// consistent with the turn never having been attempted.
    ///
    /// # Errors
    /// Returns [`RuntimeError`] on provider failure, tool failure, or max iterations.
    #[instrument(skip(self, sink), fields(provider = %self.provider.name()))]
    pub async fn run_turn(
        &mut self,
        user_input: &str,
        sink: TextSink,
    ) -> Result<TurnSummary, RuntimeError> {
        self.history.push(InputMessage::user_text(user_input));
        let result = self.run_loop(sink).await;
        if result.is_err() {
            self.history.pop();
        }
        result
    }

    async fn run_loop(
        &mut self,
        mut sink: TextSink,
    ) -> Result<TurnSummary, RuntimeError> {
        let has_tools = !self.tools.is_empty();
        let tool_defs = if has_tools {
            Some(self.tools.definitions())
        } else {
            None
        };
        let tool_choice = if has_tools {
            Some(ToolChoice::Auto)
        } else {
            None
        };

        let mut total_input_tokens: u32 = 0;
        let mut total_output_tokens: u32 = 0;
        let mut tool_calls_made: u32 = 0;
        let mut iterations: u32 = 0;

        loop {
            if iterations > 0 && iterations >= self.config.max_iterations as u32 {
                return Err(RuntimeError::MaxIterationsExceeded {
                    max: self.config.max_iterations,
                });
            }

            let request = MessageRequest {
                model: self.config.model.clone(),
                max_tokens: self.config.max_tokens,
                messages: self.history.clone(),
                system: self.config.system_prompt.clone(),
                tools: tool_defs.clone(),
                tool_choice: tool_choice.clone(),
                stream: false,
                temperature: self.config.temperature,
            };

            let use_streaming =
                iterations == 0 && self.config.streaming && sink.is_some();

            let (response_text, tool_calls, input_toks, output_toks) = if use_streaming {
                self.do_streaming_turn(&request, &mut sink).await?
            } else {
                self.do_send_turn(&request, &mut sink, iterations > 0).await?
            };

            total_input_tokens = total_input_tokens.saturating_add(input_toks);
            total_output_tokens = total_output_tokens.saturating_add(output_toks);
            iterations += 1;

            if tool_calls.is_empty() {
                if !response_text.is_empty() {
                    self.history
                        .push(InputMessage::assistant_text(&response_text));
                }
                if self.config.debug {
                    eprintln!(
                        "[tokens: in={total_input_tokens} out={total_output_tokens}]"
                    );
                }
                return Ok(TurnSummary {
                    response_text,
                    tool_calls_made,
                    iterations,
                    input_tokens: total_input_tokens,
                    output_tokens: total_output_tokens,
                });
            }

            debug!("executing {} tool call(s)", tool_calls.len());

            let tool_use_blocks: Vec<_> = tool_calls
                .iter()
                .map(|c| code_buddy_transport::InputContentBlock::ToolUse {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    input: c.input.clone(),
                })
                .collect();
            self.history.push(InputMessage {
                role: "assistant".to_string(),
                content: tool_use_blocks,
            });

            for call in &tool_calls {
                let result = self.execute_tool(call).await;
                let (content, is_error) = match result {
                    Ok(s) => (s, false),
                    Err(e) => {
                        warn!(tool = %call.name, "tool error: {e}");
                        (format!("Error: {e}"), true)
                    }
                };
                self.history
                    .push(InputMessage::tool_result(&call.id, content, is_error));
                tool_calls_made += 1;
            }
        }
    }

    async fn do_send_turn(
        &self,
        request: &MessageRequest,
        sink: &mut TextSink,
        silent: bool,
    ) -> Result<(String, Vec<ParsedToolCall>, u32, u32), RuntimeError> {
        let response = self
            .provider
            .send(request)
            .await
            .map_err(transport_to_runtime)?;

        let text = response.text_content();
        let input_toks = response.usage.input_tokens;
        let output_toks = response.usage.output_tokens;

        if !silent {
            sink.call(&text);
        }

        let calls = extract_tool_calls(&response).map_err(RuntimeError::Tool)?;
        Ok((text, calls, input_toks, output_toks))
    }

    async fn do_streaming_turn(
        &self,
        request: &MessageRequest,
        sink: &mut TextSink,
    ) -> Result<(String, Vec<ParsedToolCall>, u32, u32), RuntimeError> {
        let mut stream_request = request.clone();
        stream_request.stream = true;

        let mut source = self
            .provider
            .stream(&stream_request)
            .await
            .map_err(transport_to_runtime)?;

        let mut acc = StreamingToolCallAccumulator::new();
        let mut response_text = String::new();
        let mut input_toks: u32 = 0;
        let mut output_toks: u32 = 0;

        loop {
            match source.next_event().await.map_err(transport_to_runtime)? {
                None => break,
                Some(StreamEvent::MessageStop) => break,
                Some(StreamEvent::Usage(u)) => {
                    input_toks = u.input_tokens;
                    output_toks = u.output_tokens;
                }
                Some(event) => {
                    if let Some(delta) = acc.process_event(&event) {
                        response_text.push_str(&delta);
                        sink.call(&delta);
                    }
                }
            }
        }

        let calls = acc.finish().map_err(RuntimeError::Tool)?;
        Ok((response_text, calls, input_toks, output_toks))
    }

    async fn execute_tool(&self, call: &ParsedToolCall) -> Result<String, ToolError> {
        let span =
            tracing::info_span!("tool_exec", tool = %call.name, id = %call.id);
        let _enter = span.enter();
        self.tools.execute(&call.name, call.input.clone()).await
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn transport_to_runtime(err: TransportError) -> RuntimeError {
    RuntimeError::Provider(ProviderError::Transport {
        provider: "provider".to_string(),
        source: err,
    })
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use code_buddy_errors::TransportError;
    use code_buddy_tools::{Tool, ToolRegistry};
    use code_buddy_transport::{
        MessageRequest, MessageResponse, OutputContentBlock, StreamSource, Usage,
    };
    use serde_json::{Value, json};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    // ── Mock provider ─────────────────────────────────────────────────────────

    struct MockProvider {
        responses: Mutex<VecDeque<MessageResponse>>,
    }

    impl MockProvider {
        fn new(responses: Vec<MessageResponse>) -> Self {
            Self {
                responses: Mutex::new(VecDeque::from(responses)),
            }
        }

        fn text_response(text: &str) -> MessageResponse {
            MessageResponse {
                id: "msg".to_string(),
                model: "test".to_string(),
                content: vec![OutputContentBlock::Text {
                    text: text.to_string(),
                }],
                stop_reason: Some("end_turn".to_string()),
                usage: Usage {
                    input_tokens: 10,
                    output_tokens: 5,
                },
            }
        }

        fn tool_call_response(id: &str, name: &str, input: Value) -> MessageResponse {
            MessageResponse {
                id: "msg".to_string(),
                model: "test".to_string(),
                content: vec![OutputContentBlock::ToolUse {
                    id: id.to_string(),
                    name: name.to_string(),
                    input,
                }],
                stop_reason: Some("tool_use".to_string()),
                usage: Usage {
                    input_tokens: 20,
                    output_tokens: 10,
                },
            }
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        async fn send(
            &self,
            _req: &MessageRequest,
        ) -> Result<MessageResponse, TransportError> {
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| TransportError::ApiError {
                    provider: "mock".to_string(),
                    status: 500,
                    message: "no more responses".to_string(),
                })
        }

        async fn stream(
            &self,
            req: &MessageRequest,
        ) -> Result<Box<dyn StreamSource>, TransportError> {
            let resp = self.send(req).await?;
            Ok(Box::new(MockStream::from_response(resp)))
        }
    }

    struct MockStream {
        events: VecDeque<StreamEvent>,
    }

    impl MockStream {
        fn from_response(r: MessageResponse) -> Self {
            let mut events = VecDeque::new();
            for block in r.content {
                match block {
                    OutputContentBlock::Text { text } => {
                        events.push_back(StreamEvent::TextDelta(text));
                    }
                    OutputContentBlock::ToolUse { id, name, input } => {
                        events.push_back(StreamEvent::ToolUseDelta {
                            id,
                            name,
                            input_json: serde_json::to_string(&input).unwrap(),
                        });
                    }
                }
            }
            events.push_back(StreamEvent::MessageStop);
            Self { events }
        }
    }

    #[async_trait]
    impl StreamSource for MockStream {
        async fn next_event(&mut self) -> Result<Option<StreamEvent>, TransportError> {
            Ok(self.events.pop_front())
        }
    }

    // ── Echo tool ─────────────────────────────────────────────────────────────

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "echo"
        }
        fn input_schema(&self) -> Value {
            json!({})
        }
        async fn execute(&self, input: Value) -> Result<String, ToolError> {
            Ok(input["msg"].as_str().unwrap_or("(no msg)").to_string())
        }
    }

    fn make_runtime(responses: Vec<MessageResponse>) -> ConversationRuntime {
        let mut tools = ToolRegistry::new();
        tools.register(EchoTool);
        ConversationRuntime::new(
            Box::new(MockProvider::new(responses)),
            tools,
            RuntimeConfig::default(),
        )
    }

    fn no_sink() -> TextSink {
        TextSink::none()
    }

    // ── Basic turn ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn simple_text_response() {
        let mut rt = make_runtime(vec![MockProvider::text_response("hello")]);
        let summary = rt.run_turn("hi", no_sink()).await.unwrap();
        assert_eq!(summary.response_text, "hello");
        assert_eq!(summary.tool_calls_made, 0);
        assert_eq!(summary.iterations, 1);
    }

    #[tokio::test]
    async fn history_grows_after_successful_turn() {
        let mut rt = make_runtime(vec![MockProvider::text_response("world")]);
        rt.run_turn("hello", no_sink()).await.unwrap();
        assert_eq!(rt.history().len(), 2);
        assert_eq!(rt.history()[0].role, "user");
        assert_eq!(rt.history()[1].role, "assistant");
    }

    #[tokio::test]
    async fn history_rolled_back_on_provider_error() {
        let mut rt = make_runtime(vec![]);
        let err = rt.run_turn("hello", no_sink()).await;
        assert!(err.is_err());
        assert_eq!(rt.history().len(), 0);
    }

    // ── Tool-calling loop ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn single_tool_call_then_text() {
        let responses = vec![
            MockProvider::tool_call_response("c1", "echo", json!({ "msg": "test" })),
            MockProvider::text_response("done"),
        ];
        let mut rt = make_runtime(responses);
        let summary = rt.run_turn("use echo", no_sink()).await.unwrap();
        assert_eq!(summary.tool_calls_made, 1);
        assert_eq!(summary.response_text, "done");
        assert_eq!(summary.iterations, 2);
    }

    /// Multi-step: model calls echo twice across two separate iterations.
    #[tokio::test]
    async fn multi_step_tool_calls() {
        let responses = vec![
            MockProvider::tool_call_response("c1", "echo", json!({ "msg": "step1" })),
            MockProvider::tool_call_response("c2", "echo", json!({ "msg": "step2" })),
            MockProvider::text_response("all done"),
        ];
        let mut rt = make_runtime(responses);
        let summary = rt.run_turn("do two steps", no_sink()).await.unwrap();
        assert_eq!(summary.tool_calls_made, 2);
        assert_eq!(summary.response_text, "all done");
        assert_eq!(summary.iterations, 3);
    }

    // ── Bounded iteration (max iterations guard) ──────────────────────────────

    #[tokio::test]
    async fn max_iterations_exceeded_returns_error() {
        let responses: Vec<MessageResponse> = (0..20)
            .map(|i| {
                MockProvider::tool_call_response(
                    &format!("c{i}"),
                    "echo",
                    json!({ "msg": "loop" }),
                )
            })
            .collect();

        let mut rt = make_runtime(responses);
        let err = rt.run_turn("loop", no_sink()).await.unwrap_err();
        assert!(
            matches!(err, RuntimeError::MaxIterationsExceeded { .. }),
            "expected MaxIterationsExceeded, got {err:?}"
        );
    }

    #[tokio::test]
    async fn custom_max_iterations_honoured() {
        let responses: Vec<MessageResponse> = (0..10)
            .map(|i| {
                MockProvider::tool_call_response(
                    &format!("c{i}"),
                    "echo",
                    json!({ "msg": "loop" }),
                )
            })
            .collect();

        let mut tools = ToolRegistry::new();
        tools.register(EchoTool);
        let config = RuntimeConfig {
            max_iterations: 3,
            ..Default::default()
        };
        let mut rt = ConversationRuntime::new(
            Box::new(MockProvider::new(responses)),
            tools,
            config,
        );
        let err = rt.run_turn("loop", no_sink()).await.unwrap_err();
        assert!(matches!(err, RuntimeError::MaxIterationsExceeded { max: 3 }));
    }

    // ── TextSink callback ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn text_sink_receives_response() {
        let mut rt = make_runtime(vec![MockProvider::text_response("streaming text")]);
        let collected = Arc::new(Mutex::new(String::new()));
        let c = collected.clone();
        let sink = TextSink::new(Box::new(move |text: &str| {
            c.lock().unwrap().push_str(text);
        }));
        rt.run_turn("hi", sink).await.unwrap();
        assert_eq!(*collected.lock().unwrap(), "streaming text");
    }

    // ── Unknown tool error injected as tool result ────────────────────────────

    #[tokio::test]
    async fn unknown_tool_error_injected_as_tool_result() {
        let responses = vec![
            MockProvider::tool_call_response("c1", "nonexistent_tool", json!({})),
            MockProvider::text_response("ok, I saw the error"),
        ];
        let mut rt = make_runtime(responses);
        let summary = rt.run_turn("call bad tool", no_sink()).await.unwrap();
        assert_eq!(summary.response_text, "ok, I saw the error");
        let has_error_result = rt.history().iter().any(|msg| {
            msg.content.iter().any(|b| {
                matches!(
                    b,
                    code_buddy_transport::InputContentBlock::ToolResult {
                        is_error: true,
                        ..
                    }
                )
            })
        });
        assert!(has_error_result);
    }

    // ── clear_history ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn clear_history_empties_session() {
        let mut rt = make_runtime(vec![
            MockProvider::text_response("first"),
            MockProvider::text_response("second"),
        ]);
        rt.run_turn("one", no_sink()).await.unwrap();
        assert_eq!(rt.history().len(), 2);
        rt.clear_history();
        assert_eq!(rt.history().len(), 0);
    }

    // ── Regression: Bug §3 — no static Tokio runtime ─────────────────────────

    #[tokio::test]
    async fn runtime_runs_inside_existing_tokio_context() {
        let mut rt = make_runtime(vec![MockProvider::text_response("ok")]);
        let result = rt.run_turn("test", no_sink()).await;
        assert!(result.is_ok());
    }

    // ── Regression: Bug §4 — silent API key failure ───────────────────────────

    #[test]
    fn transport_to_runtime_wraps_missing_credentials() {
        let err = TransportError::MissingCredentials {
            provider: "OpenRouter".to_string(),
            env_var: "OPENROUTER_API_KEY".to_string(),
        };
        let rt_err = transport_to_runtime(err);
        assert!(matches!(rt_err, RuntimeError::Provider(_)));
        let msg = rt_err.to_string();
        assert!(
            msg.contains("Transport")
                || msg.contains("OpenRouter")
                || msg.contains("provider"),
            "error message should reference the provider: {msg}"
        );
    }

    // ── Token accumulation ────────────────────────────────────────────────────

    #[tokio::test]
    async fn token_counts_accumulated_across_iterations() {
        let responses = vec![
            MockProvider::tool_call_response("c1", "echo", json!({ "msg": "step1" })),
            MockProvider::text_response("done"),
        ];
        let mut rt = make_runtime(responses);
        let summary = rt.run_turn("count tokens", no_sink()).await.unwrap();
        // First response: 20 in + 10 out, second: 10 in + 5 out → totals 30/15.
        assert_eq!(summary.input_tokens, 30);
        assert_eq!(summary.output_tokens, 15);
    }
}
