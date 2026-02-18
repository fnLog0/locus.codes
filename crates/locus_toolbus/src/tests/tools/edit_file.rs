use crate::history::EditHistory;
use crate::tools::{CreateFile, EditFile, EditFileArgs, Tool};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

fn edit_file_with_history(root: PathBuf) -> EditFile {
    EditFile::new(root.clone(), Arc::new(EditHistory::load_blocking(root)))
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

async fn create_test_file(temp_dir: &TempDir, name: &str, content: &str) {
    let tool = CreateFile::new(temp_dir.path().to_path_buf());
    tool.execute(json!({
        "path": name,
        "content": content
    }))
    .await
    .unwrap();
}

#[test]
fn test_edit_file_tool_name() {
    let tool = edit_file_with_history(PathBuf::from("/tmp"));
    assert_eq!(tool.name(), "edit_file");
}

#[test]
fn test_edit_file_tool_description() {
    let tool = edit_file_with_history(PathBuf::from("/tmp"));
    assert!(tool.description().contains("edits"));
}

#[test]
fn test_edit_file_args_parsing() {
    let args: EditFileArgs = serde_json::from_value(json!({
        "path": "test.txt",
        "old_string": "old",
        "new_string": "new"
    }))
    .unwrap();

    assert_eq!(args.path, "test.txt");
    assert_eq!(args.old_string, "old");
    assert_eq!(args.new_string, "new");
    assert!(!args.replace_all);
}

#[test]
fn test_edit_file_args_with_replace_all() {
    let args: EditFileArgs = serde_json::from_value(json!({
        "path": "test.txt",
        "old_string": "old",
        "new_string": "new",
        "replace_all": true
    }))
    .unwrap();

    assert!(args.replace_all);
}

#[test]
fn test_parameters_schema() {
    let tool = edit_file_with_history(PathBuf::from("/tmp"));
    let schema = tool.parameters_schema();

    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["path"].is_object());
    assert!(schema["properties"]["old_string"].is_object());
    assert!(schema["properties"]["new_string"].is_object());
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .contains(&json!("path")));
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .contains(&json!("old_string")));
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .contains(&json!("new_string")));
}

#[test]
fn test_execute_edit_file_single_replace() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "hello old world").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "old_string": "old",
                "new_string": "new"
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["matches_found"], 1);
        assert_eq!(result["matches_replaced"], 1);

        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "hello new world");
    });
}

#[test]
fn test_execute_edit_file_multiple_matches_error() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "foo bar foo bar foo").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "old_string": "foo",
                "new_string": "baz"
            }))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Multiple matches"));
    });
}

#[test]
fn test_execute_edit_file_replace_all() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "foo bar foo bar foo").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "old_string": "foo",
                "new_string": "baz",
                "replace_all": true
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["matches_found"], 3);
        assert_eq!(result["matches_replaced"], 3);

        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "baz bar baz bar baz");
    });
}

#[test]
fn test_execute_edit_file_not_found() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let tool = edit_file_with_history(temp_dir.path().to_path_buf());

        let result = tool
            .execute(json!({
                "path": "nonexistent.txt",
                "old_string": "old",
                "new_string": "new"
            }))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("File not found"));
    });
}

#[test]
fn test_execute_edit_file_old_string_not_found() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "hello world").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "old_string": "nonexistent",
                "new_string": "new"
            }))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found in file"));
    });
}

#[test]
fn test_execute_edit_file_multiline() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "line1\nline2\nline3\n").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "old_string": "line1\nline2",
                "new_string": "new1\nnew2"
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());

        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "new1\nnew2\nline3\n");
    });
}

#[test]
fn test_execute_edit_file_empty_replacement() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "hello world").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "old_string": "world",
                "new_string": ""
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());

        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "hello ");
    });
}

#[test]
fn test_tool_bus_integration() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let bus = crate::ToolBus::new(temp_dir.path().to_path_buf());

        // Create file first
        bus.call(
            "create_file",
            json!({
                "path": "bus_test.txt",
                "content": "hello old world"
            }),
        )
        .await
        .unwrap();

        // Edit it
        let (result, _) = bus
            .call(
                "edit_file",
                json!({
                    "path": "bus_test.txt",
                    "old_string": "old",
                    "new_string": "new"
                }),
            )
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["matches_replaced"], 1);
    });
}
