use serde::{Deserialize, Serialize};

/// SSE event from TinyFish automation API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub streaming_url: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub result_json: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
}

impl SseEvent {
    pub fn is_complete(&self) -> bool {
        self.event_type == "COMPLETE"
    }

    pub fn is_error(&self) -> bool {
        self.event_type == "ERROR" || self.status.as_deref() == Some("FAILED")
    }

    pub fn is_started(&self) -> bool {
        self.event_type == "STARTED"
    }

    pub fn is_progress(&self) -> bool {
        self.event_type == "PROGRESS"
    }
}

/// Parsed result from COMPLETE event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationResult {
    pub status: String,
    pub data: serde_json::Value,
}

/// Request to start web automation.
#[derive(Debug, Clone, Serialize)]
pub struct AutomationRequest {
    pub url: String,
    pub goal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_config: Option<ProxyRequest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProxyRequest {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
}
