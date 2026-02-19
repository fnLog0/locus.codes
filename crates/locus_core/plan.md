# locus_core — Plan

Shared types for locus.codes. Every crate depends on `locus_core` for cross-boundary types.
No business logic lives here — only data structures, enums, and traits.

---

## Modules

### 1. `session`

Ephemeral conversation container. Not tied to LocusGraph — sessions are temporal, disposable.

```rust
pub struct Session {
    pub id: SessionId,
    pub status: SessionStatus,
    pub repo_root: PathBuf,
    pub config: SessionConfig,
    pub turns: Vec<Turn>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct SessionId(pub String); // uuid

pub enum SessionStatus {
    Active,
    Waiting,    // waiting for user input
    Running,    // LLM or tool executing
    Completed,
    Failed(String),
}

pub struct SessionConfig {
    pub model: String,        // e.g. "claude-sonnet-4-20250514"
    pub provider: String,     // e.g. "anthropic"
    pub max_turns: Option<u32>,
    pub sandbox_policy: SandboxPolicy,
}

pub struct SandboxPolicy {
    pub allowed_paths: Vec<PathBuf>,
    pub command_timeout_secs: u64,
}
```

### 2. `turn`

A single conversation turn. Contains content blocks — the unit the TUI renders.

```rust
pub struct Turn {
    pub role: Role,
    pub blocks: Vec<ContentBlock>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub token_usage: Option<TokenUsage>,
}

pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

pub enum ContentBlock {
    Text(String),
    Thinking(String),
    ToolUse(ToolUse),
    ToolResult(ToolResultData),
    Error(String),
}

pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
}
```

### 3. `tool_call`

Structured tool invocation — TUI renders these inline with status indicators.

```rust
pub struct ToolUse {
    pub id: String,
    pub name: String,           // "bash", "edit_file", "grep", etc.
    pub args: serde_json::Value,
    pub status: ToolStatus,
    pub file_path: Option<PathBuf>,  // for file-related tools
}

pub enum ToolStatus {
    Pending,
    Running,
    Done(ToolResultData),
    Failed(String),
}

pub struct ToolResultData {
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub is_error: bool,
}
```

### 4. `event`

Async event stream from runtime → TUI via `tokio::mpsc`.

```rust
pub enum SessionEvent {
    TurnStart { role: Role },
    TextDelta(String),            // streaming text chunk
    ThinkingDelta(String),        // streaming thinking chunk
    ToolStart(ToolUse),           // tool execution began
    ToolDone {
        tool_use_id: String,
        result: ToolResultData,
    },
    MemoryRecall {                // memories injected into prompt
        query: String,
        items_found: u64,
    },
    Status(String),               // runtime status messages (compressing, etc.)
    TurnEnd,
    Error(String),
    SessionEnd { status: SessionStatus },
}
```

### 5. `memory`

Shared types between `locus_graph` and `locus_runtime`. Defines what gets stored/retrieved.

```rust
pub enum EventKind {
    Fact,
    Action,
    Decision,
    Observation,
    Feedback,
}

pub enum ContextScope {
    Terminal,
    Editor,
    UserIntent,
    Errors,
}

pub struct MemoryEvent {
    pub event_kind: EventKind,
    pub context_scope: ContextScope,
    pub source: String,
    pub payload: serde_json::Value,
    pub related_to: Option<Vec<String>>,
    pub extends: Option<Vec<String>>,
    pub reinforces: Option<Vec<String>>,
    pub contradicts: Option<Vec<String>>,
}
```

### 6. `error`

```rust
#[derive(thiserror::Error, Debug)]
pub enum LocusError {
    #[error("session error: {0}")]
    Session(String),
    #[error("tool error: {0}")]
    Tool(String),
    #[error("memory error: {0}")]
    Memory(String),
    #[error("config error: {0}")]
    Config(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
```

---

## Dependencies

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["sync"] }
anyhow = "1"
thiserror = "2"
chrono = { version = "0.4", features = ["serde"] }
```

---

## Key Principles

1. **Types only** — no IO, no HTTP, no file system. Pure data.
2. **Serde everywhere** — all types derive `Serialize`/`Deserialize` for persistence and wire format.
3. **No graph coupling** — `Session` doesn't know about `graph_id`. LocusGraph is a separate concern.
4. **TUI-first design** — `ContentBlock` and `SessionEvent` are shaped for what the TUI needs to render.

---

## Build Order

1. `error` — no deps on other modules
2. `memory` — `EventKind`, `ContextScope`, `MemoryEvent`
3. `tool_call` — `ToolUse`, `ToolStatus`, `ToolResultData`
4. `turn` — `Role`, `ContentBlock`, `Turn` (depends on `tool_call`)
5. `session` — `Session`, `SessionConfig` (depends on `turn`)
6. `event` — `SessionEvent` (depends on `turn`, `tool_call`)
