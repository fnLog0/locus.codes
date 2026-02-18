# locus-toolbus

The **ToolBus** is the execution gateway for locus.codes. It provides a unified, safe interface for all file operations, command execution, and system interactions. Every tool call goes through ToolBus to ensure security, logging, and sandboxing.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      ToolBus                            │
│  ┌─────────────────────────────────────────────────┐   │
│  │  Tool Registry (HashMap<String, Arc<dyn Tool>>) │   │
│  └─────────────────────────────────────────────────┘   │
│                          │                              │
│         ┌────────────────┼────────────────┐            │
│         ▼                ▼                ▼            │
│    ┌─────────┐     ┌──────────┐     ┌──────────┐      │
│    │  Bash   │     │ FileRead │     │  Custom  │      │
│    │  Tool   │     │  Tool    │     │  Tools   │      │
│    └─────────┘     └──────────┘     └──────────┘      │
└─────────────────────────────────────────────────────────┘
```

## Core Concepts

### Tool Trait

All tools implement the `Tool` trait:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters_schema(&self) -> JsonValue;
    async fn execute(&self, args: JsonValue) -> ToolResult;
}
```

### ToolBus

The `ToolBus` manages tool registration and dispatch:

```rust
let bus = ToolBus::new(repo_root);

// Call a tool
let (result, duration_ms) = bus.call("bash", json!({
    "command": "echo hello"
})).await?;

// List available tools
let tools = bus.list_tools();
```

### ToolOutput

Standardized output format:

```rust
pub struct ToolOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}
```

## Directory Structure

```
src/
├── lib.rs              # ToolBus, ToolInfo, public API
├── tools/
│   ├── mod.rs          # Tool trait, ToolOutput, common types
│   └── bash/           # Bash tool implementation
│       ├── mod.rs      # Bash struct + Tool impl
│       ├── args.rs     # BashArgs (serde deserialization)
│       ├── error.rs    # BashError (thiserror)
│       └── executor.rs # BashExecutor (tokio::process)
├── mcp/                # Model Context Protocol tools
├── acp/                # Agent Communication Protocol tools
└── tests/              # Test modules
    ├── mod.rs
    ├── tool_bus.rs     # ToolBus integration tests
    └── tools/          # Per-tool tests
        └── bash.rs
```

---

## Guidelines for Adding New Tools

### Step 1: Research Existing Libraries

**CRITICAL**: Before implementing a new tool, search the web for existing Rust crates that provide the functionality. Using well-maintained libraries is preferred over custom implementations.

Search for:
- Crates on [crates.io](https://crates.io)
- GitHub repositories
- Rust forums and discussions

Examples:
- File operations → `tokio::fs`, `notify` (watching)
- Git operations → `git2`, `gix`
- HTTP requests → `reqwest`, `surf`
- JSON/YAML → `serde_json`, `serde_yaml`
- Regex → `regex`, `fancy-regex`
- Archive handling → `zip`, `tar`, `flate2`

### Step 2: Create Tool Directory

```
src/tools/
└── your_tool/
    ├── mod.rs       # Tool struct + Tool trait impl
    ├── args.rs      # Arguments struct with serde
    ├── error.rs     # Error types with thiserror
    └── executor.rs  # Core logic (optional, for complex tools)
```

### Step 3: Implementation Pattern

#### args.rs
```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct YourToolArgs {
    /// Required parameter
    pub required_field: String,

    /// Optional with default
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Optional field
    #[serde(default)]
    pub optional_field: Option<String>,
}

fn default_timeout() -> u64 {
    60
}
```

#### error.rs
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum YourToolError {
    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

#### mod.rs
```rust
mod args;
mod error;

pub use args::YourToolArgs;
pub use error::YourToolError;

use crate::tools::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;

pub struct YourTool {
    // Configuration fields
}

impl YourTool {
    pub fn new() -> Self {
        Self { /* ... */ }
    }

    pub fn with_option(mut self, option: &str) -> Self {
        // Builder pattern
        self
    }
}

impl Default for YourTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for YourTool {
    fn name(&self) -> &'static str {
        "your_tool"
    }

    fn description(&self) -> &'static str {
        "Description of what this tool does"
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "properties": {
                "required_field": {
                    "type": "string",
                    "description": "Description of the field"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds",
                    "default": 60
                }
            },
            "required": ["required_field"]
        })
    }

    async fn execute(&self, args: JsonValue) -> ToolResult {
        let tool_args: YourToolArgs = serde_json::from_value(args)?;

        // Implementation here

        Ok(serde_json::json!({
            "success": true,
            "result": "..."
        }))
    }
}
```

### Step 4: Register with ToolBus

Update `src/lib.rs`:

```rust
fn register_defaults(&mut self) {
    let bash = Bash::new().with_working_dir(self.repo_root.to_string_lossy());
    self.register(bash);

    let your_tool = YourTool::new();
    self.register(your_tool);
}
```

---

## Code Quality Standards

### No Warnings Policy

All code must compile without warnings:

```bash
cargo build 2>&1 | grep warning
# Should output nothing
```

Handle warnings by:
1. Removing unused imports
2. Using `#[allow(dead_code)]` only when necessary with a comment explaining why
3. Using `_` prefix for intentionally unused variables: `_unused_var`
4. Implementing all required trait methods

### Clippy Compliance

```bash
cargo clippy -- -D warnings
```

All clippy warnings must be addressed or explicitly allowed with justification.

### Formatting

```bash
cargo fmt -- --check
```

All code must be formatted with `cargo fmt`.

### Modular Design Principles

1. **Single Responsibility**: Each file handles one concern
   - `args.rs` → Input parsing only
   - `error.rs` → Error definitions only
   - `executor.rs` → Core logic only
   - `mod.rs` → Public API + Tool trait impl

2. **Dependency Injection**: Pass dependencies, don't create them
   ```rust
   // Good
   pub struct GitTool {
       repo_path: PathBuf,
   }

   // Avoid
   pub struct GitTool {
       repo: Repository, // Created internally
   }
   ```

3. **Builder Pattern**: Use for configuration
   ```rust
   let tool = Tool::new()
       .with_timeout(30)
       .with_working_dir("/tmp");
   ```

4. **Error Propagation**: Use `?` operator, never panic in library code
   ```rust
   let result = operation().map_err(|e| ToolError::Failed(e.to_string()))?;
   ```

5. **Async Consistency**: All tools are async, use `tokio`
   ```rust
   pub async fn execute(&self, args: JsonValue) -> ToolResult {
       // Use tokio::fs, tokio::process, etc.
   }
   ```

---

## Testing Guidelines

### Test File Location

```
src/tests/
├── mod.rs
├── tool_bus.rs        # ToolBus integration tests
└── tools/
    ├── mod.rs
    └── your_tool.rs   # Per-tool tests
```

### Test Structure

```rust
use crate::tools::{Tool, YourTool, YourToolArgs};
use serde_json::json;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

// Test 1: Tool metadata
#[test]
fn test_tool_name() {
    let tool = YourTool::new();
    assert_eq!(tool.name(), "your_tool");
}

// Test 2: Argument parsing
#[test]
fn test_args_parsing() {
    let args: YourToolArgs = serde_json::from_value(json!({
        "required_field": "value"
    })).unwrap();

    assert_eq!(args.required_field, "value");
    assert_eq!(args.timeout, 60); // Default
}

// Test 3: Successful execution
#[test]
fn test_execute_success() {
    let rt = runtime();
    rt.block_on(async {
        let tool = YourTool::new();
        let result = tool.execute(json!({
            "required_field": "test"
        })).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
    });
}

// Test 4: Error handling
#[test]
fn test_execute_missing_required_field() {
    let rt = runtime();
    rt.block_on(async {
        let tool = YourTool::new();
        let result = tool.execute(json!({})).await;

        assert!(result.is_err());
    });
}

// Test 5: Edge cases
#[test]
fn test_with_custom_options() {
    let tool = YourTool::new()
        .with_timeout(30);

    // Verify configuration
}

// Test 6: Integration with ToolBus
#[test]
fn test_tool_bus_integration() {
    let rt = runtime();
    rt.block_on(async {
        let bus = ToolBus::new(PathBuf::from("/tmp"));
        let (result, duration_ms) = bus.call("your_tool", json!({
            "required_field": "test"
        })).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
    });
}
```

### Test Coverage Requirements

Every tool must have tests covering:

1. **Metadata**: `name()`, `description()`, `parameters_schema()`
2. **Argument Parsing**: All fields, defaults, edge cases
3. **Success Path**: Normal operation returns expected output
4. **Error Paths**: Invalid inputs, missing fields, failures
5. **Builder Pattern**: All `with_*` methods
6. **Integration**: ToolBus `call()` works correctly
7. **Thread Safety**: Concurrent calls (if applicable)

### Running Tests

```bash
# All tests
cargo test -p locus-toolbus

# Specific test
cargo test -p locus-toolbus test_bash_tool_name

# With output
cargo test -p locus-toolbus -- --nocapture

# Single thread (for tests that share state)
cargo test -p locus-toolbus -- --test-threads=1
```

---

## Quick Reference

### Adding a New Tool Checklist

- [ ] Search web for existing Rust crates
- [ ] Create `src/tools/your_tool/` directory
- [ ] Create `args.rs` with `#[derive(Deserialize)]`
- [ ] Create `error.rs` with `#[derive(Error)]`
- [ ] Create `mod.rs` with Tool trait implementation
- [ ] Create `executor.rs` if logic is complex
- [ ] Export from `src/tools/mod.rs`
- [ ] Register in `ToolBus::register_defaults()`
- [ ] Create `src/tests/tools/your_tool.rs`
- [ ] Add to `src/tests/tools/mod.rs`
- [ ] Run `cargo test -p locus-toolbus`
- [ ] Run `cargo clippy -p locus-toolbus`
- [ ] Run `cargo fmt`

### Common Patterns

```rust
// Builder pattern
let tool = Tool::new().with_option("value");

// Async execution
let result = tool.execute(json!({"key": "value"})).await?;

// Error handling
.map_err(|e| ToolError::Custom(e.to_string()))?;

// Timeout
let result = tokio::time::timeout(
    Duration::from_secs(30),
    operation()
).await??;

// JSON output
Ok(json!({
    "success": true,
    "data": value
}))
```

---

## API Reference

### ToolBus

| Method | Description |
|--------|-------------|
| `new(repo_root: PathBuf)` | Create ToolBus with repo root |
| `register(tool: T)` | Register a tool |
| `call(name, args)` | Execute a tool by name |
| `list_tools()` | Get all registered tools |
| `repo_root()` | Get the repo root path |

### Tool Trait

| Method | Return Type | Description |
|--------|-------------|-------------|
| `name()` | `&'static str` | Tool identifier |
| `description()` | `&'static str` | Human-readable description |
| `parameters_schema()` | `JsonValue` | JSON Schema for args |
| `execute(args)` | `ToolResult` | Execute the tool |

### ToolOutput

| Field | Type | Description |
|-------|------|-------------|
| `stdout` | `String` | Standard output |
| `stderr` | `String` | Standard error |
| `exit_code` | `i32` | Process exit code |
| `duration_ms` | `u64` | Execution time in ms |

### ToolInfo

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Tool identifier |
| `description` | `String` | Human-readable description |
| `parameters` | `JsonValue` | JSON Schema for args |
