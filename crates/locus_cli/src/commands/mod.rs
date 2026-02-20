//! Command dispatch.

pub mod config;
pub mod providers;
pub mod run;
pub mod toolbus;

use crate::cli::{Cli, Command};
use anyhow::Result;

pub async fn handle(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Toolbus { action } => toolbus::handle(action).await,
        Command::Providers { action } => providers::handle(action).await,
        Command::Config { action } => config::handle(action).await,
        Command::Run { model, provider, workdir, max_turns, max_tokens, prompt } => {
            run::handle(model, provider, workdir, max_turns, max_tokens, prompt).await
        }
    }
}
