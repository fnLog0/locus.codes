//! Agent loop, message processing, and LLM call preparation.

use std::sync::Arc;
use std::time::Instant;

use locusgraph_observability::{agent_span, record_error};
use locus_core::{
    ContentBlock, Role, SessionEvent, SessionStatus, Turn,
};
use locus_llms::types::GenerateRequest;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::context::{self, near_context_limit};
use crate::error::RuntimeError;
use crate::memory;

use super::Runtime;

impl Runtime {
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
                self.session.set_status(SessionStatus::Waiting);
                break;
            }

            // We have pending tool results - process them by calling LLM again
            info!("Processing pending tool results");
            self.process_tool_results().await?;
        }

        Ok(self.session.status.clone())
    }

    /// Prepare a complete LLM request: recall memories, check context limits,
    /// build prompt + messages, assemble GenerateRequest.
    ///
    /// Centralizes the shared pipeline used by both `process_message` and
    /// `process_tool_results` to avoid duplication.
    pub(crate) async fn prepare_llm_call(&mut self, query: &str) -> Result<GenerateRequest, RuntimeError> {
        // Recall memories
        let memory_result = memory::recall_memories(
            &self.locus_graph,
            &self.event_tx,
            query,
            self.config.memory_limit,
            &self.context_ids,
        )
        .await;

        // Compress context if approaching limit
        if near_context_limit(&self.session, self.config.context_limit) {
            context::compress_context(&self.locus_graph, &mut self.session, &self.event_tx).await?;
        }

        // Build request from cached tools
        let system_prompt = context::build_system_prompt(&self.active_tools);
        let messages =
            context::build_messages(&system_prompt, &self.session, &memory_result.memories);

        Ok(context::build_generate_request(
            &self.config.model,
            messages,
            &self.active_tools,
            self.config.max_tokens,
        ))
    }

    /// Process pending tool results by calling the LLM.
    async fn process_tool_results(&mut self) -> Result<(), RuntimeError> {
        let query = self.last_user_message().unwrap_or_default();
        let request = self.prepare_llm_call(&query).await?;

        self.stream_llm_response(request, None).await?;

        memory::store_decision(
            Arc::clone(&self.locus_graph),
            "Processed tool results and continued reasoning".to_string(),
            None,
        );

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

        // Build and stream LLM request
        let request = self.prepare_llm_call(&message).await?;
        if let Err(e) = self.stream_llm_response(request, cancel).await {
            record_error(&e);
            return Err(e);
        }

        // Emit turn end
        let _ = self.event_tx.send(SessionEvent::turn_end()).await;

        Ok(())
    }

    /// Check if there are pending tool results that need processing.
    fn has_pending_tool_results(&self) -> bool {
        if let Some(last_turn) = self.session.turns.last() {
            matches!(last_turn.role, Role::Tool)
        } else {
            false
        }
    }

    /// Summarize user intent for memory storage.
    pub(crate) fn summarize_intent(&self, message: &str) -> String {
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
}
