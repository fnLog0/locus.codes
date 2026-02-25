//! Edit history with SQLite persistence (Crush-style `.locus/locus.db`).
//!
//! - In-memory stack per file (old/new content pairs).
//! - Persisted in `<repo_root>/.locus/locus.db` (WAL mode), table `edit_history`.
//! - Load on startup; async record/undo use spawn_blocking for DB writes.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

use locus_core::db;

const MAX_ENTRIES_PER_FILE: usize = 50;

/// One history entry: optional DB id, timestamp, old/new content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    /// Set when loaded from DB or after insert; used to delete row on undo.
    pub id: Option<i64>,
    pub ts: u64,
    pub old: String,
    pub new: String,
}

/// In-memory edit stack per file (path relative to repo root).
#[derive(Debug, Default)]
struct HistoryInner {
    stacks: HashMap<String, Vec<WalEntry>>,
}

/// Edit history: in-memory stacks + SQLite in `.locus/locus.db`.
pub struct EditHistory {
    repo_root: PathBuf,
    inner: RwLock<HistoryInner>,
}

impl EditHistory {
    /// Create history and load from `.locus/locus.db` (creates .locus/logs/commands if needed).
    pub fn load_blocking(repo_root: PathBuf) -> Self {
        let mut inner = HistoryInner::default();

        if let Ok(conn) = db::open_db(&repo_root) {
            let mut stmt = match conn.prepare(
                "SELECT id, file_path, ts, old_content, new_content FROM edit_history ORDER BY file_path, id",
            ) {
                Ok(s) => s,
                Err(_) => return Self { repo_root, inner: RwLock::new(inner) },
            };
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            });
            if let Ok(rows) = rows {
                for row in rows.flatten() {
                    let (id, file_path, ts, old_content, new_content) = row;
                    let entry = WalEntry {
                        id: Some(id),
                        ts: ts as u64,
                        old: old_content,
                        new: new_content,
                    };
                    inner.stacks.entry(file_path).or_default().push(entry);
                }
            }
            // Prune any file over limit (keep newest)
            for stack in inner.stacks.values_mut() {
                if stack.len() > MAX_ENTRIES_PER_FILE {
                    let n = stack.len() - MAX_ENTRIES_PER_FILE;
                    stack.drain(..n);
                }
            }
        }

        Self {
            repo_root,
            inner: RwLock::new(inner),
        }
    }

    /// Record an edit and persist to DB. Path must be absolute and under repo_root.
    pub async fn record(
        &self,
        file_path: &Path,
        old_content: &str,
        new_content: &str,
    ) -> Result<()> {
        let rel = path_relative_to(&self.repo_root, file_path)?;
        let rel_key = rel.to_string_lossy().to_string();

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = WalEntry {
            id: None,
            ts,
            old: old_content.to_string(),
            new: new_content.to_string(),
        };

            let repo_root = self.repo_root.clone();
            let rel_key_db = rel_key.clone();
            let inserted_id: Option<i64> = {
                let mut guard = self.inner.write().await;
                let stack = guard.stacks.entry(rel_key.clone()).or_default();
                stack.push(entry.clone());
                let to_prune: Vec<i64> = if stack.len() > MAX_ENTRIES_PER_FILE {
                    let n = stack.len() - MAX_ENTRIES_PER_FILE;
                    stack.drain(..n).filter_map(|e| e.id).collect()
                } else {
                    Vec::new()
                };
                drop(guard);

                let id = tokio::task::spawn_blocking(move || {
                    let conn = db::open_db(&repo_root)?;
                    conn.execute(
                        "INSERT INTO edit_history (file_path, ts, old_content, new_content) VALUES (?1, ?2, ?3, ?4)",
                        rusqlite::params![&rel_key_db, ts as i64, &entry.old, &entry.new],
                    )?;
                let id = conn.last_insert_rowid();
                for old_id in to_prune {
                    let _ = conn.execute("DELETE FROM edit_history WHERE id = ?1", [old_id]);
                }
                Result::<_, anyhow::Error>::Ok(Some(id))
            })
            .await
            .context("history record spawn_blocking")??;

            id
        };

        if let Some(id) = inserted_id {
            let mut guard = self.inner.write().await;
            if let Some(stack) = guard.stacks.get_mut(&rel_key) {
                if let Some(last) = stack.last_mut() {
                    last.id = Some(id);
                }
            }
        }

        Ok(())
    }

    /// Undo last edit for a file. Returns the restored content or None if nothing to undo.
    pub async fn undo(&self, file_path: &Path) -> Result<Option<String>> {
        let rel = path_relative_to(&self.repo_root, file_path)?;
        let rel_key = rel.to_string_lossy().to_string();

        let (to_restore, id_to_delete) = {
            let mut guard = self.inner.write().await;
            let popped = guard
                .stacks
                .get_mut(&rel_key)
                .and_then(|stack| stack.pop());
            match popped {
                Some(e) => (Some(e.old), e.id),
                None => (None, None),
            }
        };

        let old_content = match &to_restore {
            Some(c) => c.clone(),
            None => return Ok(None),
        };

        let abs_path = self.repo_root.join(&rel_key);
        tokio::fs::write(&abs_path, &old_content).await?;

        if let Some(id) = id_to_delete {
            let repo_root = self.repo_root.clone();
            tokio::task::spawn_blocking(move || {
                let conn = db::open_db(&repo_root)?;
                conn.execute("DELETE FROM edit_history WHERE id = ?1", [id])?;
                Result::<_, anyhow::Error>::Ok(())
            })
            .await
            .context("history undo spawn_blocking")??;
        }

        Ok(Some(old_content))
    }
}

fn path_relative_to(repo_root: &Path, path: &Path) -> Result<PathBuf> {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let base = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    path.strip_prefix(&base)
        .map(|p| p.to_path_buf())
        .context("path not under repo_root")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn record_and_undo() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        let history = EditHistory::load_blocking(repo.clone());

        let file = repo.join("src/main.rs");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "fn foo(){}").unwrap();

        history
            .record(&file, "fn foo(){}", "fn foo(){\n  // added\n}")
            .await
            .unwrap();

        let restored = history.undo(&file).await.unwrap();
        assert_eq!(restored.as_deref(), Some("fn foo(){}"));
        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "fn foo(){}");
    }
}
