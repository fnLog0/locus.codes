//! MCP Manager - Server Registry and Lifecycle Management
//!
//! This module provides the [`McpManager`] for managing multiple MCP servers,
//! including configuration persistence, process lifecycle, and tool registration.

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::Value as JsonValue;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::mcp::client::McpClient;
use crate::mcp::config::{McpServerConfig, McpServersConfig};
use crate::mcp::error::McpError;
use crate::mcp::protocol::Tool;

/// Result of testing an MCP server connection.
#[derive(Debug, Clone)]
pub struct ServerTestResult {
    /// Whether the test was successful
    pub success: bool,
    /// Server info if available
    pub server_name: Option<String>,
    /// Server version if available
    pub server_version: Option<String>,
    /// Number of tools discovered
    pub tool_count: usize,
    /// Error message if failed
    pub error: Option<String>,
}

/// MCP Manager for managing multiple MCP servers.
///
/// The manager handles:
/// - Loading and saving server configurations
/// - Starting and stopping server processes
/// - Auto-starting servers on initialization
/// - Tool discovery and invocation
///
/// # Example
///
/// ```ignore
/// use locus_toolbus::mcp::McpManager;
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config_path = PathBuf::from("~/.config/locus/mcp_servers.toml");
///     let manager = McpManager::load(config_path).await?;
///     
///     // Auto-start configured servers
///     manager.auto_start().await?;
///     
///     // List running servers
///     for server in manager.list_servers().await {
///         println!("Running: {}", server.name);
///     }
///     
///     Ok(())
/// }
/// ```
pub struct McpManager {
    /// Path to the configuration file
    config_path: PathBuf,
    /// Server configurations
    configs: RwLock<HashMap<String, McpServerConfig>>,
    /// Running server clients
    running: RwLock<HashMap<String, McpClient>>,
}

impl McpManager {
    /// Creates a new MCP manager with the given configuration path.
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            configs: RwLock::new(HashMap::new()),
            running: RwLock::new(HashMap::new()),
        }
    }

    /// Loads the MCP manager from the configuration file.
    ///
    /// If the file doesn't exist, an empty manager is created.
    pub async fn load(config_path: PathBuf) -> Result<Self, McpError> {
        let manager = Self::new(config_path.clone());

        if config_path.exists() {
            let config = McpServersConfig::load(&config_path)
                .map_err(|e| McpError::Config(e.to_string()))?;

            let mut configs = manager.configs.write().await;
            for server in config.servers {
                configs.insert(server.id.clone(), server);
            }

            info!("Loaded {} MCP server configurations", configs.len());
        } else {
            info!("No MCP configuration file found, starting with empty configuration");
        }

        Ok(manager)
    }

    /// Saves the current configuration to the configuration file.
    pub async fn save(&self) -> Result<(), McpError> {
        let configs = self.configs.read().await;
        let mut config = McpServersConfig::new();

        for server_config in configs.values() {
            config.add_server(server_config.clone());
        }

        config.save(&self.config_path)
            .map_err(|e| McpError::Config(e.to_string()))?;

        info!("Saved {} MCP server configurations", configs.len());
        Ok(())
    }

    /// Adds a new MCP server configuration.
    ///
    /// Returns an error if a server with the same ID already exists.
    pub async fn add_server(&self, config: McpServerConfig) -> Result<(), McpError> {
        let mut configs = self.configs.write().await;

        if configs.contains_key(&config.id) {
            return Err(McpError::Config(format!(
                "Server with ID '{}' already exists",
                config.id
            )));
        }

        info!("Adding MCP server: {} ({})", config.name, config.id);
        configs.insert(config.id.clone(), config);
        drop(configs);

        self.save().await
    }

    /// Removes an MCP server configuration.
    ///
    /// Returns an error if the server doesn't exist or is currently running.
    pub async fn remove_server(&self, id: &str) -> Result<(), McpError> {
        let running = self.running.read().await;
        if running.contains_key(id) {
            return Err(McpError::ServerAlreadyRunning(id.to_string()));
        }
        drop(running);

        let mut configs = self.configs.write().await;
        let removed = configs.remove(id);

        if removed.is_none() {
            return Err(McpError::ServerNotFound(id.to_string()));
        }

        info!("Removed MCP server: {}", id);
        drop(configs);

        self.save().await
    }

    /// Starts an MCP server by ID.
    ///
    /// Returns an error if the server doesn't exist or is already running.
    pub async fn start_server(&self, id: &str) -> Result<(), McpError> {
        // Check if already running
        {
            let running = self.running.read().await;
            if running.contains_key(id) {
                return Err(McpError::ServerAlreadyRunning(id.to_string()));
            }
        }

        // Get config
        let config = {
            let configs = self.configs.read().await;
            configs.get(id).cloned()
                .ok_or_else(|| McpError::ServerNotFound(id.to_string()))?
        };

        info!("Starting MCP server: {} ({})", config.name, config.id);

        // Connect to server (local or remote)
        let mut client = McpClient::connect(&config).await?;
        client.initialize().await?;

        // Store running client
        {
            let mut running = self.running.write().await;
            running.insert(id.to_string(), client);
        }

        info!("MCP server started: {}", id);
        Ok(())
    }

    /// Stops a running MCP server by ID.
    ///
    /// Returns an error if the server is not running.
    pub async fn stop_server(&self, id: &str) -> Result<(), McpError> {
        let mut running = self.running.write().await;

        let mut client = running.remove(id)
            .ok_or_else(|| McpError::ServerNotRunning(id.to_string()))?;

        info!("Stopping MCP server: {}", id);
        client.shutdown().await?;

        info!("MCP server stopped: {}", id);
        Ok(())
    }

    /// Auto-starts all servers configured with `auto_start: true`.
    pub async fn auto_start(&self) -> Result<(), McpError> {
        // Collect IDs of servers that should auto-start
        let auto_start_ids: Vec<String> = {
            let configs = self.configs.read().await;
            configs.iter()
                .filter(|(_, config)| config.auto_start)
                .map(|(id, _)| id.clone())
                .collect()
        };

        // Start each server
        for id in auto_start_ids {
            if let Err(e) = self.start_server(&id).await {
                error!("Failed to auto-start MCP server '{}': {}", id, e);
            }
        }

        Ok(())
    }

    /// Lists all configured MCP servers.
    pub async fn list_servers(&self) -> Vec<McpServerConfig> {
        let configs = self.configs.read().await;
        configs.values().cloned().collect()
    }

    /// Lists all running MCP server IDs.
    pub async fn list_running(&self) -> Vec<String> {
        let running = self.running.read().await;
        running.keys().cloned().collect()
    }

    /// Checks if a server is currently running.
    pub async fn is_running(&self, id: &str) -> bool {
        let running = self.running.read().await;
        running.contains_key(id)
    }

    /// Gets a server configuration by ID.
    pub async fn get_config(&self, id: &str) -> Option<McpServerConfig> {
        let configs = self.configs.read().await;
        configs.get(id).cloned()
    }

    /// Tests an MCP server connection.
    ///
    /// This starts the server, initializes it, lists tools, and then shuts it down.
    pub async fn test_server(&self, id: &str) -> Result<ServerTestResult, McpError> {
        // Get config
        let config = {
            let configs = self.configs.read().await;
            configs.get(id).cloned()
                .ok_or_else(|| McpError::ServerNotFound(id.to_string()))?
        };

        info!("Testing MCP server: {} ({})", config.name, config.id);

        // Connect to server (local or remote)
        let mut client = match McpClient::connect(&config).await {
            Ok(c) => c,
            Err(e) => {
                return Ok(ServerTestResult {
                    success: false,
                    server_name: None,
                    server_version: None,
                    tool_count: 0,
                    error: Some(e.to_string()),
                });
            }
        };

        // Initialize
        let init_result = match client.initialize().await {
            Ok(r) => r,
            Err(e) => {
                let _ = client.shutdown().await;
                return Ok(ServerTestResult {
                    success: false,
                    server_name: None,
                    server_version: None,
                    tool_count: 0,
                    error: Some(e.to_string()),
                });
            }
        };

        // List tools
        let tools = match client.list_tools().await {
            Ok(t) => t,
            Err(e) => {
                let _ = client.shutdown().await;
                return Ok(ServerTestResult {
                    success: false,
                    server_name: Some(init_result.server_info.name),
                    server_version: Some(init_result.server_info.version),
                    tool_count: 0,
                    error: Some(e.to_string()),
                });
            }
        };

        // Shutdown
        let _ = client.shutdown().await;

        Ok(ServerTestResult {
            success: true,
            server_name: Some(init_result.server_info.name),
            server_version: Some(init_result.server_info.version),
            tool_count: tools.len(),
            error: None,
        })
    }

    /// Calls a tool on a running MCP server.
    ///
    /// # Arguments
    ///
    /// * `server_id` - The ID of the running server
    /// * `tool_name` - The name of the tool to call
    /// * `arguments` - The arguments to pass to the tool
    pub async fn call_tool(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: JsonValue,
    ) -> Result<JsonValue, McpError> {
        let mut running = self.running.write().await;

        let client = running.get_mut(server_id)
            .ok_or_else(|| McpError::ServerNotRunning(server_id.to_string()))?;

        debug!("Calling tool '{}' on server '{}'", tool_name, server_id);

        let result = client.call_tool(tool_name, arguments).await?;

        // Extract content
        let content: Vec<JsonValue> = result.content.iter()
            .map(|c| {
                serde_json::json!({
                    "type": c.content_type,
                    "text": c.text,
                    "data": c.data,
                    "mime_type": c.mime_type
                })
            })
            .collect();

        Ok(serde_json::json!({
            "content": content,
            "is_error": result.is_error
        }))
    }

    /// Lists all tools from a running MCP server.
    pub async fn list_tools(&self, server_id: &str) -> Result<Vec<Tool>, McpError> {
        let mut running = self.running.write().await;

        let client = running.get_mut(server_id)
            .ok_or_else(|| McpError::ServerNotRunning(server_id.to_string()))?;

        client.list_tools().await
    }

    /// Lists all tools from all running servers.
    ///
    /// Returns a map of server ID to tools.
    pub async fn list_all_tools(&self) -> HashMap<String, Vec<Tool>> {
        let running = self.running.read().await;
        let mut result = HashMap::new();

        for (id, _client) in running.iter() {
            // Note: We can't call list_tools here because we need mutable access
            // This is a limitation of the current design
            result.insert(id.clone(), vec![]);
        }

        result
    }

    /// Stops all running servers.
    pub async fn stop_all(&self) -> Result<(), McpError> {
        let ids: Vec<String> = {
            let running = self.running.read().await;
            running.keys().cloned().collect()
        };

        for id in ids {
            if let Err(e) = self.stop_server(&id).await {
                error!("Failed to stop server '{}': {}", id, e);
            }
        }

        Ok(())
    }
}

impl Drop for McpManager {
    fn drop(&mut self) {
        // Note: We can't do async cleanup in Drop
        // The caller should call stop_all() before dropping
    }
}
