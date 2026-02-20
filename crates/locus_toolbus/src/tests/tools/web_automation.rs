use crate::tools::{WebAutomation, WebAutomationArgs, Tool};
use serde_json::json;

#[test]
fn test_web_automation_tool_name() {
    let tool = WebAutomation::new();
    assert_eq!(tool.name(), "web_automation");
}

#[test]
fn test_web_automation_tool_description() {
    let tool = WebAutomation::new();
    let desc = tool.description();
    assert!(desc.contains("browser automation"));
    assert!(desc.contains("TinyFish"));
    assert!(desc.contains("TINYFISH_API_KEY"));
}

#[test]
fn test_web_automation_args_parsing() {
    let args: WebAutomationArgs = serde_json::from_value(json!({
        "url": "https://example.com",
        "goal": "Extract the page title"
    }))
    .unwrap();

    assert_eq!(args.url, "https://example.com");
    assert_eq!(args.goal, "Extract the page title");
    assert_eq!(args.browser_profile, "lite");
    assert!(args.proxy_config.is_none());
}

#[test]
fn test_web_automation_args_with_options() {
    let args: WebAutomationArgs = serde_json::from_value(json!({
        "url": "https://shop.example.com",
        "goal": "List first 5 product names",
        "browser_profile": "stealth",
        "proxy_config": { "enabled": true, "country_code": "US" }
    }))
    .unwrap();

    assert_eq!(args.browser_profile, "stealth");
    let proxy = args.proxy_config.unwrap();
    assert!(proxy.enabled);
    assert_eq!(proxy.country_code.as_deref(), Some("US"));
}

#[test]
fn test_web_automation_parameters_schema() {
    let tool = WebAutomation::new();
    let schema = tool.parameters_schema();

    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["url"].is_object());
    assert!(schema["properties"]["goal"].is_object());
    assert!(schema["required"].as_array().unwrap().contains(&json!("url")));
    assert!(schema["required"].as_array().unwrap().contains(&json!("goal")));
}

#[test]
fn test_web_automation_execute_missing_api_key() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        unsafe { std::env::remove_var("TINYFISH_API_KEY") };
        let tool = WebAutomation::new();
        let result = tool
            .execute(json!({
                "url": "https://example.com",
                "goal": "Get title"
            }))
            .await;

        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("API key") || err_msg.contains("TINYFISH_API_KEY"));
    });
}

#[test]
fn test_web_automation_tool_bus_integration() {
    let tool = WebAutomation::new();
    assert_eq!(tool.name(), "web_automation");

    let bus = crate::ToolBus::new(std::path::PathBuf::from("/tmp"));
    let tools = bus.list_tools();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"web_automation"));
}
