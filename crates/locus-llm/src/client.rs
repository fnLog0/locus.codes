//! LLM client: trait + Ollama implementation (self-hosted).

use crate::parse::parse_response;
use crate::prompt::build_prompt;
use crate::types::{CompletionRequest, CompletionResponse};
use anyhow::Result;
use std::time::Duration;
use tokio::time::timeout;

#[async_trait::async_trait]
pub trait ModelClient: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
}

/// Ollama HTTP client (localhost:11434). Self-hosted, no external API.
pub struct OllamaClient {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaClient {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: model.unwrap_or_else(|| "llama3.2".to_string()),
            client: reqwest::Client::new(),
        }
    }

    async fn generate(&self, prompt: String, timeout_secs: u64) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false
        });

        let duration = Duration::from_secs(timeout_secs);
        let res = timeout(duration, async {
            self.client
                .post(&url)
                .json(&body)
                .send()
                .await
        })
        .await
        .map_err(|_| anyhow::anyhow!("request timed out after {}s", timeout_secs))??;

        let status = res.status();
        let text = res.text().await?;
        if !status.is_success() {
            anyhow::bail!("ollama error {}: {}", status, text);
        }
        let parsed: serde_json::Value = serde_json::from_str(&text)?;
        let response = parsed
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(response)
    }
}

#[async_trait::async_trait]
impl ModelClient for OllamaClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let prompt = build_prompt(&request);
        let mut last_error = None;

        for attempt in 0..=request.limits.max_retries {
            // Exponential backoff: 0s, 1s, 2s, 4s, ...
            if attempt > 0 {
                let backoff_secs = 2u64.pow((attempt - 1).try_into().unwrap_or(31)).min(10);
                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                eprintln!("Retry {}/{} after {}s backoff...", attempt, request.limits.max_retries, backoff_secs);
            }

            match self.generate(prompt.clone(), request.limits.timeout_secs).await {
                Ok(raw) => match parse_response(&raw) {
                    Ok(response) => return Ok(response),
                    Err(e) => {
                        last_error = Some(anyhow::anyhow!("Parse error on attempt {}: {}", attempt + 1, e));
                        eprintln!("Parse error on attempt {}: {}", attempt + 1, e);
                    }
                },
                Err(e) => {
                    last_error = Some(anyhow::anyhow!("Request error on attempt {}: {}", attempt + 1, e));
                    eprintln!("Request error on attempt {}: {}", attempt + 1, e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed after {} attempts", request.limits.max_retries)))
    }
}
