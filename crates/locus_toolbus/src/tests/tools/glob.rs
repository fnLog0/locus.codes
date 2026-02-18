use crate::tools::{Glob, Tool};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

fn create_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();

    fs::write(
        dir.path().join("main.rs"),
        r#"fn main() {
    println!("Hello, world!");
}
"#,
    )
    .unwrap();

    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }",
    )
    .unwrap();

    fs::create_dir(dir.path().join("src/utils")).unwrap();
    fs::write(dir.path().join("src/utils/helper.rs"), "pub fn greet() {}").unwrap();

    fs::write(dir.path().join("config.json"), r#"{ "name": "test" }"#).unwrap();

    fs::create_dir(dir.path().join("target")).unwrap();
    fs::write(dir.path().join("target/debug.exe"), "binary").unwrap();

    dir
}

#[tokio::test]
async fn test_basic_glob_pattern() {
    let dir = create_test_repo();
    let glob = Glob::new(dir.path().to_path_buf());

    let result = glob
        .execute(json!({
            "pattern": "**/*.rs"
        }))
        .await
        .unwrap();

    let files = result.get("files").unwrap().as_array().unwrap();
    assert!(!files.is_empty(), "Should find .rs files");

    for file in files {
        let path = file.as_str().unwrap();
        assert!(path.ends_with(".rs"), "Should only return .rs files");
    }
}

#[tokio::test]
async fn test_glob_json_pattern() {
    let dir = create_test_repo();
    let glob = Glob::new(dir.path().to_path_buf());

    let result = glob
        .execute(json!({
            "pattern": "*.json"
        }))
        .await
        .unwrap();

    let files = result.get("files").unwrap().as_array().unwrap();
    assert!(!files.is_empty(), "Should find .json files");

    for file in files {
        let path = file.as_str().unwrap();
        assert!(path.ends_with(".json"), "Should only return .json files");
    }
}

#[tokio::test]
async fn test_glob_with_path_filter() {
    let dir = create_test_repo();
    let glob = Glob::new(dir.path().to_path_buf());

    let result = glob
        .execute(json!({
            "pattern": "**/*.rs",
            "path": "src"
        }))
        .await
        .unwrap();

    let files = result.get("files").unwrap().as_array().unwrap();
    assert!(!files.is_empty(), "Should find files in src");

    for file in files {
        let path = file.as_str().unwrap();
        assert!(path.starts_with("src"), "Should only return files in src");
    }
}

#[tokio::test]
async fn test_glob_max_results() {
    let dir = create_test_repo();
    let glob = Glob::new(dir.path().to_path_buf());

    let result = glob
        .execute(json!({
            "pattern": "**/*.rs",
            "max_results": 2
        }))
        .await
        .unwrap();

    let files = result.get("files").unwrap().as_array().unwrap();
    assert!(files.len() <= 2, "Should respect max_results limit");
}

#[tokio::test]
async fn test_glob_exclude_target() {
    let dir = create_test_repo();
    let glob = Glob::new(dir.path().to_path_buf());

    let result = glob
        .execute(json!({
            "pattern": "**/*"
        }))
        .await
        .unwrap();

    let files = result.get("files").unwrap().as_array().unwrap();

    for file in files {
        let path = file.as_str().unwrap();
        assert!(
            !path.contains("target"),
            "Should exclude target directory by default"
        );
    }
}

#[tokio::test]
async fn test_glob_exclude_custom() {
    let dir = create_test_repo();
    let glob = Glob::new(dir.path().to_path_buf());

    let result = glob
        .execute(json!({
            "pattern": "**/*.rs",
            "exclude": ["utils"]
        }))
        .await
        .unwrap();

    let files = result.get("files").unwrap().as_array().unwrap();

    for file in files {
        let path = file.as_str().unwrap();
        assert!(!path.contains("utils"), "Should exclude files in utils");
    }
}

#[tokio::test]
async fn test_glob_no_matches() {
    let dir = create_test_repo();
    let glob = Glob::new(dir.path().to_path_buf());

    let result = glob
        .execute(json!({
            "pattern": "**/*.xyz"
        }))
        .await
        .unwrap();

    let files = result.get("files").unwrap().as_array().unwrap();
    assert!(files.is_empty(), "Should find no .xyz files");
}

#[tokio::test]
async fn test_glob_result_structure() {
    let dir = create_test_repo();
    let glob = Glob::new(dir.path().to_path_buf());

    let result = glob
        .execute(json!({
            "pattern": "*.json"
        }))
        .await
        .unwrap();

    assert!(result.get("pattern").is_some(), "Should have pattern field");
    assert!(result.get("files").is_some(), "Should have files field");
    assert!(
        result.get("truncated").is_some(),
        "Should have truncated field"
    );
    assert!(
        result.get("total_matches").is_some(),
        "Should have total_matches field"
    );
}

#[tokio::test]
async fn test_glob_invalid_pattern() {
    let dir = create_test_repo();
    let glob = Glob::new(dir.path().to_path_buf());

    let result = glob
        .execute(json!({
            "pattern": "[invalid"
        }))
        .await;

    assert!(
        result.is_err(),
        "Should return error for invalid glob pattern"
    );
}
