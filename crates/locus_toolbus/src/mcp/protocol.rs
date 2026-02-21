//! MCP (Model Context Protocol) JSON-RPC 2.0 types.
//!
//! This module defines the core types for MCP protocol communication,
//! following the JSON-RPC 2.0 specification.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// =============================================================================
// JSON-RPC 2.0 Core Types
// =============================================================================

/// A JSON-RPC 2.0 request.
///
/// Represents a request object sent to the server, containing a method name
/// and optional parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest<T = Value> {
    /// JSON-RPC version, always "2.0"
    pub jsonrpc: String,
    /// Request identifier (string or number)
    pub id: Value,
    /// Method name to invoke
    pub method: String,
    /// Optional parameters for the method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

impl<T> JsonRpcRequest<T> {
    /// Creates a new JSON-RPC request with the given id and method.
    pub fn new(id: impl Into<Value>, method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.into(),
            params: None,
        }
    }

    /// Creates a new JSON-RPC request with parameters.
    pub fn with_params(id: impl Into<Value>, method: impl Into<String>, params: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.into(),
            params: Some(params),
        }
    }
}

impl JsonRpcRequest<Option<Value>> {
    /// Creates a new JSON-RPC request with optional params as Value.
    pub fn with_params_value(id: impl Into<Value>, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.into(),
            params: Some(params),
        }
    }
}

/// A JSON-RPC 2.0 response that can be either success or error.
///
/// Represents a response object returned from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse<T = Value> {
    /// JSON-RPC version, always "2.0"
    pub jsonrpc: String,
    /// Request identifier matching the original request
    pub id: Value,
    /// The result of the method invocation (for success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    /// The error object (for errors)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcErrorObject>,
}

impl<T> JsonRpcResponse<T> {
    /// Creates a new successful JSON-RPC response with the given id and result.
    pub fn success(id: impl Into<Value>, result: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    /// Creates a new error JSON-RPC response.
    pub fn error(id: impl Into<Value>, error: JsonRpcErrorObject) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: None,
            error: Some(error),
        }
    }

    /// Returns the result if this is a successful response.
    pub fn into_result(self) -> Result<T, JsonRpcErrorObject> {
        if let Some(error) = self.error {
            Err(error)
        } else if let Some(result) = self.result {
            Ok(result)
        } else {
            Err(JsonRpcErrorObject::new(-32603, "Invalid response: neither result nor error present"))
        }
    }
}

/// A JSON-RPC 2.0 error response.
///
/// Represents an error response when a method invocation fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// JSON-RPC version, always "2.0"
    pub jsonrpc: String,
    /// Request identifier matching the original request
    pub id: Value,
    /// The error object containing details about the failure
    pub error: JsonRpcErrorObject,
}

/// The error object within a JSON-RPC error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorObject {
    /// Error code indicating the type of error
    pub code: i32,
    /// Human-readable error message
    pub message: String,
    /// Optional additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcErrorObject {
    /// Creates a new error object with the given code and message.
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Creates a new error object with additional data.
    pub fn with_data(code: i32, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }
}

/// Standard JSON-RPC error codes.
pub mod error_codes {
    /// Invalid JSON was received by the server.
    pub const PARSE_ERROR: i32 = -32700;
    /// The JSON sent is not a valid Request object.
    pub const INVALID_REQUEST: i32 = -32600;
    /// The method does not exist / is not available.
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid method parameter(s).
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal JSON-RPC error.
    pub const INTERNAL_ERROR: i32 = -32603;
}

// =============================================================================
// MCP Protocol Handshake Types
// =============================================================================

/// Parameters for the `initialize` method.
///
/// Sent by the client to begin the MCP handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    /// The MCP version the client supports
    pub protocol_version: String,
    /// Client capabilities
    pub capabilities: ClientCapabilities,
    /// Information about the client implementation
    pub client_info: Implementation,
}

impl InitializeParams {
    /// Creates new initialize parameters with the given client info.
    pub fn new(capabilities: ClientCapabilities, client_info: Implementation) -> Self {
        Self {
            protocol_version: "2024-11-05".to_string(),
            capabilities,
            client_info,
        }
    }
}

/// Result of the `initialize` method.
///
/// Returned by the server to complete the MCP handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    /// The MCP version the server is using
    pub protocol_version: String,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Information about the server implementation
    pub server_info: Implementation,
}

impl InitializeResult {
    /// Creates new initialize result with the given server info.
    pub fn new(capabilities: ServerCapabilities, server_info: Implementation) -> Self {
        Self {
            protocol_version: "2024-11-05".to_string(),
            capabilities,
            server_info,
        }
    }
}

/// Information about an implementation (client or server).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    /// The name of the implementation
    pub name: String,
    /// The version of the implementation
    pub version: String,
}

impl Implementation {
    /// Creates a new implementation info.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

/// Capabilities a client may support.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientCapabilities {
    /// Experimental, non-standard capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
    /// Capabilities for tool invocation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ClientToolsCapabilities>,
}

/// Client capabilities for tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientToolsCapabilities {
    /// Whether the client supports list changed notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Capabilities a server may support.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    /// Experimental, non-standard capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
    /// Capabilities for tool support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ServerToolsCapabilities>,
}

/// Server capabilities for tool support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerToolsCapabilities {
    /// Whether the server supports list changed notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

// =============================================================================
// MCP Tool Types
// =============================================================================

/// A tool that can be invoked by the client.
///
/// Tools are the primary mechanism for MCP servers to expose functionality
/// to clients. Each tool has a name, description, and input schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The unique name of the tool
    pub name: String,
    /// A human-readable description of what the tool does
    pub description: String,
    /// JSON Schema describing the tool's input parameters
    pub input_schema: Value,
}

impl Tool {
    /// Creates a new tool with the given name, description, and input schema.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }

    /// Creates a tool with an empty object schema (no parameters).
    pub fn simple(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }
}

/// Request to list available tools.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListToolsRequest {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Result of listing available tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResult {
    /// The list of available tools
    pub tools: Vec<Tool>,
    /// Optional cursor for pagination (if more tools available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl ListToolsResult {
    /// Creates a new list tools result with the given tools.
    pub fn new(tools: Vec<Tool>) -> Self {
        Self {
            tools,
            next_cursor: None,
        }
    }

    /// Creates a new list tools result with pagination.
    pub fn with_cursor(tools: Vec<Tool>, next_cursor: impl Into<String>) -> Self {
        Self {
            tools,
            next_cursor: Some(next_cursor.into()),
        }
    }
}

/// Request to invoke a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolRequest {
    /// The name of the tool to invoke
    pub name: String,
    /// The arguments to pass to the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

impl CallToolRequest {
    /// Creates a new call tool request with the given tool name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: None,
        }
    }

    /// Creates a new call tool request with arguments.
    pub fn with_arguments(name: impl Into<String>, arguments: Value) -> Self {
        Self {
            name: name.into(),
            arguments: Some(arguments),
        }
    }
}

/// Result of a tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
    /// The content items returned by the tool
    pub content: Vec<Content>,
    /// Whether the tool invocation resulted in an error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl CallToolResult {
    /// Creates a new call tool result with text content.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![Content::text(text)],
            is_error: None,
        }
    }

    /// Creates a new call tool result with multiple content items.
    pub fn new(content: Vec<Content>) -> Self {
        Self {
            content,
            is_error: None,
        }
    }

    /// Creates an error result.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![Content::text(message)],
            is_error: Some(true),
        }
    }
}

/// Content item in a tool result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// The type of content (e.g., "text", "image", "resource")
    #[serde(rename = "type")]
    pub content_type: String,
    /// The text content (for text type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The data content (for binary types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// The MIME type (for binary types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

impl Content {
    /// Creates a new text content item.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content_type: "text".to_string(),
            text: Some(text.into()),
            data: None,
            mime_type: None,
        }
    }

    /// Creates a new image content item.
    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            content_type: "image".to_string(),
            text: None,
            data: Some(data.into()),
            mime_type: Some(mime_type.into()),
        }
    }

    /// Creates a new resource content item.
    pub fn resource(uri: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            content_type: "resource".to_string(),
            text: Some(uri.into()),
            data: None,
            mime_type: Some(mime_type.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_rpc_request_serialization() {
        let request: JsonRpcRequest = JsonRpcRequest::new(1, "initialize");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""id":1"#));
        assert!(json.contains(r#""method":"initialize""#));
    }

    #[test]
    fn test_json_rpc_request_with_params() {
        let params = InitializeParams::new(
            ClientCapabilities::default(),
            Implementation::new("test-client", "1.0.0"),
        );
        let request = JsonRpcRequest::with_params(1, "initialize", params);
        assert!(request.params.is_some());
    }

    #[test]
    fn test_json_rpc_response_serialization() {
        let response = JsonRpcResponse::success(1, json!({"status": "ok"}));
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""id":1"#));
        assert!(json.contains(r#""result":{"status":"ok"}"#));
    }

    #[test]
    fn test_json_rpc_error_serialization() {
        let error = JsonRpcError {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            error: JsonRpcErrorObject::new(error_codes::METHOD_NOT_FOUND, "Method not found"),
        };
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains(r#""code":-32601"#));
        assert!(json.contains(r#""message":"Method not found""#));
    }

    #[test]
    fn test_tool_serialization() {
        let tool = Tool::new(
            "echo",
            "Echo the input back",
            json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message to echo"
                    }
                },
                "required": ["message"]
            }),
        );
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains(r#""name":"echo""#));
        assert!(json.contains(r#""description":"Echo the input back""#));
    }

    #[test]
    fn test_tool_simple() {
        let tool = Tool::simple("ping", "Ping the server");
        assert_eq!(tool.name, "ping");
        assert_eq!(tool.description, "Ping the server");
        assert_eq!(tool.input_schema["type"], "object");
    }

    #[test]
    fn test_call_tool_request() {
        let request = CallToolRequest::with_arguments(
            "echo",
            json!({"message": "hello"}),
        );
        assert_eq!(request.name, "echo");
        assert!(request.arguments.is_some());
    }

    #[test]
    fn test_call_tool_result() {
        let result = CallToolResult::text("Hello, world!");
        assert_eq!(result.content.len(), 1);
        assert!(result.is_error.is_none());

        let error_result = CallToolResult::error("Something went wrong");
        assert_eq!(error_result.is_error, Some(true));
    }

    #[test]
    fn test_content_types() {
        let text_content = Content::text("Hello");
        assert_eq!(text_content.content_type, "text");
        assert_eq!(text_content.text, Some("Hello".to_string()));

        let image_content = Content::image("base64data", "image/png");
        assert_eq!(image_content.content_type, "image");
        assert_eq!(image_content.mime_type, Some("image/png".to_string()));
    }

    #[test]
    fn test_initialize_params_deserialization() {
        let json = json!({
            "protocol_version": "2024-11-05",
            "capabilities": {
                "tools": {
                    "list_changed": true
                }
            },
            "client_info": {
                "name": "test-client",
                "version": "1.0.0"
            }
        });
        let params: InitializeParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.protocol_version, "2024-11-05");
        assert_eq!(params.client_info.name, "test-client");
    }

    #[test]
    fn test_list_tools_result() {
        let tools = vec![
            Tool::simple("tool1", "First tool"),
            Tool::simple("tool2", "Second tool"),
        ];
        let result = ListToolsResult::new(tools);
        assert_eq!(result.tools.len(), 2);
        assert!(result.next_cursor.is_none());
    }
}
