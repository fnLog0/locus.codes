use futures_util::StreamExt;
use tokio::sync::mpsc::Sender;

use super::args::WebAutomationArgs;
use super::error::WebAutomationError;
use super::types::{AutomationRequest, ProxyRequest, SseEvent};

const DEFAULT_BASE_URL: &str = "https://agent.tinyfish.ai";
const ENV_API_KEY: &str = "TINYFISH_API_KEY";
const AUTOMATION_PATH: &str = "/v1/automation/run-sse";

/// SSE streaming runner for web automation.
/// Sends events to the provided channel as they arrive.
pub struct SseRunner {
    base_url: String,
    event_tx: Sender<SseEvent>,
}

impl SseRunner {
    pub fn new(event_tx: Sender<SseEvent>) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            event_tx,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn api_key(&self) -> Result<String, WebAutomationError> {
        std::env::var(ENV_API_KEY).map_err(|_| WebAutomationError::MissingApiKey)
    }

    /// Run automation with SSE streaming. Returns final result on completion.
    pub async fn run(&self, args: &WebAutomationArgs) -> Result<serde_json::Value, WebAutomationError> {
        let api_key = self.api_key()?;

        let request = AutomationRequest {
            url: args.url.clone(),
            goal: args.goal.clone(),
            browser_profile: Some(args.browser_profile.clone()),
            proxy_config: args.proxy_config.as_ref().map(|p| ProxyRequest {
                enabled: p.enabled,
                country_code: p.country_code.clone(),
            }),
        };

        let url = format!(
            "{}{}",
            self.base_url.trim_end_matches('/'),
            AUTOMATION_PATH
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 min overall timeout (but SSE can run longer)
            .build()
            .map_err(|e| WebAutomationError::RequestFailed(e.to_string()))?;

        let response = client
            .post(&url)
            .header("X-API-Key", api_key)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&request)
            .send()
            .await
            .map_err(|e| WebAutomationError::RequestFailed(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(WebAutomationError::ApiError {
                code: status.as_str().to_string(),
                message: text,
            });
        }

        // Stream SSE events
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut final_result: Option<serde_json::Value> = None;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| WebAutomationError::RequestFailed(e.to_string()))?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            // Process complete SSE events in buffer
            while let Some(pos) = buffer.find("\n\n") {
                let event_text = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                // Parse SSE event
                if let Some(event) = parse_sse_event(&event_text) {
                    let is_complete = event.is_complete();
                    let is_error = event.is_error();

                    // Clone for sending before we potentially move it
                    let event_clone = event.clone();

                    // Extract result if complete
                    if is_complete {
                        final_result = event_clone.result_json.clone().or_else(|| {
                            Some(serde_json::json!({
                                "status": event_clone.status,
                                "run_id": event_clone.run_id,
                            }))
                        });
                    }

                    // Send event to channel
                    if self.event_tx.send(event_clone).await.is_err() {
                        // Receiver dropped, but continue processing
                    }

                    // Return on completion or error
                    if is_complete || is_error {
                        if is_error {
                            return Err(WebAutomationError::ApiError {
                                code: "ERROR".to_string(),
                                message: event.error.unwrap_or_else(|| "Unknown error".to_string()),
                            });
                        }
                        return Ok(final_result.unwrap_or(serde_json::json!({})));
                    }
                }
            }
        }

        // Stream ended without COMPLETE
        Ok(final_result.unwrap_or(serde_json::json!({
            "status": "incomplete",
            "message": "Stream ended without completion"
        })))
    }
}

/// Parse SSE event from text (format: "data: {...}\n" or "data: {...}\nid: ...\n")
fn parse_sse_event(text: &str) -> Option<SseEvent> {
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("data: ") {
            let json_str = line.strip_prefix("data: ")?;
            return serde_json::from_str(json_str).ok();
        }
    }
    None
}
