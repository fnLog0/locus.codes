//! LocusGraph SDK — implicit memory layer for locus.codes.
//!
//! One `graph_id`, one brain — all sessions read/write to the same graph.
//!
//! # Philosophy
//!
//! Amp-style simplicity with LocusGraph as the persistent brain.
//! No manual AGENT.md files — the agent learns conventions from actions.
//!
//! # Key Features
//!
//! - **Prevent hallucination** — retrieve relevant memories before every LLM call
//! - **Persistence** — every tool call, file edit, user intent, and error becomes a memory
//! - **Learning** — the AI improves across sessions by recalling past context
//! - **Cross-session** — start a new session, still remember project patterns
//! - **Semantic recall** — "how do we handle auth?" → relevant memories injected
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use locus_graph::{LocusGraphClient, LocusGraphConfig, CreateEventRequest, EventKind};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client from environment
//!     let config = LocusGraphConfig::from_env()?;
//!     let client = LocusGraphClient::new(config).await?;
//!
//!     // Store a memory (fire-and-forget)
//!     let event = CreateEventRequest::new(
//!         EventKind::Fact,
//!         serde_json::json!({
//!             "kind": "technical_fact",
//!             "data": {
//!                 "topic": "auth",
//!                 "value": "we use JWT tokens"
//!             }
//!         })
//!     )
//!     .context_id("fact:auth");
//!     client.store_event(event).await;
//!
//!     // Retrieve memories before LLM call
//!     let result = client.retrieve_memories("how do we handle auth?", None).await?;
//!     println!("Found {} memories", result.items_found);
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod config;
pub mod error;
pub mod hooks;
pub mod types;

// Re-export main types at crate root
pub use client::LocusGraphClient;
pub use config::LocusGraphConfig;
pub use error::{LocusGraphError, Result};
pub use types::{
    Context, ContextResult, ContextType, ContextTypeFilter, CreateEventRequest, EventKind,
    InsightResult, InsightsOptions, RetrieveOptions,
};

// Re-export context ID constants for use across crates
pub use hooks::{
    CONTEXT_DECISIONS, CONTEXT_EDITOR, CONTEXT_ERRORS, CONTEXT_TERMINAL, CONTEXT_TOOLS,
    CONTEXT_USER_INTENT,
};
