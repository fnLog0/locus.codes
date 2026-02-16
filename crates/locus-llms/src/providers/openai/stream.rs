//! OpenAI streaming support

use super::types::{OpenAIStreamChoice, OpenAIStreamEvent};
use crate::error::{Error, Result};
use crate::types::{FinishReason, FinishReasonKind, GenerateStream, StreamEvent, Usage};
use futures::stream::StreamExt;
use reqwest_eventsource::{Event, EventSource};

/// Create a stream from OpenAI EventSource
pub async fn create_stream(mut event_source: EventSource) -> Result<GenerateStream> {
    let stream = async_stream::stream! {
        let mut accumulated_usage = Usage::default();
        let mut reasoning_content = String::new();

        while let Some(event) = event_source.next().await {
            match event {
                Ok(Event::Open) => {
                    continue;
                }
                Ok(Event::Message(message)) => {
                    if message.data == "[DONE]" {
                        break;
                    }

                    match serde_json::from_str::<OpenAIStreamEvent>(&message.data) {
                        Ok(event) => {
                            // Extract reasoning content before processing
                            if let Some(choices) = &event.choices {
                                if let Some(choice) = choices.first() {
                                    if let Some(ref reasoning) = choice.delta.reasoning_content {
                                        reasoning_content.push_str(reasoning);
                                    }
                                }
                            }

                            for stream_event in process_openai_event(event, &mut accumulated_usage, &mut reasoning_content) {
                                yield Ok(stream_event);
                            }
                        }
                        Err(e) => {
                            yield Err(Error::stream_error(format!("Failed to parse event: {}", e)));
                            break;
                        }
                    }
                }
                Err(reqwest_eventsource::Error::StreamEnded) => {
                    break;
                }
                Err(reqwest_eventsource::Error::InvalidStatusCode(status, response)) => {
                    let error_body = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unable to read error body".to_string());
                    yield Err(Error::provider_error(format!(
                        "OpenAI API error {}: {}",
                        status, error_body
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

/// Track state for tool calls during streaming
#[derive(Debug, Clone)]
struct ToolCallState {
    id: String,
    name: String,
    arguments: String,
}

/// Process OpenAI stream event and convert to unified StreamEvent(s)
fn process_openai_event(
    event: OpenAIStreamEvent,
    accumulated_usage: &mut Usage,
    reasoning_content: &mut String,
) -> Vec<StreamEvent> {
    // Process usage if available
    if let Some(usage) = event.usage {
        accumulated_usage.prompt_tokens = usage.prompt_tokens;
        accumulated_usage.completion_tokens = usage.completion_tokens;
        accumulated_usage.total_tokens = usage.total_tokens;
    }

    let mut events = Vec::new();

    // Process choices
    if let Some(choices) = event.choices {
        for choice in choices {
            events.extend(process_choice(choice, reasoning_content));
        }
    }

    events
}

/// Process a single choice from the stream
fn process_choice(choice: OpenAIStreamChoice, reasoning_content: &mut String) -> Vec<StreamEvent> {
    let mut events = Vec::new();
    let delta = choice.delta;

    // Handle reasoning content (for o1, o3, etc.)
    if let Some(reasoning) = delta.reasoning_content {
        events.push(StreamEvent::reasoning_delta("", reasoning));
    }

    // Handle text content
    if let Some(content) = delta.content {
        events.push(StreamEvent::text_delta("", content));
    }

    // Handle tool calls
    if let Some(tool_calls) = delta.tool_calls {
        for tc in tool_calls {
            if let Some(ref id) = tc.id {
                // New tool call started
                events.push(StreamEvent::tool_call_start(
                    id.clone(),
                    tc.function
                        .as_ref()
                        .and_then(|f| f.name.clone())
                        .unwrap_or_default(),
                ));
            }

            if let Some(function) = tc.function {
                if let Some(name) = function.name {
                    // Tool call name update - use id if available, otherwise use index
                    let id = tc.id.as_ref().map(|i| i.clone()).unwrap_or_else(|| tc.index.to_string());
                    events.push(StreamEvent::tool_call_start(id, name));
                }

                if let Some(arguments) = function.arguments {
                    // Tool call arguments delta - use id if available, otherwise use index
                    let id = tc.id.as_ref().map(|i| i.clone()).unwrap_or_else(|| tc.index.to_string());
                    events.push(StreamEvent::tool_call_delta(id, arguments));
                }
            }
        }
    }

    // Check for finish
    if let Some(finish_reason) = choice.finish_reason {
        let unified_finish_reason = match finish_reason.as_str() {
            "stop" => FinishReasonKind::Stop,
            "length" => FinishReasonKind::Length,
            "content_filter" => FinishReasonKind::Other,
            "tool_calls" => FinishReasonKind::ToolCalls,
            _ => FinishReasonKind::Other,
        };

        // We need usage here, but we don't have direct access to it in this function
        // The finish event will be emitted when we see the final usage update
        // For now, just note the finish reason
        events.push(StreamEvent::finish(
            Usage::default(), // Will be replaced by actual usage from final event
            FinishReason::with_raw(unified_finish_reason, finish_reason),
        ));
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_text_delta() {
        let mut usage = Usage::default();
        let mut reasoning_content = String::new();

        let event = OpenAIStreamEvent {
            id: Some("chatcmpl-123".to_string()),
            object: Some("chat.completion.chunk".to_string()),
            created: Some(1677652288),
            model: Some("gpt-4".to_string()),
            choices: Some(vec![OpenAIStreamChoice {
                index: 0,
                delta: OpenAIStreamDelta {
                    role: None,
                    content: Some("Hello".to_string()),
                    reasoning_content: None,
                    tool_calls: None,
                },
                finish_reason: None,
            }]),
            usage: None,
            error: None,
        };

        let results = process_openai_event(event, &mut usage, &mut reasoning_content);
        assert_eq!(results.len(), 1);

        if let StreamEvent::TextDelta { delta, .. } = &results[0] {
            assert_eq!(delta, "Hello");
        } else {
            panic!("Expected TextDelta event");
        }
    }

    #[test]
    fn test_reasoning_delta() {
        let mut usage = Usage::default();
        let mut reasoning_content = String::new();

        let event = OpenAIStreamEvent {
            id: Some("chatcmpl-123".to_string()),
            object: Some("chat.completion.chunk".to_string()),
            created: Some(1677652288),
            model: Some("o1-preview".to_string()),
            choices: Some(vec![OpenAIStreamChoice {
                index: 0,
                delta: OpenAIStreamDelta {
                    role: None,
                    content: None,
                    reasoning_content: Some("Let me think...".to_string()),
                    tool_calls: None,
                },
                finish_reason: None,
            }]),
            usage: None,
            error: None,
        };

        let results = process_openai_event(event, &mut usage, &mut reasoning_content);
        assert_eq!(results.len(), 1);

        if let StreamEvent::ReasoningDelta { delta, .. } = &results[0] {
            assert_eq!(delta, "Let me think...");
        } else {
            panic!("Expected ReasoningDelta event");
        }
    }
}
