//! Application metadata constants

pub const NAME: &str = "locus";
pub const DISPLAY_NAME: &str = "locus.codes";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = "Terminal-native coding agent with implicit memory";
pub const REPO_URL: &str = "https://github.com/fnlog0/locus.codes";

/// Directory name for locus data within a repo
pub const DATA_DIR: &str = ".locus";
/// History subdirectory within DATA_DIR
pub const HISTORY_DIR: &str = ".locus/history";
