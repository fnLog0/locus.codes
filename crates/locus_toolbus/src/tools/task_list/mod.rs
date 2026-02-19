mod args;
mod error;

pub use args::{TaskItem, TaskListAction, TaskListArgs, TaskStatus};
pub use error::TaskListError;

use crate::tools::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tokio::sync::RwLock as AsyncRwLock;

struct TaskListState {
    plans: HashMap<String, Vec<TaskItem>>,
    next_id: u64,
}

impl TaskListState {
    fn new() -> Self {
        Self {
            plans: HashMap::new(),
            next_id: 1,
        }
    }

    fn ensure_id(&mut self, item: &mut TaskItem) {
        if item.id.is_none() || item.id.as_ref().is_none_or(|s| s.is_empty()) {
            let id = format!("t{}", self.next_id);
            self.next_id += 1;
            item.id = Some(id);
        }
    }

    fn ensure_ids(&mut self, items: &mut [TaskItem]) {
        for item in items {
            self.ensure_id(item);
        }
    }
}

pub struct TaskList {
    state: AsyncRwLock<TaskListState>,
}

impl TaskList {
    pub fn new() -> Self {
        Self {
            state: AsyncRwLock::new(TaskListState::new()),
        }
    }

    async fn run_create(&self, plan_id: &str, mut tasks: Vec<TaskItem>) -> JsonValue {
        let mut state = self.state.write().await;
        state.ensure_ids(&mut tasks);
        state.plans.insert(plan_id.to_string(), tasks.clone());
        serde_json::json!({
            "plan_id": plan_id,
            "tasks": tasks
        })
    }

    async fn run_list(&self, plan_id: &str) -> JsonValue {
        let state = self.state.read().await;
        let tasks = state.plans.get(plan_id).cloned().unwrap_or_default();
        serde_json::json!({
            "plan_id": plan_id,
            "tasks": tasks
        })
    }

    async fn run_get(&self, plan_id: &str, task_id: &str) -> Result<JsonValue, TaskListError> {
        let state = self.state.read().await;
        let tasks = state.plans.get(plan_id).ok_or_else(|| TaskListError::PlanNotFound(plan_id.to_string()))?;
        let task = tasks.iter().find(|t| t.id.as_deref() == Some(task_id))
            .ok_or_else(|| TaskListError::TaskNotFound(task_id.to_string()))?;
        Ok(serde_json::to_value(task).unwrap_or_default())
    }

    async fn run_update(
        &self,
        plan_id: &str,
        task_id: &str,
        status: Option<TaskStatus>,
        title: Option<String>,
        description: Option<String>,
    ) -> Result<JsonValue, TaskListError> {
        let mut state = self.state.write().await;
        let tasks = state.plans.get_mut(plan_id).ok_or_else(|| TaskListError::PlanNotFound(plan_id.to_string()))?;
        let task = tasks.iter_mut().find(|t| t.id.as_deref() == Some(task_id))
            .ok_or_else(|| TaskListError::TaskNotFound(task_id.to_string()))?;
        if let Some(s) = status {
            task.status = s;
        }
        if let Some(t) = title {
            task.title = t;
        }
        if let Some(d) = description {
            task.description = Some(d);
        }
        Ok(serde_json::to_value(task.clone()).unwrap_or_default())
    }

    async fn run_add(&self, plan_id: &str, mut new_tasks: Vec<TaskItem>) -> JsonValue {
        let mut state = self.state.write().await;
        for t in &mut new_tasks {
            if t.id.is_none() || t.id.as_ref().is_none_or(|s| s.is_empty()) {
                let id = format!("t{}", state.next_id);
                state.next_id += 1;
                t.id = Some(id);
            }
        }
        let tasks = state.plans.entry(plan_id.to_string()).or_default();
        tasks.extend(new_tasks.clone());
        serde_json::json!({
            "plan_id": plan_id,
            "tasks": tasks.clone()
        })
    }

    async fn run_remove(&self, plan_id: &str, task_id: &str) -> Result<JsonValue, TaskListError> {
        let mut state = self.state.write().await;
        let tasks = state.plans.get_mut(plan_id).ok_or_else(|| TaskListError::PlanNotFound(plan_id.to_string()))?;
        let len_before = tasks.len();
        tasks.retain(|t| t.id.as_deref() != Some(task_id));
        if tasks.len() == len_before {
            return Err(TaskListError::TaskNotFound(task_id.to_string()));
        }
        Ok(serde_json::json!({
            "plan_id": plan_id,
            "tasks": tasks.clone()
        }))
    }

    async fn run_reorder(&self, plan_id: &str, order: Vec<String>) -> Result<JsonValue, TaskListError> {
        let mut state = self.state.write().await;
        let tasks = state.plans.get_mut(plan_id).ok_or_else(|| TaskListError::PlanNotFound(plan_id.to_string()))?;
        let mut by_id: HashMap<String, TaskItem> = tasks.drain(..).filter_map(|t| t.id.clone().map(|id| (id, t))).collect();
        let mut reordered = Vec::new();
        for id in &order {
            if let Some(task) = by_id.remove(id) {
                reordered.push(task);
            }
        }
        reordered.extend(by_id.into_values());
        *tasks = reordered.clone();
        Ok(serde_json::json!({
            "plan_id": plan_id,
            "tasks": reordered
        }))
    }
}

impl Default for TaskList {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskList {
    fn name(&self) -> &'static str {
        "task_list"
    }

    fn description(&self) -> &'static str {
        "Plan and track tasks. Create a plan, list tasks, update status, add or remove tasks. Tasks have id, title, status (pending|in_progress|done|cancelled), and optional description."
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "list", "get", "update", "add", "remove", "reorder"],
                    "description": "Operation to perform on the task list"
                },
                "plan_id": {
                    "type": "string",
                    "description": "Optional plan/session id; if omitted, use 'default'",
                    "default": "default"
                },
                "tasks": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Optional; generated if missing" },
                            "title": { "type": "string" },
                            "status": { "type": "string", "enum": ["pending", "in_progress", "done", "cancelled"], "default": "pending" },
                            "description": { "type": "string" }
                        },
                        "required": ["title"]
                    },
                    "description": "For create: initial list. For add: one or more tasks to append."
                },
                "task_id": {
                    "type": "string",
                    "description": "For get, update, remove: task id"
                },
                "status": {
                    "type": "string",
                    "enum": ["pending", "in_progress", "done", "cancelled"],
                    "description": "For update: new status"
                },
                "title": { "type": "string", "description": "For update: new title" },
                "description": { "type": "string", "description": "For update: new description" },
                "order": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "For reorder: ordered list of task_ids"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: JsonValue) -> ToolResult {
        let a: TaskListArgs = serde_json::from_value(args)?;
        let plan_id = &a.plan_id;

        match a.action {
            TaskListAction::Create => Ok(self.run_create(plan_id, a.tasks).await),
            TaskListAction::List => Ok(self.run_list(plan_id).await),
            TaskListAction::Get => {
                let task_id = a.task_id.as_deref().ok_or_else(|| TaskListError::MissingField("task_id required for get".to_string()))?;
                self.run_get(plan_id, task_id).await.map_err(Into::into)
            }
            TaskListAction::Update => {
                let task_id = a.task_id.as_deref().ok_or_else(|| TaskListError::MissingField("task_id required for update".to_string()))?;
                self.run_update(plan_id, task_id, a.status, a.title, a.description).await.map_err(Into::into)
            }
            TaskListAction::Add => Ok(self.run_add(plan_id, a.tasks).await),
            TaskListAction::Remove => {
                let task_id = a.task_id.as_deref().ok_or_else(|| TaskListError::MissingField("task_id required for remove".to_string()))?;
                self.run_remove(plan_id, task_id).await.map_err(Into::into)
            }
            TaskListAction::Reorder => self.run_reorder(plan_id, a.order).await.map_err(Into::into),
        }
    }
}
