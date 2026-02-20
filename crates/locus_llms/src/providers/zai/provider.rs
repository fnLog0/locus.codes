//! Z.AI provider implementation

use super::convert::{from_zai_response, to_zai_request};
use super::stream::create_stream;
use super::types::{ZaiConfig, ZaiResponse};
use crate::error::{Error, Result};
use crate::provider::Provider;
use crate::types::{GenerateRequest, GenerateResponse, GenerateStream, Headers};
use async_trait::async_trait;
use reqwest::Client;
use reqwest_eventsource::EventSource;

/// Max retries for transient 429 rate-limit errors
const MAX_RETRIES: u32 = 3;
/// Base delay between retries (doubles each attempt)
const BASE_RETRY_DELAY_MS: u64 = 1000;

/// Z.AI provider
pub struct ZaiProvider {
    config: ZaiConfig,
    client: Client,
}

impl ZaiProvider {
    /// Environment variable for API key
    pub const API_KEY_ENV: &'static str = "ZAI_API_KEY";

    /// Create a new Z.AI provider
    pub fn new(config: ZaiConfig) -> Result<Self> {
        if config.api_key.is_empty() {
            return Err(Error::MissingApiKey("zai".to_string()));
        }

        let client = Client::new();
        Ok(Self { config, client })
    }

    /// Create provider from environment
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var(Self::API_KEY_ENV)
            .map_err(|_| Error::MissingApiKey("zai".to_string()))?;

        Self::new(ZaiConfig::new(api_key))
    }
}

#[async_trait]
impl Provider for ZaiProvider {
    fn provider_id(&self) -> &str {
        "zai"
    }

    fn build_headers(&self, custom_headers: Option<&Headers>) -> Headers {
        let mut headers = Headers::new();
        headers.insert("Authorization", format!("Bearer {}", self.config.api_key));
        headers.insert("Content-Type", "application/json");

        if let Some(custom) = custom_headers {
            headers.merge_with(custom);
        }

        headers
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        // Z.AI doesn't have a models endpoint, return known models
        Ok(vec![
            "glm-5".to_string(),
            "glm-4-plus".to_string(),
            "glm-4-air".to_string(),
            "glm-4-airx".to_string(),
            "glm-4-long".to_string(),
            "glm-4v-plus".to_string(),
            "glm-4v-flash".to_string(),
        ])
    }

    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        let url = format!("{}chat/completions", self.config.base_url);
        let zai_request = to_zai_request(&request, false)?;
        let headers = self.build_headers(request.options.headers.as_ref());

        for attempt in 0..=MAX_RETRIES {
            let response = self
                .client
                .post(&url)
                .headers(headers.to_reqwest_headers())
                .json(&zai_request)
                .send()
                .await?;

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let error_text = response.text().await.unwrap_or_default();
                
                // Check if it's a balance error (not a transient rate limit)
                if error_text.contains("balance") || error_text.contains("recharge") || error_text.contains("1113") {
                    return Err(Error::InsufficientBalance(error_text));
                }
                
                // Transient rate limit - retry if attempts remain
                if attempt < MAX_RETRIES {
                    let delay = BASE_RETRY_DELAY_MS * 2u64.pow(attempt);
                    eprintln!(
                        "[ZAI] Rate limited (429), retrying in {}ms (attempt {}/{})",
                        delay,
                        attempt + 1,
                        MAX_RETRIES
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    continue;
                }
                
                return Err(Error::RateLimitExceeded(format!(
                    "Z.AI rate limit exceeded: {}",
                    error_text
                )));
            }

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(Error::provider_error(format!(
                    "Z.AI API error {}: {}",
                    status, error_text
                )));
            }

            let zai_resp: ZaiResponse = response.json().await?;
            return from_zai_response(zai_resp);
        }

        Err(Error::RateLimitExceeded(
            "Z.AI rate limit exceeded after retries".to_string(),
        ))
    }

    async fn stream(&self, request: GenerateRequest) -> Result<GenerateStream> {
        let url = format!("{}chat/completions", self.config.base_url);
        let zai_request = to_zai_request(&request, true)?;
        let headers = self.build_headers(request.options.headers.as_ref());

        for attempt in 0..=MAX_RETRIES {
            let req_builder = self
                .client
                .post(&url)
                .headers(headers.to_reqwest_headers())
                .json(&zai_request);

            let event_source = EventSource::new(req_builder)
                .map_err(|e| Error::stream_error(format!("Failed to create event source: {}", e)))?;

            match create_stream(event_source).await {
                Ok(stream) => return Ok(stream),
                Err(Error::ProviderError(ref msg)) if msg.contains("429") => {
                    // Check if it's a balance error (not a transient rate limit)
                    if msg.contains("balance") || msg.contains("recharge") || msg.contains("1113") {
                        return Err(Error::InsufficientBalance(msg.clone()));
                    }
                    
                    if attempt < MAX_RETRIES {
                        let delay = BASE_RETRY_DELAY_MS * 2u64.pow(attempt);
                        eprintln!(
                            "[ZAI] Rate limited (429), retrying in {}ms (attempt {}/{})",
                            delay,
                            attempt + 1,
                            MAX_RETRIES
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        continue;
                    }
                    
                    return Err(Error::RateLimitExceeded(format!(
                        "Z.AI rate limit exceeded: {}",
                        msg
                    )));
                }
                other => return other,
            }
        }

        Err(Error::RateLimitExceeded(
            "Z.AI rate limit exceeded after retries".to_string(),
        ))
    }
}
