use serde::Deserialize;

// Re-export task types from core (single source of truth for storage and API).
pub use locus_core::db::{TaskItem, TaskStatus};

/// Action to perform on the task list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskListAction {
    Create,
    List,
    Get,
    Update,
    Add,
    Remove,
    Reorder,
}

#[derive(Debug, Deserialize)]
pub struct TaskListArgs {
    pub action: TaskListAction,

    /// Plan/session id; if omitted, use "default".
    #[serde(default = "default_plan_id")]
    pub plan_id: String,

    /// For create: initial list. For add: one or more tasks to append.
    #[serde(default)]
    pub tasks: Vec<TaskItem>,

    /// For get, update, remove: task id.
    #[serde(default)]
    pub task_id: Option<String>,

    /// For update: new status.
    #[serde(default)]
    pub status: Option<TaskStatus>,

    /// For update: new title.
    #[serde(default)]
    pub title: Option<String>,

    /// For update: new description.
    #[serde(default)]
    pub description: Option<String>,

    /// For reorder: ordered list of task_ids.
    #[serde(default)]
    pub order: Vec<String>,
}

fn default_plan_id() -> String {
    "default".to_string()
}
