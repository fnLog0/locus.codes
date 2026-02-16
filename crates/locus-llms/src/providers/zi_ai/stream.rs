//! Zi.AI streaming support

use super::types::{ZiAIStreamChoice, ZiAIStreamEvent};
use crate::error::{Error, Result};
use crate::types::{FinishReason, FinishReasonKind, GenerateStream, StreamEvent, Usage};
use futures::stream::StreamExt;
use reqwest_eventsource::{Event, EventSource};

/// Create a stream from Zi.AI EventSource
pub async fn create_stream(mut event_source: EventSource) -> Result<GenerateStream> {
    let stream = async_stream::stream! {
        let mut accumulated_usage = Usage::default();

        while let Some(event) = event_source.next().await {
            match event {
                Ok(Event::Open) => {
                    continue;
                }
                Ok(Event::Message(message)) => {
                    if message.data == "[DONE]" {
                        break;
                    }

                    match serde_json::from_str::<ZiAIStreamEvent>(&message.data) {
                        Ok(event) => {
                            for stream_event in process_zi_ai_event(event, &mut accumulated_usage) {
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
                        "Zi.AI API error {}: {}",
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

/// Process Zi.AI stream event and convert to unified StreamEvent(s)
fn process_zi_ai_event(
    event: ZiAIStreamEvent,
    accumulated_usage: &mut Usage,
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
            events.extend(process_choice(choice, accumulated_usage));
        }
    }

    events
}

/// Process a single choice from the stream
fn process_choice(choice: ZiAIStreamChoice, accumulated_usage: &Usage) -> Vec<StreamEvent> {
    let mut events = Vec::new();
    let delta = choice.delta;

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
                    tc.function.as_ref().and_then(|f| f.name.clone()).unwrap_or_default(),
                ));
            }

            if let Some(function) = tc.function {
                if let Some(name) = function.name {
                    // Tool call name update - use id if available, otherwise use index
                    let id = tc.id.as_ref().map(|i| i.clone()).unwrap_or_else(|| tc.index.to_string());
                    events.push(StreamEvent::tool_call_start(
                        id,
                        name,
                    ));
                }

                if let Some(arguments) = function.arguments {
                    // Tool call arguments delta - use id if available, otherwise use index
                    let id = tc.id.as_ref().map(|i| i.clone()).unwrap_or_else(|| tc.index.to_string());
                    events.push(StreamEvent::tool_call_delta(
                        id,
                        arguments,
                    ));
                }
            }
        }
    }

    // Check for finish
    if let Some(finish_reason) = choice.finish_reason {
        let unified_finish_reason = match finish_reason.as_str() {
            "stop" => FinishReasonKind::Stop,
            "length" => FinishReasonKind::Length,
            "tool_calls" => FinishReasonKind::ToolCalls,
            _ => FinishReasonKind::Other,
        };

        events.push(StreamEvent::finish(
            accumulated_usage.clone(),
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

        let event = ZiAIStreamEvent {
            id: Some("chatcmpl-123".to_string()),
            object: Some("chat.completion.chunk".to_string()),
            created: Some(1677652288),
            model: Some("zai".to_string()),
            choices: Some(vec![ZiAIStreamChoice {
                index: 0,
                delta: ZiAIStreamDelta {
                    role: None,
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                },
                finish_reason: None,
            }]),
            usage: None,
        };

        let results = process_zi_ai_event(event, &mut usage);
        assert_eq!(results.len(), 1);

        if let StreamEvent::TextDelta { delta, .. } = &results[0] {
            assert_eq!(delta, "Hello");
        } else {
            panic!("Expected TextDelta event");
        }
    }
}
