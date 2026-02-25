//! LLM streaming and response handling.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use futures::StreamExt;
use locusgraph_observability::{record_duration, record_error};
use locus_core::{
    ContentBlock, Role, SessionEvent, TokenUsage, ToolUse, Turn,
};
use locus_llms::types::{GenerateRequest, StreamEvent};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::error::RuntimeError;
use crate::memory;

use super::Runtime;

impl Runtime {
    /// Stream LLM response and handle events.
    ///
    /// This processes the streaming response from the LLM, emitting
    /// events to the TUI and collecting tool calls.
    /// If `cancel` is triggered, returns `Err(RuntimeError::Cancelled)`.
    pub(crate) async fn stream_llm_response(
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

        if tracing::enabled!(tracing::Level::DEBUG) {
            let req_body = serde_json::to_string_pretty(&request).unwrap_or_else(|_| format!("{:?}", request));
            tracing::debug!(
                target: "locus.trace",
                message = %format!("LLM request model={}\n{}", request.model, req_body)
            );
        }

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
        let mut tool_calls: HashMap<String, (String, String)> = HashMap::new();
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
                            tool_calls.insert(id, (name, arguments.to_string()));
                        }
                        StreamEvent::Finish { usage: u, reason } => {
                            if tracing::enabled!(tracing::Level::DEBUG) {
                                let usage_msg = serde_json::to_string_pretty(&u).unwrap_or_else(|_| format!("{:?}", u));
                                tracing::debug!(
                                    target: "locus.trace",
                                    message = %format!("LLM stream finished\n  reason={:?}\n  usage:\n{}", reason, usage_msg)
                                );
                            }
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
}
