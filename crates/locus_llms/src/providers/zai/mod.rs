//! Z.AI provider module
//!
//! Implements the Provider trait for Z.AI's GLM model family.
//! API docs: https://docs.z.ai/api-reference/llm/chat-completion

mod convert;
mod provider;
mod stream;
mod types;

pub use provider::ZaiProvider;
pub use types::{ZaiConfig, ZaiRequest, ZaiResponse};
