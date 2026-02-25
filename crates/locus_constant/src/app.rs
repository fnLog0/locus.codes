//! Application metadata constants

pub const NAME: &str = "locus";
pub const DISPLAY_NAME: &str = "locus.codes";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = "Terminal-native coding agent with implicit memory";
pub const REPO_URL: &str = "https://github.com/fnlog0/locus.codes";

/// Directory name for locus data within a repo (Crush-style layout).
pub const DATA_DIR: &str = ".locus";
/// Main project SQLite DB filename (under DATA_DIR). Edit history lives here; SQLite creates .db-wal and .db-shm.
pub const LOCUS_DB: &str = "locus.db";
/// Logs subdirectory under DATA_DIR.
pub const LOGS_DIR: &str = "logs";
/// Commands subdirectory under DATA_DIR (command history / saved commands).
pub const COMMANDS_DIR: &str = "commands";
/// Legacy: edit history was previously JSONL under this path; now in LOCUS_DB.
pub const HISTORY_DIR: &str = ".locus/history";
