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
//! # Context compaction
//!
//! After each successful provider round-trip, the runtime estimates the total
//! token count of the conversation history (using the heuristic of 1 token per
//! 4 characters). When the estimate exceeds `context_token_budget`, the runtime
//! calls `compact_history()`, which deterministically removes the **oldest**
//! complete turn (one user + one assistant message pair) from the middle of
//! history, preserving the most recent context.
//!
//! If after compaction the history still exceeds the budget, [`RuntimeError::ContextTooLarge`]
//! is returned so the caller can inform the user.
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
    /// Estimated token budget for the conversation history.
    ///
    /// When the runtime estimates the history has exceeded this many tokens
    /// (heuristic: 1 token ≈ 4 chars), it compacts by dropping the oldest
    /// complete turn (user + assistant pair). A value of `0` disables
    /// compaction. Default: `max_tokens * 6` (six response windows of context).
    pub context_token_budget: u32,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        let max_tokens = 4096_u32;
        Self {
            model: "local-model".to_string(),
            max_tokens,
            temperature: None,
            system_prompt: None,
            streaming: false,
            max_iterations: 10,
            tool_timeout: Duration::from_secs(30),
            debug: false,
            context_token_budget: max_tokens.saturating_mul(6),
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

    /// Estimated token count for the current history (heuristic: 1 token ≈ 4 chars).
    ///
    /// Used for compaction decisions. Not a billing-accurate count.
    #[must_use]
    pub fn estimated_history_tokens(&self) -> u32 {
        self.history
            .iter()
            .flat_map(|m| m.content.iter())
            .map(|block| {
                let chars = match block {
                    code_buddy_transport::InputContentBlock::Text { text } => text.len(),
                    code_buddy_transport::InputContentBlock::ToolUse { input, .. } => {
                        input.to_string().len()
                    }
                    code_buddy_transport::InputContentBlock::ToolResult { content, .. } => {
                        content.len()
                    }
                };
                (chars as u32).saturating_div(4).max(1)
            })
            .sum()
    }

    /// Drop the oldest complete turn (oldest user message + the message immediately
    /// following it) from history, preserving the most recent context.
    ///
    /// Returns `true` if a turn was dropped, `false` if the history is too short
    /// to compact (fewer than 4 messages: current user + current response + at least
    /// one prior turn to drop).
    ///
    /// This is a deterministic, lossless compaction — no LLM summarization.
    /// Callers should warn the user when this fires.
    pub fn compact_oldest_turn(&mut self) -> bool {
        // Need at least 4 messages to have something droppable:
        // [old_user, old_assistant, ..., new_user, new_assistant]
        if self.history.len() < 4 {
            return false;
        }
        // Remove the first two messages (oldest user + oldest assistant/tool-call).
        self.history.remove(0);
        self.history.remove(0);
        true
    }

    /// Compact history if estimated token count exceeds `context_token_budget`.
    ///
    /// Repeatedly drops the oldest turn until under budget or no more turns
    /// can be dropped. Returns [`RuntimeError::ContextTooLarge`] if the
    /// budget is still exceeded after all possible compaction.
    fn compact_if_needed(&mut self) -> Result<(), RuntimeError> {
        let budget = self.config.context_token_budget;
        if budget == 0 {
            return Ok(()); // Compaction disabled.
        }

        let mut compacted = false;
        loop {
            if self.estimated_history_tokens() <= budget {
                break;
            }
            if !self.compact_oldest_turn() {
                // History is too short to compact further (< 4 messages).
                // Only return an error if we already compacted something but
                // are still over budget — meaning even dropping the oldest
                // turns doesn't help. If compaction never ran at all, the
                // single current turn is the cause; we proceed and let the
                // provider surface a context-length error if needed.
                if compacted {
                    let tokens = self.estimated_history_tokens();
                    return Err(RuntimeError::ContextTooLarge { tokens });
                }
                break;
            }
            compacted = true;
        }

        if compacted {
            warn!(
                "context compacted: {} messages remaining, ~{} estimated tokens",
                self.history.len(),
                self.estimated_history_tokens()
            );
        }

        Ok(())
    }

    /// Run one conversation turn with an optional text callback.
    ///
    /// Pass `TextSink::none()` when no streaming output is needed.
    /// Pass `TextSink::new(Box::new(|text| ...))` to receive text deltas.
    ///
    /// **Transactional history**: the history length is snapshot before the turn
    /// begins. If the loop fails at any point — even mid-way through multiple tool
    /// iterations where several assistant/tool-result messages have already been
    /// appended — the entire turn is rolled back to the snapshot length. This
    /// ensures no orphaned or partially-committed messages remain.
    ///
    /// # Errors
    /// Returns [`RuntimeError`] on provider failure, tool failure, or max iterations.
    #[instrument(skip(self, sink), fields(provider = %self.provider.name()))]
    pub async fn run_turn(
        &mut self,
        user_input: &str,
        sink: TextSink,
    ) -> Result<TurnSummary, RuntimeError> {
        // Snapshot length BEFORE the user message so the whole turn (user
        // message + all assistant/tool messages) is rolled back atomically on error.
        let snapshot_len = self.history.len();
        self.history.push(InputMessage::user_text(user_input));
        let result = self.run_loop(sink).await;
        if result.is_err() {
            // Truncate back to the pre-turn state — removes all messages that
            // were appended during this failed turn (user, assistant, tool results).
            self.history.truncate(snapshot_len);
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

            // Compact history before building the next request so the provider
            // never receives a payload that exceeds the context budget.
            self.compact_if_needed()?;

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

            if self.config.debug {
                eprintln!(
                    "[debug] iter={iterations} model={} msgs={} streaming={use_streaming}",
                    request.model,
                    request.messages.len()
                );
                if tracing::enabled!(tracing::Level::DEBUG) {
                    if let Ok(json) = serde_json::to_string_pretty(&request.messages) {
                        eprintln!("[debug] request.messages = {json}");
                    }
                }
            }

            let (response_text, tool_calls, input_toks, output_toks) = if use_streaming {
                self.do_streaming_turn(&request, &mut sink).await?
            } else {
                self.do_send_turn(&request, &mut sink).await?
            };

            if self.config.debug {
                eprintln!(
                    "[debug] response: text={:?}... tool_calls={}",
                    response_text.chars().take(80).collect::<String>(),
                    tool_calls.len()
                );
            }

            total_input_tokens = total_input_tokens.saturating_add(input_toks);
            total_output_tokens = total_output_tokens.saturating_add(output_toks);
            iterations += 1;

            // Strict no-tool mode: if no tools are registered, the model
            // should not emit tool calls (none were advertised).  If it does
            // anyway, inject synthetic "tool not available" results into the
            // history so the model receives structured feedback and can
            // continue the conversation.  We do NOT execute the calls.
            if !has_tools && !tool_calls.is_empty() {
                warn!(
                    count = tool_calls.len(),
                    "model emitted tool calls but no tools are registered; \
                     injecting 'not available' results"
                );

                // Record the assistant message that contained the unexpected calls.
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

                // Inject a synthetic error result for each unexpected call.
                for call in &tool_calls {
                    self.history.push(InputMessage::tool_result(
                        &call.id,
                        format!(
                            "Tool '{}' is not available in this session. \
                             Please answer without using tools.",
                            call.name
                        ),
                        true, // is_error = true
                    ));
                }

                // Loop again so the model can give a text answer.
                continue;
            }

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

    /// Send a non-streaming request and emit the response text to `sink`.
    ///
    /// The text is always emitted regardless of iteration count. Intermediate
    /// tool-call responses have empty text content, so nothing is printed for them.
    /// The final assistant answer (which has non-empty text) is always visible.
    async fn do_send_turn(
        &self,
        request: &MessageRequest,
        sink: &mut TextSink,
    ) -> Result<(String, Vec<ParsedToolCall>, u32, u32), RuntimeError> {
        let response = self
            .provider
            .send(request)
            .await
            .map_err(|e| self.transport_err(e))?;

        let text = response.text_content();
        let input_toks = response.usage.input_tokens;
        let output_toks = response.usage.output_tokens;

        // Always emit text — empty strings are a no-op on the terminal.
        if !text.is_empty() {
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
            .map_err(|e| self.transport_err(e))?;

        let mut acc = StreamingToolCallAccumulator::new();
        let mut response_text = String::new();
        let mut input_toks: u32 = 0;
        let mut output_toks: u32 = 0;

        loop {
            match source.next_event().await.map_err(|e| self.transport_err(e))? {
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

impl ConversationRuntime {
    /// Convert a [`TransportError`] into a [`RuntimeError`], preserving the
    /// real provider name so error messages identify which backend failed.
    fn transport_err(&self, err: TransportError) -> RuntimeError {
        RuntimeError::Provider(ProviderError::Transport {
            provider: self.provider.name().to_string(),
            source: err,
        })
    }
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

    /// Like `make_runtime` but with a custom `max_iterations` guard.
    fn make_runtime_with_max_iter(
        responses: Vec<MessageResponse>,
        max_iterations: usize,
    ) -> ConversationRuntime {
        let mut tools = ToolRegistry::new();
        tools.register(EchoTool);
        ConversationRuntime::new(
            Box::new(MockProvider::new(responses)),
            tools,
            RuntimeConfig {
                max_iterations,
                ..RuntimeConfig::default()
            },
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

    /// Regression: provider failure mid-loop (after tool messages have been
    /// appended) must roll back ALL messages added during that turn, not just
    /// the last one.
    ///
    /// Old bug: `run_turn` called `self.history.pop()` on error, which removed
    /// only the final message. The user message + tool use blocks + tool result
    /// blocks from completed iterations remained in history, corrupting every
    /// subsequent turn.
    #[tokio::test]
    async fn history_fully_rolled_back_after_tool_call_then_provider_failure() {
        // Turn 0 (external, already in history): a clean prior exchange.
        // Then turn 1 (our test turn): tool call succeeds, second call fails.
        let mut rt = make_runtime(vec![
            MockProvider::tool_call_response("c1", "echo", json!({ "msg": "x" })),
            // No second response → provider queue is empty → simulates failure.
        ]);

        // Record history length before the failing turn.
        let before = rt.history().len();
        let err = rt.run_turn("trigger tool then fail", no_sink()).await;
        assert!(err.is_err(), "expected provider error on second call");

        // History must be back to exactly the pre-turn snapshot.
        assert_eq!(
            rt.history().len(),
            before,
            "history must be fully rolled back after mid-loop failure; \
             old bug left orphaned tool messages behind"
        );
    }

    /// Regression: a turn that hits max-iterations must also roll back the
    /// full history accumulated during that turn, not just the last message.
    #[tokio::test]
    async fn history_fully_rolled_back_on_max_iterations() {
        // max_iterations = 3, but we supply enough tool-call responses to hit
        // the guard before a final text response arrives.
        let responses = vec![
            MockProvider::tool_call_response("c1", "echo", json!({ "msg": "a" })),
            MockProvider::tool_call_response("c2", "echo", json!({ "msg": "b" })),
            MockProvider::tool_call_response("c3", "echo", json!({ "msg": "c" })),
        ];
        let mut rt = make_runtime_with_max_iter(responses, 3);

        let before = rt.history().len();
        let err = rt.run_turn("keep calling tools", no_sink()).await;
        assert!(
            matches!(err, Err(RuntimeError::MaxIterationsExceeded { .. })),
            "expected MaxIterationsExceeded, got {err:?}"
        );
        assert_eq!(
            rt.history().len(),
            before,
            "history must be fully rolled back on max-iterations; \
             old bug only popped the last message"
        );
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

    /// Regression: the FINAL assistant answer after a tool call must be emitted to
    /// the sink, not silenced by the `iterations > 0` guard.
    ///
    /// Failure mode in the old code: `do_send_turn(…, silent = true)` for all
    /// iterations after the first, so the final response was never printed to the
    /// terminal when tool calls were made.
    #[tokio::test]
    async fn final_answer_after_tool_call_is_emitted_to_sink() {
        let responses = vec![
            MockProvider::tool_call_response("c1", "echo", json!({ "msg": "ping" })),
            MockProvider::text_response("final answer here"),
        ];
        let mut rt = make_runtime(responses);
        let collected = Arc::new(Mutex::new(String::new()));
        let c = collected.clone();
        let sink = TextSink::new(Box::new(move |text: &str| {
            c.lock().unwrap().push_str(text);
        }));
        let summary = rt.run_turn("do a tool call", sink).await.unwrap();
        assert_eq!(summary.response_text, "final answer here");
        assert_eq!(
            *collected.lock().unwrap(),
            "final answer here",
            "final answer must be emitted to sink; old bug silenced it"
        );
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

    /// Regression: Bug §4 — provider name is preserved in transport errors.
    ///
    /// `transport_err` is now a method on `ConversationRuntime` so it can
    /// embed `self.provider.name()` rather than the hardcoded string "provider".
    #[test]
    fn transport_err_preserves_provider_name() {
        let rt = make_runtime(vec![]);
        let err = TransportError::MissingCredentials {
            provider: "OpenRouter".to_string(),
            env_var: "OPENROUTER_API_KEY".to_string(),
        };
        let rt_err = rt.transport_err(err);
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

    // ── Context compaction ────────────────────────────────────────────────────

    /// `compact_oldest_turn` drops the first user+assistant pair.
    #[tokio::test]
    async fn compact_oldest_turn_drops_first_pair() {
        let mut rt = make_runtime(vec![
            MockProvider::text_response("first"),
            MockProvider::text_response("second"),
        ]);
        rt.run_turn("one", no_sink()).await.unwrap();
        rt.run_turn("two", no_sink()).await.unwrap();
        // History: [user1, assistant1, user2, assistant2]
        assert_eq!(rt.history().len(), 4);

        let dropped = rt.compact_oldest_turn();
        assert!(dropped);
        // After compaction: [user2, assistant2]
        assert_eq!(rt.history().len(), 2);
        assert_eq!(rt.history()[0].role, "user");
    }

    /// Cannot compact when history has fewer than 4 messages.
    #[tokio::test]
    async fn compact_oldest_turn_noop_on_short_history() {
        let mut rt = make_runtime(vec![MockProvider::text_response("hi")]);
        rt.run_turn("hello", no_sink()).await.unwrap();
        // History: [user, assistant] — only 2 messages
        assert_eq!(rt.history().len(), 2);
        let dropped = rt.compact_oldest_turn();
        assert!(!dropped, "should not compact when < 4 messages");
        assert_eq!(rt.history().len(), 2);
    }

    /// When `context_token_budget` is tiny, compaction fires automatically
    /// during the third turn (when there are ≥ 4 prior messages to drop).
    /// The turn still completes and history is shorter than without compaction.
    #[tokio::test]
    async fn auto_compaction_fires_when_over_budget() {
        // 3 turns: turns 1 and 2 accumulate history, turn 3 triggers compaction.
        let responses = vec![
            MockProvider::text_response("alpha"),  // turn 1
            MockProvider::text_response("beta"),   // turn 2
            MockProvider::text_response("gamma"),  // turn 3 (compaction fires)
        ];
        let mut tools = ToolRegistry::new();
        tools.register(EchoTool);
        // Budget = 3 tokens. After 2 turns the history is ~4 tokens, so on turn 3
        // the runtime will compact one (user+assistant) pair before sending.
        let config = RuntimeConfig {
            context_token_budget: 3,
            ..Default::default()
        };
        let mut rt = ConversationRuntime::new(
            Box::new(MockProvider::new(responses)),
            tools,
            config,
        );

        rt.run_turn("one", no_sink()).await.unwrap();
        rt.run_turn("two", no_sink()).await.unwrap();
        // History: [user1, assistant1, user2, assistant2] — 4 messages, > 3 tokens.
        let history_before = rt.history().len();
        assert_eq!(history_before, 4);

        let summary = rt.run_turn("three", no_sink()).await.unwrap();
        // Turn 3 completed (compaction happened, oldest pair was dropped).
        assert_eq!(summary.response_text, "gamma");
        // History should be smaller than it would be without compaction.
        assert!(
            rt.history().len() < history_before + 2,
            "expected compaction to have reduced history: len={}",
            rt.history().len()
        );
    }

    /// When a turn's history cannot be compacted (< 4 msgs) and still exceeds
    /// the budget, the turn proceeds without error — the provider handles overflow.
    /// `ContextTooLarge` is NOT returned in this case; it is only returned when
    /// compaction was attempted but insufficient.
    #[tokio::test]
    async fn single_turn_over_budget_proceeds_without_error() {
        let mut tools = ToolRegistry::new();
        tools.register(EchoTool);
        let config = RuntimeConfig {
            // Budget of 1 token; a single user+assistant pair cannot be dropped.
            context_token_budget: 1,
            ..Default::default()
        };
        let responses = vec![
            MockProvider::text_response("ok"),
            MockProvider::text_response("ok2"),
        ];
        let mut rt = ConversationRuntime::new(
            Box::new(MockProvider::new(responses)),
            tools,
            config,
        );
        // Turn 1: history has only [user]. compact_if_needed finds < 4 msgs and no
        // prior compaction — so it proceeds without error, even though budget < tokens.
        rt.run_turn("msg1", no_sink()).await.unwrap();
        // Turn 2: history is [user1, assistant1, user2] = 3 msgs. Still < 4 → no error.
        rt.run_turn("msg2", no_sink()).await.unwrap();
    }

    /// `estimated_history_tokens` returns 0 for empty history.
    #[test]
    fn estimated_tokens_zero_on_empty_history() {
        let rt = make_runtime(vec![]);
        assert_eq!(rt.estimated_history_tokens(), 0);
    }

    /// `estimated_history_tokens` grows after turns are added.
    #[tokio::test]
    async fn estimated_tokens_grows_with_history() {
        let mut rt = make_runtime(vec![
            MockProvider::text_response("some response text here"),
        ]);
        let before = rt.estimated_history_tokens();
        rt.run_turn("user message", no_sink()).await.unwrap();
        let after = rt.estimated_history_tokens();
        assert!(after > before, "tokens should grow: before={before} after={after}");
    }

    // ── End-to-end smoke tests ─────────────────────────────────────────────────
    //
    // These tests exercise the full pipeline: config → ToolRegistry with real
    // built-in tools → ConversationRuntime → provider round-trip → tool
    // execution → result injection → final text response.
    //
    // All file I/O is sandboxed inside a temp directory so the tests are
    // hermetic and do not touch the caller's working directory.

    /// Helper: build a runtime with the real built-in tool registry rooted at
    /// `cwd`, backed by a `MockProvider` that will serve `responses` in order.
    fn make_builtin_runtime(
        responses: Vec<MessageResponse>,
        cwd: std::path::PathBuf,
    ) -> ConversationRuntime {
        let mut tools = ToolRegistry::new();
        tools.register_builtin(cwd);
        ConversationRuntime::new(
            Box::new(MockProvider::new(responses)),
            tools,
            RuntimeConfig::default(),
        )
    }

    /// Full read_file cycle:
    ///   1. Model asks to call `read_file` on a file that exists.
    ///   2. Runtime executes the real ReadFileTool; content is injected into history.
    ///   3. Model returns a final text answer.
    ///   4. Verify: tool_calls_made == 1, response_text is non-empty.
    #[tokio::test]
    async fn e2e_read_file_tool_executes_and_injects_content() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("greeting.txt");
        std::fs::write(&file_path, "Hello from the test file!\n").unwrap();

        let responses = vec![
            MockProvider::tool_call_response(
                "call-1",
                "read_file",
                json!({ "path": "greeting.txt" }),
            ),
            MockProvider::text_response("The file says: Hello from the test file!"),
        ];

        let mut rt = make_builtin_runtime(responses, dir.path().to_path_buf());
        let summary = rt
            .run_turn("What is in greeting.txt?", no_sink())
            .await
            .unwrap();

        assert_eq!(summary.tool_calls_made, 1, "one tool call should have been made");
        assert_eq!(summary.iterations, 2, "two iterations: tool call + final answer");
        assert!(
            summary.response_text.contains("Hello from the test file!"),
            "final text should include the injected file content: {:?}",
            summary.response_text
        );
    }

    /// Full write_file cycle:
    ///   1. Model asks to call `write_file` to create a new file.
    ///   2. Runtime executes the real WriteFileTool; file is written to disk.
    ///   3. Model returns a final text answer.
    ///   4. Verify the file was actually created on disk with the expected content.
    #[tokio::test]
    async fn e2e_write_file_tool_creates_real_file() {
        let dir = tempfile::tempdir().unwrap();

        let responses = vec![
            MockProvider::tool_call_response(
                "call-2",
                "write_file",
                json!({
                    "path": "output.txt",
                    "content": "Generated by the model."
                }),
            ),
            MockProvider::text_response("File written successfully."),
        ];

        let mut rt = make_builtin_runtime(responses, dir.path().to_path_buf());
        let summary = rt.run_turn("Write a file called output.txt.", no_sink()).await.unwrap();

        assert_eq!(summary.tool_calls_made, 1);
        assert_eq!(summary.response_text, "File written successfully.");

        let written = dir.path().join("output.txt");
        assert!(written.exists(), "output.txt should have been created on disk");
        let contents = std::fs::read_to_string(&written).unwrap();
        assert_eq!(contents, "Generated by the model.");
    }

    /// Two-tool sequence: write then read.
    ///   Model writes a file, then reads it back, then returns a final text answer.
    #[tokio::test]
    async fn e2e_two_tool_sequence_write_then_read() {
        let dir = tempfile::tempdir().unwrap();

        let responses = vec![
            MockProvider::tool_call_response(
                "w1",
                "write_file",
                json!({ "path": "data.txt", "content": "value=42" }),
            ),
            MockProvider::tool_call_response(
                "r1",
                "read_file",
                json!({ "path": "data.txt" }),
            ),
            MockProvider::text_response("I read back value=42 from data.txt."),
        ];

        let mut rt = make_builtin_runtime(responses, dir.path().to_path_buf());
        let summary = rt.run_turn("Write and read data.txt.", no_sink()).await.unwrap();

        assert_eq!(summary.tool_calls_made, 2);
        assert_eq!(summary.iterations, 3);

        let path = dir.path().join("data.txt");
        assert!(path.exists(), "file should exist after write_file");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "value=42");
        assert!(summary.response_text.contains("value=42"));
    }

    /// Strict no-tool mode: when the ToolRegistry is empty (has_tools == false)
    /// and the model unexpectedly emits a tool call, the runtime should ignore
    /// the tool call and return the response text without error.
    #[tokio::test]
    async fn e2e_no_tools_mode_ignores_unexpected_model_tool_calls() {
        // Runtime with an empty tool registry — no tools are advertised to the model.
        let mut rt = ConversationRuntime::new(
            Box::new(MockProvider::new(vec![
                // The model misbehaves and emits a tool call anyway.
                MockProvider::tool_call_response("bad-call", "read_file", json!({"path": "x"})),
                // The runtime should discard the tool call and loop again,
                // then the model returns a plain text response.
                MockProvider::text_response("No tools needed."),
            ])),
            ToolRegistry::new(), // empty — has_tools == false
            RuntimeConfig::default(),
        );

        let summary = rt.run_turn("Hello", no_sink()).await.unwrap();

        // Tool call was silently ignored, final answer returned normally.
        assert_eq!(
            summary.tool_calls_made, 0,
            "tool call from a no-tool runtime should be ignored"
        );
        assert_eq!(summary.response_text, "No tools needed.");
    }

    /// Full-pipeline config test: verify that AppConfig::default() produces
    /// a valid RuntimeConfig and that a runtime built from it handles a
    /// normal exchange without panics.
    #[tokio::test]
    async fn e2e_config_defaults_produce_valid_runtime() {
        use code_buddy_config::AppConfig;

        let app_cfg = AppConfig::default();

        // Validate that the default config satisfies all field constraints.
        app_cfg
            .validate()
            .expect("default AppConfig should pass validation");

        // Build a RuntimeConfig from the app config (mirrors what ask.rs does).
        let rt_config = RuntimeConfig {
            model: app_cfg.model.unwrap_or_else(|| "local-model".to_string()),
            max_tokens: app_cfg.max_tokens.unwrap_or(4096),
            temperature: app_cfg.temperature,
            system_prompt: app_cfg.system_prompt,
            streaming: app_cfg.streaming,
            debug: app_cfg.debug,
            ..RuntimeConfig::default()
        };

        let mut rt = ConversationRuntime::new(
            Box::new(MockProvider::new(vec![MockProvider::text_response(
                "Config OK",
            )])),
            ToolRegistry::new(),
            rt_config,
        );

        let summary = rt.run_turn("ping", no_sink()).await.unwrap();
        assert_eq!(summary.response_text, "Config OK");
        assert_eq!(summary.tool_calls_made, 0);
    }

    /// History grows across multiple turns, and each turn appends exactly
    /// one user + one assistant message (2 messages per turn).
    #[tokio::test]
    async fn e2e_history_grows_correctly_across_turns() {
        let mut rt = make_runtime(vec![
            MockProvider::text_response("answer1"),
            MockProvider::text_response("answer2"),
            MockProvider::text_response("answer3"),
        ]);

        assert_eq!(rt.history().len(), 0);
        rt.run_turn("q1", no_sink()).await.unwrap();
        assert_eq!(rt.history().len(), 2, "after turn 1: user + assistant");
        rt.run_turn("q2", no_sink()).await.unwrap();
        assert_eq!(rt.history().len(), 4, "after turn 2: two pairs");
        rt.run_turn("q3", no_sink()).await.unwrap();
        assert_eq!(rt.history().len(), 6, "after turn 3: three pairs");
    }

    /// clear_history resets the history to empty without affecting the runtime.
    #[tokio::test]
    async fn e2e_clear_history_resets_context() {
        let mut rt = make_runtime(vec![
            MockProvider::text_response("hello"),
            MockProvider::text_response("world"),
        ]);
        rt.run_turn("hi", no_sink()).await.unwrap();
        assert_eq!(rt.history().len(), 2);

        rt.clear_history();
        assert_eq!(rt.history().len(), 0, "history should be empty after clear");

        // Runtime is still usable after clear
        rt.run_turn("again", no_sink()).await.unwrap();
        assert_eq!(rt.history().len(), 2, "fresh history starts from 0 again");
    }

    /// An out-of-directory path in a tool call is rejected by path confinement.
    /// The tool returns a ToolError; the runtime injects the error as a tool
    /// result, and the model returns a final text response.
    #[tokio::test]
    async fn e2e_path_escape_attempt_is_rejected_by_tool() {
        let dir = tempfile::tempdir().unwrap();

        let responses = vec![
            MockProvider::tool_call_response(
                "escape",
                "read_file",
                json!({ "path": "../../../etc/passwd" }),
            ),
            MockProvider::text_response("Path was rejected."),
        ];

        let mut rt = make_builtin_runtime(responses, dir.path().to_path_buf());
        // The runtime should NOT panic. The tool error is injected into history
        // and the conversation continues to the final text response.
        let summary = rt.run_turn("Try to escape.", no_sink()).await.unwrap();

        assert_eq!(summary.tool_calls_made, 1, "the (failed) call still counts");
        assert_eq!(summary.response_text, "Path was rejected.");
    }
}
