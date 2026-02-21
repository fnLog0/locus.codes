//! `locus graph` subcommands (cache and event queue).

use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use crate::cli::GraphAction;
use crate::output;

/// Default DB path used by LocusGraphConfig::from_env() (cache + proxy event queue).
fn default_db_path() -> PathBuf {
    env::temp_dir().join("locus_graph_cache.db")
}

pub async fn handle(action: GraphAction) -> Result<()> {
    match action {
        GraphAction::ClearQueue => clear_queue().await,
    }
}

async fn clear_queue() -> Result<()> {
    let path = default_db_path();
    if path.exists() {
        fs::remove_file(&path)?;
        output::success(&format!("Removed {} (proxy queue and cache cleared).", path.display()));
    } else {
        output::dim(&format!("No file at {} (queue already empty).", path.display()));
    }
    Ok(())
}
