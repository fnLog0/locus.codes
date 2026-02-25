//! Tool call execution and sub-agent task spawning.

use std::sync::Arc;
use std::time::Instant;

use locusgraph_observability::{agent_span, record_duration, record_error};
use locus_core::{
    ContentBlock, Role, SessionEvent, ToolResultData, ToolUse,
};
use locus_graph::{EventLinks, LocusGraphClient};
use locus_llms::Provider;
use locus_toolbus::ToolBus;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::config::RuntimeConfig;
use crate::error::RuntimeError;
use crate::memory;
use crate::tool_handler;

use super::Runtime;

impl Runtime {
    /// Execute a list of tool calls.
    /// Task tools run in parallel; all others run sequentially.
    pub(crate) async fn execute_tool_calls(&mut self, tool_uses: Vec<ToolUse>) -> Result<(), RuntimeError> {
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
}
