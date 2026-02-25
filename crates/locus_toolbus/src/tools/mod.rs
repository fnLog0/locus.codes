pub mod bash;
pub mod create_file;
pub mod edit_file;
pub mod finder;
pub mod glob;
pub mod grep;
pub mod handoff;
pub mod meta;
pub mod read;
pub mod task_list;
pub mod undo_edit;
pub mod web_automation;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::time::Duration;

#[derive(Deserialize)]
pub(crate) struct ToolSchemaJson {
    pub name: String,
    pub description: String,
    pub parameters: JsonValue,
}

/// Parse a tool schema JSON (name, description, parameters) and leak name/description to 'static.
/// Each tool should have its own `OnceLock` and call this with `include_str!("schema.json")`.
pub(crate) fn parse_tool_schema(json: &'static str) -> (&'static str, &'static str, JsonValue) {
    let raw: ToolSchemaJson =
        serde_json::from_str(json).expect("tool schema.json must be valid JSON");
    (
        Box::leak(raw.name.into_boxed_str()),
        Box::leak(raw.description.into_boxed_str()),
        raw.parameters,
    )
}

pub use bash::{Bash, BashArgs, BashError, BashExecutor};
pub use create_file::{CreateFile, CreateFileArgs, CreateFileError};
pub use edit_file::{EditFile, EditFileArgs, EditFileError, EditOperation};
pub use finder::{Finder, FinderArgs, FinderError, FinderResult, SearchMatch};
pub use glob::{Glob, GlobArgs, GlobError, GlobResult};
pub use grep::{Grep, GrepArgs, GrepError, GrepMatch, GrepResult};
pub use handoff::{Handoff, HandoffArgs, HandoffError};
pub use read::{Read, ReadArgs, ReadError};
pub use task_list::{TaskItem, TaskList, TaskListAction, TaskListArgs, TaskListError, TaskStatus};
pub use undo_edit::{UndoEdit, UndoEditArgs, UndoEditError};
pub use web_automation::{ProxyConfig, WebAutomation, WebAutomationArgs, WebAutomationError};
pub use meta::{meta_tool_definitions, task_tool_definition};

pub type ToolResult = anyhow::Result<JsonValue>;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters_schema(&self) -> JsonValue;
    async fn execute(&self, args: JsonValue) -> ToolResult;
}

pub struct ToolOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

impl ToolOutput {
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    pub fn to_json(&self) -> JsonValue {
        serde_json::json!({
            "stdout": self.stdout,
            "stderr": self.stderr,
            "exit_code": self.exit_code,
            "duration_ms": self.duration_ms,
            "success": self.is_success()
        })
    }
}

pub fn default_timeout() -> Duration {
    Duration::from_secs(60)
}
