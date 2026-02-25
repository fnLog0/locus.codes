mod args;
mod error;
mod executor;

pub use args::BashArgs;
pub use error::BashError;
pub use executor::BashExecutor;

use crate::tools::{parse_tool_schema, Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::OnceLock;
use std::time::Duration;

fn schema() -> &'static (&'static str, &'static str, JsonValue) {
    static SCHEMA: OnceLock<(&'static str, &'static str, JsonValue)> = OnceLock::new();
    SCHEMA.get_or_init(|| parse_tool_schema(include_str!("schema.json")))
}

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
        schema().0
    }

    fn description(&self) -> &'static str {
        schema().1
    }

    fn parameters_schema(&self) -> JsonValue {
        schema().2.clone()
    }

    async fn execute(&self, args: JsonValue) -> ToolResult {
        let bash_args: BashArgs = serde_json::from_value(args)?;
        let output = self.executor.run(&bash_args).await?;
        Ok(output.to_json())
    }
}
