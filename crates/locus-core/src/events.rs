//! Runtime events (08_protocols/runtime_events.md)

use serde::{Deserialize, Serialize};

use crate::mode::Mode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeEvent {
    TaskStarted {
        task_id: String,
        prompt: String,
        mode: Mode,
    },
    TaskCompleted {
        task_id: String,
        summary: String,
        duration_ms: u64,
    },
    TaskFailed {
        task_id: String,
        error: String,
        step: Option<String>,
    },
    AgentSpawned {
        agent_id: String,
        agent_type: String,
        task: String,
    },
    AgentCompleted {
        agent_id: String,
        status: String,
        result: String,
    },
    ToolCalled {
        tool: String,
        args: serde_json::Value,
        agent_id: Option<String>,
    },
    ToolResult {
        tool: String,
        success: bool,
        result: serde_json::Value,
        duration_ms: u64,
    },
    DiffGenerated {
        files: Vec<String>,
        hunks_count: usize,
    },
    DiffApproved { files: Vec<String> },
    DiffRejected {
        files: Vec<String>,
        reason: String,
    },
    TestResult {
        passed: u32,
        failed: u32,
        total: u32,
        output: String,
    },
    DebugIteration {
        iteration: u32,
        failure_summary: String,
    },
    CommitCreated {
        hash: String,
        message: String,
        files: Vec<String>,
    },
    MemoryRecalled {
        locus_count: u32,
        top_confidence: f32,
    },
    MemoryStored {
        event_kind: String,
        context_id: String,
    },
    ModeChanged {
        old_mode: Mode,
        new_mode: Mode,
    },
}
