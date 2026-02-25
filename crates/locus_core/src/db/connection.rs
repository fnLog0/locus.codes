//! Open project DB with WAL and migrations.

use anyhow::{Context, Result};
use std::path::Path;

use super::layout;
use super::migrations;

/// Opens the DB at a given locus dir (e.g. ~/.locus or repo_root/.locus).
/// Creates dirs if needed, enables WAL, runs migrations.
pub fn open_db_at(locus_dir: &Path) -> Result<rusqlite::Connection> {
    let db_path = layout::ensure_locus_dir_at(locus_dir)?;
    let conn = rusqlite::Connection::open(&db_path).context("open locus.db")?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
    migrations::run_all(&conn)?;
    Ok(conn)
}

/// Opens the project DB (creates .locus/logs/commands if needed), enables WAL, runs migrations.
pub fn open_db(repo_root: &Path) -> Result<rusqlite::Connection> {
    open_db_at(&repo_root.join(".locus"))
}
