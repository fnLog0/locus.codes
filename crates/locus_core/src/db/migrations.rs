//! SQL schema for the project DB. Applied on open.

/// Edit history: one row per edit (file_path, ts, old_content, new_content).
pub const EDIT_HISTORY: &str = "
CREATE TABLE IF NOT EXISTS edit_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    ts INTEGER NOT NULL,
    old_content TEXT NOT NULL,
    new_content TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_edit_history_file_path ON edit_history(file_path);
";

/// Config key-value store (and source for .locus/env).
pub const CONFIG: &str = "
CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
";

/// Task list: one row per task, ordered by sort_order.
pub const TASK_LIST: &str = "
CREATE TABLE IF NOT EXISTS task_list (
    plan_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    description TEXT,
    sort_order INTEGER NOT NULL,
    PRIMARY KEY (plan_id, task_id)
);
CREATE INDEX IF NOT EXISTS idx_task_list_plan_order ON task_list(plan_id, sort_order);
";

/// Run all migrations on an open connection.
pub fn run_all(conn: &rusqlite::Connection) -> anyhow::Result<()> {
    conn.execute_batch(EDIT_HISTORY)?;
    conn.execute_batch(CONFIG)?;
    conn.execute_batch(TASK_LIST)?;
    Ok(())
}
