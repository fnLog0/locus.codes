//! Model Context Protocol (MCP) support for locus_toolbus.
//!
//! This module provides client and management functionality for MCP servers,
//! allowing external MCP tools to be registered at runtime.
//!
//! # Transports
//!
//! - **stdio**: Local MCP server processes via stdin/stdout
//! - **sse**: Remote MCP servers via HTTP Server-Sent Events
//!
//! # Architecture
//!
//! - **protocol**: JSON-RPC types for MCP communication
//! - **transport**: Transport abstraction (stdio, SSE)
//! - **config**: Server configuration and persistence
//! - **client**: MCP client for communicating with servers
//! - **manager**: Server lifecycle and registry management
//! - **toolbus_integration**: Adapter for registering MCP tools with ToolBus
//! - **error**: Error types for MCP operations

pub mod client;
pub mod config;
pub mod error;
pub mod manager;
pub mod protocol;
pub mod toolbus_integration;
pub mod transport;

// Re-export commonly used types for convenience
pub use client::McpClient;
pub use config::{McpAuthConfig, McpServerConfig, McpServersConfig, RestartPolicy};
pub use error::{McpError, McpResult};
pub use manager::{McpManager, ServerTestResult};
pub use protocol::{
    CallToolRequest, CallToolResult, ClientCapabilities, Content, Implementation,
    InitializeParams, InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
    ListToolsRequest, ListToolsResult, ServerCapabilities, Tool,
};
pub use transport::{SseTransport, StdioTransport, TransportEnum, TransportType};
pub use toolbus_integration::{McpToolInfo, McpToolWrapper, register_mcp_tools};
