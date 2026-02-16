//! Conversion between unified types and Zi.AI types

use super::types::{infer_max_tokens, ZiAIContentBlock, ZiAIImageUrl, ZiAIRequest,
    ZiAIResponse, ZiAIFunction, ZiAIMessage, ZiAIMessageContent, ZiAITool};
use crate::error::{Error, Result};
use crate::types::{
    CacheControlValidator, CacheWarning, ContentPart, FinishReason,
    FinishReasonKind, GenerateRequest, GenerateResponse, InputTokenDetails, Message, Role,
    OutputTokenDetails, ResponseContent, ToolCall, Usage,
};
use serde_json::json;

/// Convert unified request to Zi.AI request
pub fn to_zi_ai_request(
    req: &GenerateRequest,
    stream: bool,
) -> Result<(ZiAIRequest, Vec<CacheWarning>)> {
    let mut validator = CacheControlValidator::new();

    // Convert messages
    let messages = req
        .messages
        .iter()
        .map(|m| to_zi_ai_message(m, &mut validator))
        .collect::<Result<Vec<_>>>()?;

    let warnings = validator.take_warnings();

    // Convert tools
    let tools = req.options.tools.as_ref().map(|tools| {
        tools
            .iter()
            .map(|tool| ZiAITool {
                type_: "function".to_string(),
                function: ZiAIFunction {
                    name: tool.function.name.clone(),
                    description: tool.function.description.clone(),
                    parameters: tool.function.parameters.clone(),
                },
            })
            .collect()
    });

    // Convert tool_choice
    let tool_choice = req.options.tool_choice.as_ref().map(|choice| match choice {
        crate::types::ToolChoice::Auto => json!("auto"),
        crate::types::ToolChoice::None => json!("none"),
        crate::types::ToolChoice::Required { name } => {
            json!({"type": "function", "function": {"name": name}})
        }
    });

    Ok((
        ZiAIRequest {
            model: req.model.clone(),
            messages,
            temperature: req.options.temperature,
            top_p: req.options.top_p,
            max_tokens: Some(req.options.max_tokens.unwrap_or_else(|| infer_max_tokens(&req.model))),
            stop: req.options.stop_sequences.clone(),
            stream: if stream { Some(true) } else { None },
            tools,
            tool_choice,
        },
        warnings,
    ))
}

/// Convert unified message to Zi.AI message
fn to_zi_ai_message(
    msg: &Message,
    _validator: &mut CacheControlValidator,
) -> Result<ZiAIMessage> {
    let role = match msg.role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
        Role::Tool => {
            // Tool messages need special handling
            let content = msg.text().unwrap_or_default();
            let tool_call_id = msg
                .parts()
                .first()
                .and_then(|p| {
                    if let ContentPart::ToolResult { tool_call_id, .. } = p {
                        Some(tool_call_id.clone())
                    } else {
                        None
                    }
                })
                .ok_or_else(|| Error::invalid_response("Tool message missing tool_call_id"))?;

            return Ok(ZiAIMessage {
                role: "tool".to_string(),
                content: ZiAIMessageContent::String(content),
                tool_calls: None,
                tool_call_id: Some(tool_call_id),
            });
        }
    };

    // Convert content parts
    let parts = msg.parts();

    let content = if parts.len() == 1 {
        match &parts[0] {
            ContentPart::Text { text, .. } => ZiAIMessageContent::String(text.clone()),
            ContentPart::Image { url, .. } => ZiAIMessageContent::Array(vec![ZiAIContentBlock::ImageUrl {
                image_url: ZiAIImageUrl {
                    url: url.clone(),
                },
            }]),
            ContentPart::ToolCall { .. } => {
                return Err(Error::invalid_response(
                    "Tool calls should be in assistant message's tool_calls field",
                ));
            }
            ContentPart::ToolResult { .. } => {
                return Err(Error::invalid_response(
                    "Tool result messages should use Role::Tool",
                ));
            }
        }
    } else {
        let blocks: Vec<ZiAIContentBlock> = parts
            .iter()
            .map(|part| match part {
                ContentPart::Text { text, .. } => Ok(ZiAIContentBlock::Text {
                    text: text.clone(),
                }),
                ContentPart::Image { url, .. } => Ok(ZiAIContentBlock::ImageUrl {
                    image_url: ZiAIImageUrl {
                        url: url.clone(),
                    },
                }),
                ContentPart::ToolCall { .. } => Err(Error::invalid_response(
                    "Tool calls should be in assistant message's tool_calls field",
                )),
                ContentPart::ToolResult { .. } => Err(Error::invalid_response(
                    "Tool result messages should use Role::Tool",
                )),
            })
            .collect::<Result<Vec<_>>>()?;

        ZiAIMessageContent::Array(blocks)
    };

    // Extract tool calls from assistant messages
    let tool_calls = if role == "assistant" {
        let calls: Vec<super::types::ZiAIToolCall> = parts
            .iter()
            .filter_map(|part| {
                if let ContentPart::ToolCall {
                    id,
                    name,
                    arguments,
                    ..
                } = part
                {
                    Some(super::types::ZiAIToolCall {
                        id: id.clone(),
                        type_: "function".to_string(),
                        function: super::types::ZiAIFunctionCall {
                            name: name.clone(),
                            arguments: arguments.to_string(),
                        },
                    })
                } else {
                    None
                }
            })
            .collect();

        if calls.is_empty() {
            None
        } else {
            Some(calls)
        }
    } else {
        None
    };

    Ok(ZiAIMessage {
        role: role.to_string(),
        content,
        tool_calls,
        tool_call_id: None,
    })
}

/// Convert Zi.AI response to unified response
pub fn from_zi_ai_response(
    resp: ZiAIResponse,
    warnings: Vec<CacheWarning>,
) -> Result<GenerateResponse> {
    use crate::types::ResponseWarning;

    let choice = resp
        .choices
        .first()
        .ok_or_else(|| Error::invalid_response("No choices in response"))?;

    // Build content from message
    let content = build_content_from_zi_ai_message(&choice.message)?;

    // Determine finish reason
    let finish_reason = match choice.finish_reason.as_deref() {
        Some("stop") => FinishReason::with_raw(FinishReasonKind::Stop, "stop"),
        Some("length") => FinishReason::with_raw(FinishReasonKind::Length, "length"),
        Some("tool_calls") => FinishReason::with_raw(FinishReasonKind::ToolCalls, "tool_calls"),
        Some(raw) => FinishReason::with_raw(FinishReasonKind::Other, raw),
        None => FinishReason::other(),
    };

    // Build usage
    let usage = Usage::with_details(
        InputTokenDetails {
            total: Some(resp.usage.prompt_tokens),
            no_cache: Some(resp.usage.prompt_tokens),
            cache_read: None,
            cache_write: None,
        },
        OutputTokenDetails {
            total: Some(resp.usage.completion_tokens),
            text: Some(resp.usage.completion_tokens),
            reasoning: None,
        },
        Some(serde_json::to_value(&resp.usage).unwrap_or_default()),
    );

    // Convert cache warnings to response warnings
    let response_warnings: Option<Vec<ResponseWarning>> = if warnings.is_empty() {
        None
    } else {
        Some(warnings.into_iter().map(ResponseWarning::from).collect())
    };

    Ok(GenerateResponse {
        content,
        usage,
        finish_reason,
        metadata: Some(json!({
            "id": resp.id,
            "model": resp.model,
            "created": resp.created,
        })),
        warnings: response_warnings,
    })
}

/// Build content from Zi.AI message
fn build_content_from_zi_ai_message(
    msg: &ZiAIMessage,
) -> Result<Vec<ResponseContent>> {
    let mut content = Vec::new();

    match &msg.content {
        ZiAIMessageContent::String(text) => {
            if !text.is_empty() {
                content.push(ResponseContent::Text { text: text.clone() });
            }
        }
        ZiAIMessageContent::Array(blocks) => {
            for block in blocks {
                match block {
                    ZiAIContentBlock::Text { text } => {
                        content.push(ResponseContent::Text { text: text.clone() });
                    }
                    ZiAIContentBlock::ImageUrl { image_url } => {
                        content.push(ResponseContent::Text {
                            text: format!("[Image: {}]", image_url.url),
                        });
                    }
                }
            }
        }
    }

    // Add tool calls
    if let Some(tool_calls) = &msg.tool_calls {
        for tc in tool_calls {
            let arguments = serde_json::from_str(&tc.function.arguments)
                .unwrap_or_else(|_| serde_json::json!({}));
            content.push(ResponseContent::ToolCall(ToolCall {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                arguments,
            }));
        }
    }

    if content.is_empty() {
        return Err(Error::invalid_response("No content in response"));
    }

    Ok(content)
}
