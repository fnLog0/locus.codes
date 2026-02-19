use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WebAutomationArgs {
    /// Target website URL to automate.
    pub url: String,

    /// Natural language description of what to accomplish on the website (goal).
    pub goal: String,

    /// Browser profile: "lite" (default) or "stealth" (anti-detection).
    #[serde(default = "default_browser_profile")]
    pub browser_profile: String,

    /// Optional proxy: set enabled true and optionally country_code (US, GB, CA, DE, FR, JP, AU).
    #[serde(default)]
    pub proxy_config: Option<ProxyConfig>,
}

fn default_browser_profile() -> String {
    "lite".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProxyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub country_code: Option<String>,
}
