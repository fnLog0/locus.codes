//! Orchestrator (plan §0.7, §1.3): prompt → LLM → tool_calls → ToolBus → events.

use anyhow::Result;
use locus_core::{RuntimeEvent, SessionState};
use locus_llm::{CompletionRequest, ModeLimits, ModelClient, OllamaClient, OpenAIClient, PATCH_AGENT_SYSTEM, SecretDetector, tool_definitions};
use locus_toolbus::ToolBus;
use std::sync::Arc;
use tokio::sync::Mutex;

fn make_llm_client() -> Result<Arc<dyn ModelClient>> {
    // Default to OpenAI, use Ollama only if requested
    let use_ollama = std::env::var("LOCUS_LLM").as_deref() == Ok("ollama");

    if use_ollama {
        let base_url = std::env::var("OLLAMA_BASE_URL").ok();
        let model = std::env::var("OLLAMA_MODEL").ok();
        Ok(Arc::new(OllamaClient::new(base_url, model)))
    } else {
        let api_key = std::env::var("OPENAI_API_KEY").ok();
        let base_url = std::env::var("OPENAI_BASE_URL").ok();
        let model = std::env::var("OPENAI_MODEL").ok();
        let client = OpenAIClient::new(api_key, base_url, model)
            .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}. Set OPENAI_API_KEY or use LOCUS_LLM=ollama for Ollama", e))?;
        Ok(Arc::new(client))
    }
}

pub async fn run_loop(
    session: SessionState,
    event_tx: locus_core::EventTx,
    toolbus: ToolBus,
    mut prompt_rx: tokio::sync::mpsc::Receiver<String>,
) {
    let client = match make_llm_client() {
        Ok(c) => c,
        Err(e) => {
            // Send error as a task failure and exit
            let _ = event_tx.send(RuntimeEvent::TaskFailed {
                task_id: "init_failed".to_string(),
                error: e.to_string(),
                step: None,
            });
            return;
        }
    };
    let session = Arc::new(Mutex::new(session));

    while let Some(prompt) = prompt_rx.recv().await {
        // Handle special commands
        if let Some(command) = prompt.strip_prefix(':') {
            if command.starts_with("mode ") {
                let mode_str = command.strip_prefix("mode ").unwrap_or("");
                let new_mode = match mode_str.to_lowercase().as_str() {
                    "rush" => Some(locus_core::Mode::Rush),
                    "smart" => Some(locus_core::Mode::Smart),
                    "deep" => Some(locus_core::Mode::Deep),
                    _ => None,
                };

                if let Some(new_mode) = new_mode {
                    let mut session_guard = session.lock().await;
                    let old_mode = session_guard.mode;
                    if old_mode != new_mode {
                        session_guard.mode = new_mode;
                        let _ = event_tx.send(RuntimeEvent::ModeChanged { old_mode, new_mode });
                    }
                }
                continue;
            } else if command == "cancel" {
                // Cancel current task (TODO: implement proper cancel handling)
                let _ = event_tx.send(RuntimeEvent::TaskFailed {
                    task_id: "cancelled".to_string(),
                    error: "Task cancelled by user".to_string(),
                    step: None,
                });
                continue;
            }
        }

        // Regular task execution
        let task_id = uuid::Uuid::new_v4().to_string();
        {
            let session_guard = session.lock().await;
            let _ = event_tx.send(RuntimeEvent::TaskStarted {
                task_id: task_id.clone(),
                prompt: prompt.clone(),
                mode: session_guard.mode,
            });
        }

        let start = std::time::Instant::now();
        let result = run_task(&client, &event_tx, &toolbus, &session, &prompt).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(summary) => {
                let _ = event_tx.send(RuntimeEvent::TaskCompleted {
                    task_id,
                    summary,
                    duration_ms,
                });
            }
            Err(e) => {
                let _ = event_tx.send(RuntimeEvent::TaskFailed {
                    task_id,
                    error: e.to_string(),
                    step: None,
                });
            }
        }
    }
}

async fn run_task(
    client: &Arc<dyn ModelClient>,
    event_tx: &locus_core::EventTx,
    toolbus: &ToolBus,
    session: &Arc<Mutex<SessionState>>,
    user_prompt: &str,
) -> anyhow::Result<String> {
    // Get mode from session for limits
    let session_guard = session.lock().await;
    let mode = session_guard.mode;
    let limits = ModeLimits::for_mode(mode);
    drop(session_guard);

    // Secrets detection (Phase 1.2)
    let secret_detector = SecretDetector::new();
    if secret_detector.has_secrets(user_prompt) {
        anyhow::bail!("User prompt contains sensitive data (API keys, passwords, etc.). Please redact before continuing.");
    }

    let request = CompletionRequest {
        system_prompt: PATCH_AGENT_SYSTEM.to_string(),
        memory_bundle: String::new(), // Will be populated in Phase 4
        tool_definitions: tool_definitions(),
        user_prompt: user_prompt.to_string(),
        limits,
    };

    let response = client.complete(request).await?;

    for tc in &response.tool_calls {
        let _ = event_tx.send(RuntimeEvent::ToolCalled {
            tool: tc.tool.clone(),
            args: tc.args.clone(),
            agent_id: None,
        });
        let start = std::time::Instant::now();
        let result = toolbus.call(&tc.tool, tc.args.clone()).await;
        let duration_ms = start.elapsed().as_millis() as u64;
        match &result {
            Ok((res, _)) => {
                // Sanitize tool results for secrets (Phase 1.2)
                let sanitized_res = if secret_detector.has_secrets(&res.to_string()) {
                    serde_json::json!({ "result": "[REDACTED - contains sensitive data]" })
                } else {
                    res.clone()
                };

                let _ = event_tx.send(RuntimeEvent::ToolResult {
                    tool: tc.tool.clone(),
                    success: true,
                    result: sanitized_res,
                    duration_ms,
                });
            }
            Err(e) => {
                let _ = event_tx.send(RuntimeEvent::ToolResult {
                    tool: tc.tool.clone(),
                    success: false,
                    result: serde_json::json!({ "error": e.to_string() }),
                    duration_ms,
                });
            }
        }
        result?;
    }

    Ok(response.reasoning)
}
