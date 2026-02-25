//! `locus tui` — run the interactive TUI with runtime integration.

use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use locus_runtime::{LlmProvider, Runtime, RuntimeConfig};
use locusgraph_observability::{init, ObservabilityConfig};
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;

use locus_core::SessionEvent;
use locus_tui::run_tui_with_runtime;

use crate::output;

async fn run_runtime_loop(
    config: RuntimeConfig,
    event_tx: mpsc::Sender<SessionEvent>,
    mut user_msg_rx: mpsc::Receiver<String>,
    mut new_session_rx: mpsc::Receiver<()>,
    mut cancel_rx: mpsc::Receiver<()>,
) {
    let current_cancel_token: Arc<RwLock<Option<CancellationToken>>> = Arc::new(RwLock::new(None));
    let token_guard = Arc::clone(&current_cancel_token);
    tokio::spawn(async move {
        while cancel_rx.recv().await.is_some() {
            if let Some(t) = token_guard.write().await.take() {
                t.cancel();
            }
        }
    });

    let mut runtime_opt: Option<Runtime> = None;
    loop {
        tokio::select! {
            msg = user_msg_rx.recv() => {
                let msg = match msg {
                    Some(m) => m,
                    None => break,
                };
                let mut rt = match runtime_opt.take() {
                    None => match Runtime::new(config.clone(), event_tx.clone()).await {
                        Ok(r) => r,
                        Err(e) => {
                            output::error(&format!("Runtime failed to start: {}", e));
                            continue;
                        }
                    },
                    Some(prev) => {
                        let toolbus = std::sync::Arc::clone(&prev.toolbus);
                        let locus_graph = std::sync::Arc::clone(&prev.locus_graph);
                        let llm_client = std::sync::Arc::clone(&prev.llm_client);
                        match Runtime::new_continuing(
                            &prev.session,
                            config.clone(),
                            event_tx.clone(),
                            toolbus,
                            locus_graph,
                            llm_client,
                        ) {
                            Ok(r) => r,
                            Err(e) => {
                                output::error(&format!("Runtime continue failed: {}", e));
                                runtime_opt = Some(prev);
                                continue;
                            }
                        }
                    }
                };
                let token = CancellationToken::new();
                *current_cancel_token.write().await = Some(token.clone());
                if let Err(e) = rt.run(msg, Some(token)).await {
                    // Runtime already sends SessionEvent::error + turn_end to TUI; also log to stderr
                    output::error(&format!("Run failed: {}", e));
                }
                *current_cancel_token.write().await = None;
                if let Err(e) = rt.shutdown().await {
                    output::warning(&format!("Runtime shutdown: {}", e));
                }
                runtime_opt = Some(rt);
            }
            _ = new_session_rx.recv() => {
                runtime_opt = None;
            }
        }
    }
}

pub async fn handle(
    workdir: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    onboarding: bool,
) -> Result<()> {
    let repo_root = workdir
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Channel for runtime logs → TUI debug traces screen (Ctrl+D)
    let (log_tx, log_rx) = mpsc::channel::<String>(512);
    let log_sink: Arc<dyn Fn(String) + Send + Sync> =
        Arc::new(move |line| { let _ = log_tx.try_send(line); });

    // Init tracing without console; send logs to TUI sink. Include locus.trace=debug so
    // LocusGraph, LLM, and tool traces show in the Runtime logs screen (Ctrl+D).
    let mut obs_config = ObservabilityConfig::from_env().with_console(false).with_log_sink(log_sink);
    if obs_config.log_level.is_none() {
        obs_config = obs_config.with_log_level("info,locus.trace=debug");
    }
    if let Err(e) = init(obs_config) {
        output::warning(&format!("Observability init failed (continuing): {}", e));
    }

    let mut config = RuntimeConfig::from_env(repo_root);
    if let Some(p) = provider.as_deref() {
        if let Ok(prov) = p.parse::<LlmProvider>() {
            config = config.with_provider(prov);
        }
    }
    if let Some(m) = model {
        config = config.with_model(m);
    }
    let (event_tx, event_rx) = mpsc::channel(256);
    let (user_msg_tx, user_msg_rx) = mpsc::channel::<String>(64);
    let (new_session_tx, new_session_rx) = mpsc::channel::<()>(4);
    let (cancel_tx, cancel_rx) = mpsc::channel::<()>(4);

    // Show onboarding when no LLM key is set, or when user passes --onboarding
    let show_onboarding = onboarding || !has_any_llm_key();

    tokio::spawn(run_runtime_loop(config, event_tx, user_msg_rx, new_session_rx, cancel_rx));

    run_tui_with_runtime(
        event_rx,
        user_msg_tx,
        Some(log_rx),
        Some(new_session_tx),
        Some(cancel_tx),
        show_onboarding,
    )?;
    Ok(())
}

/// True if at least one LLM provider API key is set and non-empty (so the agent can run).
fn has_any_llm_key() -> bool {
    let non_empty = |v: Result<String, _>| v.map(|s| !s.trim().is_empty()).unwrap_or(false);
    non_empty(env::var("ANTHROPIC_API_KEY"))
        || non_empty(env::var("ZAI_API_KEY"))
        || non_empty(env::var("OPENAI_API_KEY"))
}
