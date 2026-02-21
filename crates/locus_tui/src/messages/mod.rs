//! Message rendering for the TUI. Uses crate::theme for colors.
//!
//! - **user** — User message layout (see `docs/user-message-plan.md`).
//! - **ai_message** — AI/assistant message lines.
//! - **ai_think_message** — AI thinking/reasoning (muted).
//! - **tool** — Tool list and tool call status (no dependency on locus_toolbus).
//! - **meta_tool** — Meta-tools tool_search, tool_explain, task (no dependency on locus_runtime).

pub mod ai_message;
pub mod ai_think_message;
pub mod error;
pub mod markdown;
pub mod meta_tool;
pub mod tool;
pub mod user;
