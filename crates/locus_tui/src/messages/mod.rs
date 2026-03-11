//! Message rendering for the TUI. Uses crate::theme for colors.
//!
//! - **user** — User message layout (see `docs/user-message-plan.md`).
//! - **ai_message** — AI/assistant message lines.
//! - **ai_think_message** — AI thinking/reasoning (muted).
//! - **tools** — Tool list, tool call status, and per-tool rendering modules.
//! - **meta_tools** — Meta-tools tool_search, tool_explain, task with rendering.
//! - **memory** — Memory recall/store events from LocusGraph.

pub mod ai_message;
pub mod ai_think_message;
pub mod common;
pub mod edit_diff;
pub mod error;
pub mod markdown;
pub mod memory;
pub mod meta_tools;
pub mod tools;
pub mod user;
