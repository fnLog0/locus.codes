# AGENTS.md — locus.codes Development Guide

This document helps agents work effectively in the locus.codes codebase. It documents commands, patterns, conventions, and what actually exists in the repo.

---

## Project Overview

**locus.codes** is a terminal-native coding agent with LocusGraph as implicit memory. The agent learns from every interaction — no static skill files.

The codebase consists of:
- **Rust workspace** — crates: `locus_cli`, `locus_ui`, `locus_runtime`, `locus_toolbus`, `locus_core`, `locus_agents`, `locus_llms`, `locus_graph` (folder names use underscores; package names use hyphens, e.g. `locus-toolbus`)
- **Landing page** — React + TypeScript + Vite in `apps/landing/`
- **Reference implementations** — UI building blocks, MCP, AI client in `0_references/` (not part of the build)

**Current state**: Phase 0 kernel. The only fully implemented crate is **locus_toolbus** (ToolBus, tools, edit history). Other crates are stubs or minimal. There is no `docs/` or `plan.md` in the repo yet.

---

## Essential Commands

### Rust Workspace

```bash
# Build the workspace
cargo build

# Run the CLI (when implemented)
cargo run --bin locus -- run

# Build individual crates (use hyphenated package name)
cargo build -p locus-ui
cargo build -p locus-toolbus

# Check, format, lint, test
cargo check
cargo fmt
cargo clippy
cargo test
cargo test -- --nocapture
cargo build --release
```

### Landing Page

Landing lives at **`apps/landing/`** (README may refer to it as `landing/`).

```bash
cd apps/landing
npm install
npm run dev      # http://localhost:5173
npm run build
npm run preview
npm run lint
```

---

## Workspace Structure

```
locuscodes/
├── crates/                 # Rust workspace (folders use underscores)
│   ├── locus_cli/          # CLI entry point (stub)
│   ├── locus_core/         # Shared types (stub)
│   ├── locus_ui/           # TUI — depends on locus_core (stub)
│   ├── locus_runtime/      # Orchestrator (stub)
│   ├── locus_toolbus/      # Execution gateway — IMPLEMENTED (tools, history)
│   ├── locus_agents/       # Subagents (stub)
│   ├── locus_llms/         # LLM client (locus-llms)
│   └── locus_graph/        # LocusGraph SDK (stub)
├── apps/
│   └── landing/            # Landing page (React, Vite, Oat, Geist Pixel)
├── 0_references/           # Reference code only — copy/adapt, do not import
│   ├── services/           # UI building blocks
│   ├── mcp/                # MCP reference
│   └── ai/                 # Multi-provider LLM reference
├── Cargo.toml              # Workspace definition
├── README.md               # Project overview
└── .cursor/rules/          # Cursor rules (LocusGraph, scope)
```

---

## Crate Dependencies (Actual)

| Crate (folder)   | Purpose              | Dependencies (actual)                    |
|------------------|----------------------|------------------------------------------|
| locus_core       | Shared types         | none (stub)                              |
| locus_ui         | TUI                  | locus-core, ratatui, crossterm, tokio…   |
| locus_toolbus    | Execution gateway    | serde, serde_json, tokio, anyhow, etc.   |
| locus_llms       | LLM client           | tokio, reqwest, serde (package: locus-llms) |
| locus_runtime    | Orchestrator         | none yet                                 |
| locus_cli        | CLI entry            | none yet                                 |
| locus_agents     | Subagents            | none yet                                 |
| locus_graph      | LocusGraph SDK       | none yet                                 |

No circular dependencies. Shared types will live in `locus_core` when added.

---

## ToolBus (Implemented)

All file operations, command execution, and git operations **must** go through ToolBus. This is the safety layer.

**Location**: `crates/locus_toolbus/`. Tools live in `src/tools/` (one subdir per tool: `bash/`, `create_file/`, `edit_file/`, `undo_edit/`, `glob/`, `grep/`, `finder/`).

**API** (from `src/lib.rs`):

```rust
pub struct ToolBus {
    repo_root: PathBuf,
}

impl ToolBus {
    pub fn new(repo_root: PathBuf) -> Self;
    pub async fn call(&self, tool_name: &str, args: JsonValue) -> Result<(JsonValue, u64)>;
    pub fn list_tools(&self) -> Vec<ToolInfo>;
    pub fn repo_root(&self) -> &PathBuf;
}
```

**Tool trait** (from `src/tools/mod.rs`):

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters_schema(&self) -> JsonValue;
    async fn execute(&self, args: JsonValue) -> ToolResult;
}
```

**Registered tools**: `bash`, `create_file`, `edit_file`, `undo_edit`, `glob`, `grep`, `finder`.

**Edit history**: Stored in `<repo_root>/.locus/locus.db` (SQLite, WAL mode). Used by `edit_file` and `undo_edit`. See `crates/locus_toolbus/README.md` for adding new tools.

**`.locus/` layout** (Crush-style): `locus.db` (+ WAL/shm) = main project DB (edit history + config/env); `logs/`, `commands/` = directories; `locus_graph_cache.db` = LocusGraph cache/queue (separate); `env` = synced from DB for `source .locus/env`.

**Large file writes**: Content > ~8k chars in a single `create_file` call may truncate the JSON payload. The LLM is instructed via tool descriptions to create a small skeleton first, then use multiple `edit_file` calls to build incrementally. Never send 40k+ chars in one tool call.

---

## Code Conventions

### Rust

- **Errors**: `anyhow::Result<T>` for app code; `thiserror` for library error enums. Use `?`, `bail!`, `ensure!`.
- **Async**: Tokio. `tokio::sync::mpsc` for async channels; `Arc<dyn Trait>` for shared trait objects.
- **Naming**: Structs `PascalCase`, functions `snake_case`, constants `SCREAMING_SNAKE_CASE`, private unused `_snake_case`. Use `pub(crate)` for crate-internal.
- **Docs**: Module `//!`, functions `///`. Examples on public APIs.
- **Tests**: `#[cfg(test)] mod tests` in same file; integration tests in `tests/` where used (e.g. locus_toolbus).

### Landing (TypeScript/React)

- Functional components, hooks, TypeScript strict.
- CSS in `apps/landing/src/css/`. Oat semantic UI. Light/dark via `useTheme`.

---

## Non-Negotiable Invariants

1. **ToolBus API** — stable; all agents/orchestrator depend on it.
2. **Event schema** — versioned when introduced.
3. **Secrets** — never in prompts, events, or logs.
4. **Filesystem** — access limited to repo root (sandbox).

---

## Security and Sandbox (Planned)

- File ops limited to `repo_root`; symlinks outside blocked; `/tmp` allowed.
- Command allowlist/blocklist and timeout (e.g. 60s) when implemented.
- Read/write/execute/git_write permission model as designed.

---

## Adding a New Tool (locus_toolbus)

1. Create `crates/locus_toolbus/src/tools/your_tool/` with `mod.rs`, `args.rs`, `error.rs`.
2. Implement `Tool`: `name`, `description`, `parameters_schema`, `execute`.
3. In `crates/locus_toolbus/src/tools/mod.rs`: add `pub mod your_tool` and re-export.
4. In `crates/locus_toolbus/src/lib.rs` `register_defaults()`: instantiate and `self.register(your_tool)`.

See `crates/locus_toolbus/README.md` for the full checklist and patterns.

---

## LocusGraph Integration

Use `.cursor/rules/locusgraph-updates.mdc` for when to update LocusGraph. Store decisions and validation outcomes (architecture, conventions, product, integrations). Event kinds: `fact`, `decision`, `action`, `observation`, `feedback`. Scope: locus.codes only.

---

## Landing Page

- **Stack**: React, TypeScript, Vite, Oat, Geist Pixel.
- **Commands**: `npm run dev`, `npm run build`, `npm run preview`, `npm run lint` in `apps/landing`.
- **Theme**: Light/dark in `localStorage`; Oat overrides in `oat-theme.css`.

---

## 0_references

Reference implementations only — **do not import**. Copy/adapt from:
- `services/` — UI building blocks (textarea, editor, etc.)
- `mcp/` — MCP client/server/proxy
- `ai/` — Multi-provider LLM client

---

## Common Gotchas

1. **Crate names**: Folders use underscores (`locus_toolbus`). Package names for `cargo -p` use hyphens (`locus-toolbus`).
2. **Landing path**: Always `apps/landing/`; README table may say `landing/` for short.
3. **Repo detection**: When implemented, CLI will walk up to find `.git`; no repo → fail.
4. **Environment variables** (when used): `LOCUS_LLM=ollama|zai`, `OPENAI_API_KEY`, `ZAI_API_KEY`, `ZAI_BASE_URL`, `ZAI_MODEL`, `RUST_LOG`.

---

## Testing

- **locus_toolbus**: Full tests in `src/tests/` (tool_bus, tools/*). Run: `cargo test -p locus-toolbus`.
- Other crates: minimal or no tests yet.
- Use `cargo test -- --nocapture` to see output.

---

## Memory and AI Integration

When working on this codebase, update LocusGraph with decisions and validation outcomes (see `.cursor/rules/locusgraph-updates.mdc`): store decisions as `event_kind: "decision"`, outcomes as `observation`/`fact`, use `contradicts` when correcting. Keep scope to locus.codes only.
