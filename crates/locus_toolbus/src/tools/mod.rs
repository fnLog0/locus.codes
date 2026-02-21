pub mod bash;
pub mod create_file;
pub mod edit_file;
pub mod finder;
pub mod glob;
pub mod grep;
pub mod handoff;
pub mod read;
pub mod task_list;
pub mod undo_edit;
pub mod web_automation;

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::time::Duration;

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
