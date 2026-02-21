//! Web automation UI module.
//!
//! Provides a dedicated screen for running browser automation with real-time
//! SSE progress updates. Access via Ctrl+W from the main chat.

pub mod state;
pub mod view;

pub use state::{AutomationStatus, SseEventType, WebAutomationState};
pub use view::draw_web_automation;
