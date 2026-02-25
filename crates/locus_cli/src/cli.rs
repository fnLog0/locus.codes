//! CLI argument definitions using clap derive macros.

use clap::{Parser, Subcommand, ValueEnum};

/// Terminal-native coding agent with implicit memory
#[derive(Parser)]
#[command(name = "locus", about, version, propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Output format: text (human-readable) or json (machine-readable)
    #[arg(short, long, global = true, default_value = "text")]
    pub output: OutputFormat,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum OutputFormat {
    /// Colored terminal output for humans
    #[default]
    Text,
    /// Structured JSON for AI and machine consumption
    Json,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run the interactive TUI with runtime
    Tui {
        /// Working directory (default: current directory)
        #[arg(long)]
        workdir: Option<String>,
        /// Provider to use (e.g. zai, anthropic). Uses LOCUS_PROVIDER env if not set.
        #[arg(long)]
        provider: Option<String>,
        /// Model to use (e.g. glm-5). Uses LOCUS_MODEL env if not set.
        #[arg(long)]
        model: Option<String>,
        /// Show onboarding screen first (configure API keys). Use when no keys are set or to test.
        #[arg(long)]
        onboarding: bool,
    },
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
    /// Configure locus settings
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Manage MCP (Model Context Protocol) servers
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
    /// LocusGraph cache and event queue
    Graph {
        #[command(subcommand)]
        action: GraphAction,
    },
    /// Start interactive agent session
    Run {
        /// Model to use (e.g. claude-sonnet-4-20250514, glm-5)
        #[arg(long)]
        model: Option<String>,
        /// Provider to use (anthropic, zai)
        #[arg(long)]
        provider: Option<String>,
        /// Working directory (default: current directory)
        #[arg(long)]
        workdir: Option<String>,
        /// Maximum turns per session
        #[arg(long)]
        max_turns: Option<u32>,
        /// Maximum tokens for LLM response (default: 8192)
        #[arg(long)]
        max_tokens: Option<u32>,
        /// Initial message/prompt to start with
        #[arg(short, long)]
        prompt: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Configure API keys for providers
    Api {
        /// Provider to configure (anthropic, zai)
        #[arg(short, long)]
        provider: Option<String>,
    },
    /// Configure LocusGraph connection
    Graph {
        /// LocusGraph server URL (e.g. http://127.0.0.1:50051)
        #[arg(short, long)]
        url: Option<String>,
        /// Graph ID (default: locus-agent)
        #[arg(short, long)]
        graph_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum GraphAction {
    /// Clear the LocusGraph proxy event queue (and cache) so old failing events stop retrying
    ClearQueue,
    /// Remove the LocusGraph cache and queue DB (same as clear-queue). Path: LOCUSGRAPH_DB_PATH or ~/.locus/locus_graph_cache.db
    Clean,
}

#[derive(Subcommand)]
pub enum ToolbusAction {
    /// List all registered tools
    List,
    /// Show tool details and parameter schema
    Info {
        /// Tool name
        tool: String,
    },
    /// Call a tool with JSON arguments
    Call {
        /// Tool name
        tool: String,
        /// JSON arguments
        #[arg(short, long)]
        args: String,
    },
}

#[derive(Subcommand)]
pub enum ProvidersAction {
    /// List all registered providers
    List,
    /// Show provider details
    Info {
        /// Provider ID
        provider: String,
    },
    /// Test provider connectivity
    Test {
        /// Provider ID
        provider: String,
    },
    /// List available models
    Models {
        /// Provider ID
        provider: String,
    },
}

#[derive(Subcommand)]
pub enum McpAction {
    /// Add a new MCP server (local or remote)
    Add {
        /// Unique server ID
        #[arg(short, long)]
        id: String,
        /// Human-readable server name
        #[arg(short, long)]
        name: String,
        /// Command to start the MCP server (for local servers)
        #[arg(short, long)]
        command: Option<String>,
        /// URL for remote MCP server (for remote servers)
        #[arg(short, long)]
        url: Option<String>,
        /// Transport type: stdio (local) or sse (remote)
        #[arg(short = 't', long)]
        transport: Option<String>,
        /// Command-line arguments
        #[arg(short, long)]
        args: Vec<String>,
        /// Environment variables (KEY=VALUE format)
        #[arg(short = 'e', long)]
        env: Vec<String>,
        /// Authentication type (bearer, basic, api_key)
        #[arg(long)]
        auth_type: Option<String>,
        /// Authentication token (can use $ENV_VAR for env var references)
        #[arg(long)]
        auth_token: Option<String>,
        /// Auto-start this server on launch
        #[arg(long, default_value = "true")]
        auto_start: bool,
    },

    /// List configured MCP servers
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Remove an MCP server configuration
    Remove {
        /// Server ID to remove
        server_id: String,
    },

    /// Start an MCP server
    Start {
        /// Server ID to start
        server_id: String,
    },

    /// Stop a running MCP server
    Stop {
        /// Server ID to stop
        server_id: String,
    },

    /// Test MCP server connection
    Test {
        /// Server ID to test
        server_id: String,
    },

    /// Show MCP server details and tools
    Info {
        /// Server ID to show
        server_id: String,
    },

    /// Call an MCP tool directly
    Call {
        /// Tool name (format: server_id.tool_name or mcp.server_id.tool_name)
        tool: String,
        /// JSON arguments
        #[arg(short, long)]
        args: String,
    },
}
