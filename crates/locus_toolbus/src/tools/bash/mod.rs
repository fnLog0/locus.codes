mod args;
mod error;
mod executor;

pub use args::BashArgs;
pub use error::BashError;
pub use executor::BashExecutor;

use crate::tools::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::time::Duration;

pub struct Bash {
    executor: BashExecutor,
}

impl Bash {
    pub fn new() -> Self {
        Self {
            executor: BashExecutor::default(),
        }
    }

    pub fn with_timeout(self, _timeout: Duration) -> Self {
        self
    }

    pub fn with_working_dir(mut self, working_dir: impl Into<String>) -> Self {
        self.executor = self.executor.with_working_dir(working_dir);
        self
    }
}

impl Default for Bash {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for Bash {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Executes the given shell command using bash (or sh on systems without bash)"
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 60)",
                    "default": 60
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory for the command (optional)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: JsonValue) -> ToolResult {
        let bash_args: BashArgs = serde_json::from_value(args)?;
        let output = self.executor.run(&bash_args).await?;
        Ok(output.to_json())
    }
}
