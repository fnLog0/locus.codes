use crate::tools::{Read, ReadArgs, Tool};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

#[test]
fn test_read_tool_name() {
    let tool = Read::new(PathBuf::from("/tmp"));
    assert_eq!(tool.name(), "read");
}

#[test]
fn test_read_tool_description() {
    let tool = Read::new(PathBuf::from("/tmp"));
    assert!(tool.description().contains("Read a file or list a directory"));
}

#[test]
fn test_read_args_parsing() {
    let args: ReadArgs = serde_json::from_value(json!({
        "path": "src/main.rs"
    }))
    .unwrap();

    assert_eq!(args.path, "src/main.rs");
    assert_eq!(args.max_bytes, 1_048_576);
}

#[test]
fn test_read_args_with_max_bytes() {
    let args: ReadArgs = serde_json::from_value(json!({
        "path": "foo.txt",
        "max_bytes": 4096
    }))
    .unwrap();

    assert_eq!(args.path, "foo.txt");
    assert_eq!(args.max_bytes, 4096);
}

#[test]
fn test_read_parameters_schema() {
    let tool = Read::new(PathBuf::from("/tmp"));
    let schema = tool.parameters_schema();

    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["path"].is_object());
    assert!(schema["required"].as_array().unwrap().contains(&json!("path")));
}

#[test]
fn test_read_file() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("hello.txt");
        tokio::fs::write(&path, "hello world\n").await.unwrap();

        let tool = Read::new(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({ "path": "hello.txt" }))
            .await
            .unwrap();

        assert_eq!(result["type"], "file");
        assert_eq!(result["path"], "hello.txt");
        assert_eq!(result["content"], "hello world\n");
        assert_eq!(result["truncated"], false);
        assert_eq!(result["size_bytes"], 12);
    });
}

#[test]
fn test_read_directory() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("a.txt"), "a").await.unwrap();
        tokio::fs::write(temp_dir.path().join("b.txt"), "b").await.unwrap();
        tokio::fs::create_dir(temp_dir.path().join("sub")).await.unwrap();

        let tool = Read::new(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({ "path": "." }))
            .await
            .unwrap();

        assert_eq!(result["type"], "directory");
        assert_eq!(result["path"], ".");
        let entries = result["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 3);
        let names: Vec<&str> = entries
            .iter()
            .map(|e| e["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"a.txt"));
        assert!(names.contains(&"b.txt"));
        assert!(names.contains(&"sub"));
    });
}

#[test]
fn test_read_not_found() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let tool = Read::new(temp_dir.path().to_path_buf());

        let result = tool
            .execute(json!({ "path": "nonexistent.txt" }))
            .await;

        assert!(result.is_err());
    });
}

#[test]
fn test_read_path_outside_workspace() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let tool = Read::new(temp_dir.path().to_path_buf());

        let result = tool
            .execute(json!({ "path": "../etc/passwd" }))
            .await;

        assert!(result.is_err());
    });
}

#[test]
fn test_read_tool_bus_integration() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("bus_read.txt"), "from bus")
            .await
            .unwrap();

        let bus = crate::ToolBus::new(temp_dir.path().to_path_buf());
        let (result, _) = bus
            .call("read", json!({ "path": "bus_read.txt" }))
            .await
            .unwrap();

        assert_eq!(result["type"], "file");
        assert_eq!(result["content"], "from bus");
    });
}
