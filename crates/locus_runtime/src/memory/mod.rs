//! Memory recall and storage helpers for the Runtime.
//!
//! These functions wrap LocusGraph operations for use in the agent loop.
//! All storage operations are fire-and-forget (non-blocking).

mod anchors;
mod bootstrap;
mod graph_map;
mod recall;
mod session;
mod turns;
mod utils;

pub use anchors::{
    ensure_project_anchor, ensure_session_anchor, project_anchor_id, session_anchor_id,
    session_context_id, tool_anchor_id,
};
pub use bootstrap::bootstrap_tools;
pub use graph_map::build_graph_map;
pub use recall::{
    build_context_ids, fetch_session_turns, get_active_tools, recall_memories, CORE_TOOLS,
};
pub use session::{store_session_end, store_session_start};
pub use utils::simple_hash;

pub use turns::{
    build_action_event, build_error_event, build_intent_event, build_llm_event, build_turn_end,
    build_turn_start,
};

pub(crate) use utils::safe_context_name;

#[cfg(test)]
mod tests;
