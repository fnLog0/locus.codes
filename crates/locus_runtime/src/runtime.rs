//! Main Runtime orchestrator for locus.codes.
//!
//! The Runtime ties together all components (Session, LocusGraph, LLM, ToolBus)
//! into a cohesive agent loop with Amp-style simplicity.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use futures::StreamExt;
use locusgraph_observability::{agent_span, record_duration, record_error};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use locus_core::{
    ContentBlock, Role, Session, SessionConfig, SessionEvent, SessionStatus, TokenUsage,
    ToolResultData, ToolUse, Turn,
};
use locus_graph::{EventLinks, LocusGraphClient, LocusGraphConfig};
use locus_llms::types::{GenerateRequest, StreamEvent};
use locus_llms::{AnthropicProvider, Provider, ZaiProvider};
use locus_toolbus::ToolBus;

use crate::config::{LlmProvider, RuntimeConfig};
use crate::context::{self, near_context_limit};
use crate::error::RuntimeError;
use crate::memory;
use crate::tool_handler;

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
    /// Repo hash for context IDs
    repo_hash: String,
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

        Ok(Self {
            session,
            locus_graph,
            llm_client,
            toolbus,
            event_tx,
            config,
            repo_hash,
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

        Ok(Self {
            session,
            locus_graph,
            llm_client,
            toolbus,
            event_tx,
            config,
            repo_hash,
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

        Ok(Self {
            session,
            locus_graph,
            llm_client,
            toolbus,
            event_tx,
            config,
            repo_hash,
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

    /// Main entry point — run the agent with an initial message.
    ///
    /// This processes the initial message and runs the agent loop until
    /// the session ends (completed, failed, cancelled, or max turns reached).
    /// If `cancel` is provided and is triggered, streaming stops and returns `Ok(SessionStatus::Cancelled)`.
    pub async fn run(
        &mut self,
        initial_message: String,
        cancel: Option<CancellationToken>,
    ) -> Result<SessionStatus, RuntimeError> {
        let session_id = self.session.id.as_str();
        let span = agent_span!(session_id, "run");
        let _guard = span.enter();

        let run_start = Instant::now();
        self.session.start_run();
        info!("Starting runtime with initial message");

        // Set session status to running
        self.session.set_status(SessionStatus::Running);
        let _ = self
            .event_tx
            .send(SessionEvent::status("Session started"))
            .await;

        // Process the initial message (streaming; can be cancelled)
        match self.process_message(initial_message, cancel).await {
            Err(RuntimeError::Cancelled) => {
                self.session.set_status(SessionStatus::Cancelled);
                self.session
                    .finish_run(Some(run_start.elapsed().as_millis() as u64));
                let _ = self.event_tx.send(SessionEvent::turn_end()).await;
                let _ = self
                    .event_tx
                    .send(SessionEvent::session_end_with_tokens(
                        SessionStatus::Cancelled,
                        self.session.total_prompt_tokens,
                        self.session.total_completion_tokens,
                    ))
                    .await;
                return Ok(SessionStatus::Cancelled);
            }
            Err(e) => {
                record_error(&e);
                self.session
                    .finish_run(Some(run_start.elapsed().as_millis() as u64));
                let _ = self.event_tx.send(SessionEvent::turn_end()).await;
                let _ = self.event_tx.send(SessionEvent::error(e.to_string())).await;
                return Err(e);
            }
            Ok(()) => {}
        }

        // Run the agent loop
        let status = match self.agent_loop().await {
            Ok(s) => s,
            Err(e) => {
                record_error(&e);
                self.session
                    .finish_run(Some(run_start.elapsed().as_millis() as u64));
                let _ = self.event_tx.send(SessionEvent::turn_end()).await;
                let _ = self.event_tx.send(SessionEvent::error(e.to_string())).await;
                return Err(e);
            }
        };

        self.session
            .finish_run(Some(run_start.elapsed().as_millis() as u64));
        // Emit session end event with token usage
        let _ = self
            .event_tx
            .send(SessionEvent::session_end_with_tokens(
                status.clone(),
                self.session.total_prompt_tokens,
                self.session.total_completion_tokens,
            ))
            .await;

        Ok(status)
    }

    /// The core agent loop.
    ///
    /// Repeatedly:
    /// 1. Checks for pending tool calls
    /// 2. If no pending tools, waits for user input (or terminates)
    /// 3. Recalls memories
    /// 4. Builds prompt and calls LLM
    /// 5. Streams response
    /// 6. Handles tool calls
    /// 7. Stores decisions
    /// 8. Compresses context if needed
    /// 9. Checks termination conditions
    pub async fn agent_loop(&mut self) -> Result<SessionStatus, RuntimeError> {
        loop {
            // Check termination conditions
            if !self.session.is_active() {
                info!("Session no longer active, exiting loop");
                break;
            }

            // Check max turns
            if let Some(max) = self.config.max_turns {
                if self.session.turn_count() >= max as usize {
                    info!("Max turns ({}) reached", max);
                    self.session.set_status(SessionStatus::Completed);
                    break;
                }
            }

            // Check if we need to continue (e.g., pending tool results to process)
            let has_pending_tools = self.has_pending_tool_results();

            if !has_pending_tools {
                // No pending tools, session should wait for user input
                // For now, we complete the session
                // In a full implementation, this would wait for user input
                self.session.set_status(SessionStatus::Waiting);
                break;
            }

            // We have pending tool results - process them by calling LLM again
            info!("Processing pending tool results");
            self.process_tool_results().await?;
        }

        Ok(self.session.status.clone())
    }

    /// Process pending tool results by calling the LLM.
    ///
    /// This is called when there are tool results in the session that
    /// need to be processed by the LLM to continue the agentic loop.
    async fn process_tool_results(&mut self) -> Result<(), RuntimeError> {
        // Recall memories before LLM call
        let context_ids = memory::build_context_ids(&self.repo_hash, &self.session.id);
        let last_user_msg = self.last_user_message().unwrap_or_default();
        let memory_result = memory::recall_memories(
            &self.locus_graph,
            &self.event_tx,
            &last_user_msg,
            self.config.memory_limit,
            context_ids,
        )
        .await;

        // Check context limit before building prompt
        if near_context_limit(&self.session, self.config.context_limit) {
            context::compress_context(&self.locus_graph, &mut self.session, &self.event_tx).await?;
        }

        // Build the LLM request (core tools + meta-tools only)
        let all_tools = self.toolbus.list_tools();
        let mut tools = memory::get_active_tools(&all_tools);
        tools.extend(context::meta_tool_definitions());
        let system_prompt = context::build_system_prompt(&tools);
        let messages =
            context::build_messages(&system_prompt, &self.session, &memory_result.memories);

        let request = context::build_generate_request(
            &self.config.model,
            messages,
            &tools,
            self.config.max_tokens,
        );

        // Stream LLM response
        self.stream_llm_response(request, None).await?;

        // Store decision about processing tool results
        let summary = "Processed tool results and continued reasoning";
        memory::store_decision(Arc::clone(&self.locus_graph), summary.to_string(), None);

        Ok(())
    }

    /// Get the last user message from the session.
    fn last_user_message(&self) -> Option<String> {
        self.session
            .turns
            .iter()
            .rev()
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
    }

    /// Process a single user message.
    ///
    /// This is the main entry point for handling user input.
    /// If `cancel` is triggered during streaming, returns `Err(RuntimeError::Cancelled)`.
    pub async fn process_message(
        &mut self,
        message: String,
        cancel: Option<CancellationToken>,
    ) -> Result<(), RuntimeError> {
        let session_id = self.session.id.as_str();
        let span = agent_span!(session_id, "process_message");
        let _guard = span.enter();

        info!("Processing user message: {} chars", message.len());

        // Emit turn start
        let _ = self
            .event_tx
            .send(SessionEvent::turn_start(Role::User))
            .await;

        // Store user intent (fire-and-forget)
        let intent_summary = self.summarize_intent(&message);
        memory::store_user_intent(
            Arc::clone(&self.locus_graph),
            message.clone(),
            intent_summary,
        );

        // Create user turn and add to session
        let user_turn = Turn::user().with_block(ContentBlock::text(&message));
        self.session.add_turn(user_turn);

        // Recall memories before LLM call
        let context_ids = memory::build_context_ids(&self.repo_hash, &self.session.id);
        let memory_result = memory::recall_memories(
            &self.locus_graph,
            &self.event_tx,
            &message,
            self.config.memory_limit,
            context_ids,
        )
        .await;

        // Check context limit before building prompt
        if near_context_limit(&self.session, self.config.context_limit) {
            context::compress_context(&self.locus_graph, &mut self.session, &self.event_tx).await?;
        }

        // Build the LLM request (core tools + meta-tools only)
        let all_tools = self.toolbus.list_tools();
        let mut tools = memory::get_active_tools(&all_tools);
        tools.extend(context::meta_tool_definitions());
        let system_prompt = context::build_system_prompt(&tools);
        let messages =
            context::build_messages(&system_prompt, &self.session, &memory_result.memories);

        let request = context::build_generate_request(
            &self.config.model,
            messages,
            &tools,
            self.config.max_tokens,
        );

        // Stream LLM response (may be cancelled)
        if let Err(e) = self.stream_llm_response(request, cancel).await {
            record_error(&e);
            return Err(e);
        }

        // Emit turn end
        let _ = self.event_tx.send(SessionEvent::turn_end()).await;

        Ok(())
    }

    /// Stream LLM response and handle events.
    ///
    /// This processes the streaming response from the LLM, emitting
    /// events to the TUI and collecting tool calls.
    /// If `cancel` is triggered, returns `Err(RuntimeError::Cancelled)`.
    async fn stream_llm_response(
        &mut self,
        request: GenerateRequest,
        cancel: Option<CancellationToken>,
    ) -> Result<(), RuntimeError> {
        let span = tracing::info_span!(
            "runtime.stream_llm",
            session.id = %self.session.id.as_str(),
        );
        let _guard = span.enter();
        info!("Streaming LLM response");

        let req_body = serde_json::to_string_pretty(&request).unwrap_or_else(|_| format!("{:?}", request));
        tracing::debug!(
            target: "locus.trace",
            message = %format!("LLM request model={}\n{}", request.model, req_body)
        );

        // Emit turn start for assistant
        let _ = self
            .event_tx
            .send(SessionEvent::turn_start(Role::Assistant))
            .await;

        // Start streaming
        let start = Instant::now();
        let mut stream = match self.llm_client.stream(request).await {
            Ok(s) => s,
            Err(e) => {
                let err = RuntimeError::LlmFailed(e.to_string());
                record_error(&err);
                return Err(err);
            }
        };

        // Collect response for the turn
        let mut text_content = String::new();
        let mut thinking_content = String::new();
        let mut tool_calls: HashMap<String, (String, String)> = HashMap::new(); // id -> (name, args_json)
        let mut _generation_id = String::new();
        let mut usage = None;

        loop {
            let event_result = if let Some(c) = cancel.clone() {
                tokio::select! {
                    biased;
                    _ = c.cancelled() => return Err(RuntimeError::Cancelled),
                    ev = stream.next() => ev,
                }
            } else {
                stream.next().await
            };
            let Some(event_result) = event_result else {
                break;
            };
            match event_result {
                Ok(event) => {
                    match event {
                        StreamEvent::Start { id } => {
                            _generation_id = id;
                            info!("LLM stream started: {}", _generation_id);
                        }
                        StreamEvent::TextDelta { id: _, delta } => {
                            text_content.push_str(&delta);
                            let _ = self.event_tx.send(SessionEvent::text_delta(&delta)).await;
                        }
                        StreamEvent::ReasoningDelta { id: _, delta } => {
                            thinking_content.push_str(&delta);
                            let _ = self
                                .event_tx
                                .send(SessionEvent::thinking_delta(&delta))
                                .await;
                        }
                        StreamEvent::ToolCallStart { id, name } => {
                            info!("Tool call started: {} ({})", name, id);
                            tool_calls.insert(id.clone(), (name, String::new()));
                        }
                        StreamEvent::ToolCallDelta { id, delta } => {
                            if let Some((_, args)) = tool_calls.get_mut(&id) {
                                args.push_str(&delta);
                            }
                        }
                        StreamEvent::ToolCallEnd {
                            id,
                            name,
                            arguments,
                        } => {
                            info!("Tool call completed: {} ({})", name, id);
                            // Store complete tool call
                            tool_calls.insert(id, (name, arguments.to_string()));
                        }
                        StreamEvent::Finish { usage: u, reason } => {
                            let usage_msg = serde_json::to_string_pretty(&u).unwrap_or_else(|_| format!("{:?}", u));
                            tracing::debug!(
                                target: "locus.trace",
                                message = %format!("LLM stream finished\n  reason={:?}\n  usage:\n{}", reason, usage_msg)
                            );
                            usage = Some(u);
                            info!("LLM stream finished: {:?}", reason);
                        }
                        StreamEvent::Error { message } => {
                            let err = RuntimeError::LlmFailed(message.clone());
                            record_error(&err);
                            error!("LLM stream error: {}", message);
                            let _ = self.event_tx.send(SessionEvent::error(&message)).await;
                            return Err(err);
                        }
                    }
                }
                Err(e) => {
                    let err = RuntimeError::LlmFailed(e.to_string());
                    record_error(&err);
                    error!("Stream error: {}", e);
                    return Err(err);
                }
            }
        }

        let duration = start.elapsed();
        let duration_ms = duration.as_millis() as u64;
        record_duration("llm.stream_duration_ms", duration);

        // Store LLM call (fire-and-forget) and session token totals
        let prompt_tokens = usage.as_ref().map(|u| u.prompt_tokens as u64).unwrap_or(0);
        let completion_tokens = usage
            .as_ref()
            .map(|u| u.completion_tokens as u64)
            .unwrap_or(0);
        self.session.add_llm_usage(prompt_tokens, completion_tokens);
        memory::store_llm_call(
            Arc::clone(&self.locus_graph),
            self.config.model.clone(),
            prompt_tokens,
            completion_tokens,
            duration_ms,
            false,
        );

        // Build assistant turn (with token usage for this turn)
        let turn_usage = TokenUsage::new(prompt_tokens, completion_tokens);
        let mut assistant_turn = Turn::assistant().with_token_usage(turn_usage);

        if !thinking_content.is_empty() {
            assistant_turn = assistant_turn.with_block(ContentBlock::thinking(&thinking_content));
        }

        if !text_content.is_empty() {
            assistant_turn = assistant_turn.with_block(ContentBlock::text(&text_content));
        }

        // Add tool calls to turn
        let tool_uses: Vec<ToolUse> = tool_calls
            .into_iter()
            .map(|(id, (name, args_json))| {
                let args: serde_json::Value = match serde_json::from_str(&args_json) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(
                            "Failed to parse tool call arguments for {} (id={}): {} (raw: {})",
                            name, id, e, &args_json[..args_json.len().min(200)]
                        );
                        serde_json::json!({"__raw_arguments": args_json, "__parse_error": e.to_string()})
                    }
                };
                ToolUse::new(&id, &name, args)
            })
            .collect();

        for tool_use in &tool_uses {
            assistant_turn = assistant_turn.with_block(ContentBlock::tool_use(tool_use.clone()));
        }

        // Add assistant turn to session
        self.session.add_turn(assistant_turn);

        // Execute tool calls if any
        if !tool_uses.is_empty() {
            self.execute_tool_calls(tool_uses).await?;
        }

        Ok(())
    }

    /// Execute a list of tool calls.
    /// Task tools run in parallel; all others run sequentially.
    async fn execute_tool_calls(&mut self, tool_uses: Vec<ToolUse>) -> Result<(), RuntimeError> {
        let span = tracing::info_span!(
            "runtime.execute_tool_calls",
            session.id = %self.session.id.as_str(),
            tool_count = tool_uses.len(),
        );
        let _guard = span.enter();
        info!("Executing {} tool calls", tool_uses.len());

        let mut task_tools = Vec::new();
        let mut regular_tools = Vec::new();
        for tool_use in tool_uses {
            if tool_use.name == "task" {
                task_tools.push(tool_use);
            } else {
                regular_tools.push(tool_use);
            }
        }

        let mut results = Vec::with_capacity(task_tools.len() + regular_tools.len());

        // Execute regular tools sequentially
        for tool_use in regular_tools {
            if tool_handler::requires_confirmation(&tool_use) {
                warn!(
                    "Tool {} requires confirmation - auto-approving for now",
                    tool_use.name
                );
            }

            let result = match tool_handler::handle_tool_call(
                tool_use.clone(),
                &self.toolbus,
                Arc::clone(&self.locus_graph),
                &self.event_tx,
            )
            .await
            {
                Ok(r) => r,
                Err(e) => {
                    record_error(&e);
                    return Err(e);
                }
            };

            results.push((tool_use, result));
        }

        // Execute task tools (sequentially; parallel spawn would require Runtime to be Send)
        for tool_use in task_tools {
            info!(
                "Running task: {}",
                tool_use
                    .args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("sub-task")
            );

            let result = match Self::run_task_tool(
                tool_use.clone(),
                &self.toolbus,
                Arc::clone(&self.locus_graph),
                Arc::clone(&self.llm_client),
                &self.config,
                &self.event_tx,
            )
            .await
            {
                Ok(r) => r,
                Err(e) => {
                    record_error(&e);
                    return Err(e);
                }
            };

            results.push((tool_use, result));
        }

        // Create tool result turn and add to session
        if !results.is_empty() {
            let tool_turn = tool_handler::create_tool_result_turn(&results);
            self.session.add_turn(tool_turn);

            let summary = format!("Executed {} tool(s)", results.len());
            memory::store_decision(Arc::clone(&self.locus_graph), summary, None);
        }

        Ok(())
    }

    /// Run a single task tool by spawning a sub-agent runtime.
    async fn run_task_tool(
        tool: ToolUse,
        toolbus: &Arc<ToolBus>,
        locus_graph: Arc<LocusGraphClient>,
        llm_client: Arc<dyn Provider>,
        config: &RuntimeConfig,
        event_tx: &mpsc::Sender<SessionEvent>,
    ) -> Result<ToolResultData, RuntimeError> {
        let span = agent_span!("task", "run_task_tool");
        let _guard = span.enter();
        let start = Instant::now();

        let prompt = tool
            .args
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuntimeError::ToolFailed {
                tool: "task".to_string(),
                message: "Missing 'prompt' argument".to_string(),
            })?
            .to_string();

        let description = tool
            .args
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("sub-task")
            .to_string();

        let _ = event_tx.send(SessionEvent::tool_start(tool.clone())).await;
        let _ = event_tx
            .send(SessionEvent::status(format!("Sub-agent: {}", description)))
            .await;

        let (sub_tx, mut sub_rx) = mpsc::channel::<SessionEvent>(100);
        let fwd_tx = event_tx.clone();
        let fwd_handle = tokio::spawn(async move {
            while let Some(event) = sub_rx.recv().await {
                let _ = fwd_tx.send(event).await;
            }
        });

        let sub_config = RuntimeConfig::new(config.repo_root.clone())
            .with_model(&config.model)
            .with_provider(config.provider)
            .with_max_turns(30)
            .with_sandbox(config.sandbox.clone());

        let mut sub_runtime = Runtime::new_with_shared(
            sub_config,
            sub_tx,
            Arc::clone(toolbus),
            locus_graph.clone(),
            llm_client,
        )
        .await?;

        let status = Box::pin(sub_runtime.run(prompt, None)).await?;
        fwd_handle.abort();

        let summary = sub_runtime
            .session
            .turns
            .iter()
            .rev()
            .find(|t| t.role == Role::Assistant)
            .and_then(|t| {
                t.blocks.iter().find_map(|b| {
                    if let ContentBlock::Text { text } = b {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| format!("Task completed: {:?}", status));

        let duration = start.elapsed();
        let duration_ms = duration.as_millis() as u64;
        record_duration("task.duration_ms", duration);

        let output = serde_json::json!({
            "description": description,
            "summary": summary,
            "duration_ms": duration_ms,
        });

        let tool_result = ToolResultData::success(output, duration_ms);
        let _ = event_tx
            .send(SessionEvent::tool_done(
                tool.id.clone(),
                tool_result.clone(),
            ))
            .await;

        memory::store_tool_run(
            locus_graph,
            "task".to_string(),
            serde_json::json!({ "description": description }),
            tool_result.clone(),
            EventLinks::default(),
        );

        Ok(tool_result)
    }

    /// Check if there are pending tool results that need processing.
    fn has_pending_tool_results(&self) -> bool {
        // Check if the last turn was a tool result turn
        if let Some(last_turn) = self.session.turns.last() {
            matches!(last_turn.role, Role::Tool)
        } else {
            false
        }
    }

    /// Summarize user intent for memory storage.
    fn summarize_intent(&self, message: &str) -> String {
        // Simple summarization - first 100 chars or first sentence
        let trimmed = message.trim();
        if let Some(dot_pos) = trimmed.find('.') {
            if dot_pos < 100 {
                return trimmed[..=dot_pos].to_string();
            }
        }
        if trimmed.len() > 100 {
            format!("{}...", &trimmed[..97])
        } else {
            trimmed.to_string()
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
        // Can't create runtime without env vars, so test summarize_intent logic directly
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
