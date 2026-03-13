//! `locus tui` — run the interactive TUI with runtime integration.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use locus_runtime::{LlmProvider, Runtime, RuntimeConfig};
use locusgraph_observability::{ObservabilityConfig, init};
use tokio::sync::{RwLock, mpsc};
use tokio_util::sync::CancellationToken;

use locus_core::SessionEvent;
use locus_tui::run_tui_with_runtime;
use locus_tui::theme::Appearance;

use crate::output;

async fn run_runtime_loop(
    config: RuntimeConfig,
    provider_locked: bool,
    model_locked: bool,
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
                let active_config = refreshed_runtime_config(&config, provider_locked, model_locked);
                let mut rt = match runtime_opt.take() {
                    None => match Runtime::new(active_config.clone(), event_tx.clone()).await {
                        Ok(r) => r,
                        Err(e) => {
                            output::error(&format!("Runtime failed to start: {}", e));
                            continue;
                        }
                    },
                    Some(prev) => prev,
                };
                let token = CancellationToken::new();
                *current_cancel_token.write().await = Some(token.clone());
                if let Err(e) = rt.run(msg, Some(token)).await {
                    // Runtime already sends SessionEvent::error + turn_end to TUI; also log to stderr
                    output::error(&format!("Run failed: {}", e));
                }
                *current_cancel_token.write().await = None;
                runtime_opt = Some(rt);
            }
            new_session = new_session_rx.recv() => {
                match new_session {
                    Some(()) => {
                        if let Some(mut rt) = runtime_opt.take()
                            && let Err(e) = rt.shutdown().await
                        {
                            output::warning(&format!("Runtime shutdown: {}", e));
                        }
                    }
                    None => break,
                }
            }
        }
    }

    if let Some(mut rt) = runtime_opt
        && let Err(e) = rt.shutdown().await
    {
        output::warning(&format!("Runtime shutdown: {}", e));
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
    let log_sink: Arc<dyn Fn(String) + Send + Sync> = Arc::new(move |line| {
        let _ = log_tx.try_send(line);
    });

    // Init tracing without console; send logs to TUI sink. Include locus.trace=debug so
    // LocusGraph, LLM, and tool traces show in the Runtime logs screen (Ctrl+D).
    let mut obs_config = ObservabilityConfig::from_env()
        .with_console(false)
        .with_log_sink(log_sink);
    if obs_config.log_level.is_none() {
        obs_config = obs_config.with_log_level("info,locus.trace=debug");
    }
    if let Err(e) = init(obs_config) {
        output::warning(&format!("Observability init failed (continuing): {}", e));
    }

    let mut config = RuntimeConfig::from_env(repo_root);
    let provider_locked = provider.is_some();
    let model_locked = model.is_some();
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

    // Show setup when no LLM key is set, or when user passes --onboarding.
    let show_setup = onboarding || !has_any_llm_key();

    tokio::spawn(run_runtime_loop(
        config,
        provider_locked,
        model_locked,
        event_tx,
        user_msg_rx,
        new_session_rx,
        cancel_rx,
    ));

    run_tui_with_runtime(
        event_rx,
        user_msg_tx,
        Some(log_rx),
        Some(new_session_tx),
        Some(cancel_tx),
        Appearance::Dark,
        show_setup,
    )?;
    Ok(())
}

/// True if at least one LLM provider API key is saved in the global config DB.
///
/// We check the DB directly instead of env vars because `load_locus_config()` also
/// loads `.env` files, which would mask a DB reset. The setup wizard saves to the DB,
/// so this is the canonical source of truth for "has the user configured keys?"
fn has_any_llm_key() -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    let locus_dir = home.join(".locus");
    let Ok(conn) = locus_core::db::open_db_at(&locus_dir) else {
        return false;
    };
    let keys = ["ANTHROPIC_API_KEY", "ZAI_API_KEY", "OPENAI_API_KEY"];
    for key in keys {
        if let Ok(Some(val)) = locus_core::db::get_config_value(&conn, key) {
            let raw = val.trim().trim_matches('"');
            if !raw.is_empty() {
                return true;
            }
        }
    }
    false
}

fn refreshed_runtime_config(
    base: &RuntimeConfig,
    provider_locked: bool,
    model_locked: bool,
) -> RuntimeConfig {
    let mut refreshed = RuntimeConfig::from_env(base.repo_root.clone());
    refreshed.max_turns = base.max_turns;
    refreshed.context_limit = base.context_limit;
    refreshed.memory_limit = base.memory_limit;
    refreshed.tool_token_budget = base.tool_token_budget;
    refreshed.max_tokens = base.max_tokens;
    refreshed.sandbox = base.sandbox.clone();

    if provider_locked {
        refreshed = refreshed.with_provider(base.provider);
    }
    if model_locked {
        refreshed = refreshed.with_model(base.model.clone());
    }

    refreshed
}
