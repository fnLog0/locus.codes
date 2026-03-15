//! Main Runtime orchestrator for locus.codes.
//!
//! The Runtime ties together all components (Session, LocusGraph, LLM, ToolBus)
//! into a cohesive agent loop with Amp-style simplicity.
//!
//! Split into focused submodules:
//! - **agent_loop** — run, agent loop, message processing, prepare_llm_call
//! - **llm** — LLM streaming and response handling
//! - **tools** — tool call execution and sub-agent task spawning

mod agent_loop;
mod llm;
mod tools;

use std::sync::Arc;

use locus_core::{ContentBlock, Role, Session, SessionConfig, SessionEvent, SessionStatus, Turn};
use locus_graph::{CreateEventRequest, LocusGraphClient, LocusGraphConfig, TurnSummary};
use locus_llms::{AnthropicProvider, Provider, ZaiProvider};
use locus_toolbus::{ToolBus, ToolInfo};
use tokio::sync::mpsc;
use tracing::info;

use crate::config::{LlmProvider, RuntimeConfig};
use crate::error::RuntimeError;
use crate::memory;

/// The main orchestrator for locus.codes.
///
/// Owns all components and runs the agent loop.
pub struct Runtime {
    /// The current session state
    pub session: Session,
    /// LocusGraph client for memory operations
    pub locus_graph: Arc<LocusGraphClient>,
    /// LLM provider for generating responses
    pub llm_client: Arc<dyn Provider>,
    /// ToolBus for executing tools
    pub toolbus: Arc<ToolBus>,
    /// Event channel for streaming events to TUI
    pub event_tx: mpsc::Sender<SessionEvent>,
    /// Runtime configuration
    pub config: RuntimeConfig,
    /// Cached context IDs for memory queries (updated when new turns are created)
    context_ids: Vec<String>,
    /// Cached active tools (stable per runtime lifetime)
    active_tools: Vec<ToolInfo>,
    /// Current turn sequence number (1-based, increments per turn)
    turn_sequence: u32,
    /// Event sequence counter within current turn (1-based, resets per turn)
    event_seq: u32,
    /// Session slug (kebab-case from first user message, set on first turn)
    session_slug: String,
    /// Current turn slug (kebab-case from user message, set each turn)
    turn_slug: String,
    /// Repo hash for context IDs
    repo_hash: String,
    /// Project name derived from repo root directory name
    project_name: String,
    /// Write buffer for turn events (flushed at turn end)
    turn_event_buffer: Vec<CreateEventRequest>,
    /// Cached graph map (structural hierarchy, built at startup)
    graph_map: String,
}

impl Runtime {
    /// Create a new Runtime with the given configuration.
    ///
    /// This initializes:
    /// - LocusGraph client from environment
    /// - ToolBus with default tools
    /// - LLM provider based on configuration
    /// - A new Session
    pub async fn new(
        config: RuntimeConfig,
        event_tx: mpsc::Sender<SessionEvent>,
    ) -> Result<Self, RuntimeError> {
        // Create repo hash and project name for context IDs
        let repo_hash = memory::simple_hash(config.repo_root.to_str().unwrap_or("unknown"));
        let project_name = config
            .repo_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();

        // Initialize LocusGraph client
        let locus_graph_config = LocusGraphConfig::from_env()
            .map_err(|e| RuntimeError::ConfigError(format!("LocusGraph config: {}", e)))?;
        let locus_graph = LocusGraphClient::new(locus_graph_config)
            .await
            .map_err(|e| RuntimeError::MemoryFailed(format!("LocusGraph client: {}", e)))?;

        // Initialize ToolBus
        let toolbus = Arc::new(ToolBus::new(config.repo_root.clone()));

        // Initialize LLM provider
        let llm_client = Self::create_provider(&config.provider)?;

        let locus_graph = Arc::new(locus_graph);

        // Create session
        let session_config = SessionConfig::new(&config.model, config.provider.as_str())
            .with_max_turns(config.max_turns.unwrap_or(0))
            .with_sandbox_policy(config.sandbox.clone());

        let session = Session::new(config.repo_root.clone(), session_config);

        // Get tools for bootstrap
        let toolbus_tools = toolbus.list_tools();
        let meta_tools = locus_toolbus::meta_tool_definitions();

        // Ensure project root anchor exists (idempotent)
        memory::ensure_project_anchor(&locus_graph, &project_name, &repo_hash, &config.repo_root)
            .await;

        // Ensure session anchor exists (idempotent)
        memory::ensure_session_anchor(&locus_graph, &project_name, &repo_hash).await;

        // Bootstrap tools in LocusGraph (idempotent — safe to call every time)
        memory::bootstrap_tools(
            Arc::clone(&locus_graph),
            project_name.clone(),
            repo_hash.clone(),
            memory::project_anchor_id(&project_name, &repo_hash),
            toolbus_tools.clone(),
            meta_tools.clone(),
            locus_constant::app::VERSION.to_string(),
        );

        // Build graph map (walk hierarchy 2 levels deep from project root)
        let graph_map =
            memory::build_graph_map(&locus_graph, &project_name, &repo_hash).await;

        // Cache context IDs and active tools (starts empty for turns, populated at session start)
        let context_ids = memory::build_context_ids(&project_name, &repo_hash, "", "", &[]);
        let mut active_tools = memory::get_active_tools(&toolbus_tools);
        active_tools.extend(meta_tools);

        Ok(Self {
            session,
            locus_graph,
            llm_client,
            toolbus,
            event_tx,
            config,
            context_ids,
            active_tools,
            turn_sequence: 0,
            event_seq: 0,
            session_slug: String::new(),
            turn_slug: String::new(),
            repo_hash: repo_hash.clone(),
            project_name,
            turn_event_buffer: Vec::new(),
            graph_map,
        })
    }

    /// Create a sub-agent Runtime that shares ToolBus, LocusGraph, and LLM client
    /// with a parent Runtime.
    ///
    /// The sub-agent gets its own Session and event channel.
    pub async fn new_with_shared(
        config: RuntimeConfig,
        event_tx: mpsc::Sender<SessionEvent>,
        toolbus: Arc<ToolBus>,
        locus_graph: Arc<LocusGraphClient>,
        llm_client: Arc<dyn Provider>,
    ) -> Result<Self, RuntimeError> {
        let repo_hash = memory::simple_hash(config.repo_root.to_str().unwrap_or("unknown"));
        let project_name = config
            .repo_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();

        let session_config = SessionConfig::new(&config.model, config.provider.as_str())
            .with_max_turns(config.max_turns.unwrap_or(30))
            .with_sandbox_policy(config.sandbox.clone());

        let session = Session::new(config.repo_root.clone(), session_config);

        let context_ids = memory::build_context_ids(&project_name, &repo_hash, "", "", &[]);
        let mut active_tools = memory::get_active_tools(&toolbus.list_tools());
        active_tools.extend(locus_toolbus::meta_tool_definitions());

        Ok(Self {
            session,
            locus_graph,
            llm_client,
            toolbus,
            event_tx,
            config,
            context_ids,
            active_tools,
            turn_sequence: 0,
            event_seq: 0,
            session_slug: String::new(),
            turn_slug: String::new(),
            repo_hash: repo_hash.clone(),
            project_name,
            turn_event_buffer: Vec::new(),
            graph_map: String::new(),
        })
    }

    /// Create a new Runtime that continues from a previous session (same repo/config, new session id).
    /// Shares ToolBus, LocusGraph, and LLM client with the existing runtime. Use from CLI for "continue in new session".
    pub fn new_continuing(
        prev_session: &Session,
        config: RuntimeConfig,
        event_tx: mpsc::Sender<SessionEvent>,
        toolbus: Arc<ToolBus>,
        locus_graph: Arc<LocusGraphClient>,
        llm_client: Arc<dyn Provider>,
    ) -> Result<Self, RuntimeError> {
        let repo_hash = memory::simple_hash(config.repo_root.to_str().unwrap_or("unknown"));
        let project_name = config
            .repo_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();
        let session = Session::new_continuing(prev_session);

        let context_ids = memory::build_context_ids(&project_name, &repo_hash, "", "", &[]);
        let mut active_tools = memory::get_active_tools(&toolbus.list_tools());
        active_tools.extend(locus_toolbus::meta_tool_definitions());

        Ok(Self {
            session,
            locus_graph,
            llm_client,
            toolbus,
            event_tx,
            config,
            context_ids,
            active_tools,
            turn_sequence: 0,
            event_seq: 0,
            session_slug: String::new(),
            turn_slug: String::new(),
            repo_hash: repo_hash.clone(),
            project_name,
            turn_event_buffer: Vec::new(),
            graph_map: String::new(),
        })
    }

    /// Get the current turn_id as zero-padded string (e.g. "001").
    fn turn_id(&self) -> String {
        format!("{:03}", self.turn_sequence)
    }

    /// Get the current turn context_id (e.g. "turn:fix-jwt_validate-token").
    fn turn_ctx(&self) -> String {
        format!("turn:{}_{}", self.session_slug, self.turn_slug)
    }

    /// Add a turn context to the context_ids list for retrieval.
    fn add_turn_context(&mut self, turn_ctx: String) {
        if !self.context_ids.contains(&turn_ctx) {
            self.context_ids.push(turn_ctx);
        }
    }

    /// Increment event_seq and return the new value.
    fn next_seq(&mut self) -> u32 {
        self.event_seq += 1;
        self.event_seq
    }

    /// Buffer a turn event for later flush.
    fn buffer_event(&mut self, event: CreateEventRequest) {
        self.turn_event_buffer.push(event);
    }

    /// Flush buffered turn events asynchronously.
    async fn flush_turn_events(&mut self) {
        let events: Vec<CreateEventRequest> = self.turn_event_buffer.drain(..).collect();
        if events.is_empty() {
            return;
        }
        let locus_graph = Arc::clone(&self.locus_graph);
        tokio::spawn(async move {
            for event in events {
                let _ = locus_graph.store_event(event).await;
            }
        });
    }

    /// Return the first 8 chars of the session ID.
    fn short_session_id(&self) -> String {
        let id = self.session.id.as_str();
        if id.len() > 8 {
            id[..8].to_string()
        } else {
            id.to_string()
        }
    }

    /// Build an event context_id for the given type and sequence.
    fn event_ctx(&self, event_type: &str, seq: u32) -> String {
        format!(
            "{}:{}_{}_{}",
            event_type,
            self.short_session_id(),
            self.turn_id(),
            format!("{:03}", seq)
        )
    }

    /// Build a TurnSummary from the current round of activity.
    fn build_turn_summary(&self, user_message: &str) -> TurnSummary {
        let mut actions_taken = Vec::new();
        let mut files_read = Vec::new();
        let mut files_modified = Vec::new();
        let mut decisions = Vec::new();

        let mut turns_this_round: Vec<&Turn> = Vec::new();
        for turn in self.session.turns.iter().rev() {
            if turn.role == Role::User {
                break;
            }
            turns_this_round.push(turn);
        }
        turns_this_round.reverse();

        for turn in &turns_this_round {
            for block in &turn.blocks {
                if let ContentBlock::ToolUse { tool_use } = block {
                    actions_taken.push(format!("{}()", tool_use.name));
                    if let Some(path) = tool_use.args.get("path").and_then(|v| v.as_str()) {
                        match tool_use.name.as_str() {
                            "edit_file" | "create_file" => {
                                if !files_modified.iter().any(|p| p == path) {
                                    files_modified.push(path.to_string());
                                }
                            }
                            "glob" | "grep" | "finder" => {
                                if !files_read.iter().any(|p| p == path) {
                                    files_read.push(path.to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                } else if let ContentBlock::Text { text } = block {
                    if turn.role == Role::Assistant {
                        if let Some(first_line) = text.lines().next() {
                            let trimmed = first_line.trim();
                            if !trimmed.is_empty() && trimmed.len() < 200 {
                                decisions.push(trimmed.to_string());
                            }
                        }
                    }
                }
            }
        }

        let title = if user_message.chars().count() > 80 {
            let truncated: String = user_message.chars().take(77).collect();
            format!("{}...", truncated)
        } else {
            user_message.to_string()
        };

        let user_request = if user_message.chars().count() > 500 {
            let truncated: String = user_message.chars().take(497).collect();
            format!("{}...", truncated)
        } else {
            user_message.to_string()
        };

        let outcome = self
            .session
            .turns
            .iter()
            .rev()
            .find(|t| t.role == Role::Assistant)
            .and_then(|t| {
                t.blocks.iter().find_map(|b| {
                    if let ContentBlock::Text { text } = b {
                        let trimmed = text.trim();
                        if trimmed.is_empty() {
                            return None;
                        }
                        if trimmed.chars().count() > 200 {
                            let truncated: String = trimmed.chars().take(197).collect();
                            Some(format!("{}...", truncated))
                        } else {
                            Some(trimmed.to_string())
                        }
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| "Turn completed".to_string());

        TurnSummary {
            title,
            user_request,
            actions_taken,
            outcome,
            decisions,
            files_read,
            files_modified,
            event_count: self.event_seq,
        }
    }

    /// Generate a slug from user message (kebab-case, max 30 chars).
    fn slugify(message: &str) -> String {
        let slug: String = message
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == ' ' {
                    c
                } else {
                    ' '
                }
            })
            .collect::<String>()
            .split_whitespace()
            .take(6)
            .collect::<Vec<_>>()
            .join("-");
        if slug.len() < 4 {
            format!("session-{}", &slug)
        } else if slug.len() > 30 {
            slug[..30].trim_end_matches('-').to_string()
        } else {
            slug
        }
    }

    /// Generate a short turn slug from user message (kebab-case, max 20 chars).
    fn slugify_turn(message: &str) -> String {
        let slug: String = message
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == ' ' {
                    c
                } else {
                    ' '
                }
            })
            .collect::<String>()
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>()
            .join("-");
        if slug.len() < 3 {
            format!("turn-{}", &slug)
        } else if slug.len() > 20 {
            slug[..20].trim_end_matches('-').to_string()
        } else {
            slug
        }
    }

    /// Create an LLM provider based on configuration.
    fn create_provider(provider: &LlmProvider) -> Result<Arc<dyn Provider>, RuntimeError> {
        match provider {
            LlmProvider::Anthropic => {
                let anthropic = AnthropicProvider::from_env()
                    .map_err(|e| RuntimeError::ProviderNotFound(format!("Anthropic: {}", e)))?;
                Ok(Arc::new(anthropic))
            }
            LlmProvider::ZAI => {
                let zai = ZaiProvider::from_env()
                    .map_err(|e| RuntimeError::ProviderNotFound(format!("ZAI: {}", e)))?;
                Ok(Arc::new(zai))
            }
            LlmProvider::OpenAI => Err(RuntimeError::ProviderNotFound(
                "OpenAI provider is not yet implemented. Use 'anthropic' or 'zai' provider instead. \
                 OpenAI support is planned for a future release.".to_string(),
            )),
            LlmProvider::Ollama => Err(RuntimeError::ProviderNotFound(
                "Ollama provider is not yet implemented. Use 'anthropic' or 'zai' provider instead. \
                 Ollama support is planned for a future release.".to_string(),
            )),
        }
    }

    /// Graceful shutdown.
    ///
    /// Sets session status to completed and flushes any pending operations.
    pub async fn shutdown(&mut self) -> Result<(), RuntimeError> {
        info!("Shutting down runtime");

        // Close session in LocusGraph
        if !self.session_slug.is_empty() {
            memory::store_session_end(
                &self.locus_graph,
                &self.project_name,
                &self.repo_hash,
                &self.session_slug,
                self.session.id.as_str(),
                self.turn_sequence,
            )
            .await;
        }

        self.session.set_status(SessionStatus::Completed);
        let _ = self
            .event_tx
            .send(SessionEvent::status("Session ended"))
            .await;
        Ok(())
    }

    /// Get the current task description.
    pub fn current_task(&self) -> String {
        self.session
            .turns
            .iter()
            .find(|t| t.role == Role::User)
            .and_then(|t| {
                t.blocks.iter().find_map(|b| {
                    if let ContentBlock::Text { text } = b {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| "No active task".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_config() -> RuntimeConfig {
        RuntimeConfig::new(PathBuf::from("/test/repo"))
            .with_model("test-model")
            .with_provider(LlmProvider::Anthropic)
            .with_max_turns(10)
    }

    #[test]
    fn test_summarize_intent_short() {
        let _config = test_config();
        let (_tx, _rx) = mpsc::channel::<SessionEvent>(100);
        let message = "Hello world";
        let trimmed = message.trim();
        let summary = if trimmed.len() > 100 {
            format!("{}...", &trimmed[..97])
        } else {
            trimmed.to_string()
        };
        assert_eq!(summary, "Hello world");
    }

    #[test]
    fn test_summarize_intent_long() {
        let message = "x".repeat(150);
        let trimmed = message.trim();
        let summary = if trimmed.len() > 100 {
            format!("{}...", &trimmed[..97])
        } else {
            trimmed.to_string()
        };
        assert!(summary.ends_with("..."));
        assert_eq!(summary.len(), 100);
    }

    #[test]
    fn test_summarize_intent_sentence() {
        let message = "This is a complete sentence. And more text follows.";
        let trimmed = message.trim();
        let summary = if let Some(dot_pos) = trimmed.find('.') {
            if dot_pos < 100 {
                trimmed[..=dot_pos].to_string()
            } else if trimmed.len() > 100 {
                format!("{}...", &trimmed[..97])
            } else {
                trimmed.to_string()
            }
        } else if trimmed.len() > 100 {
            format!("{}...", &trimmed[..97])
        } else {
            trimmed.to_string()
        };
        assert_eq!(summary, "This is a complete sentence.");
    }
}
