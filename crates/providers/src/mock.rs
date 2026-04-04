//! Mock provider for unit testing.
//!
//! [`MockProvider`] replays pre-configured responses or errors without making any
//! real network calls. Use it in all downstream tests that need a provider.
//!
//! # Examples
//!
//! ```rust
//! use code_buddy_providers::MockProvider;
//! use code_buddy_transport::{MessageRequest, MessageResponse, OutputContentBlock, Usage};
//!
//! let response = MessageResponse {
//!     id: "test-1".to_string(),
//!     model: "mock".to_string(),
//!     content: vec![OutputContentBlock::Text { text: "Hello!".to_string() }],
//!     stop_reason: Some("end_turn".to_string()),
//!     usage: Usage { input_tokens: 5, output_tokens: 10 },
//! };
//!
//! let provider = MockProvider::with_response(response);
//! ```

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use code_buddy_errors::TransportError;
use code_buddy_transport::{MessageRequest, MessageResponse, Provider, StreamEvent, StreamSource};

// ── Canned response entry ─────────────────────────────────────────────────────

/// A single canned interaction for the mock provider.
#[derive(Debug, Clone)]
pub enum MockAction {
    /// Return this response from `send`.
    Response(MessageResponse),
    /// Yield these events from `stream`, then `MessageStop`.
    StreamEvents(Vec<StreamEvent>),
    /// Return this error from `send` or `stream`.
    Error(TransportError),
}

// ── MockProvider ──────────────────────────────────────────────────────────────

/// A provider that replays pre-configured responses or errors.
///
/// Actions are consumed in FIFO order. After the queue is exhausted the mock
/// panics (to catch tests that send unexpected requests).
#[derive(Debug, Clone)]
pub struct MockProvider {
    name: String,
    queue: Arc<Mutex<VecDeque<MockAction>>>,
    /// Track all requests received.
    pub received: Arc<Mutex<Vec<MessageRequest>>>,
}

impl MockProvider {
    /// Create a mock with an empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "Mock".to_string(),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            received: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a mock that returns a single successful response.
    #[must_use]
    pub fn with_response(response: MessageResponse) -> Self {
        let m = Self::new();
        m.push(MockAction::Response(response));
        m
    }

    /// Create a mock that returns a single error.
    #[must_use]
    pub fn with_error(error: TransportError) -> Self {
        let m = Self::new();
        m.push(MockAction::Error(error));
        m
    }

    /// Create a mock that streams a sequence of text deltas.
    #[must_use]
    pub fn with_text_stream(chunks: Vec<&str>) -> Self {
        let m = Self::new();
        let events = chunks
            .into_iter()
            .map(|t| StreamEvent::TextDelta(t.to_string()))
            .collect();
        m.push(MockAction::StreamEvents(events));
        m
    }

    /// Push an additional action onto the queue.
    pub fn push(&self, action: MockAction) {
        self.queue.lock().expect("mock lock").push_back(action);
    }

    fn pop(&self) -> MockAction {
        self.queue
            .lock()
            .expect("mock lock")
            .pop_front()
            .expect("MockProvider queue exhausted — did the test push enough actions?")
    }

    fn record(&self, req: &MessageRequest) {
        self.received
            .lock()
            .expect("mock lock")
            .push(req.clone());
    }

    /// Return all requests received so far.
    pub fn requests(&self) -> Vec<MessageRequest> {
        self.received.lock().expect("mock lock").clone()
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn send(&self, request: &MessageRequest) -> Result<MessageResponse, TransportError> {
        self.record(request);
        match self.pop() {
            MockAction::Response(r) => Ok(r),
            MockAction::Error(e) => Err(e),
            MockAction::StreamEvents(_) => panic!(
                "MockProvider: send() called but next action is StreamEvents — use stream() instead"
            ),
        }
    }

    async fn stream(
        &self,
        request: &MessageRequest,
    ) -> Result<Box<dyn StreamSource>, TransportError> {
        self.record(request);
        match self.pop() {
            MockAction::StreamEvents(events) => Ok(Box::new(MockStreamSource::new(events))),
            MockAction::Error(e) => Err(e),
            MockAction::Response(_) => panic!(
                "MockProvider: stream() called but next action is Response — use send() instead"
            ),
        }
    }
}

// ── MockStreamSource ──────────────────────────────────────────────────────────

/// A stream source that drains a pre-loaded event queue.
///
/// Automatically appends [`StreamEvent::MessageStop`] after all events.
pub struct MockStreamSource {
    events: VecDeque<StreamEvent>,
    done: bool,
}

impl MockStreamSource {
    #[must_use]
    pub fn new(events: Vec<StreamEvent>) -> Self {
        Self {
            events: events.into_iter().collect(),
            done: false,
        }
    }
}

#[async_trait]
impl StreamSource for MockStreamSource {
    async fn next_event(&mut self) -> Result<Option<StreamEvent>, TransportError> {
        if let Some(event) = self.events.pop_front() {
            return Ok(Some(event));
        }
        if !self.done {
            self.done = true;
            return Ok(Some(StreamEvent::MessageStop));
        }
        Ok(None)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use code_buddy_transport::{MessageRequest, OutputContentBlock, Usage};

    fn test_response() -> MessageResponse {
        MessageResponse {
            id: "mock-1".to_string(),
            model: "mock".to_string(),
            content: vec![OutputContentBlock::Text {
                text: "Hi there".to_string(),
            }],
            stop_reason: Some("end_turn".to_string()),
            usage: Usage {
                input_tokens: 5,
                output_tokens: 10,
            },
        }
    }

    #[tokio::test]
    async fn mock_send_returns_response() {
        let provider = MockProvider::with_response(test_response());
        let req = MessageRequest::simple("mock", "hello");
        let resp = provider.send(&req).await.unwrap();
        assert_eq!(resp.text_content(), "Hi there");
    }

    #[tokio::test]
    async fn mock_send_returns_error() {
        let provider = MockProvider::with_error(TransportError::Timeout {
            provider: "Mock".to_string(),
            timeout_secs: 30,
        });
        let req = MessageRequest::simple("mock", "hello");
        let err = provider.send(&req).await.err().unwrap();
        assert!(matches!(err, TransportError::Timeout { .. }));
    }

    #[tokio::test]
    async fn mock_stream_yields_events_then_stop() {
        let provider = MockProvider::with_text_stream(vec!["Hello", ", ", "world"]);
        let req = MessageRequest::simple("mock", "hello");
        let mut stream = provider.stream(&req).await.unwrap();

        let e1 = stream.next_event().await.unwrap();
        assert!(matches!(e1, Some(StreamEvent::TextDelta(ref t)) if t == "Hello"));

        let e2 = stream.next_event().await.unwrap();
        assert!(matches!(e2, Some(StreamEvent::TextDelta(ref t)) if t == ", "));

        let e3 = stream.next_event().await.unwrap();
        assert!(matches!(e3, Some(StreamEvent::TextDelta(ref t)) if t == "world"));

        let stop = stream.next_event().await.unwrap();
        assert!(matches!(stop, Some(StreamEvent::MessageStop)));

        let none = stream.next_event().await.unwrap();
        assert!(none.is_none());
    }

    #[tokio::test]
    async fn mock_records_requests() {
        let provider = MockProvider::new();
        provider.push(MockAction::Response(test_response()));
        provider.push(MockAction::Response(test_response()));

        let req1 = MessageRequest::simple("model-a", "question 1");
        let req2 = MessageRequest::simple("model-b", "question 2");
        provider.send(&req1).await.unwrap();
        provider.send(&req2).await.unwrap();

        let received = provider.requests();
        assert_eq!(received.len(), 2);
        assert_eq!(received[0].model, "model-a");
        assert_eq!(received[1].model, "model-b");
    }

    #[tokio::test]
    async fn mock_stream_error() {
        let provider = MockProvider::with_error(TransportError::Network {
            provider: "Mock".to_string(),
            detail: "connection refused".to_string(),
        });
        let req = MessageRequest::simple("mock", "hello");
        let err = provider.stream(&req).await.err().unwrap();
        assert!(matches!(err, TransportError::Network { .. }));
    }

    #[tokio::test]
    async fn mock_multi_action_queue() {
        let provider = MockProvider::new();
        provider.push(MockAction::Response(test_response()));
        provider.push(MockAction::Error(TransportError::Timeout {
            provider: "Mock".to_string(),
            timeout_secs: 5,
        }));

        let req = MessageRequest::simple("mock", "hi");
        assert!(provider.send(&req).await.is_ok());
        assert!(provider.send(&req).await.is_err());
    }
}
