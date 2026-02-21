//! MCP Server Configuration Types
//!
//! This module provides configuration types for MCP (Model Context Protocol) servers.
//! Configuration can be loaded from TOML files and supports environment variable
//! interpolation for sensitive values like authentication tokens.
//!
//! # Example TOML Configuration
//!
//! ```toml
//! # Local MCP server (stdio transport)
//! [[servers]]
//! id = "github"
//! name = "GitHub MCP Server"
//! command = "github-mcp-server"
//! args = ["--stdio"]
//! env = { GITHUB_API_URL = "https://api.github.com" }
//! auto_start = true
//!
//! [servers.auth]
//! auth_type = "bearer"
//! token = "$GITHUB_TOKEN"
//!
//! # Remote MCP server (SSE transport)
//! [[servers]]
//! id = "remote-mcp"
//! name = "Remote MCP Server"
//! url = "https://api.example.com/mcp"
//! transport = "sse"
//!
//! [servers.auth]
//! auth_type = "bearer"
//! token = "$API_KEY"
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::fs;
use thiserror::Error;

use super::transport::TransportType;

/// Errors that can occur during configuration operations.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Failed to read the configuration file.
    #[error("Failed to read configuration file: {0}")]
    ReadError(#[from] std::io::Error),
    
    /// Failed to parse the TOML configuration.
    #[error("Failed to parse TOML configuration: {0}")]
    ParseError(#[from] toml::de::Error),
    
    /// Failed to serialize the configuration to TOML.
    #[error("Failed to serialize configuration to TOML: {0}")]
    SerializeError(#[from] toml::ser::Error),
    
    /// Environment variable not found during token resolution.
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),
}

/// Restart policy for MCP servers.
/// 
/// Defines how the server should behave when it stops or crashes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    /// Never restart the server automatically.
    Never,
    
    /// Restart the server only on failure, with a maximum number of retries.
    OnFailure {
        /// Maximum number of restart attempts before giving up.
        max_retries: u32,
    },
    
    /// Always restart the server when it stops, regardless of exit status.
    Always,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self::OnFailure { max_retries: 3 }
    }
}

/// Authentication configuration for MCP servers.
/// 
/// Supports various authentication methods including bearer tokens,
/// basic authentication, and API keys. Tokens can reference environment
/// variables using the `$VAR_NAME` syntax.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpAuthConfig {
    /// The type of authentication to use.
    /// 
    /// Common values: "bearer", "basic", "api_key"
    pub auth_type: String,
    
    /// The authentication token or credentials.
    /// 
    /// Can reference environment variables using `$VAR_NAME` syntax.
    /// Use [`McpAuthConfig::resolve_token`] to get the actual value.
    pub token: String,
    
    /// Optional custom header name for the authentication.
    /// 
    /// If not specified, defaults based on `auth_type`:
    /// - "bearer" -> "Authorization"
    /// - "basic" -> "Authorization"
    /// - "api_key" -> "X-API-Key"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
}

impl McpAuthConfig {
    /// Creates a new authentication configuration.
    pub fn new(auth_type: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            auth_type: auth_type.into(),
            token: token.into(),
            header: None,
        }
    }
    
    /// Creates a bearer token authentication configuration.
    pub fn bearer(token: impl Into<String>) -> Self {
        Self {
            auth_type: "bearer".to_string(),
            token: token.into(),
            header: Some("Authorization".to_string()),
        }
    }
    
    /// Creates an API key authentication configuration.
    pub fn api_key(key: impl Into<String>) -> Self {
        Self {
            auth_type: "api_key".to_string(),
            token: key.into(),
            header: Some("X-API-Key".to_string()),
        }
    }
    
    /// Resolves the token value, expanding environment variable references.
    /// 
    /// Environment variables are referenced using `$VAR_NAME` syntax.
    /// If the entire token is an environment variable reference (e.g., `$GITHUB_TOKEN`),
    /// the value of that variable is returned.
    /// 
    /// # Errors
    /// 
    /// Returns an error if a referenced environment variable is not set.
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use locus_toolbus::mcp::config::McpAuthConfig;
    /// 
    /// let auth = McpAuthConfig::bearer("$GITHUB_TOKEN");
    /// let resolved = auth.resolve_token()?;
    /// # Ok::<(), locus_toolbus::mcp::config::ConfigError>(())
    /// ```
    pub fn resolve_token(&self) -> Result<String, ConfigError> {
        let token = &self.token;
        
        // Check if the entire token is an environment variable reference
        if token.starts_with('$') {
            let var_name = &token[1..];
            env::var(var_name).map_err(|_| {
                ConfigError::EnvVarNotFound(var_name.to_string())
            })
        } else {
            // Handle inline environment variable references like "Bearer $TOKEN"
            let mut result = token.clone();
            for (key, value) in env::vars() {
                let placeholder = format!("${}", key);
                result = result.replace(&placeholder, &value);
            }
            Ok(result)
        }
    }
    
    /// Returns the header name to use for authentication.
    /// 
    /// If a custom header is specified, returns that. Otherwise, returns
    /// a default based on the authentication type.
    pub fn header_name(&self) -> &str {
        self.header.as_deref().unwrap_or_else(|| {
            match self.auth_type.as_str() {
                "bearer" | "basic" => "Authorization",
                "api_key" => "X-API-Key",
                _ => "Authorization",
            }
        })
    }
    
    /// Returns the formatted header value for authentication.
    /// 
    /// Resolves the token and formats it according to the authentication type.
    pub fn header_value(&self) -> Result<String, ConfigError> {
        let token = self.resolve_token()?;
        
        match self.auth_type.as_str() {
            "bearer" => Ok(format!("Bearer {}", token)),
            "basic" => Ok(format!("Basic {}", token)),
            _ => Ok(token),
        }
    }
}

/// Configuration for a single MCP server.
///
/// Contains all the information needed to start and manage an MCP server,
/// supporting both local processes (stdio transport) and remote servers (SSE transport).
///
/// # Local Server (stdio)
///
/// ```toml
/// [[servers]]
/// id = "local"
/// name = "Local Server"
/// command = "mcp-server"
/// args = ["--stdio"]
/// ```
///
/// # Remote Server (SSE)
///
/// ```toml
/// [[servers]]
/// id = "remote"
/// name = "Remote Server"
/// url = "https://api.example.com/mcp"
/// transport = "sse"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Unique identifier for this server configuration.
    ///
    /// Used to reference the server in logs and API calls.
    pub id: String,

    /// Human-readable name for the server.
    ///
    /// Displayed in UI and logs for easier identification.
    pub name: String,

    /// The command to execute to start the MCP server (for stdio transport).
    ///
    /// This can be a path to an executable or a command name
    /// that will be resolved using the system PATH.
    ///
    /// For remote servers, this field is optional.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub command: String,

    /// URL for remote MCP servers (for SSE transport).
    ///
    /// When specified, the client will connect to this URL instead of
    /// spawning a local process.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Transport type for communication.
    ///
    /// - `stdio`: Local process via stdin/stdout (default if `command` is set)
    /// - `sse`: HTTP Server-Sent Events (required if `url` is set)
    #[serde(default)]
    pub transport: TransportType,

    /// Command-line arguments to pass to the server (stdio only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    /// Environment variables to set for the server process (stdio only).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,

    /// Working directory for the server process (stdio only).
    ///
    /// If not specified, the current directory will be used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<std::path::PathBuf>,

    /// Authentication configuration for the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<McpAuthConfig>,

    /// Whether to automatically start this server when the toolbus initializes.
    #[serde(default = "default_auto_start")]
    pub auto_start: bool,

    /// The restart policy for this server.
    #[serde(default)]
    pub restart_policy: RestartPolicy,
}

fn default_auto_start() -> bool {
    true
}

impl McpServerConfig {
    /// Creates a new local MCP server configuration with the given ID and command.
    pub fn new(id: impl Into<String>, command: impl Into<String>) -> Self {
        let id = id.into();
        let name = id.clone();
        Self {
            id,
            name,
            command: command.into(),
            url: None,
            transport: TransportType::Stdio,
            args: Vec::new(),
            env: HashMap::new(),
            working_dir: None,
            auth: None,
            auto_start: true,
            restart_policy: RestartPolicy::default(),
        }
    }

    /// Creates a new remote MCP server configuration with the given ID and URL.
    pub fn remote(id: impl Into<String>, url: impl Into<String>) -> Self {
        let id = id.into();
        let name = id.clone();
        Self {
            id,
            name,
            command: String::new(),
            url: Some(url.into()),
            transport: TransportType::Sse,
            args: Vec::new(),
            env: HashMap::new(),
            working_dir: None,
            auth: None,
            auto_start: true,
            restart_policy: RestartPolicy::default(),
        }
    }

    /// Sets the human-readable name for the server.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Adds command-line arguments to the server configuration.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Adds an environment variable to the server configuration.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Sets the working directory for the server process.
    pub fn with_working_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Sets the authentication configuration for the server.
    pub fn with_auth(mut self, auth: McpAuthConfig) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Sets whether the server should auto-start.
    pub fn with_auto_start(mut self, auto_start: bool) -> Self {
        self.auto_start = auto_start;
        self
    }

    /// Sets the restart policy for the server.
    pub fn with_restart_policy(mut self, policy: RestartPolicy) -> Self {
        self.restart_policy = policy;
        self
    }

    /// Sets the transport type for the server.
    pub fn with_transport(mut self, transport: TransportType) -> Self {
        self.transport = transport;
        self
    }

    /// Returns true if this is a remote server configuration.
    pub fn is_remote(&self) -> bool {
        self.url.is_some()
    }

    /// Returns true if this is a local server configuration.
    pub fn is_local(&self) -> bool {
        !self.command.is_empty() && self.url.is_none()
    }

    /// Returns the resolved authentication token, if authentication is configured.
    ///
    /// Returns `None` if no authentication is configured.
    /// Returns an error if the token cannot be resolved (e.g., missing env var).
    pub fn resolve_token(&self) -> Result<Option<String>, ConfigError> {
        match &self.auth {
            Some(auth) => Ok(Some(auth.resolve_token()?)),
            None => Ok(None),
        }
    }
}

/// Root configuration for MCP servers loaded from a TOML file.
/// 
/// This is the top-level structure that contains all MCP server configurations.
/// It supports loading from and saving to TOML files.
/// 
/// # File Format
/// 
/// ```toml
/// [[servers]]
/// id = "server1"
/// name = "First Server"
/// command = "mcp-server"
/// # ... additional fields
/// 
/// [[servers]]
/// id = "server2"
/// name = "Second Server"
/// command = "another-server"
/// # ... additional fields
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct McpServersConfig {
    /// List of MCP server configurations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub servers: Vec<McpServerConfig>,
}

impl McpServersConfig {
    /// Creates an empty configuration.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Loads MCP server configuration from a TOML file.
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML content is invalid
    /// - The configuration structure is invalid
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use std::path::Path;
    /// use locus_toolbus::mcp::config::McpServersConfig;
    /// 
    /// let config = McpServersConfig::load(Path::new("mcp_servers.toml"))?;
    /// println!("Loaded {} server configurations", config.servers.len());
    /// # Ok::<(), locus_toolbus::mcp::config::ConfigError>(())
    /// ```
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Saves the MCP server configuration to a TOML file.
    /// 
    /// Creates parent directories if they don't exist.
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - Parent directories cannot be created
    /// - The file cannot be written
    /// - The configuration cannot be serialized to TOML
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use std::path::Path;
    /// use locus_toolbus::mcp::config::{McpServersConfig, McpServerConfig};
    /// 
    /// let mut config = McpServersConfig::new();
    /// config.servers.push(McpServerConfig::new("github", "github-mcp-server"));
    /// config.save(Path::new("mcp_servers.toml"))?;
    /// # Ok::<(), locus_toolbus::mcp::config::ConfigError>(())
    /// ```
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
    
    /// Adds a server configuration to the list.
    pub fn add_server(&mut self, config: McpServerConfig) -> &mut Self {
        self.servers.push(config);
        self
    }
    
    /// Finds a server configuration by ID.
    pub fn find_server(&self, id: &str) -> Option<&McpServerConfig> {
        self.servers.iter().find(|s| s.id == id)
    }
    
    /// Finds a mutable server configuration by ID.
    pub fn find_server_mut(&mut self, id: &str) -> Option<&mut McpServerConfig> {
        self.servers.iter_mut().find(|s| s.id == id)
    }
    
    /// Removes a server configuration by ID.
    /// 
    /// Returns the removed configuration if found.
    pub fn remove_server(&mut self, id: &str) -> Option<McpServerConfig> {
        let pos = self.servers.iter().position(|s| s.id == id)?;
        Some(self.servers.remove(pos))
    }
    
    /// Returns the number of server configurations.
    pub fn len(&self) -> usize {
        self.servers.len()
    }
    
    /// Returns true if there are no server configurations.
    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }
    
    /// Returns an iterator over server configurations that should auto-start.
    pub fn auto_start_servers(&self) -> impl Iterator<Item = &McpServerConfig> {
        self.servers.iter().filter(|s| s.auto_start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restart_policy_serialization() {
        let policy = RestartPolicy::OnFailure { max_retries: 5 };
        let toml = toml::to_string(&policy).unwrap();
        assert!(toml.contains("on_failure"));
        
        let deserialized: RestartPolicy = toml::from_str(&toml).unwrap();
        assert_eq!(policy, deserialized);
    }
    
    #[test]
    fn test_auth_config_resolve_token() {
        // SAFETY: This is a test and we're setting/removing a unique test variable
        unsafe {
            env::set_var("TEST_TOKEN", "secret_value");
        }

        let auth = McpAuthConfig::bearer("$TEST_TOKEN");
        let resolved = auth.resolve_token().unwrap();
        assert_eq!(resolved, "secret_value");

        // SAFETY: This is a test and we're cleaning up the variable we set
        unsafe {
            env::remove_var("TEST_TOKEN");
        }
    }
    
    #[test]
    fn test_auth_config_missing_env_var() {
        let auth = McpAuthConfig::bearer("$NONEXISTENT_TOKEN_12345");
        let result = auth.resolve_token();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_server_config_builder() {
        let config = McpServerConfig::new("test", "test-server")
            .with_name("Test Server")
            .with_args(vec!["--port".to_string(), "8080".to_string()])
            .with_env("DEBUG", "true")
            .with_auto_start(false);
        
        assert_eq!(config.id, "test");
        assert_eq!(config.name, "Test Server");
        assert_eq!(config.args, vec!["--port", "8080"]);
        assert_eq!(config.env.get("DEBUG"), Some(&"true".to_string()));
        assert!(!config.auto_start);
    }
    
    #[test]
    fn test_servers_config_load_save() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_config.toml");
        
        let mut config = McpServersConfig::new();
        config.add_server(McpServerConfig::new("server1", "cmd1"));
        config.add_server(McpServerConfig::new("server2", "cmd2"));
        
        config.save(&path).unwrap();
        
        let loaded = McpServersConfig::load(&path).unwrap();
        assert_eq!(loaded.servers.len(), 2);
        assert_eq!(loaded.servers[0].id, "server1");
        assert_eq!(loaded.servers[1].id, "server2");
    }
    
    #[test]
    fn test_find_server() {
        let mut config = McpServersConfig::new();
        config.add_server(McpServerConfig::new("server1", "cmd1"));
        config.add_server(McpServerConfig::new("server2", "cmd2"));
        
        assert!(config.find_server("server1").is_some());
        assert!(config.find_server("server3").is_none());
    }
    
    #[test]
    fn test_header_value_formatting() {
        let auth = McpAuthConfig::bearer("my-token");
        let value = auth.header_value().unwrap();
        assert_eq!(value, "Bearer my-token");
        
        let auth = McpAuthConfig::api_key("my-api-key");
        let value = auth.header_value().unwrap();
        assert_eq!(value, "my-api-key");
    }
}
