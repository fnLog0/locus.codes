//! Conversion between unified types and Z.AI types

use super::types::{ZaiMessage, ZaiRequest, ZaiResponse, ZaiThinkingConfig, ZaiToolCall};
use crate::error::{Error, Result};
use crate::types::{
    ContentPart, FinishReason, FinishReasonKind, GenerateRequest, GenerateResponse, Message,
    ResponseContent, Role, ToolCall, Usage,
};
use serde_json::json;

/// Convert unified request to Z.AI request
pub fn to_zai_request(req: &GenerateRequest, stream: bool) -> Result<ZaiRequest> {
    let messages: Vec<ZaiMessage> = req
        .messages
        .iter()
        .map(to_zai_message)
        .collect::<Result<Vec<_>>>()?;

    let tools = req.options.tools.as_ref().map(|tools| {
        tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.function.name,
                        "description": tool.function.description,
                        "parameters": sanitize_schema_for_zai(&tool.function.parameters),
                    }
                })
            })
            .collect()
    });

    let tool_choice = req.options.tool_choice.as_ref().map(|choice| match choice {
        crate::types::ToolChoice::Auto => json!("auto"),
        crate::types::ToolChoice::None => json!("none"),
        crate::types::ToolChoice::Required { name } => json!({
            "type": "function",
            "function": { "name": name }
        }),
    });

    // Enable thinking if requested via provider options
    let thinking = req.provider_options.as_ref().and_then(|opts| {
        if let crate::types::ProviderOptions::Anthropic(anthropic) = opts {
            anthropic.thinking.as_ref().map(|_| ZaiThinkingConfig {
                type_: "enabled".to_string(),
            })
        } else {
            None
        }
    });

    Ok(ZaiRequest {
        model: req.model.clone(),
        messages,
        temperature: req.options.temperature,
        top_p: req.options.top_p,
        max_tokens: req.options.max_tokens,
        stream: if stream { Some(true) } else { None },
        stop: req.options.stop_sequences.clone(),
        tools,
        tool_choice,
        thinking,
    })
}

/// Convert unified message to Z.AI message
fn to_zai_message(msg: &Message) -> Result<ZaiMessage> {
    let role = match msg.role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    };

    let parts = msg.parts();

    // Check for tool calls in assistant messages
    let tool_calls: Vec<ZaiToolCall> = parts
        .iter()
        .filter_map(|p| {
            if let ContentPart::ToolCall {
                id,
                name,
                arguments,
                ..
            } = p
            {
                Some(ZaiToolCall {
                    id: id.clone(),
                    type_: "function".to_string(),
                    function: super::types::ZaiFunction {
                        name: name.clone(),
                        // Z.AI expects arguments as a JSON string, not a JSON object
                        arguments: if arguments.is_string() {
                            arguments.clone()
                        } else {
                            serde_json::Value::String(
                                serde_json::to_string(arguments).unwrap_or_default(),
                            )
                        },
                    },
                })
            } else {
                None
            }
        })
        .collect();

    // Get tool_call_id for tool result messages
    let tool_call_id = parts.iter().find_map(|p| {
        if let ContentPart::ToolResult { tool_call_id, .. } = p {
            Some(tool_call_id.clone())
        } else {
            None
        }
    });

    // Get text content (or tool result content)
    let content = if let Some(text) = msg.text() {
        Some(text)
    } else {
        parts.iter().find_map(|p| {
            if let ContentPart::ToolResult { content, .. } = p {
                Some(content.to_string())
            } else {
                None
            }
        })
    };

    Ok(ZaiMessage {
        role: role.to_string(),
        content,
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        },
        tool_call_id,
    })
}

/// Convert Z.AI response to unified response
pub fn from_zai_response(resp: ZaiResponse) -> Result<GenerateResponse> {
    let choice = resp
        .choices
        .first()
        .ok_or_else(|| Error::invalid_response("No choices in Z.AI response"))?;

    let mut content = Vec::new();

    // Add reasoning content if present
    if let Some(ref reasoning) = choice.message.reasoning_content {
        if !reasoning.is_empty() {
            content.push(ResponseContent::Reasoning {
                reasoning: reasoning.clone(),
            });
        }
    }

    // Add text content
    if let Some(ref text) = choice.message.content {
        if !text.is_empty() {
            content.push(ResponseContent::Text { text: text.clone() });
        }
    }

    // Add tool calls
    if let Some(ref tool_calls) = choice.message.tool_calls {
        for tc in tool_calls {
            // Z.AI returns arguments as a JSON string; parse it back to a Value
            let arguments = match &tc.function.arguments {
                serde_json::Value::String(s) => {
                    serde_json::from_str(s).unwrap_or(serde_json::json!({}))
                }
                other => other.clone(),
            };
            content.push(ResponseContent::ToolCall(ToolCall {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                arguments,
            }));
        }
    }

    if content.is_empty() {
        return Err(Error::invalid_response("No content in Z.AI response"));
    }

    let finish_reason = parse_finish_reason(&choice.finish_reason);

    let cached_tokens = resp
        .usage
        .prompt_tokens_details
        .as_ref()
        .and_then(|d| d.cached_tokens);

    let usage = Usage {
        prompt_tokens: resp.usage.prompt_tokens,
        completion_tokens: resp.usage.completion_tokens,
        total_tokens: resp.usage.total_tokens,
        input_token_details: cached_tokens.map(|ct| crate::types::InputTokenDetails {
            total: Some(resp.usage.prompt_tokens),
            no_cache: Some(resp.usage.prompt_tokens.saturating_sub(ct)),
            cache_read: Some(ct),
            cache_write: None,
        }),
        output_token_details: None,
        raw: Some(serde_json::to_value(&resp.usage).unwrap_or_default()),
    };

    Ok(GenerateResponse {
        content,
        usage,
        finish_reason,
        metadata: Some(json!({
            "id": resp.id,
            "model": resp.model,
        })),
        warnings: None,
    })
}

/// Parse Z.AI finish reason to unified finish reason
pub fn parse_finish_reason(reason: &Option<String>) -> FinishReason {
    match reason.as_deref() {
        Some("stop") => FinishReason::with_raw(FinishReasonKind::Stop, "stop"),
        Some("length") => FinishReason::with_raw(FinishReasonKind::Length, "length"),
        Some("tool_calls") => FinishReason::with_raw(FinishReasonKind::ToolCalls, "tool_calls"),
        Some("content_filter") => {
            FinishReason::with_raw(FinishReasonKind::ContentFilter, "content_filter")
        }
        Some(raw) => FinishReason::with_raw(FinishReasonKind::Other, raw),
        None => FinishReason::other(),
    }
}

/// Strip JSON Schema fields that Z.AI doesn't support in tool parameters.
///
/// Z.AI (GLM models) rejects schemas containing `default`, `format`, and
/// `additionalProperties`. This walks the schema recursively and removes them.
/// Only used in the Z.AI provider — Anthropic/OpenAI handle these fields fine.
fn sanitize_schema_for_zai(schema: &serde_json::Value) -> serde_json::Value {
    match schema {
        serde_json::Value::Object(map) => {
            let mut clean = serde_json::Map::new();
            for (k, v) in map {
                if matches!(k.as_str(), "default" | "format" | "additionalProperties") {
                    continue;
                }
                clean.insert(k.clone(), sanitize_schema_for_zai(v));
            }
            serde_json::Value::Object(clean)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(sanitize_schema_for_zai).collect())
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_schema_strips_unsupported_fields() {
        let schema = json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "format": "uri",
                    "description": "Target URL"
                },
                "mode": {
                    "type": "string",
                    "enum": ["lite", "stealth"],
                    "default": "lite"
                },
                "nested": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "count": {
                            "type": "integer",
                            "default": 5
                        }
                    }
                }
            },
            "required": ["url"]
        });

        let sanitized = sanitize_schema_for_zai(&schema);

        // format, default, additionalProperties should be gone
        let props = sanitized["properties"].as_object().unwrap();
        assert!(props["url"].get("format").is_none());
        assert!(props["mode"].get("default").is_none());
        assert!(props["nested"].get("additionalProperties").is_none());
        assert!(
            props["nested"]["properties"]["count"]
                .get("default")
                .is_none()
        );

        // type, description, enum, required should remain
        assert_eq!(props["url"]["type"], "string");
        assert_eq!(props["url"]["description"], "Target URL");
        assert_eq!(props["mode"]["enum"][0], "lite");
        assert_eq!(sanitized["required"][0], "url");
    }

    #[test]
    fn test_sanitize_schema_preserves_clean_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Command to run" }
            },
            "required": ["command"]
        });

        let sanitized = sanitize_schema_for_zai(&schema);
        assert_eq!(schema, sanitized);
    }

    /// Regression: tool call arguments must serialize as a JSON string, not a JSON object.
    /// Z.AI error 1210 occurs if arguments is `{"pattern":"*.rs"}` instead of `"{\"pattern\":\"*.rs\"}"`.
    #[test]
    fn test_tool_call_arguments_serialize_as_string() {
        let msg = Message::new(
            crate::types::Role::Assistant,
            vec![ContentPart::tool_call(
                "call_1",
                "glob",
                json!({"pattern": "**/*.rs", "max_results": 100}),
            )],
        );

        let zai_msg = to_zai_message(&msg).unwrap();
        let tc = &zai_msg.tool_calls.unwrap()[0];

        // arguments MUST be a string, not an object
        assert!(
            tc.function.arguments.is_string(),
            "tool call arguments must be a JSON string, got: {}",
            tc.function.arguments
        );

        // The string must parse back to the original JSON
        let parsed: serde_json::Value =
            serde_json::from_str(tc.function.arguments.as_str().unwrap()).unwrap();
        assert_eq!(parsed["pattern"], "**/*.rs");
        assert_eq!(parsed["max_results"], 100);
    }

    /// Regression: tool result messages must have role "tool", content as string, and tool_call_id.
    #[test]
    fn test_tool_result_message_format() {
        let msg = Message::new(
            crate::types::Role::Tool,
            vec![ContentPart::tool_result(
                "call_1",
                json!({"files": ["src/main.rs"], "total": 1}),
            )],
        );

        let zai_msg = to_zai_message(&msg).unwrap();

        assert_eq!(zai_msg.role, "tool");
        assert_eq!(zai_msg.tool_call_id.as_deref(), Some("call_1"));
        assert!(zai_msg.content.is_some(), "tool result must have content");
    }

    /// Regression: response tool call arguments (string from API) must be parsed back to Value.
    #[test]
    fn test_response_string_arguments_parsed_to_value() {
        let resp = ZaiResponse {
            id: "r1".into(),
            model: "glm-5".into(),
            choices: vec![super::super::types::ZaiChoice {
                index: 0,
                message: super::super::types::ZaiResponseMessage {
                    role: "assistant".into(),
                    content: None,
                    reasoning_content: None,
                    tool_calls: Some(vec![ZaiToolCall {
                        id: "call_1".into(),
                        type_: "function".into(),
                        function: super::super::types::ZaiFunction {
                            name: "bash".into(),
                            // API returns arguments as a JSON string
                            arguments: json!("{\"command\":\"ls -la\"}"),
                        },
                    }]),
                },
                finish_reason: Some("tool_calls".into()),
            }],
            usage: super::super::types::ZaiUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                prompt_tokens_details: None,
            },
        };

        let gen_resp = from_zai_response(resp).unwrap();
        let tc = gen_resp.tool_calls();
        assert_eq!(tc.len(), 1);
        // arguments must be a parsed object, not a raw string
        assert!(
            tc[0].arguments.is_object(),
            "arguments should be parsed to object"
        );
        assert_eq!(tc[0].arguments["command"], "ls -la");
    }

    /// Full round-trip: simulate assistant tool call → tool result → re-serialized request.
    /// This is the exact flow that caused error 1210.
    #[test]
    fn test_multi_turn_tool_call_roundtrip() {
        let req = GenerateRequest {
            model: "glm-5".into(),
            messages: vec![
                Message::new(crate::types::Role::System, "You are a helpful assistant."),
                Message::new(crate::types::Role::User, "read readme.md"),
                // Assistant made a tool call (arguments stored as parsed Value)
                Message::new(
                    crate::types::Role::Assistant,
                    vec![ContentPart::tool_call("call_1", "bash", json!({"command": "cat README.md"}))],
                ),
                // Tool result
                Message::new(
                    crate::types::Role::Tool,
                    vec![ContentPart::tool_result("call_1", json!({"stdout": "# Hello"}))],
                ),
            ],
            options: Default::default(),
            provider_options: None,
            telemetry_metadata: None,
        };

        let zai_req = to_zai_request(&req, true).unwrap();
        let serialized = serde_json::to_string(&zai_req).unwrap();

        // Must not contain `"arguments":{` (object) — must be `"arguments":"` (string)
        assert!(
            !serialized.contains(r#""arguments":{"#),
            "arguments must not serialize as a JSON object in the request body:\n{}",
            serialized
        );

        // tool result content must be a string
        let tool_msg = &zai_req.messages[3];
        assert_eq!(tool_msg.role, "tool");
        assert!(tool_msg.content.is_some());
        assert_eq!(tool_msg.tool_call_id.as_deref(), Some("call_1"));
    }

    #[test]
    fn test_parse_finish_reason() {
        assert_eq!(
            parse_finish_reason(&Some("stop".to_string())).unified,
            FinishReasonKind::Stop
        );
        assert_eq!(
            parse_finish_reason(&Some("length".to_string())).unified,
            FinishReasonKind::Length
        );
        assert_eq!(
            parse_finish_reason(&Some("tool_calls".to_string())).unified,
            FinishReasonKind::ToolCalls
        );
        assert_eq!(parse_finish_reason(&None).unified, FinishReasonKind::Other);
    }
}
