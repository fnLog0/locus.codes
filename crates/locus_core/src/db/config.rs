//! Config table and .locus/env sync.

use anyhow::{Context, Result};
use std::path::Path;

use super::layout;

/// Reads all config key-value pairs from the DB.
pub fn get_config(conn: &rusqlite::Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT key, value FROM config ORDER BY key")?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

/// Reads one config value by key, if present.
pub fn get_config_value(conn: &rusqlite::Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM config WHERE key = ?1")?;
    let mut rows = stmt.query(rusqlite::params![key])?;
    Ok(rows.next()?.map(|row| row.get::<_, String>(0)).transpose()?)
}

/// Sets one config key (insert or replace).
pub fn set_config(conn: &rusqlite::Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO config (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

/// Writes `locus_dir/env` from config entries (for `source .locus/env`).
/// Values are shell-quoted (one layer) so URLs and secrets are valid when sourced.
pub fn sync_env_file(locus_dir: &Path, config: &[(String, String)]) -> Result<()> {
    let path = locus_dir.join(layout::ENV_FILE);
    let mut content = String::from("# Locus CLI configuration\n# Source this file: source ~/.locus/env\n\n");
    for (k, v) in config {
        let raw = unquote_value(v);
        let escaped = raw.replace('\\', "\\\\").replace('"', "\\\"");
        content.push_str(&format!("export {}=\"{}\"\n", k, escaped));
    }
    std::fs::write(&path, content).context("write env file")?;
    Ok(())
}

/// Strip one layer of surrounding double quotes (DB may store quoted).
fn unquote_value(v: &str) -> &str {
    let v = v.trim();
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        &v[1..v.len() - 1]
    } else {
        v
    }
}
