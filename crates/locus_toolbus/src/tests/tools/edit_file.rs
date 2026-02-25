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
    assert!(tool.description().contains("Edit"));
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
    assert_eq!(args.old_string, Some("old".to_string()));
    assert_eq!(args.new_string, Some("new".to_string()));
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
    assert!(schema["properties"]["edits"].is_object());
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .contains(&json!("path")));
    // old_string is no longer required (can be empty for overwrite)
    assert!(!schema["required"]
        .as_array()
        .unwrap()
        .contains(&json!("old_string")));
    // new_string is no longer required (can use edits array instead)
    assert!(!schema["required"]
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

#[test]
fn test_execute_edit_file_overwrite_empty_old_string() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "old content").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "old_string": "",
                "new_string": "new content"
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["mode"], "overwrite");
        assert_eq!(result["bytes_written"], 11);

        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "new content");
    });
}

#[test]
fn test_execute_edit_file_overwrite_missing_old_string() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "old content").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "new_string": "new content"
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["mode"], "overwrite");

        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "new content");
    });
}

#[test]
fn test_execute_edit_file_overwrite_creates_file() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "new_file.txt",
                "old_string": "",
                "new_string": "created content"
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["mode"], "overwrite");

        let content = tokio::fs::read_to_string(temp_dir.path().join("new_file.txt"))
            .await
            .unwrap();
        assert_eq!(content, "created content");
    });
}

#[test]
fn test_execute_edit_file_overwrite_creates_dirs() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "subdir/nested/new_file.txt",
                "old_string": "",
                "new_string": "nested content"
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());

        let content =
            tokio::fs::read_to_string(temp_dir.path().join("subdir/nested/new_file.txt"))
                .await
                .unwrap();
        assert_eq!(content, "nested content");
    });
}

// Multiedit tests

#[test]
fn test_execute_edit_file_multiedit_basic() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "foo bar baz\n").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "edits": [
                    {
                        "old_string": "foo",
                        "new_string": "one"
                    },
                    {
                        "old_string": "bar",
                        "new_string": "two"
                    },
                    {
                        "old_string": "baz",
                        "new_string": "three"
                    }
                ]
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["mode"], "multiedit");
        assert_eq!(result["edits_applied"], 3);
        assert_eq!(result["total_matches_found"], 3);
        assert_eq!(result["total_matches_replaced"], 3);

        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "one two three\n");
    });
}

#[test]
fn test_execute_edit_file_multiedit_with_replace_all() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "foo foo foo\nbar bar\n").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "edits": [
                    {
                        "old_string": "foo",
                        "new_string": "x",
                        "replace_all": true
                    },
                    {
                        "old_string": "bar",
                        "new_string": "y",
                        "replace_all": true
                    }
                ]
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["edits_applied"], 2);
        assert_eq!(result["total_matches_found"], 5);
        assert_eq!(result["total_matches_replaced"], 5);

        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "x x x\ny y\n");
    });
}

#[test]
fn test_execute_edit_file_multiedit_mixed_replace() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "foo foo bar baz\n").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "edits": [
                    {
                        "old_string": "foo",
                        "new_string": "one",
                        "replace_all": true
                    },
                    {
                        "old_string": "bar",
                        "new_string": "two"
                    },
                    {
                        "old_string": "baz",
                        "new_string": "three"
                    }
                ]
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["edits_applied"], 3);
        assert_eq!(result["total_matches_found"], 4);
        assert_eq!(result["total_matches_replaced"], 4);

        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "one one two three\n");
    });
}

#[test]
fn test_execute_edit_file_multiedit_old_string_not_found() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "hello world\n").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "edits": [
                    {
                        "old_string": "hello",
                        "new_string": "hi"
                    },
                    {
                        "old_string": "nonexistent",
                        "new_string": "replacement"
                    }
                ]
            }))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Edit 2"));
        assert!(err.contains("not found in file"));
    });
}

#[test]
fn test_execute_edit_file_multiedit_multiple_matches_error() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "foo foo foo\n").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "edits": [
                    {
                        "old_string": "foo",
                        "new_string": "bar"
                    }
                ]
            }))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Edit 1"));
        assert!(err.contains("multiple matches"));
    });
}

#[test]
fn test_execute_edit_file_multiedit_empty_old_string_error() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "hello world\n").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "edits": [
                    {
                        "old_string": "",
                        "new_string": "replacement"
                    }
                ]
            }))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("old_string cannot be empty in multiedit mode"));
    });
}

#[test]
fn test_execute_edit_file_multiedit_file_not_found() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "nonexistent.txt",
                "edits": [
                    {
                        "old_string": "foo",
                        "new_string": "bar"
                    }
                ]
            }))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("File not found"));
    });
}

#[test]
fn test_execute_edit_file_multiedit_sequential_application() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(&temp_dir, "test.txt", "step1\n").await;

        let tool = edit_file_with_history(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "test.txt",
                "edits": [
                    {
                        "old_string": "step1",
                        "new_string": "step2"
                    },
                    {
                        "old_string": "step2",
                        "new_string": "step3"
                    },
                    {
                        "old_string": "step3",
                        "new_string": "final"
                    }
                ]
            }))
            .await
            .unwrap();

        assert!(result["success"].as_bool().unwrap());

        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "final\n");
    });
}

