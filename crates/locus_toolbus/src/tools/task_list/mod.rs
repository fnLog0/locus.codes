mod args;
mod error;

pub use args::{TaskItem, TaskListAction, TaskListArgs, TaskStatus};
pub use error::TaskListError;

use crate::tools::{parse_tool_schema, Tool, ToolResult};
use async_trait::async_trait;
use locus_core::db;
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct TaskList {
    repo_root: PathBuf,
}

impl TaskList {
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }

    async fn run_create(&self, plan_id: &str, tasks: Vec<TaskItem>) -> ToolResult {
        let repo = self.repo_root.clone();
        let plan_id = plan_id.to_string();
        tokio::task::spawn_blocking(move || {
            db::create(&repo, &plan_id, tasks).map(|(_, out)| out).map_err(Into::into)
        })
        .await
        .map_err(|e| anyhow::anyhow!("task_list spawn_blocking: {}", e))?
    }

    async fn run_list(&self, plan_id: &str) -> ToolResult {
        let repo = self.repo_root.clone();
        let plan_id = plan_id.to_string();
        tokio::task::spawn_blocking(move || db::list(&repo, &plan_id).map_err(Into::into))
            .await
            .map_err(|e| anyhow::anyhow!("task_list spawn_blocking: {}", e))?
    }

    async fn run_get(&self, plan_id: &str, task_id: &str) -> Result<JsonValue, TaskListError> {
        let repo = self.repo_root.clone();
        let plan_id = plan_id.to_string();
        let task_id_clone = task_id.to_string();
        let out = tokio::task::spawn_blocking(move || db::get(&repo, &plan_id, &task_id_clone))
            .await
            .map_err(|e| TaskListError::MissingField(format!("spawn_blocking: {}", e)))??;
        let task = out.ok_or_else(|| TaskListError::TaskNotFound(task_id.to_string()))?;
        serde_json::to_value(task).map_err(|e| TaskListError::MissingField(e.to_string()))
    }

    async fn run_update(
        &self,
        plan_id: &str,
        task_id: &str,
        status: Option<TaskStatus>,
        title: Option<String>,
        description: Option<String>,
    ) -> Result<JsonValue, TaskListError> {
        let repo = self.repo_root.clone();
        let plan_id = plan_id.to_string();
        let task_id_clone = task_id.to_string();
        let out = tokio::task::spawn_blocking(move || {
            db::update(&repo, &plan_id, &task_id_clone, status, title, description)
        })
        .await
        .map_err(|e| TaskListError::MissingField(format!("spawn_blocking: {}", e)))??;
        let task = out.ok_or_else(|| TaskListError::TaskNotFound(task_id.to_string()))?;
        serde_json::to_value(task).map_err(|e| TaskListError::MissingField(e.to_string()))
    }

    async fn run_add(&self, plan_id: &str, new_tasks: Vec<TaskItem>) -> ToolResult {
        let repo = self.repo_root.clone();
        let plan_id = plan_id.to_string();
        tokio::task::spawn_blocking(move || db::add(&repo, &plan_id, new_tasks).map_err(Into::into))
            .await
            .map_err(|e| anyhow::anyhow!("task_list spawn_blocking: {}", e))?
    }

    async fn run_remove(&self, plan_id: &str, task_id: &str) -> Result<JsonValue, TaskListError> {
        let repo = self.repo_root.clone();
        let plan_id = plan_id.to_string();
        let task_id_clone = task_id.to_string();
        let result = tokio::task::spawn_blocking(move || db::remove(&repo, &plan_id, &task_id_clone).map_err(TaskListError::from))
            .await
            .map_err(|e| TaskListError::MissingField(format!("spawn_blocking: {}", e)))??;
        if result.is_null() {
            return Err(TaskListError::TaskNotFound(task_id.to_string()));
        }
        Ok(result)
    }

    async fn run_reorder(&self, plan_id: &str, order: Vec<String>) -> Result<JsonValue, TaskListError> {
        let repo = self.repo_root.clone();
        let plan_id = plan_id.to_string();
        let out = tokio::task::spawn_blocking(move || db::reorder(&repo, &plan_id, &order).map_err(TaskListError::from))
            .await
            .map_err(|e| TaskListError::MissingField(format!("spawn_blocking: {}", e)))??;
        Ok(out)
    }
}

impl Default for TaskList {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

fn schema() -> &'static (&'static str, &'static str, JsonValue) {
    static SCHEMA: OnceLock<(&'static str, &'static str, JsonValue)> = OnceLock::new();
    SCHEMA.get_or_init(|| parse_tool_schema(include_str!("schema.json")))
}

#[async_trait]
impl Tool for TaskList {
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
        let a: TaskListArgs = serde_json::from_value(args)?;
        let plan_id = &a.plan_id;

        match a.action {
            TaskListAction::Create => Ok(self.run_create(plan_id, a.tasks).await?),
            TaskListAction::List => Ok(self.run_list(plan_id).await?),
            TaskListAction::Get => {
                let task_id = a.task_id.as_deref().ok_or_else(|| TaskListError::MissingField("task_id required for get".to_string()))?;
                self.run_get(plan_id, task_id).await.map_err(Into::into)
            }
            TaskListAction::Update => {
                let task_id = a.task_id.as_deref().ok_or_else(|| TaskListError::MissingField("task_id required for update".to_string()))?;
                self.run_update(plan_id, task_id, a.status, a.title, a.description).await.map_err(Into::into)
            }
            TaskListAction::Add => Ok(self.run_add(plan_id, a.tasks).await?),
            TaskListAction::Remove => {
                let task_id = a.task_id.as_deref().ok_or_else(|| TaskListError::MissingField("task_id required for remove".to_string()))?;
                self.run_remove(plan_id, task_id).await.map_err(Into::into)
            }
            TaskListAction::Reorder => self.run_reorder(plan_id, a.order).await.map_err(Into::into),
        }
    }
}
