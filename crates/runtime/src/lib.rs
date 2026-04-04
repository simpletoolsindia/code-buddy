//! Conversation runtime for Code Buddy.
//!
//! The runtime manages the full tool-calling loop: send → detect tool calls →
//! execute → inject results → loop. It enforces a maximum iteration limit and
//! provides an optional text callback for streaming output.

pub mod conversation;

pub use conversation::{ConversationRuntime, RuntimeConfig, TextSink, TurnSummary};
