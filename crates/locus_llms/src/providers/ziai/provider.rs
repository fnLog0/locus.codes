//! Z.AI provider implementation

use super::convert::{from_ziai_response, to_ziai_request};
use super::stream::create_stream;
use super::types::{ZiaiConfig, ZiaiResponse};
use crate::error::{Error, Result};
use crate::provider::Provider;
use crate::types::{GenerateRequest, GenerateResponse, GenerateStream, Headers};
use async_trait::async_trait;
use reqwest::Client;
use reqwest_eventsource::EventSource;

/// Z.AI provider
pub struct ZiaiProvider {
    config: ZiaiConfig,
    client: Client,
}

impl ZiaiProvider {
    /// Environment variable for API key
    pub const API_KEY_ENV: &'static str = "ZAI_API_KEY";

    /// Create a new Z.AI provider
    pub fn new(config: ZiaiConfig) -> Result<Self> {
        if config.api_key.is_empty() {
            return Err(Error::MissingApiKey("ziai".to_string()));
        }

        let client = Client::new();
        Ok(Self { config, client })
    }

    /// Create provider from environment
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var(Self::API_KEY_ENV)
            .map_err(|_| Error::MissingApiKey("ziai".to_string()))?;

        Self::new(ZiaiConfig::new(api_key))
    }
}

#[async_trait]
impl Provider for ZiaiProvider {
    fn provider_id(&self) -> &str {
        "ziai"
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
            "glm-4-plus".to_string(),
            "glm-4-air".to_string(),
            "glm-4-airx".to_string(),
            "glm-4-flash".to_string(),
            "glm-4-long".to_string(),
            "glm-4v-plus".to_string(),
            "glm-4v-flash".to_string(),
        ])
    }

    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        let url = format!("{}chat/completions", self.config.base_url);
        let ziai_request = to_ziai_request(&request, false)?;
        let headers = self.build_headers(request.options.headers.as_ref());

        let response = self
            .client
            .post(&url)
            .headers(headers.to_reqwest_headers())
            .json(&ziai_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::provider_error(format!(
                "Z.AI API error {}: {}",
                status, error_text
            )));
        }

        let ziai_resp: ZiaiResponse = response.json().await?;
        from_ziai_response(ziai_resp)
    }

    async fn stream(&self, request: GenerateRequest) -> Result<GenerateStream> {
        let url = format!("{}chat/completions", self.config.base_url);
        let ziai_request = to_ziai_request(&request, true)?;
        let headers = self.build_headers(request.options.headers.as_ref());

        let req_builder = self
            .client
            .post(&url)
            .headers(headers.to_reqwest_headers())
            .json(&ziai_request);

        let event_source = EventSource::new(req_builder)
            .map_err(|e| Error::stream_error(format!("Failed to create event source: {}", e)))?;

        create_stream(event_source).await
    }
}
