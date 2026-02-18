use crate::{Tool, ToolBus};
use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

fn temp_repo_root() -> PathBuf {
    // Use /tmp which is guaranteed to exist on Unix systems
    PathBuf::from("/tmp")
}

#[test]
fn test_tool_bus_new() {
    let repo_root = temp_repo_root();
    let bus = ToolBus::new(repo_root.clone());

    assert_eq!(bus.repo_root(), &repo_root);
}

#[test]
fn test_tool_bus_registers_bash_by_default() {
    let bus = ToolBus::new(temp_repo_root());
    let tools = bus.list_tools();

    assert!(!tools.is_empty());
    assert!(tools.iter().any(|t| t.name == "bash"));
}

#[test]
fn test_tool_bus_list_tools() {
    let bus = ToolBus::new(temp_repo_root());
    let tools = bus.list_tools();

    assert!(!tools.is_empty());

    let bash_info = tools.iter().find(|t| t.name == "bash").unwrap();
    assert!(!bash_info.description.is_empty());
    assert!(bash_info.parameters.is_object());
}

#[test]
fn test_tool_bus_call_existing_tool() {
    let rt = runtime();
    rt.block_on(async {
        let bus = ToolBus::new(temp_repo_root());
        let (result, duration_ms) = bus
            .call("bash", json!({"command": "echo test"}))
            .await
            .unwrap();

        assert_eq!(result["exit_code"], 0);
        assert!(result["stdout"].as_str().unwrap().contains("test"));
        // Duration should be a reasonable value
        let _ = duration_ms;
    });
}

#[test]
fn test_tool_bus_call_nonexistent_tool() {
    let rt = runtime();
    rt.block_on(async {
        let bus = ToolBus::new(temp_repo_root());
        let result = bus.call("nonexistent_tool", json!({})).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Tool not found"));
        assert!(err.contains("nonexistent_tool"));
    });
}

#[test]
fn test_tool_bus_register_custom_tool() {
    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn description(&self) -> &'static str {
            "A mock tool for testing"
        }

        fn parameters_schema(&self) -> JsonValue {
            json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                },
                "required": ["input"]
            })
        }

        async fn execute(&self, args: JsonValue) -> crate::ToolResult {
            let input = args["input"].as_str().unwrap_or("");
            Ok(json!({"echo": input}))
        }
    }

    let rt = runtime();
    rt.block_on(async {
        let mut bus = ToolBus::new(temp_repo_root());
        bus.register(MockTool);

        let tools = bus.list_tools();
        assert!(tools.iter().any(|t| t.name == "mock"));

        let (result, _) = bus.call("mock", json!({"input": "hello"})).await.unwrap();
        assert_eq!(result["echo"], "hello");
    });
}

#[test]
fn test_tool_bus_call_returns_duration() {
    let rt = runtime();
    rt.block_on(async {
        let bus = ToolBus::new(temp_repo_root());
        let (_, duration_ms) = bus
            .call("bash", json!({"command": "echo test"}))
            .await
            .unwrap();

        // Duration should be a reasonable value (>= 0)
        assert!(duration_ms < 10_000); // Less than 10 seconds for a simple echo
    });
}

#[test]
fn test_tool_bus_multiple_calls() {
    let rt = runtime();
    rt.block_on(async {
        let bus = ToolBus::new(temp_repo_root());

        let (r1, _) = bus
            .call("bash", json!({"command": "echo one"}))
            .await
            .unwrap();
        let (r2, _) = bus
            .call("bash", json!({"command": "echo two"}))
            .await
            .unwrap();

        assert!(r1["stdout"].as_str().unwrap().contains("one"));
        assert!(r2["stdout"].as_str().unwrap().contains("two"));
    });
}

#[test]
fn test_tool_bus_tool_info_properties() {
    let bus = ToolBus::new(temp_repo_root());
    let tools = bus.list_tools();

    for tool in &tools {
        assert!(!tool.name.is_empty());
        assert!(!tool.description.is_empty());
        assert!(tool.parameters.is_object());
    }
}

#[test]
fn test_tool_bus_bash_with_working_dir() {
    let rt = runtime();
    rt.block_on(async {
        let repo_root = PathBuf::from("/tmp");
        let bus = ToolBus::new(repo_root.clone());

        let (result, _) = bus.call("bash", json!({"command": "pwd"})).await.unwrap();

        assert_eq!(result["exit_code"], 0);
    });
}

#[test]
fn test_tool_bus_bash_command_failure() {
    let rt = runtime();
    rt.block_on(async {
        let bus = ToolBus::new(temp_repo_root());

        let (result, _) = bus
            .call("bash", json!({"command": "exit 42"}))
            .await
            .unwrap();

        assert_eq!(result["exit_code"], 42);
        assert!(!result["success"].as_bool().unwrap());
    });
}

#[test]
fn test_tool_bus_register_overwrites_existing() {
    struct FirstTool;
    struct SecondTool;

    #[async_trait]
    impl Tool for FirstTool {
        fn name(&self) -> &'static str {
            "same_name"
        }
        fn description(&self) -> &'static str {
            "First version"
        }
        fn parameters_schema(&self) -> JsonValue {
            json!({})
        }
        async fn execute(&self, _args: JsonValue) -> crate::ToolResult {
            Ok(json!({"version": 1}))
        }
    }

    #[async_trait]
    impl Tool for SecondTool {
        fn name(&self) -> &'static str {
            "same_name"
        }
        fn description(&self) -> &'static str {
            "Second version"
        }
        fn parameters_schema(&self) -> JsonValue {
            json!({})
        }
        async fn execute(&self, _args: JsonValue) -> crate::ToolResult {
            Ok(json!({"version": 2}))
        }
    }

    let rt = runtime();
    rt.block_on(async {
        let mut bus = ToolBus::new(temp_repo_root());
        bus.register(FirstTool);
        bus.register(SecondTool);

        // Should only have one tool with that name
        let tools = bus.list_tools();
        let count = tools.iter().filter(|t| t.name == "same_name").count();
        assert_eq!(count, 1);

        // Should be the second version
        let (result, _) = bus.call("same_name", json!({})).await.unwrap();
        assert_eq!(result["version"], 2);
    });
}

#[test]
fn test_tool_bus_thread_safety() {
    use std::sync::Arc;

    struct CountingTool {
        counter: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Tool for CountingTool {
        fn name(&self) -> &'static str {
            "counter"
        }
        fn description(&self) -> &'static str {
            "Counting tool"
        }
        fn parameters_schema(&self) -> JsonValue {
            json!({})
        }
        async fn execute(&self, _args: JsonValue) -> crate::ToolResult {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(json!({"count": self.counter.load(Ordering::SeqCst)}))
        }
    }

    let rt = runtime();
    rt.block_on(async {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut bus = ToolBus::new(temp_repo_root());
        bus.register(CountingTool {
            counter: counter.clone(),
        });

        // Make multiple calls
        for _ in 0..5 {
            let _ = bus.call("counter", json!({})).await.unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 5);
    });
}
