//! Edit history with WAL persistence for undo.
//!
//! - In-memory stack per file (old/new content pairs).
//! - Append-only WAL under `<repo_root>/.locus/history/<rel_path>.jsonl`.
//! - Replay on startup to restore stacks; async append on each edit.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

const MAX_ENTRIES_PER_FILE: usize = 50;
const HISTORY_DIR: &str = ".locus/history";
const MANIFEST_FILE: &str = ".locus/history/manifest.json";

/// One WAL entry: before/after content and timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    pub ts: u64,
    pub old: String,
    pub new: String,
}

/// In-memory edit stack per file (path relative to repo root).
#[derive(Debug, Default)]
struct HistoryInner {
    /// Key: path relative to repo_root (e.g. "src/main.rs")
    stacks: HashMap<String, Vec<WalEntry>>,
}

/// Edit history: in-memory stacks + async WAL persistence.
pub struct EditHistory {
    repo_root: PathBuf,
    inner: RwLock<HistoryInner>,
}

impl EditHistory {
    /// Create empty history and optionally load from disk (sync, for use in ToolBus::new).
    pub fn load_blocking(repo_root: PathBuf) -> Self {
        let mut inner = HistoryInner::default();
        let _history_dir = repo_root.join(HISTORY_DIR);
        let manifest_path = repo_root.join(MANIFEST_FILE);

        if let Ok(manifest_data) = std::fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<Manifest>(&manifest_data) {
                for file_path in manifest.files {
                    let wal_path = wal_path_for(&repo_root, &file_path);
                    if let Ok(content) = std::fs::read_to_string(&wal_path) {
                        let entries: Vec<WalEntry> = content
                            .lines()
                            .filter_map(|line| {
                                let line = line.trim();
                                if line.is_empty() {
                                    return None;
                                }
                                serde_json::from_str(line).ok()
                            })
                            .collect();
                        if !entries.is_empty() {
                            inner.stacks.insert(file_path, entries);
                        }
                    }
                }
            }
        }

        Self {
            repo_root,
            inner: RwLock::new(inner),
        }
    }

    /// Record an edit and append to WAL. Path must be absolute and under repo_root.
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
            ts,
            old: old_content.to_string(),
            new: new_content.to_string(),
        };

        // Update in-memory stack (prune if over limit)
        {
            let mut guard = self.inner.write().await;
            let stack = guard.stacks.entry(rel_key.clone()).or_default();
            stack.push(entry.clone());
            if stack.len() > MAX_ENTRIES_PER_FILE {
                stack.remove(0);
            }
        }

        // Append to WAL (async)
        let wal_path = wal_path_for(&self.repo_root, &rel_key);
        if let Some(parent) = wal_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let line = serde_json::to_string(&entry)? + "\n";
        tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)
            .await?
            .write_all(line.as_bytes())
            .await?;

        // Update manifest (list of files with history)
        self.update_manifest(&rel_key).await?;

        Ok(())
    }

    /// Undo last edit for a file. Path can be relative or absolute under repo_root.
    /// Returns the restored content (old string) or None if nothing to undo.
    pub async fn undo(&self, file_path: &Path) -> Result<Option<String>> {
        let rel = path_relative_to(&self.repo_root, file_path)?;
        let rel_key = rel.to_string_lossy().to_string();

        let to_restore = {
            let mut guard = self.inner.write().await;
            guard
                .stacks
                .get_mut(&rel_key)
                .and_then(|stack| stack.pop())
                .map(|e| e.old)
        };

        if let Some(old_content) = &to_restore {
            let abs_path = self.repo_root.join(&rel_key);
            tokio::fs::write(&abs_path, old_content).await?;
        }

        Ok(to_restore)
    }

    async fn update_manifest(&self, new_file: &str) -> Result<()> {
        let manifest_path = self.repo_root.join(MANIFEST_FILE);
        let history_dir = self.repo_root.join(HISTORY_DIR);
        tokio::fs::create_dir_all(&history_dir).await?;

        let mut manifest = {
            if let Ok(data) = tokio::fs::read_to_string(&manifest_path).await {
                serde_json::from_str::<Manifest>(&data).unwrap_or_else(|_| Manifest::default())
            } else {
                Manifest::default()
            }
        };
        if !manifest.files.contains(&new_file.to_string()) {
            manifest.files.push(new_file.to_string());
            manifest.files.sort();
        }
        let data = serde_json::to_string_pretty(&manifest)?;
        tokio::fs::write(manifest_path, data).await?;
        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Manifest {
    files: Vec<String>,
}

fn wal_path_for(repo_root: &Path, rel_path: &str) -> PathBuf {
    repo_root
        .join(HISTORY_DIR)
        .join(format!("{}.jsonl", rel_path))
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
