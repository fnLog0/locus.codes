//! MCP Client Implementation
//!
//! This module provides the [`McpClient`] for communicating with MCP servers
//! over various transports (stdio, SSE).
//!
//! The client automatically selects the appropriate transport based on the
//! configuration:
//! - Local servers use stdio transport (spawn process)
//! - Remote servers use SSE transport (HTTP)

use serde_json::Value as JsonValue;
use tracing::{debug, info, warn};

use crate::mcp::config::McpServerConfig;
use crate::mcp::error::McpError;
use crate::mcp::protocol::{
    CallToolRequest, CallToolResult, ClientCapabilities, Implementation, InitializeParams,
    InitializeResult, ListToolsResult, Tool,
};
use crate::mcp::transport::{SseTransport, StdioTransport, TransportEnum};

/// MCP client for communicating with an MCP server.
///
/// The client manages the server lifecycle and handles JSON-RPC
/// communication. It supports both local (stdio) and remote (SSE) servers.
///
/// # Example
///
/// ```ignore
/// use locus_toolbus::mcp::{McpClient, McpServerConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Local server
///     let config = McpServerConfig::new("test", "mcp-server");
///     let mut client = McpClient::connect(&config).await?;
///
///     let result = client.initialize().await?;
///     println!("Connected to: {}", result.server_info.name);
///
///     let tools = client.list_tools().await?;
///     println!("Available tools: {:?}", tools.len());
///
///     client.shutdown().await?;
///     Ok(())
/// }
/// ```
pub struct McpClient {
    transport: TransportEnum,
    server_id: String,
    initialized: bool,
    is_remote: bool,
}

impl McpClient {
    /// Connects to an MCP server based on the configuration.
    ///
    /// Automatically selects the appropriate transport:
    /// - If `url` is set: uses SSE transport
    /// - If `command` is set: uses stdio transport
    ///
    /// # Arguments
    ///
    /// * `config` - The server configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The server process cannot be started (stdio)
    /// - The connection fails (SSE)
    /// - Neither `url` nor `command` is specified
    ///
    /// # Example
    ///
    /// ```ignore
    /// use locus_toolbus::mcp::{McpClient, McpServerConfig};
    ///
    /// // Local server
    /// let config = McpServerConfig::new("github", "github-mcp-server")
    ///     .with_args(vec!["--stdio".to_string()]);
    /// let client = McpClient::connect(&config).await?;
    ///
    /// // Remote server
    /// let config = McpServerConfig::remote("remote", "https://api.example.com/mcp");
    /// let client = McpClient::connect(&config).await?;
    /// # Ok::<(), locus_toolbus::mcp::McpError>(())
    /// ```
    pub async fn connect(config: &McpServerConfig) -> Result<Self, McpError> {
        info!("Connecting to MCP server: {} ({})", config.name, config.id);

        let (transport, is_remote) = if let Some(url) = &config.url {
            // Remote server via SSE
            Self::create_sse_transport(config, url).await?
        } else if !config.command.is_empty() {
            // Local server via stdio
            Self::create_stdio_transport(config)?
        } else {
            return Err(McpError::Config(
                "Either 'url' or 'command' must be specified".to_string()
            ));
        };

        Ok(Self {
            transport,
            server_id: config.id.clone(),
            initialized: false,
            is_remote,
        })
    }

    /// Creates an SSE transport for remote servers.
    async fn create_sse_transport(
        config: &McpServerConfig,
        url: &str,
    ) -> Result<(TransportEnum, bool), McpError> {
        info!("[MCP:{}] Connecting to remote server: {}", config.id, url);

        let mut headers = reqwest::header::HeaderMap::new();

        // Add authentication headers if configured
        if let Some(auth) = &config.auth {
            let header_name = auth.header_name();
            let header_value = auth.header_value()
                .map_err(|e| McpError::AuthFailed(e.to_string()))?;

            let header_name = reqwest::header::HeaderName::try_from(header_name)
                .map_err(|e| McpError::Config(format!("Invalid header name: {}", e)))?;
            let header_value = reqwest::header::HeaderValue::try_from(&header_value)
                .map_err(|e| McpError::Config(format!("Invalid header value: {}", e)))?;

            headers.insert(header_name, header_value);
        }

        let transport = SseTransport::new(url, headers, &config.id);
        Ok((TransportEnum::Sse(transport), true))
    }

    /// Creates a stdio transport for local servers.
    fn create_stdio_transport(
        config: &McpServerConfig,
    ) -> Result<(TransportEnum, bool), McpError> {
        info!("[MCP:{}] Starting local server: {}", config.id, config.command);

        let transport = StdioTransport::spawn(
            &config.command,
            &config.args,
            &config.env,
            config.working_dir.as_deref(),
            &config.id,
        )?;

        Ok((TransportEnum::Stdio(transport), false))
    }

    /// Initializes the MCP server with the client capabilities.
    ///
    /// This must be called after connecting and before using other methods.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = client.initialize().await?;
    /// println!("Server: {} v{}", result.server_info.name, result.server_info.version);
    /// ```
    pub async fn initialize(&mut self) -> Result<InitializeResult, McpError> {
        let capabilities = ClientCapabilities::default();
        let client_info = Implementation::new("locus-toolbus", env!("CARGO_PKG_VERSION"));
        let params = InitializeParams::new(capabilities, client_info);

        let result: InitializeResult = self.transport
            .send_request("initialize", Some(serde_json::to_value(params)?))
            .await?;

        info!(
            "[MCP:{}] Initialized: {} v{}",
            self.server_id,
            result.server_info.name,
            result.server_info.version
        );

        // Send initialized notification
        self.transport
            .send_notification("notifications/initialized", None)
            .await?;

        self.initialized = true;
        Ok(result)
    }

    /// Lists all tools available on the MCP server.
    ///
    /// # Errors
    ///
    /// Returns an error if the server has not been initialized or if
    /// the server returns an error.
    pub async fn list_tools(&mut self) -> Result<Vec<Tool>, McpError> {
        if !self.initialized {
            return Err(McpError::Protocol("Server not initialized".to_string()));
        }

        let result: ListToolsResult = self.transport.send_request("tools/list", None).await?;

        debug!("[MCP:{}] Found {} tools", self.server_id, result.tools.len());
        Ok(result.tools)
    }

    /// Calls a tool on the MCP server.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the tool to call
    /// * `arguments` - The arguments to pass to the tool
    ///
    /// # Example
    ///
    /// ```ignore
    /// let args = serde_json::json!({ "path": "/home/user/file.txt" });
    /// let result = client.call_tool("read_file", args).await?;
    /// println!("Result: {:?}", result);
    /// ```
    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: JsonValue,
    ) -> Result<CallToolResult, McpError> {
        if !self.initialized {
            return Err(McpError::Protocol("Server not initialized".to_string()));
        }

        let request = CallToolRequest::with_arguments(name, arguments);
        let result: CallToolResult = self.transport
            .send_request("tools/call", Some(serde_json::to_value(request)?))
            .await?;

        if result.is_error.unwrap_or(false) {
            let error_msg = result.content.iter()
                .filter_map(|c| c.text.as_ref())
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            warn!("[MCP:{}] Tool '{}' returned error: {}", self.server_id, name, error_msg);
        }

        Ok(result)
    }

    /// Shuts down the MCP server gracefully.
    ///
    /// Sends a shutdown request and waits for the process to exit (stdio only).
    pub async fn shutdown(&mut self) -> Result<(), McpError> {
        if !self.initialized {
            return Ok(());
        }

        info!("[MCP:{}] Shutting down", self.server_id);

        // For stdio transport, send shutdown request
        if !self.is_remote {
            let _: JsonValue = self.transport.send_request("shutdown", None).await?;
            self.transport.send_notification("exit", None).await?;
        }

        self.transport.close().await?;

        self.initialized = false;
        Ok(())
    }

    /// Returns whether the client has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Returns the server ID.
    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    /// Returns whether this is a remote server.
    pub fn is_remote(&self) -> bool {
        self.is_remote
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_is_local() {
        let config = McpServerConfig::new("test", "test-server");
        assert!(config.is_local());
        assert!(!config.is_remote());
    }

    #[test]
    fn test_config_is_remote() {
        let config = McpServerConfig::remote("test", "https://api.example.com/mcp");
        assert!(config.is_remote());
        assert!(!config.is_local());
    }
}
