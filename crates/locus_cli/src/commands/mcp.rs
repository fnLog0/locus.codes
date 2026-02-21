//! MCP server management commands.

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use locus_toolbus::mcp::{
    McpAuthConfig, McpManager, McpServerConfig,
    TransportType,
};

use crate::cli::McpAction;
use crate::output;

/// Returns the default MCP configuration path.
fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("locus")
        .join("mcp_servers.toml")
}

/// Handles MCP CLI actions.
pub async fn handle(action: McpAction) -> Result<()> {
    let manager = McpManager::load(config_path()).await?;

    match action {
        McpAction::Add {
            id,
            name,
            command,
            url,
            transport,
            args,
            env,
            auth_type,
            auth_token,
            auto_start,
        } => add_server(manager, id, name, command, url, transport, args, env, auth_type, auth_token, auto_start).await,
        McpAction::List { detailed } => list_servers(manager, detailed).await,
        McpAction::Remove { server_id } => remove_server(manager, server_id).await,
        McpAction::Start { server_id } => start_server(manager, server_id).await,
        McpAction::Stop { server_id } => stop_server(manager, server_id).await,
        McpAction::Test { server_id } => test_server(manager, server_id).await,
        McpAction::Info { server_id } => show_server_info(manager, server_id).await,
        McpAction::Call { tool, args } => call_mcp_tool(manager, tool, args).await,
    }
}

async fn add_server(
    manager: McpManager,
    id: String,
    name: String,
    command: Option<String>,
    url: Option<String>,
    transport: Option<String>,
    args: Vec<String>,
    env: Vec<String>,
    auth_type: Option<String>,
    auth_token: Option<String>,
    auto_start: bool,
) -> Result<()> {
    // Validate: either command OR url must be set (not both)
    match (&command, &url) {
        (Some(_), Some(_)) => {
            return Err(anyhow!("Cannot specify both --command and --url. Use --command for local servers, --url for remote servers."));
        }
        (None, None) => {
            return Err(anyhow!("Either --command (for local servers) or --url (for remote servers) is required."));
        }
        _ => {}
    }

    // Parse environment variables
    let mut env_map = std::collections::HashMap::new();
    for env_str in env {
        let parts: Vec<&str> = env_str.splitn(2, '=').collect();
        if parts.len() == 2 {
            env_map.insert(parts[0].to_string(), parts[1].to_string());
        } else {
            return Err(anyhow!("Invalid environment variable format: {}. Expected KEY=VALUE", env_str));
        }
    }

    // Create auth config if provided
    let auth = match (auth_type, auth_token) {
        (Some(auth_type), Some(token)) => Some(McpAuthConfig::new(auth_type, token)),
        (None, Some(_)) => {
            return Err(anyhow!("--auth-type is required when --auth-token is provided"));
        }
        _ => None,
    };

    // Parse transport type
    let transport_type = match transport.as_deref() {
        Some("stdio") => TransportType::Stdio,
        Some("sse") => TransportType::Sse,
        Some(other) => {
            return Err(anyhow!("Invalid transport type: '{}'. Expected 'stdio' or 'sse'.", other));
        }
        None => {
            // Auto-detect based on command vs url
            if url.is_some() {
                TransportType::Sse
            } else {
                TransportType::Stdio
            }
        }
    };

    // Create config based on whether it's local or remote
    let config = if let Some(cmd) = command {
        McpServerConfig::new(&id, cmd)
            .with_name(&name)
            .with_args(args)
            .with_auto_start(auto_start)
            .with_transport(transport_type)
    } else if let Some(remote_url) = url {
        McpServerConfig::remote(&id, remote_url)
            .with_name(&name)
            .with_auto_start(auto_start)
            .with_transport(transport_type)
    } else {
        unreachable!("Already validated that command or url is set");
    };

    // Add auth if provided
    let config = if let Some(auth) = auth {
        config.with_auth(auth)
    } else {
        config
    };

    manager.add_server(config).await?;

    output::success(&format!("Added MCP server: {} ({})", name, id));
    output::dim(&format!("Start it with: locus mcp start {}", id));

    Ok(())
}

async fn list_servers(manager: McpManager, detailed: bool) -> Result<()> {
    let servers = manager.list_servers().await;
    let running = manager.list_running().await;

    if servers.is_empty() {
        output::warning("No MCP servers configured.");
        output::dim("Add one with: locus mcp add --id <id> --name <name> --command <command>");
        return Ok(());
    }

    output::header("MCP Servers");

    for server in servers {
        let status = if running.contains(&server.id) {
            "🟢 running"
        } else if server.auto_start {
            "⚪ auto-start"
        } else {
            "⚫ stopped"
        };

        if detailed {
            output::kv("ID:", &server.id);
            output::kv("Name:", &server.name);
            if server.is_remote() {
                output::kv("URL:", server.url.as_deref().unwrap_or(""));
                output::kv("Transport:", &server.transport.to_string());
            } else {
                output::kv("Command:", &server.command);
                if !server.args.is_empty() {
                    output::kv("Args:", &server.args.join(" "));
                }
            }
            output::kv("Status:", status);
            output::kv("Auto-start:", &server.auto_start.to_string());
            if let Some(auth) = &server.auth {
                output::kv("Auth:", &auth.auth_type);
            }
            println!();
        } else {
            let endpoint = if server.is_remote() {
                server.url.as_deref().unwrap_or("").to_string()
            } else {
                server.command.clone()
            };
            println!(
                "  {} {} - {} ({})",
                status,
                server.id,
                server.name,
                endpoint
            );
        }
    }

    Ok(())
}

async fn remove_server(manager: McpManager, server_id: String) -> Result<()> {
    manager.remove_server(&server_id).await?;
    output::success(&format!("Removed MCP server: {}", server_id));
    Ok(())
}

async fn start_server(manager: McpManager, server_id: String) -> Result<()> {
    output::dim(&format!("Starting MCP server: {}...", server_id));
    manager.start_server(&server_id).await?;
    output::success(&format!("Started MCP server: {}", server_id));

    // Show available tools
    let tools = manager.list_tools(&server_id).await?;
    if !tools.is_empty() {
        println!();
        output::header("Available Tools");
        for tool in tools {
            println!("  • {} - {}", tool.name, tool.description.lines().next().unwrap_or(""));
        }
    }

    Ok(())
}

async fn stop_server(manager: McpManager, server_id: String) -> Result<()> {
    output::dim(&format!("Stopping MCP server: {}...", server_id));
    manager.stop_server(&server_id).await?;
    output::success(&format!("Stopped MCP server: {}", server_id));
    Ok(())
}

async fn test_server(manager: McpManager, server_id: String) -> Result<()> {
    output::dim(&format!("Testing MCP server: {}...", server_id));

    let result = manager.test_server(&server_id).await?;

    if result.success {
        output::success("Connection successful!");
        if let Some(name) = result.server_name {
            output::kv("Server:", &name);
        }
        if let Some(version) = result.server_version {
            output::kv("Version:", &version);
        }
        output::kv("Tools:", &result.tool_count.to_string());
    } else {
        output::error(&result.error.unwrap_or_else(|| "Unknown error".to_string()));
        return Err(anyhow!("Server test failed"));
    }

    Ok(())
}

async fn show_server_info(manager: McpManager, server_id: String) -> Result<()> {
    let config = manager.get_config(&server_id).await
        .ok_or_else(|| anyhow!("Server not found: {}", server_id))?;

    let is_running = manager.is_running(&server_id).await;

    output::header(&format!("MCP Server: {}", config.name));
    output::kv("ID:", &config.id);

    if config.is_remote() {
        output::kv("URL:", config.url.as_deref().unwrap_or(""));
        output::kv("Transport:", &config.transport.to_string());
    } else {
        output::kv("Command:", &config.command);

        if !config.args.is_empty() {
            output::kv("Arguments:", &config.args.join(" "));
        }

        if !config.env.is_empty() {
            output::kv("Environment:", "");
            for (key, value) in &config.env {
                println!("    {} = {}", key, value);
            }
        }
    }

    output::kv("Status:", if is_running { "running" } else { "stopped" });
    output::kv("Auto-start:", &config.auto_start.to_string());
    output::kv("Restart Policy:", &format!("{:?}", config.restart_policy));

    if let Some(auth) = &config.auth {
        output::kv("Authentication:", "");
        output::kv("  Type:", &auth.auth_type);
        output::kv("  Header:", auth.header_name());
    }

    // Show tools if running
    if is_running {
        println!();
        output::header("Available Tools");

        let tools = manager.list_tools(&server_id).await?;
        if tools.is_empty() {
            output::dim("  No tools available");
        } else {
            for tool in tools {
                println!("\n  📦 {}", tool.name);
                println!("     {}", tool.description.lines().next().unwrap_or(""));
            }
        }
    }

    Ok(())
}

async fn call_mcp_tool(manager: McpManager, tool: String, args: String) -> Result<()> {
    // Parse the tool name (format: server_id.tool_name or mcp.server_id.tool_name)
    let parts: Vec<&str> = tool.split('.').collect();
    let (server_id, tool_name) = match parts.as_slice() {
        ["mcp", server_id, tool_name] => (*server_id, *tool_name),
        [server_id, tool_name] => (*server_id, *tool_name),
        [_tool_name] => {
            // Try to find the server that has this tool
            return Err(anyhow!(
                "Tool name must include server ID. Format: server_id.tool_name or mcp.server_id.tool_name"
            ));
        }
        _ => {
            return Err(anyhow!(
                "Invalid tool name format. Use: server_id.tool_name or mcp.server_id.tool_name"
            ));
        }
    };

    // Parse arguments
    let args_value: serde_json::Value = serde_json::from_str(&args)
        .map_err(|e| anyhow!("Invalid JSON arguments: {}", e))?;

    // Check if server is running
    if !manager.is_running(server_id).await {
        output::dim(&format!("Server '{}' is not running. Starting...", server_id));
        manager.start_server(server_id).await?;
    }

    output::dim(&format!("Calling tool: {} on server: {}", tool_name, server_id));

    let result = manager.call_tool(server_id, tool_name, args_value).await?;

    // Display result
    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        for item in content {
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                println!("{}", text);
            } else {
                println!("{}", serde_json::to_string_pretty(item)?);
            }
        }
    } else {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    Ok(())
}
