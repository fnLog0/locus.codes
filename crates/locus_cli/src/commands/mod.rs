//! Command dispatch.

pub mod config;
pub mod graph;
pub mod mcp;
pub mod providers;
pub mod run;
pub mod toolbus;
pub mod tui;

use crate::cli::{Cli, Command};
use anyhow::Result;

pub async fn handle(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Tui { workdir, provider, model, onboarding } => tui::handle(workdir, provider, model, onboarding).await,
        Command::Toolbus { action } => toolbus::handle(action).await,
        Command::Providers { action } => providers::handle(action).await,
        Command::Config { action } => config::handle(action).await,
        Command::Graph { action } => graph::handle(action).await,
        Command::Mcp { action } => mcp::handle(action).await,
        Command::Run {
            model,
            provider,
            workdir,
            max_turns,
            max_tokens,
            prompt,
        } => run::handle(model, provider, workdir, max_turns, max_tokens, prompt).await,
    }
}
