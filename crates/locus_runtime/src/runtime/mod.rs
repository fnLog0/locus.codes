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

use locus_core::{
    ContentBlock, Role, Session, SessionConfig, SessionEvent, SessionStatus,
};
use locus_graph::{EventLinks, LocusGraphClient, LocusGraphConfig};
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
    /// Cached context IDs for memory queries (stable per session)
    context_ids: Vec<String>,
    /// Cached active tools (stable per runtime lifetime)
    active_tools: Vec<ToolInfo>,
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
        // Create repo hash for context IDs
        let repo_hash = memory::simple_hash(config.repo_root.to_str().unwrap_or("unknown"));

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

        // Register tool schemas in LocusGraph for discovery (fire-and-forget)
        let graph_for_register = Arc::clone(&locus_graph);
        let tools_to_register = toolbus.list_tools();
        tokio::spawn(async move {
            for tool in tools_to_register {
                graph_for_register
                    .store_tool_schema(
                        &tool.name,
                        &tool.description,
                        &tool.parameters,
                        "toolbus",
                        vec!["core"],
                        EventLinks::default(),
                    )
                    .await;
            }
        });

        // Create session
        let session_config = SessionConfig::new(&config.model, config.provider.as_str())
            .with_max_turns(config.max_turns.unwrap_or(0))
            .with_sandbox_policy(config.sandbox.clone());

        let session = Session::new(config.repo_root.clone(), session_config);

        // Cache context IDs and active tools (stable per session)
        let context_ids = memory::build_context_ids(&repo_hash, &session.id);
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

        let session_config = SessionConfig::new(&config.model, config.provider.as_str())
            .with_max_turns(config.max_turns.unwrap_or(30))
            .with_sandbox_policy(config.sandbox.clone());

        let session = Session::new(config.repo_root.clone(), session_config);

        let context_ids = memory::build_context_ids(&repo_hash, &session.id);
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
        let session = Session::new_continuing(prev_session);

        let context_ids = memory::build_context_ids(&repo_hash, &session.id);
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
        })
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
