# locus-cli

The **CLI entry point** for locus.codes. It provides a terminal interface to interact with the ToolBus, LLM providers, and the agent runtime. Built with `clap` for argument parsing and colored output for a polished terminal experience.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         locus CLI                           │
│                                                             │
│  locus <command> [options]                                  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                    Commands                           │  │
│  │                                                       │  │
│  │  ┌─────────┐  ┌────────────┐  ┌──────────────────┐  │  │
│  │  │  run    │  │  toolbus   │  │    providers     │  │  │
│  │  │ (agent) │  │  (tools)   │  │    (llms)        │  │  │
│  │  └────┬────┘  └─────┬──────┘  └────────┬─────────┘  │  │
│  └───────┼─────────────┼──────────────────┼─────────────┘  │
│          │             │                  │                 │
│          ▼             ▼                  ▼                 │
│   ┌────────────┐ ┌──────────┐  ┌───────────────────┐      │
│   │locus_runtime│ │locus_    │  │  locus_llms       │      │
│   │(future)     │ │toolbus   │  │  (ProviderRegistry│      │
│   └────────────┘ └──────────┘  └───────────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

## CLI Commands

```
locus — Terminal-native coding agent with implicit memory

USAGE:
    locus [OPTIONS] <COMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information
    -v, --verbose    Enable verbose output

COMMANDS:
    run          Start the interactive agent session (future)
    toolbus      Inspect and call ToolBus tools
    providers    Inspect and test LLM providers
```

### `locus toolbus`

Inspect registered tools, view schemas, and call tools directly.

```
USAGE:
    locus toolbus <SUBCOMMAND>

SUBCOMMANDS:
    list             List all registered tools
    info <tool>      Show tool details and parameter schema
    call <tool>      Call a tool with JSON arguments
```

**Examples:**

```bash
# List all tools
locus toolbus list

# Show tool schema
locus toolbus info grep

# Call a tool
locus toolbus call bash --args '{"command": "echo hello"}'
```

### `locus providers`

Inspect registered LLM providers and test connectivity.

```
USAGE:
    locus providers <SUBCOMMAND>

SUBCOMMANDS:
    list             List all registered providers
    info <provider>  Show provider details
    test <provider>  Test provider connectivity with a ping
    models <provider> List available models
```

**Examples:**

```bash
# List providers (shows which have API keys set)
locus providers list

# Test connectivity
locus providers test anthropic

# List models
locus providers models anthropic
```

### `locus run` (future)

Start an interactive agent session. This will be implemented when `locus_runtime` is ready.

```
USAGE:
    locus run [OPTIONS]

OPTIONS:
    --model <MODEL>      Model to use (e.g., claude-sonnet-4-20250514)
    --provider <ID>      Provider to use (default: auto-detect)
```

## Directory Structure

```
src/
├── main.rs             # Entry point — parse args, dispatch commands
├── cli.rs              # Clap app definition (commands, args, options)
├── commands/
│   ├── mod.rs          # Command dispatch
│   ├── toolbus.rs      # `locus toolbus` subcommands
│   ├── providers.rs    # `locus providers` subcommands
│   └── run.rs          # `locus run` (stub for future)
└── output.rs           # Colored terminal output helpers
```

## Core Concepts

### CLI Definition (cli.rs)

All commands and arguments defined with `clap` derive macros:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "locus", about = "Terminal-native coding agent with implicit memory")]
#[command(version, propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Inspect and call ToolBus tools
    Toolbus {
        #[command(subcommand)]
        action: ToolbusAction,
    },
    /// Inspect and test LLM providers
    Providers {
        #[command(subcommand)]
        action: ProvidersAction,
    },
    /// Start interactive agent session
    Run {
        /// Model to use
        #[arg(long)]
        model: Option<String>,
    },
}
```

### Command Dispatch (commands/mod.rs)

Each command module returns `anyhow::Result<()>`:

```rust
pub async fn handle(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Toolbus { action } => toolbus::handle(action).await,
        Command::Providers { action } => providers::handle(action).await,
        Command::Run { model } => run::handle(model).await,
    }
}
```

### Output Helpers (output.rs)

Consistent terminal formatting:

```rust
pub fn header(text: &str);           // Bold section header
pub fn item(name: &str, desc: &str); // Name + description row
pub fn success(text: &str);          // Green checkmark
pub fn error(text: &str);            // Red error
pub fn json_pretty(value: &Value);   // Pretty-printed JSON
```

---

## Guidelines for Adding New Commands

### Step 1: Define the Command

Add a new variant to the `Command` enum in `cli.rs`, with subcommands if needed.

### Step 2: Create Command Module

```
src/commands/
└── your_command.rs
```

```rust
use anyhow::Result;

pub async fn handle(action: YourAction) -> Result<()> {
    match action {
        YourAction::List => list().await,
        YourAction::Info { name } => info(&name).await,
    }
}
```

### Step 3: Wire It Up

1. Add `pub mod your_command;` to `commands/mod.rs`
2. Add match arm in `handle()` dispatch
3. Export the action enum from `cli.rs`

### Modular Design Principles

1. **Thin main.rs**: Only parses args and calls `commands::handle()`
2. **No business logic in CLI**: Commands call into `locus_toolbus` / `locus_llms` — the CLI is just a presentation layer
3. **Consistent output**: All user-facing output goes through `output.rs` helpers
4. **Async by default**: All command handlers are `async fn` even if currently sync

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | Argument parsing (derive macros) |
| `anyhow` | Error handling |
| `tokio` | Async runtime |
| `serde_json` | JSON argument parsing for tool calls |
| `locus-toolbus` | ToolBus access |
| `locus-llms` | Provider registry access |

---

## Testing

```bash
# Build the CLI binary
cargo build -p locus-cli

# Run it
cargo run --bin locus -- --help
cargo run --bin locus -- --version
cargo run --bin locus -- toolbus list
cargo run --bin locus -- providers list
```

---

## Quick Reference

### Adding a New Command Checklist

- [ ] Add variant to `Command` enum in `cli.rs`
- [ ] Add subcommand enum if needed
- [ ] Create `src/commands/your_command.rs`
- [ ] Add `pub mod` + match arm in `commands/mod.rs`
- [ ] Use `output.rs` helpers for all terminal output
- [ ] Run `cargo build -p locus-cli`
- [ ] Run `cargo clippy -p locus-cli`
- [ ] Run `cargo fmt`

### CLI at a Glance

| Command | Description | Status |
|---------|-------------|--------|
| `locus --help` | Show help | ✅ planned |
| `locus --version` | Show version | ✅ planned |
| `locus toolbus list` | List registered tools | ✅ planned |
| `locus toolbus info <tool>` | Show tool schema | ✅ planned |
| `locus toolbus call <tool>` | Call a tool | ✅ planned |
| `locus providers list` | List LLM providers | ✅ planned |
| `locus providers info <id>` | Show provider details | ✅ planned |
| `locus providers test <id>` | Test connectivity | ✅ planned |
| `locus providers models <id>` | List models | ✅ planned |
| `locus run` | Interactive agent | 🔮 future |
