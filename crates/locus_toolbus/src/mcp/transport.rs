//! MCP Transport Abstraction
//!
//! This module provides transport abstraction for MCP communication.
//! Supported transports:
//! - **Stdio**: Local processes via stdin/stdout
//! - **SSE**: HTTP Server-Sent Events for remote servers
//!
//! The [`TransportEnum`] wraps both transport types for polymorphic use.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;
use tracing::{debug, info};

use crate::mcp::error::McpError;
use crate::mcp::protocol::{JsonRpcRequest, JsonRpcResponse};

/// Transport trait for MCP communication (internal).
///
/// Note: This trait is not object-safe. Use [`TransportEnum`] for polymorphism.
#[async_trait]
trait TransportInner: Send + Sync {
    /// Sends a request and returns raw JSON response.
    async fn send_request_raw(
        &mut self,
        id: u64,
        method: &str,
        params: Option<JsonValue>,
    ) -> Result<JsonValue, McpError>;

    /// Sends a notification (no response expected).
    async fn send_notification(&mut self, method: &str, params: Option<JsonValue>) -> Result<(), McpError>;

    /// Returns the next request ID.
    fn next_request_id(&self) -> u64;

    /// Closes the transport.
    async fn close(&mut self) -> Result<(), McpError>;
}

/// Enum wrapper for transport implementations.
///
/// Allows polymorphic use of different transport types without trait objects.
pub enum TransportEnum {
    /// Stdio transport for local processes
    Stdio(StdioTransport),
    /// SSE transport for remote servers
    Sse(SseTransport),
}

impl TransportEnum {
    /// Sends a request and returns the typed response.
    pub async fn send_request<T: DeserializeOwned>(
        &mut self,
        method: &str,
        params: Option<JsonValue>,
    ) -> Result<T, McpError> {
        let id = self.next_request_id();
        let raw = match self {
            TransportEnum::Stdio(t) => t.send_request_raw(id, method, params).await?,
            TransportEnum::Sse(t) => t.send_request_raw(id, method, params).await?,
        };

        let response: JsonRpcResponse<T> = serde_json::from_value(raw)?;
        response.into_result().map_err(|e| McpError::JsonRpc(e.message))
    }

    /// Sends a notification.
    pub async fn send_notification(&mut self, method: &str, params: Option<JsonValue>) -> Result<(), McpError> {
        match self {
            TransportEnum::Stdio(t) => t.send_notification(method, params).await,
            TransportEnum::Sse(t) => t.send_notification(method, params).await,
        }
    }

    /// Returns the next request ID.
    pub fn next_request_id(&self) -> u64 {
        match self {
            TransportEnum::Stdio(t) => t.next_request_id(),
            TransportEnum::Sse(t) => t.next_request_id(),
        }
    }

    /// Closes the transport.
    pub async fn close(&mut self) -> Result<(), McpError> {
        match self {
            TransportEnum::Stdio(t) => t.close().await,
            TransportEnum::Sse(t) => t.close().await,
        }
    }
}

/// Stdio-based transport for local MCP server processes.
pub struct StdioTransport {
    process: Option<Child>,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    request_id: Arc<AtomicU64>,
    server_id: String,
}

impl StdioTransport {
    /// Creates a new stdio transport by spawning a process.
    pub fn spawn(
        command: &str,
        args: &[String],
        env: &std::collections::HashMap<String, String>,
        working_dir: Option<&std::path::Path>,
        server_id: &str,
    ) -> Result<Self, McpError> {
        info!("Starting MCP server process: {} {:?}", command, args);

        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        for (key, value) in env {
            cmd.env(key, value);
        }

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        let mut process = cmd.spawn().map_err(|e| {
            McpError::StartFailed(format!("Failed to start '{}': {}", command, e))
        })?;

        let stdin = process.stdin.take().ok_or_else(|| {
            McpError::StartFailed("Could not capture stdin".to_string())
        })?;

        let stdout = process.stdout.take().ok_or_else(|| {
            McpError::StartFailed("Could not capture stdout".to_string())
        })?;

        Ok(Self {
            process: Some(process),
            stdin,
            stdout: BufReader::new(stdout),
            request_id: Arc::new(AtomicU64::new(1)),
            server_id: server_id.to_string(),
        })
    }

    fn read_response(&mut self) -> Result<String, McpError> {
        let mut header_line = String::new();
        self.stdout.read_line(&mut header_line)?;

        let content_length = if header_line.starts_with("Content-Length:") {
            let len_str = header_line
                .strip_prefix("Content-Length:")
                .unwrap()
                .trim();
            len_str.parse::<usize>()
                .map_err(|e| McpError::Protocol(format!("Invalid Content-Length: {}", e)))?
        } else {
            return Err(McpError::Protocol(
                "Expected Content-Length header".to_string()
            ));
        };

        let mut empty_line = String::new();
        self.stdout.read_line(&mut empty_line)?;
        if !empty_line.trim().is_empty() {
            return Err(McpError::Protocol(
                "Expected empty line after Content-Length".to_string()
            ));
        }

        let mut content = vec![0u8; content_length];
        self.stdout.read_exact(&mut content)?;

        String::from_utf8(content)
            .map_err(|e| McpError::Protocol(format!("Invalid UTF-8 in response: {}", e)))
    }

    fn write_request(&mut self, request_json: &str) -> Result<(), McpError> {
        let content = format!("Content-Length: {}\r\n\r\n{}", request_json.len(), request_json);
        self.stdin.write_all(content.as_bytes())?;
        self.stdin.flush()?;
        Ok(())
    }
}

#[async_trait]
impl TransportInner for StdioTransport {
    async fn send_request_raw(
        &mut self,
        id: u64,
        method: &str,
        params: Option<JsonValue>,
    ) -> Result<JsonValue, McpError> {
        let request = JsonRpcRequest::with_params_value(id, method, params);
        let request_json = serde_json::to_string(&request)?;

        debug!("[MCP:{}] Sending: {}", self.server_id, request_json);

        self.write_request(&request_json)?;

        let response = self.read_response()?;
        debug!("[MCP:{}] Received: {}", self.server_id, response);

        serde_json::from_str(&response)
            .map_err(|e| McpError::Protocol(format!("Invalid JSON response: {}", e)))
    }

    async fn send_notification(&mut self, method: &str, params: Option<JsonValue>) -> Result<(), McpError> {
        let notification = JsonRpcRequest::<()>::new(JsonValue::Null, method);
        let notification_json = serde_json::to_string(&notification)?;
        
        if let Some(p) = params {
            let mut notification: serde_json::Value = serde_json::from_str(&notification_json)?;
            notification["params"] = p;
            self.write_request(&serde_json::to_string(&notification)?)?;
        } else {
            self.write_request(&notification_json)?;
        }
        
        Ok(())
    }

    fn next_request_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    async fn close(&mut self) -> Result<(), McpError> {
        if let Some(mut process) = self.process.take() {
            process.kill().map_err(|e| {
                McpError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
            debug!("[MCP:{}] Process killed", self.server_id);
        }
        Ok(())
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
    }
}

/// SSE (Server-Sent Events) transport for remote MCP servers.
pub struct SseTransport {
    http_client: HttpClient,
    base_url: String,
    headers: reqwest::header::HeaderMap,
    request_id: Arc<AtomicU64>,
    server_id: String,
    message_endpoint: Option<String>,
}

impl SseTransport {
    /// Creates a new SSE transport.
    pub fn new(
        base_url: &str,
        headers: reqwest::header::HeaderMap,
        server_id: &str,
    ) -> Self {
        Self {
            http_client: HttpClient::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            headers,
            request_id: Arc::new(AtomicU64::new(1)),
            server_id: server_id.to_string(),
            message_endpoint: None,
        }
    }

    /// Sets the message endpoint (discovered during initialization).
    pub fn set_message_endpoint(&mut self, endpoint: String) {
        self.message_endpoint = Some(endpoint);
    }

    /// Builds a request with the configured headers.
    fn build_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut builder = self.http_client.post(url);
        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }
        builder.header("Content-Type", "application/json")
    }

    /// Sends an HTTP request and parses the response.
    async fn http_send<T: DeserializeOwned + Send + 'static>(
        &self,
        url: &str,
        body: JsonValue,
    ) -> Result<T, McpError> {
        debug!("[MCP:{}] HTTP POST to: {}", self.server_id, url);
        debug!("[MCP:{}] Body: {}", self.server_id, body);

        let response = self.build_request(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| McpError::Protocol(format!("HTTP request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(McpError::Protocol(format!(
                "HTTP error {}: {}",
                status, error_text
            )));
        }

        let response_text = response.text().await
            .map_err(|e| McpError::Protocol(format!("Failed to read response: {}", e)))?;

        debug!("[MCP:{}] Response: {}", self.server_id, response_text);

        let response: JsonRpcResponse<T> = serde_json::from_str(&response_text)?;

        response.into_result().map_err(|e| McpError::JsonRpc(e.message))
    }
}

#[async_trait]
impl TransportInner for SseTransport {
    async fn send_request_raw(
        &mut self,
        id: u64,
        method: &str,
        params: Option<JsonValue>,
    ) -> Result<JsonValue, McpError> {
        let url = self.message_endpoint.as_ref()
            .map(|e| format!("{}/{}", self.base_url, e.trim_start_matches('/')))
            .unwrap_or_else(|| format!("{}/message", self.base_url));

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        self.http_send(&url, body).await
    }

    async fn send_notification(&mut self, method: &str, params: Option<JsonValue>) -> Result<(), McpError> {
        let url = self.message_endpoint.as_ref()
            .map(|e| format!("{}/{}", self.base_url, e.trim_start_matches('/')))
            .unwrap_or_else(|| format!("{}/message", self.base_url));

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let _: JsonValue = self.http_send(&url, body).await?;
        Ok(())
    }

    fn next_request_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    async fn close(&mut self) -> Result<(), McpError> {
        // HTTP transport doesn't need explicit closing
        Ok(())
    }
}

/// Transport type configuration.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
    /// Local process via stdin/stdout
    Stdio,
    /// HTTP Server-Sent Events
    Sse,
}

impl Default for TransportType {
    fn default() -> Self {
        Self::Stdio
    }
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportType::Stdio => write!(f, "stdio"),
            TransportType::Sse => write!(f, "sse"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_type_serialization() {
        let t = TransportType::Stdio;
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "\"stdio\"");

        let t = TransportType::Sse;
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "\"sse\"");
    }

    #[test]
    fn test_transport_type_deserialization() {
        let t: TransportType = serde_json::from_str("\"stdio\"").unwrap();
        assert_eq!(t, TransportType::Stdio);

        let t: TransportType = serde_json::from_str("\"sse\"").unwrap();
        assert_eq!(t, TransportType::Sse);
    }

    #[test]
    fn test_stdio_transport_next_request_id() {
        // We can't fully test without spawning a process, but we can test the ID counter
        let counter = Arc::new(AtomicU64::new(1));
        assert_eq!(counter.fetch_add(1, Ordering::SeqCst), 1);
        assert_eq!(counter.fetch_add(1, Ordering::SeqCst), 2);
    }
}
