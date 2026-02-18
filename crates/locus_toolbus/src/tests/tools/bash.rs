use crate::tools::{Bash, BashArgs, BashExecutor, Tool, ToolOutput};
use serde_json::json;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

#[test]
fn test_bash_tool_name() {
    let bash = Bash::new();
    assert_eq!(bash.name(), "bash");
}

#[test]
fn test_bash_tool_description() {
    let bash = Bash::new();
    assert!(bash.description().contains("shell command"));
}

#[test]
fn test_bash_args_parsing() {
    let args: BashArgs = serde_json::from_value(json!({
        "command": "echo hello"
    }))
    .unwrap();

    assert_eq!(args.command, "echo hello");
    assert_eq!(args.timeout, 60);
    assert!(args.working_dir.is_none());
}

#[test]
fn test_bash_args_with_timeout() {
    let args: BashArgs = serde_json::from_value(json!({
        "command": "echo hello",
        "timeout": 30
    }))
    .unwrap();

    assert_eq!(args.timeout, 30);
}

#[test]
fn test_bash_args_with_working_dir() {
    let args: BashArgs = serde_json::from_value(json!({
        "command": "echo hello",
        "working_dir": "/tmp"
    }))
    .unwrap();

    assert_eq!(args.working_dir, Some("/tmp".to_string()));
}

#[test]
fn test_tool_output_is_success() {
    let output = ToolOutput {
        stdout: "hello".to_string(),
        stderr: "".to_string(),
        exit_code: 0,
        duration_ms: 10,
    };
    assert!(output.is_success());

    let failed = ToolOutput {
        stdout: "".to_string(),
        stderr: "error".to_string(),
        exit_code: 1,
        duration_ms: 10,
    };
    assert!(!failed.is_success());
}

#[test]
fn test_tool_output_to_json() {
    let output = ToolOutput {
        stdout: "hello".to_string(),
        stderr: "".to_string(),
        exit_code: 0,
        duration_ms: 10,
    };
    let json = output.to_json();

    assert_eq!(json["stdout"], "hello");
    assert_eq!(json["exit_code"], 0);
    assert_eq!(json["success"], true);
    assert!(json["duration_ms"].is_number());
}

#[test]
fn test_execute_echo_command() {
    let rt = runtime();
    rt.block_on(async {
        let bash = Bash::new();
        let result = bash
            .execute(json!({
                "command": "echo hello"
            }))
            .await
            .unwrap();

        assert_eq!(result["exit_code"], 0);
        assert!(result["stdout"].as_str().unwrap().contains("hello"));
        assert!(result["success"].as_bool().unwrap());
    });
}

#[test]
fn test_execute_failing_command() {
    let rt = runtime();
    rt.block_on(async {
        let bash = Bash::new();
        let result = bash
            .execute(json!({
                "command": "exit 1"
            }))
            .await
            .unwrap();

        assert_eq!(result["exit_code"], 1);
        assert!(!result["success"].as_bool().unwrap());
    });
}

#[test]
fn test_execute_with_working_dir() {
    let rt = runtime();
    rt.block_on(async {
        let bash = Bash::new();
        let result = bash
            .execute(json!({
                "command": "pwd",
                "working_dir": "/tmp"
            }))
            .await
            .unwrap();

        assert_eq!(result["exit_code"], 0);
        assert!(result["stdout"].as_str().unwrap().contains("/tmp"));
    });
}

#[test]
fn test_executor_with_working_dir() {
    let rt = runtime();
    rt.block_on(async {
        let executor = BashExecutor::new().with_working_dir("/tmp");
        let args = BashArgs {
            command: "pwd".to_string(),
            timeout: 10,
            working_dir: None,
        };
        let output = executor.run(&args).await.unwrap();

        assert!(output.is_success());
        assert!(output.stdout.contains("/tmp"));
    });
}

#[test]
fn test_bash_default() {
    let bash = Bash::default();
    assert_eq!(bash.name(), "bash");
}

#[test]
fn test_parameters_schema() {
    let bash = Bash::new();
    let schema = bash.parameters_schema();

    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["command"].is_object());
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .contains(&json!("command")));
}
