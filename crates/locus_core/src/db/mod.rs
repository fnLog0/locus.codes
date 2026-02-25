//! Project SQLite DB under `.locus/` (Crush-style layout).
//!
//! - `locus.db` + WAL: main project DB (edit history, config, task list).
//! - `logs/`, `commands/`: directories for logs and command data.
//! - LocusGraph uses a separate `.locus/locus_graph_cache.db`.
//! - `env`: optional file synced from config table for `source .locus/env`.

mod config;
mod connection;
mod layout;
mod migrations;
mod task_list;

pub use config::{get_config, get_config_value, set_config, sync_env_file};
pub use connection::{open_db, open_db_at};
pub use layout::{ensure_locus_dir, ensure_locus_dir_at, COMMANDS_DIR, ENV_FILE, LOCUS_DB, LOGS_DIR};
pub use migrations::{run_all as run_migrations};
pub use task_list::{add, create, get, list, remove, reorder, update, TaskItem, TaskStatus};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_locus_dir_creates_layout() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        let db_path = ensure_locus_dir(repo).unwrap();
        assert_eq!(db_path, repo.join(".locus").join(LOCUS_DB));
        assert!(repo.join(".locus").is_dir());
        assert!(repo.join(".locus").join(LOGS_DIR).is_dir());
        assert!(repo.join(".locus").join(COMMANDS_DIR).is_dir());
    }
}
