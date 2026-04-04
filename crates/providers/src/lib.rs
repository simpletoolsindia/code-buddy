//! LLM provider adapters for Code Buddy.
//!
//! This crate provides concrete implementations of the [`code_buddy_transport::Provider`] trait
//! for all supported providers. All providers speak the OpenAI chat-completions wire format,
//! so a single generic adapter handles the HTTP layer, with per-provider config for
//! auth, base URL, and any quirks.
//!
//! # Provider selection
//!
//! Use [`ProviderRegistry::from_config`] to get the correct adapter for the current
//! [`AppConfig`]. The returned boxed [`Provider`] can be used for both non-streaming
//! and streaming calls.

pub mod mock;
pub mod openai_compat;
pub mod registry;

pub use mock::MockProvider;
pub use openai_compat::{AdapterConfig, OpenAiCompatAdapter, SseStreamSource};
pub use registry::ProviderRegistry;
