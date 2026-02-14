//! locus-runtime — orchestrator, event bus, mode (plan §0). Uses locus-core for shared types.

pub mod orchestrator;

pub use locus_core::{detect_repo_root, event_bus, EventRx, EventTx, Mode, RuntimeEvent, SessionState};

use anyhow::Result;
use locus_ui::run_ui;
use std::io::IsTerminal;
use std::path::PathBuf;

/// Boot: resolve repo, create session and event bus, spawn orchestrator + event bridge, run TUI.
pub fn run_app(mode: Mode, repo: Option<PathBuf>) -> Result<()> {
    if !std::io::stdout().is_terminal() {
        anyhow::bail!(
            "locus run requires an interactive terminal (TTY). \
             Run from a terminal (e.g. Terminal.app, iTerm): cargo run --bin locus -- run"
        );
    }
    let repo_root = detect_repo_root(repo)?;
    let session = SessionState::new(repo_root.clone(), mode);
    let (event_tx, event_rx) = event_bus();
    let (prompt_tx, prompt_rx) = tokio::sync::mpsc::channel(32);
    let (ui_ev_tx, ui_ev_rx) = std::sync::mpsc::channel();
    let toolbus = locus_toolbus::ToolBus::new(repo_root);

    let event_tx_loop = event_tx.clone();
    let session_loop = session.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            tokio::spawn(orchestrator::run_loop(
                session_loop,
                event_tx_loop,
                toolbus,
                prompt_rx,
            ));
            let mut rx = event_rx;
            while let Ok(ev) = rx.recv().await {
                let _ = ui_ev_tx.send(ev);
            }
        });
    });

    run_ui(session, event_tx, ui_ev_rx, prompt_tx)?;
    Ok(())
}
