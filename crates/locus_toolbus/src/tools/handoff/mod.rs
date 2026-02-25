mod args;
mod error;

pub use args::HandoffArgs;
pub use error::HandoffError;

use crate::tools::{parse_tool_schema, Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::OnceLock;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::process::Command;

static NEXT_HANDOFF_ID: AtomicU64 = AtomicU64::new(1);

pub struct Handoff {
    working_dir: PathBuf,
}

impl Handoff {
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }

    #[cfg(unix)]
    fn shell_and_arg(&self) -> (&'static str, &'static str) {
        ("/bin/bash", "-c")
    }

    #[cfg(windows)]
    fn shell_and_arg(&self) -> (&'static str, &'static str) {
        ("cmd", "/c")
    }

    async fn spawn_background(&self, command: &str, working_dir: &PathBuf) -> Result<u64, HandoffError> {
        let (shell, flag) = self.shell_and_arg();
        let mut child = Command::new(shell)
            .arg(flag)
            .arg(command)
            .current_dir(working_dir)
            .kill_on_drop(false)
            .spawn()
            .map_err(|e| HandoffError::SpawnFailed(e.to_string()))?;

        let handoff_id = NEXT_HANDOFF_ID.fetch_add(1, Ordering::Relaxed);

        tokio::spawn(async move {
            let _ = child.wait().await;
        });

        Ok(handoff_id)
    }
}

fn schema() -> &'static (&'static str, &'static str, JsonValue) {
    static SCHEMA: OnceLock<(&'static str, &'static str, JsonValue)> = OnceLock::new();
    SCHEMA.get_or_init(|| parse_tool_schema(include_str!("schema.json")))
}

#[async_trait]
impl Tool for Handoff {
    fn name(&self) -> &'static str {
        schema().0
    }

    fn description(&self) -> &'static str {
        schema().1
    }

    fn parameters_schema(&self) -> JsonValue {
        schema().2.clone()
    }

    async fn execute(&self, args: JsonValue) -> ToolResult {
        let a: HandoffArgs = serde_json::from_value(args)?;
        let working_dir = a
            .working_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.working_dir.clone());

        let handoff_id = self.spawn_background(&a.command, &working_dir).await?;

        Ok(serde_json::json!({
            "handoff_id": handoff_id,
            "status": "started",
            "command": a.command
        }))
    }
}
