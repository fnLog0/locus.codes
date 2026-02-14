# AGENTS.md — locus.codes Development Guide

This document helps agents work effectively in the locus.codes codebase. It documents commands, patterns, conventions, and architectural decisions.

---

## Project Overview

**locus.codes** is a terminal-native coding agent with LocusGraph as implicit memory. The agent learns from every interaction — no AGENTS.md files, no static skill files.

The codebase consists of:
- **Rust workspace** (locus-cli, locus-ui, locus-runtime, locus-toolbus, locus-core, locus-agents, locus-llm, locus-graph)
- **Landing page** (React + TypeScript + Vite in `apps/landing/`)
- **Architecture docs** (comprehensive documentation in `docs/`)
- **Reference implementations** (UI building blocks, MCP clients, AI client in `0_references/`)

**Current Status**: Phase 0 — Building the kernel (see `plan.md`)

---

## Essential Commands

### Rust Workspace

```bash
# Build the workspace
cargo build

# Run the CLI
cargo run --bin locus -- run                    # Start TUI (Smart mode, auto-detect repo)
cargo run --bin locus -- run --mode rush        # Rush mode
cargo run --bin locus -- run --mode deep        # Deep mode
cargo run --bin locus -- run --repo /path/to/repo  # Specific repo

# Build individual crates
cargo build -p locus-ui
cargo build -p locus-runtime
cargo build -p locus-toolbus

# Check compilation without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Build release version
cargo build --release
```

### Landing Page

```bash
cd apps/landing

# Install dependencies
npm install

# Start dev server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview

# Run linter
npm run lint
```

---

## Workspace Structure

```
locuscodes/
├── crates/                    # Rust workspace members
│   ├── locus-cli/            # CLI entry point (clap args, `locus run`)
│   ├── locus-core/           # Shared types (RuntimeEvent, Mode, SessionState, event_bus)
│   ├── locus-ui/             # TUI (ratatui + crossterm) - nav bar, views, prompt bar
│   ├── locus-runtime/        # Orchestrator, session boot, app entry
│   ├── locus-toolbus/        # Execution gateway - ALL file/cmd/git operations
│   ├── locus-agents/         # Subagent implementations (RepoAgent, PatchAgent, etc.)
│   ├── locus-llm/             # Model routing, prompt builder, response parser
│   └── locus-graph/           # LocusGraph SDK client, memory integration
│
├── apps/
│   └── landing/              # Landing page (React + TypeScript + Vite + Oat)
│
├── 0_references/             # Reference implementations (NOT part of the build)
│   ├── services/             # UI building blocks (textarea, wrapping, editor, etc.)
│   ├── mcp/                  # MCP protocol reference implementations
│   └── ai/                   # AI client reference (multi-provider LLM client)
│
├── docs/                     # Architecture documentation (30+ files)
│   ├── 00_overview/          # Vision, principles, glossary
│   ├── 01_system_architecture/  # Layers, components, dataflow, modes
│   ├── 02_ui_layer/          # TUI views, input, routing, keybindings
│   ├── 03_runtime_core/      # Orchestrator, scheduler, subagents, ToolBus
│   ├── 04_locusgraph_memory/ # Memory system, events, constraints
│   ├── 06_llm_engine/        # Multi-model, prompts, response schema
│   ├── 07_execution_engine/  # Patch pipeline, diff, tests, debug loop
│   ├── 08_protocols/         # ToolBus API, runtime events, agent reports
│   ├── 09_security/          # Permissions, sandbox, secrets
│   └── 10_examples/          # Task flow, memory events, diff review examples
│
├── plan.md                   # Build lifecycle — bootstrapping plan (Phase 0–6)
├── Cargo.toml                # Workspace definition
├── README.md                 # Project overview
└── .cursor/rules/            # Cursor AI rules (LocusGraph integration)
```

---

## Crate Dependencies

The workspace follows a layered dependency pattern:

| Crate | Purpose | Dependencies |
|-------|---------|--------------|
| `locus-core` | Shared types, no deps | std only |
| `locus-ui` | TUI (ratatui + crossterm) | locus-core, textwrap, unicode-segmentation, unicode-width |
| `locus-toolbus` | Execution gateway | serde, serde_json, tokio, async-trait, anyhow |
| `locus-llm` | LLM client, routing | tokio, reqwest, serde, async-trait |
| `locus-runtime` | Orchestrator, session | locus-ui, locus-toolbus, locus-llm, tokio |
| `locus-cli` | CLI entry point | locus-runtime, clap, tokio |
| `locus-agents` | Subagent implementations | locus-toolbus, locus-llm |
| `locus-graph` | LocusGraph SDK | tokio, reqwest, serde |

**No circular dependencies allowed.** If you need a type from another crate, consider moving shared types to `locus-core`.

---

## Code Conventions

### Rust Code

**Error Handling:**
- Use `anyhow::Result<T>` for application errors
- Use `thiserror` for library error types (custom error enums)
- Use `anyhow::bail!` and `anyhow::ensure!` for quick errors
- Use `?` operator for propagation

**Async Runtime:**
- All async code uses `tokio` runtime
- Use `tokio::sync::mpsc` for async channels
- Use `std::sync::mpsc` for blocking UI thread communication
- Use `Arc<dyn Trait>` for shared trait objects

**Naming:**
- Structs: `PascalCase` (e.g., `ToolBus`, `SessionState`)
- Functions: `snake_case` (e.g., `run_app`, `detect_repo_root`)
- Constants: `SCREAMING_SNAKE_CASE`
- Private fields: `_snake_case` for unused
- Use `pub(crate)` for crate-internal visibility

**Pattern Matching:**
- Prefer exhaustive match on enums
- Use `if let` and `while let` for option handling
- Use `match` with guards when needed

**Documentation:**
- Module-level docs: `//! Module description`
- Function docs: `/// Function description`
- Include examples for public APIs

**Testing:**
- Unit tests in same file: `#[cfg(test)] mod tests { ... }`
- Integration tests in `tests/` directory (not used yet)
- Use `anyhow::Result` in test functions for better error messages

### TypeScript/React Code (Landing)

**Component Structure:**
- Functional components with hooks
- Props interfaces: `interface ComponentProps { ... }`
- Use TypeScript strict mode

**Styling:**
- Split CSS into modular files (see `apps/landing/src/css/`)
- Oat semantic UI for components
- Light/dark theme support via `useTheme` hook

---

## Architecture Patterns

### ToolBus Pattern

**Critical**: ALL file operations, command execution, and git operations MUST go through `ToolBus`. This is the safety layer.

```rust
// From locus-runtime/orchestrator.rs
let result = toolbus.call(&tc.tool, tc.args.clone()).await;
```

Tools are defined in `locus-toolbus/src/tools.rs`:
- `file_read` - Read file contents
- `file_write` - Write file contents
- `run_cmd` - Execute shell command (sandboxed)
- Future: `git_status`, `git_diff`, `git_add`, `git_commit`, `grep`, `glob`

Every ToolBus call emits `ToolCalled` and `ToolResult` events on the event bus.

### Event-Driven Architecture

The runtime uses a pub/sub event bus for real-time updates between runtime and UI.

**Event Types** (from `locus-core/src/events.rs`):
- `TaskStarted`, `TaskCompleted`, `TaskFailed`
- `AgentSpawned`, `AgentCompleted`
- `ToolCalled`, `ToolResult`
- `DiffGenerated`, `DiffApproved`, `DiffRejected`
- `TestResult`, `DebugIteration`, `CommitCreated`
- `MemoryRecalled`, `MemoryStored`
- `ModeChanged`

**Usage**:
```rust
use locus_core::{event_bus, EventTx, EventRx, RuntimeEvent};

let (event_tx, event_rx) = event_bus();
let _ = event_tx.send(RuntimeEvent::TaskStarted { ... });
let ev = event_rx.recv().await?;
```

### Orchestrator Loop

The orchestrator runs in an async loop:
1. Receive prompt from UI
2. Call LLM with context + tool definitions
3. Execute tool calls via ToolBus
4. Emit events for each action
5. Return result to UI

See `crates/locus-runtime/src/orchestrator.rs`.

### Mode System

Three operating modes control the entire agent stack:

| Dimension | Rush | Smart | Deep |
|-----------|------|-------|------|
| Model | Cheap/fast | Balanced SOTA | Strongest |
| Max concurrent agents | 2 | 4 | 6 |
| Memory retrieval | 5 (~500 tokens) | 10 (~2K tokens) | 20 (~5K tokens) |
| Input token budget | 4K | 16K | 24K |
| Output token budget | 2K | 8K | 16K |
| Timeout | 30s | 120s | 300s |
| Retry limit | 1 | 3 | 5 |
| Debug loop iterations | 0 (fail fast) | 3 | 5 |
| Test strictness | Skip if unnecessary | Run tests | Full suite + benchmarks |

**CLI**: `locus run --mode rush|smart|deep`
**Environment**: `session.mode` (from `SessionState`)

---

## Key Types

### locus-core

```rust
// Mode: Rush / Smart / Deep
pub enum Mode { Rush, Smart, Deep }

// Session state
pub struct SessionState {
    pub repo_root: PathBuf,
    pub branch: String,
    pub mode: Mode,
    // ... more fields
}

// Runtime events
pub enum RuntimeEvent {
    TaskStarted { task_id, prompt, mode },
    TaskCompleted { task_id, summary, duration_ms },
    TaskFailed { task_id, error, step },
    ToolCalled { tool, args, agent_id },
    ToolResult { tool, success, result, duration_ms },
    // ... more variants
}
```

### locus-toolbus

```rust
pub trait Tool: Send + Sync {
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value>;
}

pub struct ToolBus {
    repo_root: PathBuf,
}

impl ToolBus {
    pub async fn call(&self, tool: &str, args: serde_json::Value)
        -> Result<(serde_json::Value, u64)>;
}
```

### locus-llm

```rust
pub trait ModelClient: Send + Sync {
    async fn complete(&self, request: CompletionRequest)
        -> Result<CompletionResponse>;
}

pub struct CompletionRequest {
    pub system_prompt: String,
    pub memory_bundle: String,
    pub tool_definitions: Vec<ToolDefinition>,
    pub user_prompt: String,
}

pub struct CompletionResponse {
    pub reasoning: String,
    pub tool_calls: Vec<ToolCall>,
}
```

---

## Non-Negotiable Invariants

These are the core guarantees that prevent self-corruption:

1. **ToolBus API never changes randomly** - Every agent and Orchestrator depend on it
2. **Event schema is versioned** (v1, v2, ...) - Prevents memory corruption
3. **RuntimeEvent protocol is stable** - UI ↔ Runtime contract
4. **Diff review required for all writes** - Self-corruption prevention
5. **Secrets never in prompts/events/logs** - Security baseline
6. **Filesystem access limited to repo root** - Sandbox safety

---

## Security and Sandbox

### Filesystem Restrictions
- All file operations limited to `repo_root`
- Symlinks outside repo root blocked
- `/tmp` directory allowed for temporary files

### Command Restrictions (Planned)
- **Blocked**: `rm -rf`, `sudo`, `curl`, `wget`
- **Allowed allowlist**: `cargo test`, `cargo build`, `npm test`, `pytest`, `go test`, `make test`
- Default timeout: 60s per command
- Environment variables: sensitive vars stripped before injection

### Permission Model
- `read`: Always allowed
- `write`: Configurable (ask user)
- `execute`: Configurable (ask user)
- `git_write`: Always ask (e.g., git push)

---

## Development Workflow

### Adding a New Tool to ToolBus

1. Define tool in `crates/locus-toolbus/src/tools.rs`:
```rust
pub struct NewTool { pub repo_root: PathBuf }

impl Tool for NewTool {
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        // Implementation
    }
}
```

2. Add to `ToolBus::call()` dispatcher in `crates/locus-toolbus/src/lib.rs`

3. Add tool definition to `locus-llm` for LLM awareness

### Adding a New View to UI

1. Create view module in `crates/locus-ui/src/` (e.g., `mod my_view.rs`)

2. Implement rendering using ratatui widgets:
```rust
pub fn render_my_view(f: &mut Frame, area: Rect, state: &MyViewState) {
    f.render_widget(
        Paragraph::new("Content"),
        area,
    );
}
```

3. Add to router/view switcher in `run_ui()`

### Adding a New Subagent

1. Implement `Agent` trait in `crates/locus-agents/`:
```rust
pub trait Agent: Send + Sync {
    async fn run(&self, context: AgentContext) -> AgentReport;
}

pub struct MyAgent;

impl Agent for MyAgent {
    async fn run(&self, context: AgentContext) -> AgentReport {
        // Implementation
    }
}
```

2. Register with scheduler in orchestrator

### Debugging

```bash
# Enable logging (once implemented)
RUST_LOG=debug cargo run --bin locus -- run

# Check specific crate
RUST_LOG=locus_runtime=debug cargo run --bin locus -- run

# Use `dbg!` macro for quick prints:
dbg!(&variable);
```

---

## LocusGraph Integration

The project integrates with LocusGraph for deterministic memory. See `.cursor/rules/locusgraph-updates.mdc` for when to update LocusGraph.

**When to store events:**
- Architecture/stack decisions
- Coding conventions established
- Product/UX decisions (modes, features)
- Integration patterns
- Validation outcomes (what worked/what didn't)

**Event kinds:**
- `fact` - Objective information
- `decision` - Choices made (with context_id like `decision:locus_codes_xyz`)
- `action` - Actions taken
- `observation` - Outcomes of actions
- `feedback` - User or system feedback

**Example:**
```rust
// When making an architectural decision
mcp_locusgraph_effortless-labs_store_event {
    graph_id: "locus_codes",
    event_kind: "decision",
    context_id: "decision:locus_codes_error_handling",
    payload: {
        "data": {
            "topic": "error_handling_pattern",
            "value": "We use anyhow::Result for application errors and thiserror for library error types"
        }
    }
}
```

---

## Landing Page Specifics

### Stack
- React 19 + TypeScript
- Vite (dev server + build)
- Oat (minimal semantic UI)
- Geist Pixel (Vercel pixel font)

### Commands
```bash
cd apps/landing
npm run dev      # http://localhost:5173
npm run build    # dist/
npm run preview
npm run lint
```

### Structure
```
apps/landing/
├── src/
│   ├── css/           # Split styles (main.css, hero.css, etc.)
│   ├── components/    # React components
│   ├── hooks/         # useTheme
│   └── App.tsx
└── public/
    ├── locus.svg      # Favicon
    └── fonts/         # Geist Pixel
```

### Theme
- Light/dark toggle stored in `localStorage`
- Default: light
- Oat variable overrides in `oat-theme.css`

---

## 0_references Directory

This directory contains **reference implementations** that are NOT part of the cargo workspace build. These are building blocks and patterns to copy/adapt:

**`services/`** - UI building blocks (from Cursor implementation)
- `textarea.rs` - Multi-line input with cursor, wrap, Emacs keys
- `wrapping.rs` - Text wrapping utilities
- `editor.rs` - External editor integration
- `message.rs` - Message display
- And 20+ more service components

**`mcp/`** - Model Context Protocol reference
- `client/` - MCP client implementation
- `server/` - MCP server implementation
- `proxy/` - MCP proxy

**`ai/`** - Multi-provider LLM client reference
- Support for OpenAI, Anthropic, Gemini
- Streaming support
- Tool calling

**Usage**: Copy/adapt code from here, don't import directly.

---

## Plan Reference

See `plan.md` for the complete build lifecycle:
- **Phase 0** — Kernel (manual, current phase)
- **Phase 1** — Single Agent MVP
- **Phase 2** — Diff-First Workflow (safety layer)
- **Phase 3** — Parallel Subagents
- **Phase 4** — LocusGraph Integration
- **Phase 5** — Constraint Engine
- **Phase 6** — Modes (Rush/Smart/Deep)

After Phase 2, locus.codes can safely modify its own code using the diff-review workflow.

---

## Common Gotchas

1. **TTY Requirement**: `locus run` requires an interactive terminal. Running in a non-TTY context will fail with an error message.

2. **Event Bus Channels**: UI uses `std::sync::mpsc` (blocking), runtime uses `tokio::sync::mpsc` (async). The bridge is in `run_app()`.

3. **Thread Safety**: Use `Arc` for shared state across threads. Use `Arc<dyn Trait>` for shared trait objects.

4. **Repo Detection**: The CLI walks up the directory tree to find `.git`. If no git repo is found, it fails.

5. **Environment Variables**:
   - `LOCUS_LLM=ollama` - Use Ollama instead of OpenAI
   - `OPENAI_API_KEY` - Required for OpenAI (default)
   - `RUST_LOG` - Logging control (when implemented)

6. **Ratatui Widget State**: Many widgets use `StatefulWidgetRef` pattern. Always pass state to `render_with_state()`.

7. **Unicode Handling**: Use `unicode-segmentation` for cursor movement and `unicode-width` for display width calculation.

---

## Testing Approach

Currently, the project has minimal test coverage. This will expand as Phase 0 progresses.

**Planned testing strategy**:
- Unit tests in `locus-core` (pure logic)
- Integration tests for ToolBus tools
- TTY tests for CLI (see `crates/locus-cli/tests/run_requires_tty.rs`)
- UI component tests (using `rand` and `chrono` in dev-deps)

**Running tests**:
```bash
cargo test                    # All tests
cargo test -p locus-core      # Specific crate
cargo test -- --nocapture     # Show output
```

---

## Build and Release

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run release binary
./target/release/locus run --mode smart
```

Release binary will be at `target/release/locus`.

---

## Documentation References

- **Architecture**: `docs/01_system_architecture/architecture.md`
- **Component Map**: `docs/01_system_architecture/component_map.md`
- **Modes**: `docs/01_system_architecture/modes.md`
- **ToolBus API**: `docs/08_protocols/toolbus_api.md`
- **Runtime Events**: `docs/08_protocols/runtime_events.md`
- **Execution Pipeline**: `docs/01_system_architecture/execution_pipeline.md`
- **Plan**: `plan.md` (bootstrap lifecycle)

---

## Memory and AI Integration

When working on this codebase, remember to update LocusGraph with decisions and validation outcomes (see `.cursor/rules/locusgraph-updates.mdc`):

1. Store architectural decisions as `event_kind: "decision"`
2. Store what worked/failed as `event_kind: "observation"` or `"fact"`
3. Use `contradicts` when correcting old assumptions
4. Keep scope to locus.codes only

This ensures future sessions stay consistent and build on past learning.
