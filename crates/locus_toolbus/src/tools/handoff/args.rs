use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HandoffArgs {
    /// Shell command to run in the background.
    pub command: String,

    /// Working directory for the command. If omitted, uses repo root (when set on the tool).
    #[serde(default)]
    pub working_dir: Option<String>,
}
