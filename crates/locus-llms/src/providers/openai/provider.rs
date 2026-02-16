//! OpenAI provider implementation

use super::convert::{from_openai_response, to_openai_request};
use super::stream::create_stream;
use super::types::OpenAIConfig;
use crate::error::{Error, Result};
use crate::provider::Provider;
use crate::types::{GenerateRequest, GenerateResponse, GenerateStream, Headers};
use async_trait::async_trait;
use reqwest::Client;

/// OpenAI provider
pub struct OpenAIProvider {
    config: OpenAIConfig,
    client: Client,
}

impl OpenAIProvider {
    /// Environment variable for API key
    pub const API_KEY_ENV: &'static str = "OPENAI_API_KEY";

    /// Create a new OpenAI provider
    pub fn new(config: OpenAIConfig) -> Result<Self> {
        if config.api_key.is_empty() {
            return Err(Error::MissingApiKey("openai".to_string()));
        }

        let client = Client::new();
        Ok(Self { config, client })
    }

    /// Create provider from environment
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var(Self::API_KEY_ENV)
            .map_err(|_| Error::MissingApiKey("openai".to_string()))?;

        Self::new(OpenAIConfig::new(api_key))
    }

    /// Create provider with custom base URL
    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self> {
        let api_key = std::env::var(Self::API_KEY_ENV)
            .map_err(|_| Error::MissingApiKey("openai".to_string()))?;

        Self::new(OpenAIConfig::new(api_key).with_base_url(base_url))
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn provider_id(&self) -> &str {
        "openai"
    }

    fn build_headers(&self, custom_headers: Option<&Headers>) -> Headers {
        let mut headers = Headers::new();

        headers.insert("Authorization", format!("Bearer {}", self.config.api_key));
        headers.insert("Content-Type", "application/json");

        // Add organization ID if present
        if let Some(ref org_id) = self.config.organization_id {
            headers.insert("OpenAI-Organization", org_id);
        }

        // Add project ID if present
        if let Some(ref project_id) = self.config.project_id {
            headers.insert("OpenAI-Project", project_id);
        }

        // Merge custom headers
        if let Some(custom) = custom_headers {
            headers.merge_with(custom);
        }

        headers
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/models", self.config.base_url);
        let headers = self.build_headers(None);

        let response = self
            .client
            .get(&url)
            .headers(headers.to_reqwest_headers())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::provider_error(format!(
                "OpenAI API error {}: {}",
                status, error_text
            )));
        }

        let resp: serde_json::Value = response.json().await?;

        // Extract model IDs from the response
        // Response format: { "data": [{ "id": "model-id", ... }, ...] }
        let models = resp
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("id").and_then(|id| id.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }

    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);
        let (openai_req, warnings) = to_openai_request(&request, false)?;

        let headers = self.build_headers(request.options.headers.as_ref());

        let response = self
            .client
            .post(&url)
            .headers(headers.to_reqwest_headers())
            .json(&openai_req)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::provider_error(format!(
                "OpenAI API error {}: {}",
                status, error_text
            )));
        }

        let openai_resp: super::types::OpenAIResponse = response.json().await?;
        from_openai_response(openai_resp, warnings)
    }

    async fn stream(&self, request: GenerateRequest) -> Result<GenerateStream> {
        let url = format!("{}/chat/completions", self.config.base_url);
        let (openai_req, _warnings) = to_openai_request(&request, true)?;

        let headers = self.build_headers(request.options.headers.as_ref());

        let req_builder = self
            .client
            .post(&url)
            .headers(headers.to_reqwest_headers())
            .json(&openai_req);

        let event_source = reqwest_eventsource::EventSource::new(req_builder)
            .map_err(|e| Error::stream_error(format!("Failed to create event source: {}", e)))?;

        create_stream(event_source).await
    }
}
