use crate::history::EditHistory;
use crate::tools::{CreateFile, EditFile, Tool, UndoEdit, UndoEditArgs};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

#[test]
fn test_undo_edit_args_parsing() {
    let args: UndoEditArgs = serde_json::from_value(json!({ "path": "src/main.rs" })).unwrap();
    assert_eq!(args.path, "src/main.rs");
}

#[test]
fn test_undo_edit_tool_name() {
    let root = PathBuf::from("/tmp");
    let tool = UndoEdit::new(root.clone(), Arc::new(EditHistory::load_blocking(root)));
    assert_eq!(tool.name(), "undo_edit");
}

#[test]
fn test_undo_edit_tool_description() {
    let root = PathBuf::from("/tmp");
    let tool = UndoEdit::new(root.clone(), Arc::new(EditHistory::load_blocking(root)));
    assert!(tool.description().contains("Undo") && tool.description().contains("edit"));
}

#[test]
fn test_undo_edit_parameters_schema() {
    let root = PathBuf::from("/tmp");
    let tool = UndoEdit::new(root.clone(), Arc::new(EditHistory::load_blocking(root)));
    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["path"].is_object());
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .contains(&json!("path")));
}

#[test]
fn test_execute_undo_edit_after_edit() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();
        let history = Arc::new(EditHistory::load_blocking(root.clone()));

        // Create file
        let create = CreateFile::new(root.clone());
        create
            .execute(json!({ "path": "f.txt", "content": "original" }))
            .await
            .unwrap();

        // Edit it
        let edit = EditFile::new(root.clone(), Arc::clone(&history));
        edit.execute(json!({
            "path": "f.txt",
            "old_string": "original",
            "new_string": "edited"
        }))
        .await
        .unwrap();

        let content_before_undo = tokio::fs::read_to_string(root.join("f.txt")).await.unwrap();
        assert_eq!(content_before_undo, "edited");

        // Undo
        let undo = UndoEdit::new(root.clone(), history);
        let result = undo.execute(json!({ "path": "f.txt" })).await.unwrap();
        assert!(result["success"].as_bool().unwrap());

        let content_after = tokio::fs::read_to_string(root.join("f.txt")).await.unwrap();
        assert_eq!(content_after, "original");
    });
}

#[test]
fn test_execute_undo_edit_nothing_to_undo() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();
        let history = Arc::new(EditHistory::load_blocking(root.clone()));

        CreateFile::new(root.clone())
            .execute(json!({ "path": "f.txt", "content": "only" }))
            .await
            .unwrap();

        let undo = UndoEdit::new(root, history);
        let result = undo.execute(json!({ "path": "f.txt" })).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Nothing to undo"));
    });
}

#[test]
fn test_tool_bus_edit_then_undo() {
    let rt = runtime();
    rt.block_on(async {
        let temp_dir = TempDir::new().unwrap();
        let bus = crate::ToolBus::new(temp_dir.path().to_path_buf());

        bus.call(
            "create_file",
            json!({ "path": "undo_test.txt", "content": "before" }),
        )
        .await
        .unwrap();

        bus.call(
            "edit_file",
            json!({
                "path": "undo_test.txt",
                "old_string": "before",
                "new_string": "after"
            }),
        )
        .await
        .unwrap();

        let (undo_result, _) = bus
            .call("undo_edit", json!({ "path": "undo_test.txt" }))
            .await
            .unwrap();

        assert!(undo_result["success"].as_bool().unwrap());

        let content = tokio::fs::read_to_string(temp_dir.path().join("undo_test.txt"))
            .await
            .unwrap();
        assert_eq!(content, "before");
    });
}
