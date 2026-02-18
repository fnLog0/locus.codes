use crate::tools::{CreateFile, CreateFileArgs, Tool};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

#[test]
fn test_create_file_tool_name() {
    let tool = CreateFile::new(PathBuf::from("/tmp"));
    assert_eq!(tool.name(), "create_file");
}

#[test]
fn test_create_file_tool_description() {
    let tool = CreateFile::new(PathBuf::from("/tmp"));
    assert!(tool.description().contains("Create or overwrite"));
}

#[test]
fn test_create_file_args_parsing() {
    let args: CreateFileArgs = serde_json::from_value(json!({
        "path": "test.txt",
        "content": "hello world"
    }))
    .unwrap();

    assert_eq!(args.path, "test.txt");
    assert_eq!(args.content, "hello world");
    assert!(args.create_dirs); // Default is true
}

#[test]
fn test_create_file_args_with_create_dirs_false() {
    let args: CreateFileArgs = serde_json::from_value(json!({
        "path": "test.txt",
        "content": "hello",
        "create_dirs": false
    }))
    .unwrap();

    assert!(!args.create_dirs);
}

#[test]
fn test_parameters_schema() {
    let tool = CreateFile::new(PathBuf::from("/tmp"));
    let schema = tool.parameters_schema();

    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["path"].is_object());
    assert!(schema["properties"]["content"].is_object());
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .contains(&json!("path")));
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .contains(&json!("content")));
}

#[test]
fn test_execute_create_file() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let tool = CreateFile::new(temp_dir.path().to_path_buf());

        let result = tool
            .execute(json!({
                "path": "test.txt",
                "content": "hello world"
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["path"], "test.txt");
        assert_eq!(result["bytes_written"], 11);

        // Verify file was created
        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "hello world");
    });
}

#[test]
fn test_execute_create_file_with_subdirs() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let tool = CreateFile::new(temp_dir.path().to_path_buf());

        let result = tool
            .execute(json!({
                "path": "sub/dir/test.txt",
                "content": "nested content"
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());

        // Verify file and directories were created
        let content = tokio::fs::read_to_string(temp_dir.path().join("sub/dir/test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "nested content");
    });
}

#[test]
fn test_execute_overwrite_existing_file() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let tool = CreateFile::new(temp_dir.path().to_path_buf());

        // Create initial file
        tool.execute(json!({
            "path": "test.txt",
            "content": "original"
        }))
        .await
        .unwrap();

        // Overwrite it
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "content": "overwritten"
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());

        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "overwritten");
    });
}

#[test]
fn test_execute_empty_path() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let tool = CreateFile::new(temp_dir.path().to_path_buf());

        let result = tool
            .execute(json!({
                "path": "",
                "content": "test"
            }))
            .await;

        assert!(result.is_err());
    });
}

#[test]
fn test_tool_bus_integration() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let bus = crate::ToolBus::new(temp_dir.path().to_path_buf());

        let (result, _) = bus
            .call(
                "create_file",
                json!({
                    "path": "bus_test.txt",
                    "content": "from bus"
                }),
            )
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["path"], "bus_test.txt");
    });
}
