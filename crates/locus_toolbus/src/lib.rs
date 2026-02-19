pub mod history;
pub mod tools;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Result};
use serde_json::Value as JsonValue;

pub use history::EditHistory;
pub use tools::{
    default_timeout, Bash, BashArgs, BashError, BashExecutor, CreateFile, CreateFileArgs,
    CreateFileError, EditFile, EditFileArgs, EditFileError, Finder, FinderArgs, FinderError,
    FinderResult, Glob, GlobArgs, GlobError, GlobResult, Grep, GrepArgs, GrepError, GrepMatch,
    GrepResult, Handoff, HandoffArgs, HandoffError, Read, ReadArgs, ReadError, SearchMatch,
    TaskItem, TaskList, TaskListAction,
    TaskListArgs, TaskListError, TaskStatus, Tool, ToolOutput, ToolResult, UndoEdit, UndoEditArgs,
    UndoEditError, WebAutomation, WebAutomationArgs, WebAutomationError,
};

pub struct ToolBus {
    repo_root: PathBuf,
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolBus {
    pub fn new(repo_root: PathBuf) -> Self {
        let mut bus = Self {
            repo_root,
            tools: HashMap::new(),
        };
        bus.register_defaults();
        bus
    }

    fn register_defaults(&mut self) {
        let bash = Bash::new().with_working_dir(self.repo_root.to_string_lossy());
        self.register(bash);

        let create_file = CreateFile::new(self.repo_root.clone());
        self.register(create_file);

        let history = Arc::new(EditHistory::load_blocking(self.repo_root.clone()));
        let edit_file = EditFile::new(self.repo_root.clone(), Arc::clone(&history));
        self.register(edit_file);

        let undo_edit = UndoEdit::new(self.repo_root.clone(), history);
        self.register(undo_edit);

        let glob = Glob::new(self.repo_root.clone());
        self.register(glob);

        let grep = Grep::new(self.repo_root.clone());
        self.register(grep);

        let finder = Finder::new(self.repo_root.clone());
        self.register(finder);

        let read = Read::new(self.repo_root.clone());
        self.register(read);

        let task_list = TaskList::new();
        self.register(task_list);

        let handoff = Handoff::new(self.repo_root.clone());
        self.register(handoff);

        let web_automation = WebAutomation::new();
        self.register(web_automation);
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    pub async fn call(&self, tool_name: &str, args: JsonValue) -> Result<(JsonValue, u64)> {
        let tool = self
            .tools
            .get(tool_name)
            .ok_or_else(|| anyhow!("Tool not found: {}", tool_name))?;

        let start = Instant::now();
        let result = tool.execute(args).await?;
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok((result, duration_ms))
    }

    pub fn list_tools(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .map(|t| ToolInfo {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect()
    }

    pub fn repo_root(&self) -> &PathBuf {
        &self.repo_root
    }
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: JsonValue,
}
