use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BashArgs {
    pub command: String,

    #[serde(default = "default_timeout")]
    pub timeout: u64,

    #[serde(default)]
    pub working_dir: Option<String>,
}

fn default_timeout() -> u64 {
    60
}
