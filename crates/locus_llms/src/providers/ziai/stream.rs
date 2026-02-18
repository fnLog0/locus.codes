//! Z.AI streaming support
//!
//! Z.AI uses OpenAI-compatible SSE format:
//! - `data: {"choices":[{"delta":{"content":"..."}}]}` for text deltas
//! - `data: {"choices":[{"delta":{"reasoning_content":"..."}}]}` for reasoning
//! - `data: {"choices":[{"delta":{"tool_calls":[...]}}]}` for tool calls
//! - `data: [DONE]` to signal stream end

use super::convert::parse_finish_reason;
use super::types::ZiaiStreamChunk;
use crate::error::{Error, Result};
use crate::types::{GenerateStream, StreamEvent, Usage};
use futures::stream::StreamExt;
use reqwest_eventsource::{Event, EventSource};

/// Track accumulated tool call state during streaming
#[derive(Debug, Clone)]
struct AccumulatedToolCall {
    id: String,
    name: String,
    arguments: String,
}

/// Create a stream from Z.AI EventSource
pub async fn create_stream(mut event_source: EventSource) -> Result<GenerateStream> {
    let stream = async_stream::stream! {
        let mut tool_calls: std::collections::HashMap<u32, AccumulatedToolCall> =
            std::collections::HashMap::new();

        while let Some(event) = event_source.next().await {
            match event {
                Ok(Event::Open) => continue,
                Ok(Event::Message(message)) => {
                    if message.data == "[DONE]" {
                        break;
                    }

                    match serde_json::from_str::<ZiaiStreamChunk>(&message.data) {
                        Ok(chunk) => {
                            for stream_event in process_chunk(chunk, &mut tool_calls) {
                                yield Ok(stream_event);
                            }
                        }
                        Err(e) => {
                            yield Err(Error::stream_error(format!(
                                "Failed to parse Z.AI chunk: {}", e
                            )));
                            break;
                        }
                    }
                }
                Err(reqwest_eventsource::Error::StreamEnded) => break,
                Err(reqwest_eventsource::Error::InvalidStatusCode(status, response)) => {
                    let error_body = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unable to read error body".to_string());
                    yield Err(Error::provider_error(format!(
                        "Z.AI API error {}: {}", status, error_body
                    )));
                    break;
                }
                Err(e) => {
                    yield Err(Error::stream_error(format!("Stream error: {}", e)));
                    break;
                }
            }
        }

        event_source.close();
    };

    Ok(GenerateStream::new(Box::pin(stream)))
}

/// Process a single Z.AI stream chunk into unified StreamEvent(s)
fn process_chunk(
    chunk: ZiaiStreamChunk,
    tool_calls: &mut std::collections::HashMap<u32, AccumulatedToolCall>,
) -> Vec<StreamEvent> {
    let mut events = Vec::new();

    for choice in &chunk.choices {
        let delta = &choice.delta;

        // Reasoning content
        if let Some(ref reasoning) = delta.reasoning_content {
            if !reasoning.is_empty() {
                events.push(StreamEvent::reasoning_delta("", reasoning.clone()));
            }
        }

        // Text content
        if let Some(ref content) = delta.content {
            if !content.is_empty() {
                events.push(StreamEvent::text_delta("", content.clone()));
            }
        }

        // Tool calls
        if let Some(ref delta_tool_calls) = delta.tool_calls {
            for tc in delta_tool_calls {
                let index = tc.index;

                if let Some(ref id) = tc.id {
                    // New tool call starting
                    let name = tc
                        .function
                        .name
                        .clone()
                        .unwrap_or_default();
                    tool_calls.insert(
                        index,
                        AccumulatedToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: String::new(),
                        },
                    );
                    events.push(StreamEvent::tool_call_start(id.clone(), name));
                }

                // Accumulate arguments
                if let Some(ref args) = tc.function.arguments {
                    if !args.is_empty() {
                        if let Some(accumulated) = tool_calls.get_mut(&index) {
                            accumulated.arguments.push_str(args);
                            events.push(StreamEvent::tool_call_delta(
                                accumulated.id.clone(),
                                args.clone(),
                            ));
                        }
                    }
                }
            }
        }

        // Finish reason — emit tool call ends + finish event
        if let Some(ref _reason) = choice.finish_reason {
            // Emit ToolCallEnd for all accumulated tool calls
            for (_, tc) in tool_calls.drain() {
                let input_json = if tc.arguments.is_empty() {
                    serde_json::json!({})
                } else {
                    serde_json::from_str(&tc.arguments).unwrap_or(serde_json::json!({}))
                };
                events.push(StreamEvent::tool_call_end(tc.id, tc.name, input_json));
            }

            let usage = chunk
                .usage
                .as_ref()
                .map(|u| Usage::new(u.prompt_tokens, u.completion_tokens))
                .unwrap_or_default();

            events.push(StreamEvent::finish(
                usage,
                parse_finish_reason(&choice.finish_reason),
            ));
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::ziai::types::*;

    #[test]
    fn test_process_text_delta() {
        let mut tool_calls = std::collections::HashMap::new();
        let chunk = ZiaiStreamChunk {
            id: "1".to_string(),
            model: "glm-5".to_string(),
            choices: vec![ZiaiStreamChoice {
                index: 0,
                delta: ZiaiDelta {
                    role: None,
                    content: Some("Hello".to_string()),
                    reasoning_content: None,
                    tool_calls: None,
                },
                finish_reason: None,
            }],
            usage: None,
        };

        let results = process_chunk(chunk, &mut tool_calls);
        assert_eq!(results.len(), 1);
        if let StreamEvent::TextDelta { delta, .. } = &results[0] {
            assert_eq!(delta, "Hello");
        } else {
            panic!("Expected TextDelta");
        }
    }

    #[test]
    fn test_process_reasoning_delta() {
        let mut tool_calls = std::collections::HashMap::new();
        let chunk = ZiaiStreamChunk {
            id: "1".to_string(),
            model: "glm-5".to_string(),
            choices: vec![ZiaiStreamChoice {
                index: 0,
                delta: ZiaiDelta {
                    role: None,
                    content: None,
                    reasoning_content: Some("Let me think...".to_string()),
                    tool_calls: None,
                },
                finish_reason: None,
            }],
            usage: None,
        };

        let results = process_chunk(chunk, &mut tool_calls);
        assert_eq!(results.len(), 1);
        if let StreamEvent::ReasoningDelta { delta, .. } = &results[0] {
            assert_eq!(delta, "Let me think...");
        } else {
            panic!("Expected ReasoningDelta");
        }
    }

    #[test]
    fn test_process_finish() {
        let mut tool_calls = std::collections::HashMap::new();
        let chunk = ZiaiStreamChunk {
            id: "1".to_string(),
            model: "glm-5".to_string(),
            choices: vec![ZiaiStreamChoice {
                index: 0,
                delta: ZiaiDelta {
                    role: Some("assistant".to_string()),
                    content: Some(String::new()),
                    reasoning_content: None,
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(ZiaiUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
                prompt_tokens_details: None,
            }),
        };

        let results = process_chunk(chunk, &mut tool_calls);
        assert_eq!(results.len(), 1);
        if let StreamEvent::Finish { usage, reason } = &results[0] {
            assert_eq!(usage.prompt_tokens, 10);
            assert_eq!(usage.completion_tokens, 20);
            assert_eq!(reason.unified, crate::types::FinishReasonKind::Stop);
        } else {
            panic!("Expected Finish");
        }
    }

    #[test]
    fn test_process_tool_call_flow() {
        let mut tool_calls = std::collections::HashMap::new();

        // Start tool call
        let chunk1 = ZiaiStreamChunk {
            id: "1".to_string(),
            model: "glm-5".to_string(),
            choices: vec![ZiaiStreamChoice {
                index: 0,
                delta: ZiaiDelta {
                    role: None,
                    content: None,
                    reasoning_content: None,
                    tool_calls: Some(vec![ZiaiStreamToolCall {
                        index: 0,
                        id: Some("call_1".to_string()),
                        type_: Some("function".to_string()),
                        function: ZiaiStreamFunction {
                            name: Some("get_weather".to_string()),
                            arguments: Some(String::new()),
                        },
                    }]),
                },
                finish_reason: None,
            }],
            usage: None,
        };

        let results = process_chunk(chunk1, &mut tool_calls);
        assert_eq!(results.len(), 1);
        if let StreamEvent::ToolCallStart { id, name } = &results[0] {
            assert_eq!(id, "call_1");
            assert_eq!(name, "get_weather");
        } else {
            panic!("Expected ToolCallStart");
        }

        // Tool call arguments delta
        let chunk2 = ZiaiStreamChunk {
            id: "1".to_string(),
            model: "glm-5".to_string(),
            choices: vec![ZiaiStreamChoice {
                index: 0,
                delta: ZiaiDelta {
                    role: None,
                    content: None,
                    reasoning_content: None,
                    tool_calls: Some(vec![ZiaiStreamToolCall {
                        index: 0,
                        id: None,
                        type_: None,
                        function: ZiaiStreamFunction {
                            name: None,
                            arguments: Some(r#"{"city":"Beijing"}"#.to_string()),
                        },
                    }]),
                },
                finish_reason: None,
            }],
            usage: None,
        };

        let results = process_chunk(chunk2, &mut tool_calls);
        assert_eq!(results.len(), 1);
        if let StreamEvent::ToolCallDelta { id, delta } = &results[0] {
            assert_eq!(id, "call_1");
            assert_eq!(delta, r#"{"city":"Beijing"}"#);
        } else {
            panic!("Expected ToolCallDelta");
        }

        // Finish with tool_calls reason
        let chunk3 = ZiaiStreamChunk {
            id: "1".to_string(),
            model: "glm-5".to_string(),
            choices: vec![ZiaiStreamChoice {
                index: 0,
                delta: ZiaiDelta {
                    role: None,
                    content: None,
                    reasoning_content: None,
                    tool_calls: None,
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            usage: Some(ZiaiUsage {
                prompt_tokens: 15,
                completion_tokens: 5,
                total_tokens: 20,
                prompt_tokens_details: None,
            }),
        };

        let results = process_chunk(chunk3, &mut tool_calls);
        assert_eq!(results.len(), 2); // ToolCallEnd + Finish

        if let StreamEvent::ToolCallEnd {
            id,
            name,
            arguments,
        } = &results[0]
        {
            assert_eq!(id, "call_1");
            assert_eq!(name, "get_weather");
            assert_eq!(arguments["city"], "Beijing");
        } else {
            panic!("Expected ToolCallEnd");
        }

        if let StreamEvent::Finish { reason, .. } = &results[1] {
            assert_eq!(reason.unified, crate::types::FinishReasonKind::ToolCalls);
        } else {
            panic!("Expected Finish");
        }
    }
}
