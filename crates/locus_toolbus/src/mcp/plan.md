# MCP Server Implementation Plan Guide

## Overview

Implement Model Context Protocol (MCP) server support that allows users to attach external MCP servers to the locus ToolBus. Users can configure MCP servers via CLI with or without authentication.

## Goals

1. **Runtime MCP Server Attachment**: Allow users to attach MCP servers at runtime
2. **Flexible Authentication**: Support both no-auth and authenticated connections
3. **CLI Management**: Provide intuitive CLI commands for MCP server management
4. **Tool Registration**: Automatically register MCP tools in the ToolBus
5. **Schema Storage**: Store tool schemas in LocusGraph for discovery
6. **Process Management**: Manage MCP server lifecycle (start, stop, restart)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI Layer                            │
│  `locus mcp add|list|remove|start|stop|test`                │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│                    MCP Manager                               │
│  - Server registry (config persistence)                      │
│  - Process lifecycle management                              │
│  - Connection pooling                                        │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│                    MCP Client                                │
│  - JSON-RPC communication                                    │
│  - Protocol handshake                                        │
│  - Tool discovery                                            │
│  - Tool invocation                                           │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│                    ToolBus Integration                        │
│  - Register MCP tools as ToolBus tools                       │
│  - Tool namespace (e.g., `mcp.filesystem_read`)              │
│  - Schema conversion (MCP → ToolBus)                         │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Core MCP Client

#### 1.1 MCP Protocol Types
**File**: `crates/locus_toolbus/src/mcp/protocol.rs`

```rust
// MCP JSON-RPC types
pub struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

pub struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

// MCP protocol messages
pub struct InitializeParams {
    protocol_version: String,
    capabilities: ClientCapabilities,
    client_info: Implementation,
}

pub struct InitializeResult {
    protocol_version: String,
    capabilities: ServerCapabilities,
    server_info: Implementation,
}

pub struct Tool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}
```

#### 1.2 MCP Client
**File**: `crates/locus_toolbus/src/mcp/client.rs`

```rust
pub struct McpClient {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    request_id: AtomicU64,
}

impl McpClient {
    pub async fn start(config: &McpServerConfig) -> Result<Self>;
    pub async fn initialize(&mut self) -> Result<InitializeResult>;
    pub async fn list_tools(&mut self) -> Result<Vec<Tool>>;
    pub async fn call_tool(&mut self, name: &str, args: Value) -> Result<Value>;
    pub async fn shutdown(&mut self) -> Result<()>;
}
```

### Phase 2: MCP Server Configuration

#### 2.1 Configuration Schema
**File**: `crates/locus_toolbus/src/mcp/config.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<PathBuf>,
    pub auth: Option<McpAuthConfig>,
    pub auto_start: bool,
    pub restart_policy: RestartPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAuthConfig {
    pub auth_type: String,  // "bearer", "basic", "api_key"
    pub token: String,       // can reference env var with $VAR_NAME
    pub header: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RestartPolicy {
    Never,
    OnFailure { max_retries: u32 },
    Always,
}
```

#### 2.2 Configuration Storage
**File**: `~/.config/locus/mcp_servers.toml`

```toml
[[servers]]
id = "filesystem"
name = "Filesystem MCP Server"
command = "mcp-filesystem"
args = ["--root", "/home/user/projects"]
auto_start = true

[[servers]]
id = "github"
name = "GitHub MCP Server"
command = "mcp-github"
args = ["--repo", "owner/repo"]
auth = { auth_type = "bearer", token = "$GITHUB_TOKEN" }
auto_start = true
```

### Phase 3: CLI Commands

#### 3.1 CLI Structure
**File**: `crates/locus_cli/src/cli.rs`

Add to `Command` enum:

```rust
#[derive(Subcommand)]
pub enum Command {
    // ... existing commands ...

    /// Manage MCP servers
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
}

#[derive(Subcommand)]
pub enum McpAction {
    /// Add a new MCP server
    Add {
        #[arg(short, long)]
        id: String,
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        command: String,
        #[arg(short, long)]
        args: Vec<String>,
        #[arg(short = 'e', long)]
        env: Vec<String>,
        #[arg(long)]
        auth_type: Option<String>,
        #[arg(long)]
        auth_token: Option<String>,
        #[arg(long)]
        auto_start: bool,
    },

    /// List configured MCP servers
    List { #[arg(short, long)] detailed: bool },

    /// Remove an MCP server configuration
    Remove { server_id: String },

    /// Start an MCP server
    Start { server_id: String },

    /// Stop a running MCP server
    Stop { server_id: String },

    /// Test MCP server connection
    Test { server_id: String },

    /// Show MCP server details and tools
    Info { server_id: String },

    /// Call an MCP tool directly
    Call {
        tool: String,
        #[arg(short, long)]
        args: String,
    },
}
```

#### 3.2 CLI Implementation
**File**: `crates/locus_cli/src/commands/mcp.rs`

```rust
pub async fn handle_mcp_action(action: McpAction) -> Result<()> {
    match action {
        McpAction::Add { id, name, command, args, env, auth_type, auth_token, auto_start } => {
            add_server(id, name, command, args, env, auth_type, auth_token, auto_start).await
        }
        McpAction::List { detailed } => list_servers(detailed).await,
        McpAction::Remove { server_id } => remove_server(server_id).await,
        McpAction::Start { server_id } => start_server(server_id).await,
        McpAction::Stop { server_id } => stop_server(server_id).await,
        McpAction::Test { server_id } => test_server(server_id).await,
        McpAction::Info { server_id } => show_server_info(server_id).await,
        McpAction::Call { tool, args } => call_mcp_tool(tool, args).await,
    }
}
```

### Phase 4: MCP Manager

#### 4.1 Server Registry
**File**: `crates/locus_toolbus/src/mcp/manager.rs`

```rust
pub struct McpManager {
    config_path: PathBuf,
    servers: RwLock<HashMap<String, McpServerConfig>>,
    running: RwLock<HashMap<String, McpClient>>,
    toolbus: Arc<ToolBus>,
}

impl McpManager {
    pub async fn load(config_path: PathBuf) -> Result<Self>;
    pub async fn save(&self) -> Result<()>;
    pub async fn add_server(&self, config: McpServerConfig) -> Result<()>;
    pub async fn remove_server(&self, id: &str) -> Result<()>;
    pub async fn start_server(&self, id: &str) -> Result<()>;
    pub async fn stop_server(&self, id: &str) -> Result<()>;
    pub async fn auto_start(&self) -> Result<()>;
    pub async fn list_servers(&self) -> Vec<McpServerConfig>;
    pub async fn test_server(&self, id: &str) -> Result<ServerTestResult>;
    pub async fn call_tool(&self, server_id: &str, tool_name: &str, args: Value) -> Result<Value>;
}
```

#### 4.2 ToolBus Integration
**File**: `crates/locus_toolbus/src/mcp/toolbus_integration.rs`

```rust
pub struct McpToolWrapper {
    server_id: String,
    tool: Tool,
    manager: Arc<McpManager>,
}

#[async_trait]
impl Tool for McpToolWrapper {
    fn name(&self) -> &str {
        // Format: mcp.{server_id}.{tool_name}
        &self.tool.name
    }

    fn description(&self) -> &str {
        &self.tool.description
    }

    fn parameters_schema(&self) -> &serde_json::Value {
        &self.tool.input_schema
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        self.manager.call_tool(&self.server_id, &self.tool.name, args).await
    }
}
```

### Phase 5: Error Handling

**File**: `crates/locus_toolbus/src/mcp/error.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Server not found: {0}")]
    ServerNotFound(String),

    #[error("Server already running: {0}")]
    ServerAlreadyRunning(String),

    #[error("Server not running: {0}")]
    ServerNotRunning(String),

    #[error("Failed to start server: {0}")]
    StartFailed(String),

    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

## CLI Usage Examples

### Adding MCP Servers

```bash
# Add a simple MCP server (no auth)
locus mcp add \
  --id filesystem \
  --name "Filesystem Server" \
  --command mcp-filesystem \
  --args "--root" --args "/home/user/projects" \
  --auto-start

# Add MCP server with authentication
locus mcp add \
  --id github \
  --name "GitHub Server" \
  --command mcp-github \
  --args "--repo" --args "owner/repo" \
  --auth-type bearer \
  --auth-token "$GITHUB_TOKEN" \
  --auto-start

# Add with environment variables
locus mcp add \
  --id database \
  --name "PostgreSQL Server" \
  --command mcp-postgres \
  --env "DATABASE_URL=postgresql://localhost/mydb" \
  --auto-start
```

### Managing Servers

```bash
locus mcp list
locus mcp list --detailed
locus mcp info filesystem
locus mcp test github
locus mcp start database
locus mcp stop filesystem
locus mcp remove old-server
```

### Using MCP Tools

```bash
locus mcp call filesystem.read_file --args '{"path": "/home/user/file.txt"}'
locus toolbus call mcp.filesystem_read_file --args '{"path": "/home/user/file.txt"}'
```

## File Structure

```
crates/locus_toolbus/src/mcp/
├── mod.rs              # Module exports
├── plan.md             # This plan
├── protocol.rs         # MCP JSON-RPC types
├── client.rs           # MCP client implementation
├── config.rs           # Server configuration types
├── manager.rs          # Server registry and lifecycle
├── error.rs            # Error types
└── toolbus_integration.rs  # ToolBus adapter
```

## Dependencies

Add to `crates/locus_toolbus/Cargo.toml`:

```toml
[dependencies]
tokio = { version = "1", features = ["process", "io-util", "time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
thiserror = "1"
tracing = "0.1"
async-trait = "0.1"
```

## Security Considerations

1. **Token Storage**: Store auth tokens securely (consider OS keychain integration)
2. **Process Isolation**: Run MCP servers with minimal permissions
3. **Input Validation**: Validate all tool inputs before passing to MCP servers
4. **Rate Limiting**: Implement rate limiting for tool calls
5. **Audit Logging**: Log all MCP tool invocations for security auditing
