//! `locus graph` subcommands (cache and event queue).
//!
//! Cache/queue DB path: `LOCUSGRAPH_DB_PATH` env, or `~/.locus/locus_graph_cache.db`, or
//! `$TMPDIR/locus_graph_cache.db`. Use a project-local path by setting e.g.
//! `LOCUSGRAPH_DB_PATH=.locus/locus_graph_cache.db`.

use std::fs;

use anyhow::Result;
use locus_graph::default_db_path;

use crate::cli::GraphAction;
use crate::output;

pub async fn handle(action: GraphAction) -> Result<()> {
    match action {
        GraphAction::ClearQueue => clear_queue().await,
        GraphAction::Clean => clean_cache().await,
    }
}

/// Clear the event queue and cache (same as clean; kept for backward compatibility).
async fn clear_queue() -> Result<()> {
    clean_cache().await
}

/// Remove the LocusGraph cache/queue DB so old failing events stop retrying and cache is fresh.
async fn clean_cache() -> Result<()> {
    let path = default_db_path();
    if path.exists() {
        fs::remove_file(&path)?;
        output::success(&format!(
            "Removed {} (cache and proxy queue cleared).",
            path.display()
        ));
    } else {
        output::dim(&format!(
            "No file at {} (cache already empty).",
            path.display()
        ));
    }
    Ok(())
}
