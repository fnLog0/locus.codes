//! Conversion between unified types and OpenAI types

use super::types::{
    infer_max_tokens, OpenAIContentBlock, OpenAIFunction, OpenAIImageUrl, OpenAIMessage,
    OpenAIMessageContent, OpenAIRequest, OpenAIResponse, OpenAITool,
};
use crate::error::{Error, Result};
use crate::types::{
    CacheControlValidator, CacheWarning, ContentPart, FinishReason, FinishReasonKind,
    GenerateRequest, GenerateResponse, InputTokenDetails, Message, OutputTokenDetails,
    ResponseContent, Role, ToolCall, Usage,
};
use serde_json::json;

/// Convert unified request to OpenAI request
pub fn to_openai_request(
    req: &GenerateRequest,
    stream: bool,
) -> Result<(OpenAIRequest, Vec<CacheWarning>)> {
    let mut validator = CacheControlValidator::new();

    // Convert messages
    let messages = req
        .messages
        .iter()
        .map(|m| to_openai_message(m, &mut validator))
        .collect::<Result<Vec<_>>>()?;

    let warnings = validator.take_warnings();

    // Determine max_tokens vs max_completion_tokens
    // For reasoning models (o1, o3), use max_completion_tokens
    let is_reasoning_model = req.model.contains("o1") || req.model.contains("o3");
    let max_tokens = if is_reasoning_model {
        Some(
            req.options
                .max_tokens
                .unwrap_or_else(|| infer_max_tokens(&req.model)),
        )
    } else {
        req.options.max_tokens
    };

    let max_completion_tokens = if is_reasoning_model {
        Some(
            req.options
                .max_tokens
                .unwrap_or_else(|| infer_max_tokens(&req.model)),
        )
    } else {
        None
    };

    // Convert tools
    let tools = req.options.tools.as_ref().map(|tools| {
        tools
            .iter()
            .map(|tool| OpenAITool {
                type_: "function".to_string(),
                function: OpenAIFunction {
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
        crate::types::ToolChoice::Required { name: tool_name } => {
            json!({"type": "function", "function": {"name": tool_name}})
        }
    });

    // Extract OpenAI-specific options
    let openai_opts = req.provider_options.as_ref().and_then(|opts| match opts {
        crate::types::ProviderOptions::OpenAI(o) => Some(o),
        _ => None,
    });

    let user = openai_opts.and_then(|o| o.user.clone());

    let reasoning_effort = openai_opts.and_then(|o| {
        o.reasoning_effort.map(|effort| match effort {
            crate::types::ReasoningEffort::Low => "low".to_string(),
            crate::types::ReasoningEffort::Medium => "medium".to_string(),
            crate::types::ReasoningEffort::High => "high".to_string(),
        })
    });

    Ok((
        OpenAIRequest {
            model: req.model.clone(),
            messages,
            temperature: req.options.temperature,
            top_p: req.options.top_p,
            max_tokens,
            max_completion_tokens,
            stop: req.options.stop_sequences.clone(),
            stream: if stream { Some(true) } else { None },
            tools,
            tool_choice,
            response_format: None,
            user,
            reasoning_effort,
        },
        warnings,
    ))
}

/// Convert unified message to OpenAI message
fn to_openai_message(
    msg: &Message,
    _validator: &mut CacheControlValidator,
) -> Result<OpenAIMessage> {
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

            return Ok(OpenAIMessage {
                role: "tool".to_string(),
                content: OpenAIMessageContent::String(content),
                tool_calls: None,
                tool_call_id: Some(tool_call_id),
            });
        }
    };

    // Convert content parts
    let parts = msg.parts();

    let content = if parts.len() == 1 {
        match &parts[0] {
            ContentPart::Text { text, .. } => OpenAIMessageContent::String(text.clone()),
            ContentPart::Image { url, .. } => {
                OpenAIMessageContent::Array(vec![OpenAIContentBlock::ImageUrl {
                    image_url: OpenAIImageUrl {
                        url: url.clone(),
                        detail: Some("auto".to_string()),
                    },
                }])
            }
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
        let blocks: Vec<OpenAIContentBlock> = parts
            .iter()
            .map(|part| match part {
                ContentPart::Text { text, .. } => {
                    Ok(OpenAIContentBlock::Text { text: text.clone() })
                }
                ContentPart::Image { url, .. } => Ok(OpenAIContentBlock::ImageUrl {
                    image_url: OpenAIImageUrl {
                        url: url.clone(),
                        detail: Some("auto".to_string()),
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

        OpenAIMessageContent::Array(blocks)
    };

    // Extract tool calls from assistant messages
    let tool_calls = if role == "assistant" {
        let calls: Vec<super::types::OpenAIToolCall> = parts
            .iter()
            .filter_map(|part| {
                if let ContentPart::ToolCall {
                    id,
                    name,
                    arguments,
                    ..
                } = part
                {
                    Some(super::types::OpenAIToolCall {
                        id: id.clone(),
                        type_: "function".to_string(),
                        function: super::types::OpenAIFunctionCall {
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

    Ok(OpenAIMessage {
        role: role.to_string(),
        content,
        tool_calls,
        tool_call_id: None,
    })
}

/// Convert OpenAI response to unified response
pub fn from_openai_response(
    resp: OpenAIResponse,
    warnings: Vec<CacheWarning>,
) -> Result<GenerateResponse> {
    use crate::types::ResponseWarning;

    let choice = resp
        .choices
        .first()
        .ok_or_else(|| Error::invalid_response("No choices in response"))?;

    // Build content from message
    let content = build_content_from_openai_message(&choice.message)?;

    // Determine finish reason
    let finish_reason = match choice.finish_reason.as_deref() {
        Some("stop") => FinishReason::with_raw(FinishReasonKind::Stop, "stop"),
        Some("length") => FinishReason::with_raw(FinishReasonKind::Length, "length"),
        Some("content_filter") => FinishReason::with_raw(FinishReasonKind::Other, "content_filter"),
        Some("tool_calls") => FinishReason::with_raw(FinishReasonKind::ToolCalls, "tool_calls"),
        Some(raw) => FinishReason::with_raw(FinishReasonKind::Other, raw),
        None => FinishReason::other(),
    };

    // Build usage
    let cache_read = resp.usage.prompt_cache_hit_tokens.unwrap_or(0);
    let cache_write = resp.usage.prompt_cache_miss_tokens.unwrap_or(0);
    let reasoning_tokens = resp.usage.reasoning_tokens.unwrap_or(0);

    let usage = Usage::with_details(
        InputTokenDetails {
            total: Some(resp.usage.total_tokens - resp.usage.completion_tokens),
            no_cache: Some(resp.usage.prompt_tokens - cache_read - cache_write),
            cache_read: if cache_read > 0 {
                Some(cache_read)
            } else {
                None
            },
            cache_write: if cache_write > 0 {
                Some(cache_write)
            } else {
                None
            },
        },
        OutputTokenDetails {
            total: Some(resp.usage.completion_tokens),
            text: Some(resp.usage.completion_tokens - reasoning_tokens),
            reasoning: if reasoning_tokens > 0 {
                Some(reasoning_tokens)
            } else {
                None
            },
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

/// Build content from OpenAI message
fn build_content_from_openai_message(msg: &OpenAIMessage) -> Result<Vec<ResponseContent>> {
    let mut content = Vec::new();

    match &msg.content {
        OpenAIMessageContent::String(text) => {
            if !text.is_empty() {
                content.push(ResponseContent::Text { text: text.clone() });
            }
        }
        OpenAIMessageContent::Array(blocks) => {
            for block in blocks {
                match block {
                    OpenAIContentBlock::Text { text } => {
                        content.push(ResponseContent::Text { text: text.clone() });
                    }
                    OpenAIContentBlock::ImageUrl { image_url } => {
                        // OpenAI images are represented as text in the response
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
