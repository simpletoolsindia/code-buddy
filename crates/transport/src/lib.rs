//! Transport layer for Code Buddy.
//!
//! This crate defines the core message types and the abstract `Provider` trait.
//! Concrete provider implementations live in a future `code-buddy-providers` crate.
//! The transport crate contains only the shared contract, keeping provider logic isolated.

use async_trait::async_trait;
use code_buddy_errors::TransportError;

pub mod message;
pub mod sse;

pub use message::{
    ContentBlock, InputContentBlock, InputMessage, MessageRequest, MessageResponse,
    OutputContentBlock, StreamEvent, ToolChoice, ToolDefinition, Usage,
};

/// The core provider trait.
///
/// Every LLM provider adapter must implement this trait. The trait provides
/// two modes of operation: non-streaming (full response) and streaming (SSE events).
///
/// Provider-specific request/response translation happens inside each adapter.
/// No provider-specific logic should leak outside the provider module.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider name for error messages and logging.
    fn name(&self) -> &str;

    /// Send a request and receive the full response.
    ///
    /// # Errors
    /// Returns [`TransportError`] on network, parse, or server errors.
    async fn send(&self, request: &MessageRequest) -> Result<MessageResponse, TransportError>;

    /// Send a request and receive a stream of events.
    ///
    /// Returns a boxed async stream of [`StreamEvent`] items. Each event
    /// arrives as the model produces tokens.
    ///
    /// # Errors
    /// Returns [`TransportError`] if the stream cannot be initiated.
    async fn stream(
        &self,
        request: &MessageRequest,
    ) -> Result<Box<dyn StreamSource>, TransportError>;
}

/// A stream source that yields `StreamEvent` items one at a time.
#[async_trait]
pub trait StreamSource: Send {
    /// Get the next event from the stream.
    ///
    /// Returns `Ok(None)` when the stream is complete.
    ///
    /// # Errors
    /// Returns [`TransportError`] if the stream encounters an error.
    async fn next_event(&mut self) -> Result<Option<StreamEvent>, TransportError>;
}
