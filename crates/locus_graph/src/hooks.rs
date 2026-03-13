//! Context ID helpers for LocusGraph are defined in `locus_runtime::memory`.
//!
//! Backend requires format `type:name` (e.g. fact:redis_caching). Type is aligned
//! with `event_kind`. Runtime-specific helpers like
//! `tool_anchor:{project_name}_{repo_hash}` live in the runtime crate.
