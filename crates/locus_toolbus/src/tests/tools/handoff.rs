use crate::tools::{Handoff, HandoffArgs, Tool};
use serde_json::json;
use std::path::PathBuf;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

#[test]
fn test_handoff_tool_name() {
    let tool = Handoff::new(PathBuf::from("/tmp"));
    assert_eq!(tool.name(), "handoff");
}

#[test]
fn test_handoff_tool_description() {
    let tool = Handoff::new(PathBuf::from("/tmp"));
    assert!(tool.description().contains("background"));
}

#[test]
fn test_handoff_args_parsing() {
    let args: HandoffArgs = serde_json::from_value(json!({
        "command": "sleep 1"
    }))
    .unwrap();

    assert_eq!(args.command, "sleep 1");
    assert!(args.working_dir.is_none());
}

#[test]
fn test_handoff_parameters_schema() {
    let tool = Handoff::new(PathBuf::from("/tmp"));
    let schema = tool.parameters_schema();

    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["command"].is_object());
    assert!(schema["required"].as_array().unwrap().contains(&json!("command")));
}

#[test]
fn test_handoff_execute_returns_immediately() {
    let rt = runtime();
    rt.block_on(async {
        let tool = Handoff::new(PathBuf::from("/tmp"));
        // Run a long sleep in background; we should return right away.
        let result = tool
            .execute(json!({ "command": "sleep 10" }))
            .await
            .unwrap();

        assert_eq!(result["status"], "started");
        assert!(result["handoff_id"].as_u64().is_some());
        assert_eq!(result["command"], "sleep 10");
    });
}

#[test]
fn test_handoff_tool_bus_integration() {
    let rt = runtime();
    rt.block_on(async {
        let bus = crate::ToolBus::new(PathBuf::from("/tmp"));
        let (result, _) = bus
            .call("handoff", json!({ "command": "sleep 2" }))
            .await
            .unwrap();

        assert_eq!(result["status"], "started");
        assert!(result["handoff_id"].as_u64().is_some());
    });
}
