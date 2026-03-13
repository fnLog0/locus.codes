# Plan: `cargo run` → TUI with In-App Onboarding Setup

## Goal

When the user runs `cargo run` (or `locus tui`), the TUI launches automatically. If required configuration (API keys, LocusGraph) is missing, an **interactive onboarding wizard inside the TUI** guides the user step-by-step to enter keys — no separate CLI commands needed.

---

## Current State

| What exists | Where |
|---|---|
| CLI entry: `locus tui` command | `crates/locus_cli/src/cli.rs` — `Command::Tui` |
| TUI launch + runtime loop | `crates/locus_cli/src/commands/tui.rs` |
| Static onboarding screen (text-only, tells user to run CLI commands) | `crates/locus_tui/src/view.rs` → `draw_onboarding()` |
| `Screen::Onboarding` variant | `crates/locus_tui/src/state.rs` |
| CLI config commands (`locus config api`, `locus config graph`) | `crates/locus_cli/src/commands/config.rs` |
| Config DB read/write + env sync | `crates/locus_core/src/db/config.rs` |
| `has_any_llm_key()` check | `crates/locus_cli/src/commands/tui.rs` |
| Default command is `Tui` subcommand (requires `locus tui`) | `crates/locus_cli/src/cli.rs` |

### Problems

1. **`cargo run` requires a subcommand** — no default; user must type `cargo run -- tui`.
2. **Onboarding is static text** — just says "run `locus config api`", doesn't let user enter keys in the TUI.
3. **No interactive input fields** in the TUI — only a single chat input line exists.
4. **After entering keys via CLI, user must restart** — TUI doesn't reload config.

---

## Architecture

```
cargo run (no subcommand)
    │
    ▼
main.rs → load_locus_config() → default to Tui command
    │
    ▼
tui::handle() → has_any_llm_key()?
    │                │
    │ yes            │ no
    │                ▼
    │         Screen::Setup (new)
    │         ┌─────────────────────────┐
    │         │ Step 1: Welcome         │
    │         │ Step 2: Select Provider │
    │         │ Step 3: Enter API Key   │
    │         │ Step 4: LocusGraph      │
    │         │   (optional, skip)      │
    │         │ Step 5: Confirm + Save  │
    │         └─────────────────────────┘
    │                │
    │                ▼ saves to ~/.locus/locus.db + env
    │                │
    ▼                ▼
Screen::Main (chat ready)
```

---

## Implementation Steps

### Step 1: Make TUI the Default Command

**File**: `crates/locus_cli/src/cli.rs`

- Add `#[command(default_subcommand)]` or use clap's `default_subcommand` feature so `cargo run` (no args) defaults to `Tui`.
- Alternative (simpler): In `main.rs`, if no subcommand is matched, fall through to `tui::handle()`.

**Approach**: Use clap's `SubcommandRequired = false` and check in `main.rs`:
```rust
// cli.rs — make command optional
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,  // Option<Command> instead of Command
    ...
}

// main.rs
let cli = Cli::parse();
match cli.command {
    Some(cmd) => commands::handle_command(cmd).await,
    None => commands::tui::handle(None, None, None, false).await,  // default to TUI
}
```

---

### Step 2: Add Setup Screen State

**File**: `crates/locus_tui/src/state.rs`

Add new screen variant and setup-specific state:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Main,
    Onboarding,  // keep for backward compat (can remove later)
    Setup,       // NEW: interactive setup wizard
    DebugTraces,
    WebAutomation,
}

/// Which step of the setup wizard we're on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupStep {
    Welcome,          // Step 0: Welcome message + Enter to begin
    SelectProvider,   // Step 1: Choose provider (1=Anthropic, 2=ZAI)
    EnterApiKey,      // Step 2: Type API key (masked with *)
    LocusGraphChoice, // Step 3: Configure LocusGraph? (y/n)
    LocusGraphUrl,    // Step 3a: Enter LocusGraph URL
    LocusGraphSecret, // Step 3b: Enter LocusGraph secret
    LocusGraphId,     // Step 3c: Enter Graph ID
    Confirm,          // Step 4: Review + confirm
    Done,             // Step 5: Success, press Enter to start
}

/// Setup wizard state, held in TuiState.
#[derive(Debug, Clone)]
pub struct SetupState {
    pub step: SetupStep,
    pub selected_provider: Option<String>,      // "anthropic" or "zai"
    pub api_key: String,                         // masked input
    pub api_key_display: String,                 // "****" for display
    pub configure_graph: bool,
    pub graph_url: String,
    pub graph_secret: String,
    pub graph_secret_display: String,
    pub graph_id: String,
    pub input_buffer: String,                    // current field input
    pub input_cursor: usize,
    pub is_secret_field: bool,                   // mask input with *
    pub error_message: Option<String>,           // validation error
    pub provider_cursor: usize,                  // 0=anthropic, 1=zai for arrow selection
}
```

Add `pub setup: SetupState` to `TuiState`.

---

### Step 3: Add Setup View (Draw)

**File**: `crates/locus_tui/src/view.rs` (or new file `crates/locus_tui/src/layouts/setup.rs`)

Create `draw_setup(frame, state, area)` that renders each step:

#### Welcome Screen
```
┌─ locus.codes — First-time Setup ─────────────────────┐
│                                                       │
│   Welcome to locus.codes!                             │
│                                                       │
│   Let's set up your configuration.                    │
│   You'll need an API key for at least one LLM         │
│   provider (Anthropic or ZAI).                        │
│                                                       │
│   Press Enter to begin →                              │
│                                                       │
└───────────────────────────────────────────────────────┘
```

#### Select Provider (arrow keys + Enter)
```
┌─ Select LLM Provider ────────────────────────────────┐
│                                                       │
│   Choose your LLM provider:                           │
│                                                       │
│   › Anthropic  — Claude models (sonnet, opus, haiku)  │
│     ZAI        — GLM models (glm-5, glm-4-plus)      │
│                                                       │
│   ↑/↓ to select, Enter to confirm                     │
│                                                       │
└───────────────────────────────────────────────────────┘
```

#### Enter API Key (masked input)
```
┌─ Anthropic API Key ──────────────────────────────────┐
│                                                       │
│   Enter your Anthropic API key:                       │
│                                                       │
│   > sk-ant-****************************              │
│                                                       │
│   Paste or type your key. It will be saved securely   │
│   to ~/.locus/locus.db                                │
│                                                       │
│   Enter to continue, Esc to go back                   │
│                                                       │
└───────────────────────────────────────────────────────┘
```

#### LocusGraph (Optional)
```
┌─ LocusGraph Memory (Optional) ───────────────────────┐
│                                                       │
│   LocusGraph provides persistent memory for the       │
│   agent across sessions.                              │
│                                                       │
│   Configure LocusGraph now?                           │
│                                                       │
│   › Yes                                               │
│     Skip for now                                      │
│                                                       │
│   ↑/↓ to select, Enter to confirm                     │
│                                                       │
└───────────────────────────────────────────────────────┘
```

If yes → show URL input → Secret input → Graph ID input (with defaults shown).

#### Confirm Screen
```
┌─ Confirm Configuration ─────────────────────────────┐
│                                                       │
│   Provider:    Anthropic                              │
│   API Key:     sk-a...xY4z                            │
│   LocusGraph:  Configured (grpc-dev.locusgraph.com)   │
│                                                       │
│   Config will be saved to ~/.locus/locus.db           │
│                                                       │
│   Enter to save and start, Esc to go back             │
│                                                       │
└───────────────────────────────────────────────────────┘
```

#### Done Screen
```
┌─ Setup Complete ─────────────────────────────────────┐
│                                                       │
│   ✓ Configuration saved successfully!                 │
│                                                       │
│   Press Enter to start using locus.codes →            │
│                                                       │
└───────────────────────────────────────────────────────┘
```

---

### Step 4: Add Setup Key Handling

**File**: `crates/locus_tui/src/run.rs`

Add key event handling for `Screen::Setup` in `run_loop`:

```rust
// Inside the match on KeyCode when state.screen == Screen::Setup
KeyCode::Enter if state.screen == Screen::Setup => {
    handle_setup_enter(state);
}
KeyCode::Esc if state.screen == Screen::Setup => {
    handle_setup_back(state);  // go to previous step
}
KeyCode::Up if state.screen == Screen::Setup => {
    handle_setup_up(state);    // move selection up
}
KeyCode::Down if state.screen == Screen::Setup => {
    handle_setup_down(state);  // move selection down
}
KeyCode::Char(c) if state.screen == Screen::Setup => {
    handle_setup_char(state, c);  // type into current field
}
KeyCode::Backspace if state.screen == Screen::Setup => {
    handle_setup_backspace(state);
}
```

Create a new module `crates/locus_tui/src/setup.rs` with these handler functions:
- `handle_setup_enter(state)` — advance to next step, validate
- `handle_setup_back(state)` — go to previous step
- `handle_setup_up/down(state)` — move provider/choice cursor
- `handle_setup_char(state, c)` — append to input (mask if secret)
- `handle_setup_backspace(state)` — remove last char

---

### Step 5: Save Config from TUI

**Option A — Direct DB write from TUI** (preferred, simpler):

The TUI crate already depends on `locus-core`. On the Confirm → Done transition:

```rust
// In setup handler (new module in locus_tui or in locus_cli command handler)
fn save_setup_config(setup: &SetupState) -> anyhow::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("no home dir"))?;
    let locus_dir = home.join(".locus");
    std::fs::create_dir_all(&locus_dir)?;
    let conn = locus_core::db::open_db_at(&locus_dir)?;

    // Save API key
    let env_var = match setup.selected_provider.as_deref() {
        Some("anthropic") => "ANTHROPIC_API_KEY",
        Some("zai") => "ZAI_API_KEY",
        _ => return Err(anyhow!("no provider selected")),
    };
    locus_core::db::set_config(&conn, env_var, &format!("\"{}\"", setup.api_key))?;

    // Save provider preference
    locus_core::db::set_config(&conn, "LOCUS_PROVIDER", setup.selected_provider.as_deref().unwrap_or("anthropic"))?;

    // Save LocusGraph config if configured
    if setup.configure_graph {
        locus_core::db::set_config(&conn, "LOCUSGRAPH_AGENT_SECRET", &setup.graph_secret)?;
        locus_core::db::set_config(&conn, "LOCUSGRAPH_SERVER_URL", &setup.graph_url)?;
        locus_core::db::set_config(&conn, "LOCUSGRAPH_GRAPH_ID", &setup.graph_id)?;
    }

    // Sync env file
    let config = locus_core::db::get_config(&conn)?;
    locus_core::db::sync_env_file(&locus_dir, &config)?;

    // Also set in current process env so runtime picks it up without restart
    unsafe {
        std::env::set_var(env_var, &setup.api_key);
        if let Some(p) = &setup.selected_provider {
            std::env::set_var("LOCUS_PROVIDER", p);
        }
        if setup.configure_graph {
            std::env::set_var("LOCUSGRAPH_AGENT_SECRET", &setup.graph_secret);
            std::env::set_var("LOCUSGRAPH_SERVER_URL", &setup.graph_url);
            std::env::set_var("LOCUSGRAPH_GRAPH_ID", &setup.graph_id);
        }
    }

    Ok(())
}
```

**Option B — Message-based** (send config via channel to CLI layer):

Add a `mpsc::Sender<SetupConfig>` channel from CLI → TUI. When setup completes, TUI sends config back, CLI saves to DB. More complex, only needed if we want to keep DB writes out of TUI crate.

**Recommendation**: Option A. The TUI already depends on `locus-core` which has the DB API. Add `dirs` to `locus_tui/Cargo.toml` (already in `locus_cli`).

---

### Step 6: Hot-Reload Config After Setup

After setup saves, the runtime isn't started yet (first user message triggers it). So no restart needed — the next `Runtime::new()` call in `run_runtime_loop` will pick up the newly-set env vars.

The only thing needed: after `save_setup_config()`, set env vars in the current process (done in Step 5 above), then transition `state.screen = Screen::Main`.

---

## File Changes Summary

| File | Change |
|---|---|
| `crates/locus_cli/src/cli.rs` | Make `command` field `Option<Command>` for default-to-TUI |
| `crates/locus_cli/src/main.rs` | Handle `None` command → default to `tui::handle(...)` |
| `crates/locus_cli/src/commands/mod.rs` | Update `handle()` for `Option<Command>` |
| `crates/locus_tui/src/state.rs` | Add `Screen::Setup`, `SetupStep`, `SetupState` structs |
| `crates/locus_tui/src/setup.rs` | **NEW**: Setup wizard logic (handlers + save) |
| `crates/locus_tui/src/view.rs` | Add `Screen::Setup => draw_setup(...)` dispatch |
| `crates/locus_tui/src/layouts/setup.rs` | **NEW**: Setup screen layout/rendering |
| `crates/locus_tui/src/layouts/mod.rs` | Export setup layout |
| `crates/locus_tui/src/run.rs` | Add key handling for `Screen::Setup` |
| `crates/locus_tui/src/lib.rs` | Export setup module |
| `crates/locus_tui/Cargo.toml` | Add `dirs = "6"` dependency |
| `crates/locus_cli/src/commands/tui.rs` | Change `show_onboarding` → `show_setup` when no keys |

---

## New Files

### `crates/locus_tui/src/setup.rs`

Responsibilities:
- `SetupState` impl (new, defaults for graph URL/ID)
- `handle_setup_enter(state)` — step transitions + validation
- `handle_setup_back(state)` — go back one step
- `handle_setup_up(state)` / `handle_setup_down(state)` — cursor movement
- `handle_setup_char(state, c)` — input (masked for secrets)
- `handle_setup_backspace(state)` — delete char
- `save_setup_config(setup)` — write to DB + env + process env
- Validation: non-empty API key, valid URL format for LocusGraph

### `crates/locus_tui/src/layouts/setup.rs`

Responsibilities:
- `draw_setup(frame, state, area)` — dispatch to step-specific renderers
- `draw_welcome(frame, palette, area)`
- `draw_select_provider(frame, palette, area, cursor)`
- `draw_enter_key(frame, palette, area, provider, display, cursor_pos)`
- `draw_graph_choice(frame, palette, area, cursor)`
- `draw_graph_input(frame, palette, area, field, value, cursor_pos)`
- `draw_confirm(frame, palette, area, setup)`
- `draw_done(frame, palette, area)`

---

## Testing

1. `cargo build` — verify compilation
2. `cargo run` — should launch TUI directly (no subcommand needed)
3. With no API keys set → Setup wizard appears
4. Walk through all steps → keys saved to `~/.locus/locus.db`
5. After setup → chat screen works, runtime starts on first message
6. `cargo run -- tui --onboarding` → still shows setup
7. With keys already set → `cargo run` goes straight to chat

---

## Migration Notes

- The old `Screen::Onboarding` (static text) can be kept temporarily or removed. The new `Screen::Setup` replaces its purpose.
- `--onboarding` flag on `locus tui` should trigger `Screen::Setup` instead.
- The CLI `locus config api` / `locus config graph` commands remain unchanged (power-user path).

---

## Dependencies

No new external crates needed beyond what's already in the workspace:
- `dirs` — add to `locus_tui/Cargo.toml` (already used in `locus_cli`)
- `locus-core` — already a dependency of `locus_tui`
- `ratatui`, `crossterm` — already in `locus_tui`
