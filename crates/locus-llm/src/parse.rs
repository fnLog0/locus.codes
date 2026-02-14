//! Response parser: extract JSON (reasoning, tool_calls, confidence).

use crate::types::CompletionResponse;
use anyhow::{Context, Result};

/// Parse LLM output into structured response. Tolerates markdown code fences.
pub fn parse_response(raw: &str) -> Result<CompletionResponse> {
    let trimmed = raw.trim();
    let json_str = if let Some(start) = trimmed.find('{') {
        let end = trimmed.rfind('}').context("no closing brace")?;
        &trimmed[start..=end]
    } else {
        trimmed
    };
    let response: CompletionResponse =
        serde_json::from_str(json_str).context("parse LLM JSON response")?;
    Ok(response)
}
