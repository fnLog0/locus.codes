mod args;
mod error;

pub use args::{ProxyConfig, WebAutomationArgs};
pub use error::WebAutomationError;

use crate::tools::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;

const DEFAULT_BASE_URL: &str = "https://agent.tinyfish.ai";
const ENV_API_KEY: &str = "TINYFISH_API_KEY";
/// TinyFish docs use run-sse; response is SSE with final event type "COMPLETE" and resultJson.
const AUTOMATION_PATH: &str = "/v1/automation/run-sse";

pub struct WebAutomation {
    base_url: String,
}

impl WebAutomation {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn api_key(&self) -> Result<String, WebAutomationError> {
        std::env::var(ENV_API_KEY).map_err(|_| WebAutomationError::MissingApiKey)
    }

    async fn run_automation(&self, args: &WebAutomationArgs) -> Result<JsonValue, WebAutomationError> {
        let api_key = self.api_key()?;

        let mut body = serde_json::json!({
            "url": args.url,
            "goal": args.goal,
            "browser_profile": args.browser_profile,
        });
        if let Some(ref proxy) = args.proxy_config {
            body["proxy_config"] = serde_json::json!({
                "enabled": proxy.enabled,
                "country_code": proxy.country_code,
            });
        }

        let url = format!(
            "{}{}",
            self.base_url.trim_end_matches('/'),
            AUTOMATION_PATH
        );
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("X-API-Key", api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| WebAutomationError::RequestFailed(e.to_string()))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| WebAutomationError::RequestFailed(e.to_string()))?;

        if !status.is_success() {
            let err_json: Result<serde_json::Value, _> = serde_json::from_str(&text);
            if let Ok(js) = err_json {
                if let Some(err) = js.get("error") {
                    let code = err
                        .get("code")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    let message = err
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or(&text)
                        .to_string();
                    return Err(WebAutomationError::ApiError { code, message });
                }
            }
            return Err(WebAutomationError::ApiError {
                code: status.as_str().to_string(),
                message: text,
            });
        }

        // Parse SSE: lines "data: {...}"; final event is type "COMPLETE" with resultJson.
        let mut result_json = None;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with("data: ") {
                let payload = line.strip_prefix("data: ").unwrap_or(line);
                if let Ok(ev) = serde_json::from_str::<JsonValue>(payload) {
                    if ev.get("type").and_then(|t| t.as_str()) == Some("COMPLETE") {
                        if let Some(rj) = ev.get("resultJson") {
                            result_json = Some(rj.clone());
                            break;
                        }
                        result_json = Some(ev);
                        break;
                    }
                }
            }
        }
        let result = result_json
            .unwrap_or_else(|| serde_json::json!({ "raw": text }));
        Ok(result)
    }
}

impl Default for WebAutomation {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebAutomation {
    fn name(&self) -> &'static str {
        "web_automation"
    }

    fn description(&self) -> &'static str {
        "Run browser automation on a URL using TinyFish Web Agent. Give a URL and a natural-language goal (e.g. extract product names and prices). Returns structured JSON when the automation completes. Requires TINYFISH_API_KEY. Use for scraping, form filling, or multi-step web tasks."
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "format": "uri",
                    "description": "Target website URL to automate"
                },
                "goal": {
                    "type": "string",
                    "description": "Natural language description of what to accomplish on the website"
                },
                "browser_profile": {
                    "type": "string",
                    "enum": ["lite", "stealth"],
                    "default": "lite",
                    "description": "lite = standard browser, stealth = anti-detection browser"
                },
                "proxy_config": {
                    "type": "object",
                    "properties": {
                        "enabled": { "type": "boolean" },
                        "country_code": { "type": "string", "enum": ["US", "GB", "CA", "DE", "FR", "JP", "AU"] }
                    },
                    "description": "Optional proxy (e.g. for geo-specific content)"
                }
            },
            "required": ["url", "goal"]
        })
    }

    async fn execute(&self, args: JsonValue) -> ToolResult {
        let a: WebAutomationArgs = serde_json::from_value(args)?;
        self.run_automation(&a).await.map_err(Into::into)
    }
}
