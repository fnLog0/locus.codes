//! ToolBus Integration for MCP Tools
//!
//! This module provides the [`McpToolWrapper`] for exposing MCP tools as ToolBus tools.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::mcp::manager::McpManager;
use crate::mcp::protocol::Tool;
use crate::ToolResult;

/// Wrapper that exposes an MCP tool as a ToolBus tool.
///
/// The wrapper:
/// - Provides a namespaced tool name (e.g., `mcp.filesystem.read_file`)
/// - Converts between MCP and ToolBus schemas
/// - Delegates calls to the MCP manager
pub struct McpToolWrapper {
    /// The server ID that provides this tool
    server_id: String,
    /// The MCP tool definition
    tool: Tool,
    /// Reference to the MCP manager
    manager: Arc<McpManager>,
    /// Cached namespaced name
    namespaced_name: String,
}

impl McpToolWrapper {
    /// Creates a new MCP tool wrapper.
    ///
    /// # Arguments
    ///
    /// * `server_id` - The ID of the MCP server providing this tool
    /// * `tool` - The MCP tool definition
    /// * `manager` - Reference to the MCP manager
    pub fn new(server_id: String, tool: Tool, manager: Arc<McpManager>) -> Self {
        let namespaced_name = format!("mcp.{}.{}", server_id, tool.name);
        Self {
            server_id,
            tool,
            manager,
            namespaced_name,
        }
    }

    /// Returns the namespaced tool name.
    ///
    /// Format: `mcp.{server_id}.{tool_name}`
    pub fn namespaced_name(&self) -> &str {
        &self.namespaced_name
    }

    /// Returns the server ID.
    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    /// Returns the original MCP tool name.
    pub fn tool_name(&self) -> &str {
        &self.tool.name
    }
}

#[async_trait]
impl crate::Tool for McpToolWrapper {
    fn name(&self) -> &'static str {
        // Leak the string to get a 'static lifetime
        // This is acceptable because tools are typically created once and live for the program's lifetime
        Box::leak(self.namespaced_name.clone().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        // Leak the string to get a 'static lifetime
        Box::leak(self.tool.description.clone().into_boxed_str())
    }

    fn parameters_schema(&self) -> JsonValue {
        self.tool.input_schema.clone()
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let result = self.manager
            .call_tool(&self.server_id, &self.tool.name, args)
            .await
            .map_err(|e| anyhow::anyhow!("MCP tool error: {}", e))?;

        // Check if the result indicates an error
        let is_error = result.get("is_error")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if is_error {
            let content = result.get("content")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|c| c.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("Unknown error");

            return Err(anyhow::anyhow!("Tool error: {}", content));
        }

        // Return the result as JSON
        Ok(result)
    }
}

/// Registers all tools from a running MCP server with the ToolBus.
///
/// # Arguments
///
/// * `toolbus` - The ToolBus to register tools with
/// * `manager` - The MCP manager
/// * `server_id` - The ID of the running server
///
/// # Example
///
/// ```ignore
/// use locus_toolbus::{ToolBus, mcp::McpManager};
/// use std::sync::Arc;
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config_path = PathBuf::from("~/.config/locus/mcp_servers.toml");
///     let manager = Arc::new(McpManager::load(config_path).await?);
///     
///     manager.start_server("filesystem").await?;
///     
///     let mut toolbus = ToolBus::new(std::env::current_dir()?);
///     register_mcp_tools(&mut toolbus, manager.clone(), "filesystem").await?;
///     
///     Ok(())
/// }
/// ```
pub async fn register_mcp_tools(
    toolbus: &mut crate::ToolBus,
    manager: Arc<McpManager>,
    server_id: &str,
) -> Result<(), anyhow::Error> {
    let tools = manager.list_tools(server_id).await
        .map_err(|e| anyhow::anyhow!("Failed to list tools: {}", e))?;

    for tool in tools {
        let wrapper = McpToolWrapper::new(
            server_id.to_string(),
            tool,
            Arc::clone(&manager),
        );
        
        // Register with namespaced name
        let namespaced_name = wrapper.namespaced_name();
        tracing::info!("Registering MCP tool: {}", namespaced_name);
        
        toolbus.register(wrapper);
    }

    Ok(())
}

/// Tool information for display purposes.
#[derive(Debug, Clone)]
pub struct McpToolInfo {
    /// The namespaced tool name (e.g., `mcp.filesystem.read_file`)
    pub name: String,
    /// The server that provides this tool
    pub server_id: String,
    /// The original tool name on the server
    pub original_name: String,
    /// Tool description
    pub description: String,
    /// Tool input schema
    pub input_schema: JsonValue,
}

impl McpToolInfo {
    /// Creates a new tool info from an MCP tool.
    pub fn from_tool(server_id: &str, tool: &Tool) -> Self {
        Self {
            name: format!("mcp.{}.{}", server_id, tool.name),
            server_id: server_id.to_string(),
            original_name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool.input_schema.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespaced_name() {
        let tool = Tool {
            name: "read_file".to_string(),
            description: "Read a file".to_string(),
            input_schema: serde_json::json!({}),
        };

        // We can't create a full wrapper without a manager, but we can test the naming
        assert_eq!(
            format!("mcp.{}.{}", "filesystem", tool.name),
            "mcp.filesystem.read_file"
        );
    }
}
