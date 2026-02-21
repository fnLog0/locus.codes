//! Web automation state and UI for SSE-based browser automation.
//!
//! Provides a dedicated screen (Ctrl+W) for running and monitoring web automation
//! tasks with real-time SSE progress updates.

use serde::{Deserialize, Serialize};

/// State for a single web automation run.
#[derive(Debug, Clone)]
pub struct WebAutomationState {
    /// Current status of the automation.
    pub status: AutomationStatus,
    /// URL being automated.
    pub url: String,
    /// Goal of the automation.
    pub goal: String,
    /// Run ID from the API (set on STARTED event).
    pub run_id: Option<String>,
    /// Streaming URL for browser view (set on STREAMING_URL event).
    pub streaming_url: Option<String>,
    /// Progress messages received.
    pub progress_messages: Vec<String>,
    /// Final result (set on COMPLETE).
    pub result: Option<serde_json::Value>,
    /// Error message if failed.
    pub error: Option<String>,
    /// When the automation started.
    pub started_at: Option<std::time::Instant>,
    /// Duration in milliseconds (set on completion).
    pub duration_ms: Option<u64>,
    /// Scroll offset for progress view.
    pub scroll: usize,
}

impl Default for WebAutomationState {
    fn default() -> Self {
        Self {
            status: AutomationStatus::Idle,
            url: String::new(),
            goal: String::new(),
            run_id: None,
            streaming_url: None,
            progress_messages: Vec::new(),
            result: None,
            error: None,
            started_at: None,
            duration_ms: None,
            scroll: 0,
        }
    }
}

impl WebAutomationState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new automation run.
    pub fn start(&mut self, url: String, goal: String) {
        *self = Self {
            status: AutomationStatus::Starting,
            url,
            goal,
            started_at: Some(std::time::Instant::now()),
            ..Self::default()
        };
    }

    /// Handle an SSE event from the automation API.
    pub fn handle_event(&mut self, event: &SseEventType) {
        match event {
            SseEventType::Started { run_id } => {
                self.status = AutomationStatus::Running;
                self.run_id = Some(run_id.clone());
                self.add_progress(format!("Started: run_id={}", run_id));
            }
            SseEventType::StreamingUrl { run_id: _, streaming_url } => {
                self.streaming_url = Some(streaming_url.clone());
                self.add_progress(format!("Browser ready: {}", streaming_url));
            }
            SseEventType::Progress { run_id: _, purpose } => {
                self.add_progress(purpose.clone());
            }
            SseEventType::Complete { run_id: _, status, result } => {
                self.status = AutomationStatus::Completed;
                self.result = Some(result.clone());
                self.duration_ms = self.started_at.map(|t| t.elapsed().as_millis() as u64);
                self.add_progress(format!("Completed: {}", status));
            }
            SseEventType::Error { run_id: _, message } => {
                self.status = AutomationStatus::Failed;
                self.error = Some(message.clone());
                self.add_progress(format!("Error: {}", message));
            }
        }
    }

    fn add_progress(&mut self, message: String) {
        self.progress_messages.push(message);
    }

    /// Check if automation is currently running.
    pub fn is_running(&self) -> bool {
        matches!(self.status, AutomationStatus::Starting | AutomationStatus::Running)
    }

    /// Get elapsed time in human-readable format.
    pub fn elapsed(&self) -> String {
        if let Some(started) = self.started_at {
            let elapsed = started.elapsed();
            let secs = elapsed.as_secs();
            let mins = secs / 60;
            let secs = secs % 60;
            if mins > 0 {
                format!("{}m {}s", mins, secs)
            } else {
                format!("{}s", secs)
            }
        } else {
            String::new()
        }
    }

    /// Scroll up in the progress view.
    pub fn scroll_up(&mut self, delta: usize) {
        self.scroll = self.scroll.saturating_add(delta);
    }

    /// Scroll down in the progress view.
    pub fn scroll_down(&mut self, delta: usize) {
        self.scroll = self.scroll.saturating_sub(delta);
    }

    /// Reset to idle state.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Status of an automation run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomationStatus {
    /// No automation running.
    Idle,
    /// Automation starting (request sent, waiting for STARTED).
    Starting,
    /// Automation running (received STARTED).
    Running,
    /// Automation completed successfully.
    Completed,
    /// Automation failed.
    Failed,
}

/// SSE event types from the TinyFish API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SseEventType {
    #[serde(rename = "STARTED")]
    Started { run_id: String },
    #[serde(rename = "STREAMING_URL")]
    StreamingUrl { run_id: String, streaming_url: String },
    #[serde(rename = "PROGRESS")]
    Progress { run_id: String, purpose: String },
    #[serde(rename = "COMPLETE")]
    Complete {
        run_id: String,
        status: String,
        #[serde(rename = "resultJson")]
        result: serde_json::Value,
    },
    #[serde(rename = "ERROR")]
    Error { run_id: String, message: String },
}

impl TryFrom<serde_json::Value> for SseEventType {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}
