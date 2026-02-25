//! Task list persistence in the project DB (task_list table).

use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::{get_config_value, open_db, set_config};

/// Status of a single task (stored as snake_case string in DB).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Pending,
    InProgress,
    Done,
    Cancelled,
}

/// A single task in a plan.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskItem {
    #[serde(default)]
    pub id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub status: TaskStatus,
    #[serde(default)]
    pub description: Option<String>,
}

const NEXT_ID_KEY_PREFIX: &str = "task_list:next_id:";

fn status_to_str(s: TaskStatus) -> &'static str {
    match s {
        TaskStatus::Pending => "pending",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Done => "done",
        TaskStatus::Cancelled => "cancelled",
    }
}

fn str_to_status(s: &str) -> TaskStatus {
    match s {
        "in_progress" => TaskStatus::InProgress,
        "done" => TaskStatus::Done,
        "cancelled" => TaskStatus::Cancelled,
        _ => TaskStatus::Pending,
    }
}

fn next_id(conn: &rusqlite::Connection, plan_id: &str) -> Result<u64> {
    let key = format!("{}{}", NEXT_ID_KEY_PREFIX, plan_id);
    let current: u64 = get_config_value(conn, &key)?
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let next = current + 1;
    set_config(conn, &key, &next.to_string())?;
    Ok(current)
}

fn ensure_next_ids(conn: &rusqlite::Connection, plan_id: &str, items: &mut [TaskItem]) -> Result<()> {
    for item in items.iter_mut() {
        if item.id.as_ref().map_or(true, |s| s.is_empty()) {
            let id = next_id(conn, plan_id)?;
            item.id = Some(format!("t{}", id));
        }
    }
    Ok(())
}

/// Create a plan with the given tasks; returns the created tasks and JSON response.
pub fn create(
    repo_root: &Path,
    plan_id: &str,
    mut tasks: Vec<TaskItem>,
) -> Result<(Vec<TaskItem>, serde_json::Value)> {
    let conn = open_db(repo_root)?;
    ensure_next_ids(&conn, plan_id, &mut tasks)?;
    let mut sort_order: i64 = 0;
    for item in &tasks {
        conn.execute(
            "INSERT INTO task_list (plan_id, task_id, title, status, description, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                plan_id,
                item.id.as_deref().unwrap_or(""),
                item.title,
                status_to_str(item.status),
                item.description,
                sort_order,
            ],
        )?;
        sort_order += 1;
    }
    let out = serde_json::json!({ "plan_id": plan_id, "tasks": tasks });
    Ok((tasks, out))
}

/// List all tasks for a plan.
pub fn list(repo_root: &Path, plan_id: &str) -> Result<serde_json::Value> {
    let conn = open_db(repo_root)?;
    let mut stmt = conn.prepare(
        "SELECT task_id, title, status, description, sort_order FROM task_list WHERE plan_id = ?1 ORDER BY sort_order",
    )?;
    let rows = stmt.query_map(params![plan_id], |row| {
        Ok(TaskItem {
            id: Some(row.get::<_, String>(0)?),
            title: row.get(1)?,
            status: str_to_status(&row.get::<_, String>(2)?),
            description: row.get(3)?,
        })
    })?;
    let tasks: Vec<TaskItem> = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(serde_json::json!({ "plan_id": plan_id, "tasks": tasks }))
}

/// Get one task by plan_id and task_id.
pub fn get(repo_root: &Path, plan_id: &str, task_id: &str) -> Result<Option<TaskItem>> {
    let conn = open_db(repo_root)?;
    let mut stmt = conn.prepare(
        "SELECT task_id, title, status, description FROM task_list WHERE plan_id = ?1 AND task_id = ?2",
    )?;
    let mut rows = stmt.query(params![plan_id, task_id])?;
    let row = match rows.next()? {
        Some(r) => r,
        None => return Ok(None),
    };
    Ok(Some(TaskItem {
        id: Some(row.get::<_, String>(0)?),
        title: row.get(1)?,
        status: str_to_status(&row.get::<_, String>(2)?),
        description: row.get(3)?,
    }))
}

/// Update a task; returns the updated task if found.
pub fn update(
    repo_root: &Path,
    plan_id: &str,
    task_id: &str,
    status: Option<TaskStatus>,
    title: Option<String>,
    description: Option<String>,
) -> Result<Option<TaskItem>> {
    let conn = open_db(repo_root)?;
    if let Some(s) = status {
        conn.execute(
            "UPDATE task_list SET status = ?1 WHERE plan_id = ?2 AND task_id = ?3",
            params![status_to_str(s), plan_id, task_id],
        )?;
    }
    if let Some(ref t) = title {
        conn.execute(
            "UPDATE task_list SET title = ?1 WHERE plan_id = ?2 AND task_id = ?3",
            params![t, plan_id, task_id],
        )?;
    }
    if let Some(ref d) = description {
        conn.execute(
            "UPDATE task_list SET description = ?1 WHERE plan_id = ?2 AND task_id = ?3",
            params![d, plan_id, task_id],
        )?;
    }
    get(repo_root, plan_id, task_id)
}

/// Add tasks to a plan; returns full list JSON.
pub fn add(repo_root: &Path, plan_id: &str, mut new_tasks: Vec<TaskItem>) -> Result<serde_json::Value> {
    let conn = open_db(repo_root)?;
    ensure_next_ids(&conn, plan_id, &mut new_tasks)?;
    let mut sort_order: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM task_list WHERE plan_id = ?1",
        params![plan_id],
        |row| row.get(0),
    )?;
    for item in &new_tasks {
        conn.execute(
            "INSERT INTO task_list (plan_id, task_id, title, status, description, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                plan_id,
                item.id.as_deref().unwrap_or(""),
                item.title,
                status_to_str(item.status),
                item.description,
                sort_order,
            ],
        )?;
        sort_order += 1;
    }
    list(repo_root, plan_id)
}

/// Remove a task; returns list JSON on success, or Null if task not found.
pub fn remove(repo_root: &Path, plan_id: &str, task_id: &str) -> Result<serde_json::Value> {
    let conn = open_db(repo_root)?;
    let n = conn.execute("DELETE FROM task_list WHERE plan_id = ?1 AND task_id = ?2", params![plan_id, task_id])?;
    if n == 0 {
        return Ok(serde_json::Value::Null);
    }
    list(repo_root, plan_id)
}

/// Reorder tasks by the given task_id list; returns list JSON.
pub fn reorder(repo_root: &Path, plan_id: &str, order: &[String]) -> Result<serde_json::Value> {
    let conn = open_db(repo_root)?;
    for (idx, task_id) in order.iter().enumerate() {
        conn.execute(
            "UPDATE task_list SET sort_order = ?1 WHERE plan_id = ?2 AND task_id = ?3",
            params![idx as i64, plan_id, task_id],
        )?;
    }
    list(repo_root, plan_id)
}
