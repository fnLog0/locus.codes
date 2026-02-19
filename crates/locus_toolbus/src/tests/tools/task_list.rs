use crate::tools::{TaskList, TaskListArgs, Tool};
use serde_json::json;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

#[test]
fn test_task_list_tool_name() {
    let tool = TaskList::new();
    assert_eq!(tool.name(), "task_list");
}

#[test]
fn test_task_list_tool_description() {
    let tool = TaskList::new();
    assert!(tool.description().contains("Plan and track tasks"));
}

#[test]
fn test_task_list_args_parsing() {
    let args: TaskListArgs = serde_json::from_value(json!({
        "action": "list"
    }))
    .unwrap();

    assert!(matches!(args.action, crate::tools::TaskListAction::List));
    assert_eq!(args.plan_id, "default");
}

#[test]
fn test_task_list_args_create() {
    let args: TaskListArgs = serde_json::from_value(json!({
        "action": "create",
        "tasks": [{ "title": "First" }, { "title": "Second" }]
    }))
    .unwrap();

    assert!(matches!(args.action, crate::tools::TaskListAction::Create));
    assert_eq!(args.tasks.len(), 2);
    assert_eq!(args.tasks[0].title, "First");
}

#[test]
fn test_task_list_parameters_schema() {
    let tool = TaskList::new();
    let schema = tool.parameters_schema();

    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["action"].is_object());
    assert!(schema["required"].as_array().unwrap().contains(&json!("action")));
}

#[test]
fn test_task_list_create_and_list() {
    let rt = runtime();
    rt.block_on(async {
        let tool = TaskList::new();

        let create_result = tool
            .execute(json!({
                "action": "create",
                "tasks": [
                    { "title": "Task A" },
                    { "title": "Task B", "description": "B desc" }
                ]
            }))
            .await
            .unwrap();

        assert_eq!(create_result["plan_id"], "default");
        let tasks = create_result["tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0]["title"], "Task A");
        assert!(tasks[0]["id"].as_str().is_some());
        assert_eq!(tasks[1]["title"], "Task B");

        let list_result = tool.execute(json!({ "action": "list" })).await.unwrap();
        assert_eq!(list_result["tasks"].as_array().unwrap().len(), 2);
    });
}

#[test]
fn test_task_list_get_update_remove() {
    let rt = runtime();
    rt.block_on(async {
        let tool = TaskList::new();
        tool.execute(json!({
            "action": "create",
            "tasks": [{ "title": "Only" }]
        }))
        .await
        .unwrap();

        let list_result = tool.execute(json!({ "action": "list" })).await.unwrap();
        let task_id = list_result["tasks"][0]["id"].as_str().unwrap();

        let get_result = tool
            .execute(json!({ "action": "get", "task_id": task_id }))
            .await
            .unwrap();
        assert_eq!(get_result["title"], "Only");

        tool.execute(json!({
            "action": "update",
            "task_id": task_id,
            "status": "in_progress"
        }))
        .await
        .unwrap();

        let list_result = tool.execute(json!({ "action": "list" })).await.unwrap();
        assert_eq!(list_result["tasks"][0]["status"], "in_progress");

        tool.execute(json!({ "action": "remove", "task_id": task_id }))
            .await
            .unwrap();

        let list_result = tool.execute(json!({ "action": "list" })).await.unwrap();
        assert!(list_result["tasks"].as_array().unwrap().is_empty());
    });
}

#[test]
fn test_task_list_add_and_reorder() {
    let rt = runtime();
    rt.block_on(async {
        let tool = TaskList::new();
        tool.execute(json!({
            "action": "create",
            "tasks": [{ "title": "First" }, { "title": "Second" }]
        }))
        .await
        .unwrap();

        tool.execute(json!({
            "action": "add",
            "tasks": [{ "title": "Third" }]
        }))
        .await
        .unwrap();

        let list_result = tool.execute(json!({ "action": "list" })).await.unwrap();
        let tasks = list_result["tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 3);
        let ids: Vec<&str> = tasks.iter().map(|t| t["id"].as_str().unwrap()).collect();

        tool.execute(json!({
            "action": "reorder",
            "order": [ids[2], ids[0], ids[1]]
        }))
        .await
        .unwrap();

        let list_result = tool.execute(json!({ "action": "list" })).await.unwrap();
        let reordered = list_result["tasks"].as_array().unwrap();
        assert_eq!(reordered[0]["title"], "Third");
        assert_eq!(reordered[1]["title"], "First");
        assert_eq!(reordered[2]["title"], "Second");
    });
}

#[test]
fn test_task_list_get_missing_task() {
    let rt = runtime();
    rt.block_on(async {
        let tool = TaskList::new();
        tool.execute(json!({ "action": "create", "tasks": [] }))
            .await
            .unwrap();

        let result = tool
            .execute(json!({ "action": "get", "task_id": "nonexistent" }))
            .await;

        assert!(result.is_err());
    });
}

#[test]
fn test_task_list_tool_bus_integration() {
    let rt = runtime();
    rt.block_on(async {
        let bus = crate::ToolBus::new(std::path::PathBuf::from("/tmp"));
        let (result, _) = bus
            .call(
                "task_list",
                json!({
                    "action": "create",
                    "tasks": [{ "title": "From bus" }]
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["plan_id"], "default");
        assert_eq!(result["tasks"].as_array().unwrap().len(), 1);
    });
}
