//! OpenAI Chat Completions client (api.openai.com).

use crate::parse::parse_response;
use crate::prompt::build_user_content;
use crate::types::{CompletionRequest, CompletionResponse};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

use super::client::ModelClient;

/// OpenAI HTTP client. Uses OPENAI_API_KEY env and Chat Completions API.
pub struct OpenAIClient {
    base_url: String,
    model: String,
    api_key: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

impl OpenAIClient {
    /// Create client. `api_key` required; `base_url` and `model` optional (defaults: api.openai.com, gpt-4o-mini).
    pub fn new(api_key: Option<String>, base_url: Option<String>, model: Option<String>) -> Result<Self> {
        let api_key = api_key
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| anyhow::anyhow!("OpenAI API key required: set OPENAI_API_KEY or pass api_key"))?;
        Ok(Self {
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com".to_string()),
            model: model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
            api_key,
            client: reqwest::Client::new(),
        })
    }

    async fn chat(&self, request: CompletionRequest) -> Result<String> {
        let user_content = build_user_content(&request);
        let url = format!("{}/v1/chat/completions", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": request.system_prompt },
                { "role": "user", "content": user_content }
            ],
            "response_format": { "type": "json_object" }
        });
        let res = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;
        let status = res.status();
        let text = res.text().await?;
        if !status.is_success() {
            anyhow::bail!("openai error {}: {}", status, text);
        }
        let parsed: ChatResponse = serde_json::from_str(&text)?;
        let content = parsed
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("")
            .to_string();
        Ok(content)
    }
}

#[async_trait]
impl ModelClient for OpenAIClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let raw = self.chat(request).await?;
        parse_response(&raw)
    }
}
