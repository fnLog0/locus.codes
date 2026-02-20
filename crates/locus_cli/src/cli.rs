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
