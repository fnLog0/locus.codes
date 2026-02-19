# locus-core

**locus-core** provides shared types for locus.codes. Every crate depends on `locus-core` for cross-boundary types. No business logic lives here — only data structures, enums, and traits.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        locus-core                                │
│                                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐    │
│  │  error   │  │  memory  │  │tool_call │  │    turn      │    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────────┘    │
│       │             │              │               │            │
│       └─────────────┴──────────────┴───────────────┘            │
│                              │                                   │
│                    ┌─────────┴─────────┐                        │
│                    │     session       │                        │
│                    └─────────┬─────────┘                        │
│                              │                                   │
│                    ┌─────────┴─────────┐                        │
│                    │      event        │                        │
│                    └───────────────────┘                        │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
   locus_runtime        locus_graph          locus_ui
```

## Core Types

### 1. `Session`

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

pub enum SessionStatus {
    Active,
    Waiting,    // waiting for user input
    Running,    // LLM or tool executing
    Completed,
    Failed(String),
}
```

### 2. `Turn`

A single conversation turn with content blocks — the unit the TUI renders.

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
```

### 3. `ToolUse` / `ToolResultData`

Structured tool invocation — TUI renders these inline with status indicators.

```rust
pub struct ToolUse {
    pub id: String,
    pub name: String,           // "bash", "edit_file", "grep", etc.
    pub args: serde_json::Value,
    pub status: ToolStatus,
    pub file_path: Option<PathBuf>,
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

### 4. `SessionEvent`

Async event stream from runtime → TUI via `tokio::mpsc`.

```rust
pub enum SessionEvent {
    TurnStart { role: Role },
    TextDelta(String),
    ThinkingDelta(String),
    ToolStart(ToolUse),
    ToolDone {
        tool_use_id: String,
        result: ToolResultData,
    },
    MemoryRecall {
        query: String,
        items_found: u64,
    },
    Status(String),
    TurnEnd,
    Error(String),
    SessionEnd { status: SessionStatus },
}
```

### 5. `MemoryEvent`

Types for LocusGraph integration.

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

### 6. `LocusError`

Unified error type.

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

## Directory Structure

```
src/
├── lib.rs              # Public exports, re-exports
├── error.rs            # LocusError enum
├── memory.rs           # EventKind, ContextScope, MemoryEvent
├── tool_call.rs        # ToolUse, ToolStatus, ToolResultData
├── turn.rs             # Turn, Role, ContentBlock, TokenUsage
├── session.rs          # Session, SessionId, SessionStatus, SessionConfig, SandboxPolicy
└── event.rs            # SessionEvent
```

---

## Key Principles

1. **Types only** — no IO, no HTTP, no file system. Pure data.
2. **Serde everywhere** — all types derive `Serialize`/`Deserialize` for persistence and wire format.
3. **No graph coupling** — `Session` doesn't know about `graph_id`. LocusGraph is a separate concern.
4. **TUI-first design** — `ContentBlock` and `SessionEvent` are shaped for what the TUI needs to render.

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

## Build Order

Implement modules in this order (each depends on previous):

1. `error` — no deps on other modules
2. `memory` — `EventKind`, `ContextScope`, `MemoryEvent`
3. `tool_call` — `ToolUse`, `ToolStatus`, `ToolResultData`
4. `turn` — `Role`, `ContentBlock`, `Turn` (depends on `tool_call`)
5. `session` — `Session`, `SessionConfig` (depends on `turn`)
6. `event` — `SessionEvent` (depends on `turn`, `tool_call`)

---

## Code Quality Standards

### No Warnings Policy

All code must compile without warnings:

```bash
cargo build 2>&1 | grep warning
# Should output nothing
```

### Clippy Compliance

```bash
cargo clippy -p locus-core -- -D warnings
```

### Formatting

```bash
cargo fmt -- --check
```

### Design Principles

1. **Pure Data**: No functions that perform IO or have side effects
2. **Serde Derive**: All public types must derive `Serialize` and `Deserialize`
3. **No External Dependencies**: Only use `serde`, `serde_json`, `tokio::sync`, `chrono`, `anyhow`, `thiserror`
4. **Newtype Pattern**: Use for IDs (`SessionId(String)`)
5. **Builder Pattern**: Optional for complex configuration types

---

## Testing Guidelines

### Test File Location

```
src/
├── error.rs        # #[cfg(test)] mod tests { ... }
├── memory.rs       # #[cfg(test)] mod tests { ... }
├── tool_call.rs    # #[cfg(test)] mod tests { ... }
├── turn.rs         # #[cfg(test)] mod tests { ... }
├── session.rs      # #[cfg(test)] mod tests { ... }
└── event.rs        # #[cfg(test)] mod tests { ... }
```

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_status_serialization() {
        let status = SessionStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"Active\"");
    }

    #[test]
    fn test_content_block_text() {
        let block = ContentBlock::Text("hello".to_string());
        assert!(matches!(block, ContentBlock::Text(_)));
    }

    #[test]
    fn test_tool_use_deserialization() {
        let json = serde_json::json!({
            "id": "tool-1",
            "name": "bash",
            "args": {"command": "ls"},
            "status": "Pending"
        });
        let tool: ToolUse = serde_json::from_value(json).unwrap();
        assert_eq!(tool.id, "tool-1");
    }

    #[test]
    fn test_session_event_variants() {
        let event = SessionEvent::TextDelta("hello".to_string());
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("TextDelta"));
    }
}
```

### Test Coverage Requirements

Every type must have tests covering:

1. **Serialization**: `serde_json::to_string` works
2. **Deserialization**: `serde_json::from_str` / `from_value` works
3. **Enum Variants**: All variants serialize/deserialize correctly
4. **Optional Fields**: `Option<T>` handles `None` and `Some(T)`
5. **Default Values**: `#[serde(default)]` fields work correctly

### Running Tests

```bash
cargo test -p locus-core
cargo test -p locus-core -- --nocapture
```

---

## Implementation Checklist

### Per-Module Checklist

- [ ] Define all types from plan.md
- [ ] Add `#[derive(Debug, Clone, Serialize, Deserialize)]` to structs
- [ ] Add `#[derive(Debug, Clone, Serialize, Deserialize)]` to enums
- [ ] Use `#[serde(rename_all = "snake_case")]` for enums
- [ ] Add `pub` visibility to all fields
- [ ] Implement `Default` where appropriate
- [ ] Add `#[cfg(test)] mod tests` with serialization tests
- [ ] Export from `lib.rs`

### Module-by-Module

#### error.rs
- [ ] `LocusError` enum with all variants
- [ ] `#[from]` for `IoError`, `JsonError`, `Other`
- [ ] Tests for each error variant

#### memory.rs
- [ ] `EventKind` enum (Fact, Action, Decision, Observation, Feedback)
- [ ] `ContextScope` enum (Terminal, Editor, UserIntent, Errors)
- [ ] `MemoryEvent` struct with all fields
- [ ] Tests for serialization

#### tool_call.rs
- [ ] `ToolUse` struct
- [ ] `ToolStatus` enum
- [ ] `ToolResultData` struct
- [ ] Tests for all variants

#### turn.rs
- [ ] `Role` enum
- [ ] `ContentBlock` enum with all variants
- [ ] `TokenUsage` struct
- [ ] `Turn` struct
- [ ] Tests for all content block types

#### session.rs
- [ ] `SessionId` newtype
- [ ] `SessionStatus` enum
- [ ] `SessionConfig` struct
- [ ] `SandboxPolicy` struct
- [ ] `Session` struct
- [ ] Tests for session creation and serialization

#### event.rs
- [ ] `SessionEvent` enum with all variants
- [ ] Tests for all event types
- [ ] Tests for nested data (ToolDone, MemoryRecall, etc.)

---

## Quick Reference

### Common Patterns

```rust
// Newtype for IDs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionId(pub String);

// Enum with serde rename
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Waiting,
    Running,
    Completed,
    Failed(String),
}

// Struct with optional fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u64>,
}

// Nested enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Thinking { thinking: String },
    ToolUse { tool_use: ToolUse },
    ToolResult { tool_result: ToolResultData },
    Error { error: String },
}
```

### API Summary

| Type | Purpose |
|------|---------|
| `Session` | Conversation container |
| `Turn` | Single exchange in conversation |
| `ContentBlock` | Renderable content unit |
| `ToolUse` | Tool invocation request |
| `ToolResultData` | Tool execution result |
| `SessionEvent` | Event for TUI streaming |
| `MemoryEvent` | Event for LocusGraph |
| `LocusError` | Unified error type |
