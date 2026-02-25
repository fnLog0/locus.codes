//! Types for LocusGraph operations.
//!
//! Higher-level types that map to the gRPC request/response types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Kind of event being stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// A factual piece of information
    Fact,
    /// An action that was taken
    Action,
    /// A decision that was made
    Decision,
    /// An observation from the system
    Observation,
    /// Feedback from user or system
    Feedback,
}

impl EventKind {
    /// Convert to string for gRPC.
    pub fn as_str(&self) -> &'static str {
        match self {
            EventKind::Fact => "fact",
            EventKind::Action => "action",
            EventKind::Decision => "decision",
            EventKind::Observation => "observation",
            EventKind::Feedback => "feedback",
        }
    }
}

impl std::fmt::Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Request to store a memory event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEventRequest {
    /// Kind of event (fact, action, decision, observation, feedback)
    pub event_kind: EventKind,
    /// Context ID (e.g., "terminal", "editor", "user_intent", "errors", "decisions")
    pub context_id: Option<String>,
    /// Source of the event. Priority (high→low): validator (0.9), executor (0.8), user (0.7), agent (0.6), system (0.5)
    pub source: Option<String>,
    /// Event payload as JSON
    pub payload: serde_json::Value,
    /// Related context IDs
    pub related_to: Option<Vec<String>>,
    /// Context IDs this event extends
    pub extends: Option<Vec<String>>,
    /// Context IDs this event reinforces
    pub reinforces: Option<Vec<String>>,
    /// Context IDs this event contradicts
    pub contradicts: Option<Vec<String>>,
    /// Optional timestamp (ISO 8601 or Unix timestamp)
    pub timestamp: Option<String>,
}

impl CreateEventRequest {
    /// Create a new event request with the given kind and payload.
    pub fn new(event_kind: EventKind, payload: serde_json::Value) -> Self {
        Self {
            event_kind,
            payload,
            context_id: None,
            source: None,
            related_to: None,
            extends: None,
            reinforces: None,
            contradicts: None,
            timestamp: None,
        }
    }

    /// Set the context ID.
    pub fn context_id(mut self, id: impl Into<String>) -> Self {
        self.context_id = Some(id.into());
        self
    }

    /// Set the source.
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set related context IDs.
    pub fn related_to(mut self, ids: Vec<String>) -> Self {
        self.related_to = Some(ids);
        self
    }

    /// Set extends context IDs.
    pub fn extends(mut self, ids: Vec<String>) -> Self {
        self.extends = Some(ids);
        self
    }

    /// Set reinforces context IDs.
    pub fn reinforces(mut self, ids: Vec<String>) -> Self {
        self.reinforces = Some(ids);
        self
    }

    /// Set contradicts context IDs.
    pub fn contradicts(mut self, ids: Vec<String>) -> Self {
        self.contradicts = Some(ids);
        self
    }

    /// Set timestamp.
    pub fn timestamp(mut self, ts: impl Into<String>) -> Self {
        self.timestamp = Some(ts.into());
        self
    }
}

/// Result from retrieve_memories operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResult {
    /// Markdown-formatted string to inject into prompt
    pub memories: String,
    /// Number of items found
    pub items_found: u64,
    /// True if the server was unreachable and we returned empty results
    pub degraded: bool,
}

/// Result from generate_insights operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightResult {
    /// The generated insight
    pub insight: String,
    /// Recommended action
    pub recommendation: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
}

/// Context type summary from list_context_types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextType {
    /// Context type name (e.g., "fact", "decision")
    pub context_type: String,
    /// Number of contexts of this type
    pub count: u64,
}

/// Context from list_contexts_by_type or search_contexts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Context ID (e.g., "fact:api_design")
    pub context_id: String,
    /// Context type (e.g., "fact")
    pub context_type: String,
    /// Human-readable name
    pub context_name: String,
    /// Creation timestamp (Unix epoch)
    pub created_at: u64,
    /// Last update timestamp (Unix epoch)
    pub updated_at: u64,
    /// Number of events referencing this context
    pub reference_count: u64,
}

/// Filter for context types in retrieve and insights operations.
#[derive(Debug, Clone, Default)]
pub struct ContextTypeFilter {
    /// Names of contexts to include (empty = all)
    pub names: Vec<String>,
}

impl ContextTypeFilter {
    /// Create a new filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a context name to the filter.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.names.push(name.into());
        self
    }

    /// Add multiple context names.
    pub fn names(mut self, names: Vec<String>) -> Self {
        self.names.extend(names);
        self
    }
}

/// Options for retrieve_memories operation.
#[derive(Debug, Clone, Default)]
pub struct RetrieveOptions {
    /// Maximum number of results
    pub limit: Option<u64>,
    /// Filter by specific context IDs
    pub context_ids: Vec<String>,
    /// Filter by context types
    pub context_types: HashMap<String, ContextTypeFilter>,
}

impl RetrieveOptions {
    /// Create default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set limit.
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Add a context ID filter.
    pub fn context_id(mut self, id: impl Into<String>) -> Self {
        self.context_ids.push(id.into());
        self
    }

    /// Add a context type filter.
    pub fn context_type(mut self, type_name: impl Into<String>, filter: ContextTypeFilter) -> Self {
        self.context_types.insert(type_name.into(), filter);
        self
    }
}

/// Links connecting this event to related contexts.
///
/// These build the experience graph — connecting events the way human memory does:
/// - `related_to`: "this is connected to that" (association)
/// - `extends`: "this adds detail to that" (deepening understanding)
/// - `reinforces`: "this confirms that was correct" (building confidence)
/// - `contradicts`: "this proves that was wrong" (correcting beliefs)
#[derive(Debug, Clone, Default)]
pub struct EventLinks {
    /// Associated context IDs
    pub related_to: Vec<String>,
    /// Context IDs this event adds detail to
    pub extends: Vec<String>,
    /// Context IDs this event supports with new evidence
    pub reinforces: Vec<String>,
    /// Context IDs this event conflicts with
    pub contradicts: Vec<String>,
}

impl EventLinks {
    /// Create empty links.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a related_to link.
    pub fn related_to(mut self, id: impl Into<String>) -> Self {
        self.related_to.push(id.into());
        self
    }

    /// Add an extends link.
    pub fn extends(mut self, id: impl Into<String>) -> Self {
        self.extends.push(id.into());
        self
    }

    /// Add a reinforces link.
    pub fn reinforces(mut self, id: impl Into<String>) -> Self {
        self.reinforces.push(id.into());
        self
    }

    /// Add a contradicts link.
    pub fn contradicts(mut self, id: impl Into<String>) -> Self {
        self.contradicts.push(id.into());
        self
    }

    /// Merge another set of links into this one.
    pub fn merge(mut self, other: EventLinks) -> Self {
        self.related_to.extend(other.related_to);
        self.extends.extend(other.extends);
        self.reinforces.extend(other.reinforces);
        self.contradicts.extend(other.contradicts);
        self
    }
}

/// Summary of a completed turn, stored at turn end.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSummary {
    /// Human-readable title for the turn
    pub title: String,
    /// Compressed user request
    pub user_request: String,
    /// High-level actions taken
    pub actions_taken: Vec<String>,
    /// Outcome description
    pub outcome: String,
    /// Key decisions made
    pub decisions: Vec<String>,
    /// Files that were read
    pub files_read: Vec<String>,
    /// Files that were modified
    pub files_modified: Vec<String>,
    /// Total events recorded in this turn
    pub event_count: u32,
}

/// Options for generate_insights operation.
#[derive(Debug, Clone, Default)]
pub struct InsightsOptions {
    /// Optional locus query for filtering
    pub locus_query: Option<String>,
    /// Maximum number of memories to consider
    pub limit: Option<u64>,
    /// Filter by specific context IDs
    pub context_ids: Vec<String>,
    /// Filter by context types
    pub context_types: HashMap<String, ContextTypeFilter>,
}

impl InsightsOptions {
    /// Create default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set locus query.
    pub fn locus_query(mut self, query: impl Into<String>) -> Self {
        self.locus_query = Some(query.into());
        self
    }

    /// Set limit.
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Add a context ID filter.
    pub fn context_id(mut self, id: impl Into<String>) -> Self {
        self.context_ids.push(id.into());
        self
    }

    /// Add a context type filter.
    pub fn context_type(mut self, type_name: impl Into<String>, filter: ContextTypeFilter) -> Self {
        self.context_types.insert(type_name.into(), filter);
        self
    }
}
