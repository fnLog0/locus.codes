//! Context and prompt building for the Runtime.
//!
//! Split into focused submodules:
//! - **prompt** — system prompt construction and tool formatting
//! - **messages** — session-to-LLM message conversion and request building
//! - **window** — context window management (token estimation, compression)
//! - **extract** — file path extraction from session turns

mod extract;
mod messages;
mod prompt;
mod window;

pub use messages::{build_generate_request, build_messages, build_session_context};
pub use prompt::build_system_prompt;
pub use window::{compress_context, near_context_limit};
