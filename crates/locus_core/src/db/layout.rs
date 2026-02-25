//! `.locus/` directory layout (Crush-style).
//!
//! - `locus.db` + WAL: main project DB (edit history, config, task list).
//! - `logs/`, `commands/`: subdirs for logs and command data.
//! - LocusGraph uses a separate `.locus/locus_graph_cache.db`.
//! - `env`: optional file synced from config table for `source .locus/env`.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Basename of the main project DB (SQLite creates .db-wal and .db-shm alongside).
pub const LOCUS_DB: &str = "locus.db";
/// Env file under locus dir (synced from config table).
pub const ENV_FILE: &str = "env";
/// Subdir for log files.
pub const LOGS_DIR: &str = "logs";
/// Subdir for command history / saved commands.
pub const COMMANDS_DIR: &str = "commands";

/// Ensures `locus_dir`, `locus_dir/logs`, `locus_dir/commands` exist; returns path to locus.db.
pub fn ensure_locus_dir_at(locus_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(locus_dir).context("create locus dir")?;
    std::fs::create_dir_all(locus_dir.join(LOGS_DIR)).context("create logs dir")?;
    std::fs::create_dir_all(locus_dir.join(COMMANDS_DIR)).context("create commands dir")?;
    Ok(locus_dir.join(LOCUS_DB))
}

/// Ensures `.locus`, `.locus/logs`, `.locus/commands` exist and returns path to locus.db.
pub fn ensure_locus_dir(repo_root: &Path) -> Result<PathBuf> {
    ensure_locus_dir_at(&repo_root.join(".locus"))
}
