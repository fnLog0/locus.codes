//! Context ID constants for LocusGraph.
//!
//! Backend requires format `type:name` (e.g. fact:redis_caching). Type is aligned with event_kind.

/// TODO(Phase 2): replace this with the `tool_anchor:{project_name}_{repo_hash}` helper.
pub const CONTEXT_TOOLS: &str = "fact:tools";
