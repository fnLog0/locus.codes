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
                        "parameters": tool.function.parameters,
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
            anthropic
                .thinking
                .as_ref()
                .map(|_| ZaiThinkingConfig {
                    type_: "enabled".to_string(),
                })
        } else {
            None
        }
    });

    // Enable tool_stream when streaming with tools
    let tool_stream = if stream && tools.is_some() {
        Some(true)
    } else {
        None
    };

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
        tool_stream,
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
                        arguments: arguments.clone(),
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
            content.push(ResponseContent::ToolCall(ToolCall {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                arguments: tc.function.arguments.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(
            parse_finish_reason(&None).unified,
            FinishReasonKind::Other
        );
    }
}
