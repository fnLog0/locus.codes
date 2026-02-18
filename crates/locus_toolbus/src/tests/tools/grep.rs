use crate::tools::{Grep, Tool};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

fn create_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();

    fs::write(
        dir.path().join("main.rs"),
        r#"fn main() {
    println!("Hello, world!");
    let x = 42;
    println!("x = {}", x);
}
"#,
    )
    .unwrap();

    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("src/lib.rs"),
        r#"pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#,
    )
    .unwrap();

    fs::write(
        dir.path().join("config.json"),
        r#"{ "name": "test-project", "debug": true }"#,
    )
    .unwrap();

    dir
}

#[tokio::test]
async fn test_grep_basic_search() {
    let dir = create_test_repo();
    let grep = Grep::new(dir.path().to_path_buf());

    let result = grep
        .execute(json!({
            "pattern": "println"
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();
    assert!(!matches.is_empty(), "Should find println matches");
}

#[tokio::test]
async fn test_grep_case_insensitive() {
    let dir = create_test_repo();
    let grep = Grep::new(dir.path().to_path_buf());

    let result = grep
        .execute(json!({
            "pattern": "HELLO",
            "case_sensitive": false
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();
    assert!(
        !matches.is_empty(),
        "Should find matches with case insensitive search"
    );
}

#[tokio::test]
async fn test_grep_case_sensitive() {
    let dir = create_test_repo();
    let grep = Grep::new(dir.path().to_path_buf());

    let result = grep
        .execute(json!({
            "pattern": "HELLO",
            "case_sensitive": true
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();
    assert!(
        matches.is_empty(),
        "Should not find matches with case sensitive search"
    );
}

#[tokio::test]
async fn test_grep_regex_mode() {
    let dir = create_test_repo();
    let grep = Grep::new(dir.path().to_path_buf());

    let result = grep
        .execute(json!({
            "pattern": r#"fn \w+\("#,
            "regex": true
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();
    assert!(
        !matches.is_empty(),
        "Should find function definitions with regex"
    );
}

#[tokio::test]
async fn test_grep_path_filter() {
    let dir = create_test_repo();
    let grep = Grep::new(dir.path().to_path_buf());

    let result = grep
        .execute(json!({
            "pattern": "fn",
            "path": "src"
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();

    for m in matches {
        let file = m.get("file").unwrap().as_str().unwrap();
        assert!(
            file.starts_with("src"),
            "Should only match files in src path"
        );
    }
}

#[tokio::test]
async fn test_grep_context_lines() {
    let dir = create_test_repo();
    let grep = Grep::new(dir.path().to_path_buf());

    let result = grep
        .execute(json!({
            "pattern": "a + b",
            "context_lines": 2
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();
    assert!(!matches.is_empty(), "Should find matches");

    let first_match = &matches[0];
    let context_before = first_match
        .get("context_before")
        .unwrap()
        .as_array()
        .unwrap();
    let context_after = first_match
        .get("context_after")
        .unwrap()
        .as_array()
        .unwrap();

    assert!(
        context_before.len() <= 2,
        "Should have at most 2 lines before"
    );
    assert!(
        context_after.len() <= 2,
        "Should have at most 2 lines after"
    );
}

#[tokio::test]
async fn test_grep_max_results() {
    let dir = create_test_repo();
    let grep = Grep::new(dir.path().to_path_buf());

    let result = grep
        .execute(json!({
            "pattern": "fn",
            "max_results": 2
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();
    assert!(matches.len() <= 2, "Should respect max_results limit");
}

#[tokio::test]
async fn test_grep_files_only() {
    let dir = create_test_repo();
    let grep = Grep::new(dir.path().to_path_buf());

    let result = grep
        .execute(json!({
            "pattern": "fn",
            "files_only": true
        }))
        .await
        .unwrap();

    let files_with_matches = result.get("files_with_matches").unwrap().as_u64().unwrap();
    assert!(files_with_matches > 0, "Should find files with matches");
}

#[tokio::test]
async fn test_grep_empty_pattern_error() {
    let dir = create_test_repo();
    let grep = Grep::new(dir.path().to_path_buf());

    let result = grep
        .execute(json!({
            "pattern": ""
        }))
        .await;

    assert!(result.is_err(), "Should return error for empty pattern");
}

#[tokio::test]
async fn test_grep_no_matches() {
    let dir = create_test_repo();
    let grep = Grep::new(dir.path().to_path_buf());

    let result = grep
        .execute(json!({
            "pattern": "nonexistent_pattern_xyz123"
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();
    assert!(
        matches.is_empty(),
        "Should find no matches for non-existent pattern"
    );
}

#[tokio::test]
async fn test_grep_result_structure() {
    let dir = create_test_repo();
    let grep = Grep::new(dir.path().to_path_buf());

    let result = grep
        .execute(json!({
            "pattern": "fn main"
        }))
        .await
        .unwrap();

    assert!(result.get("pattern").is_some(), "Should have pattern field");
    assert!(result.get("matches").is_some(), "Should have matches field");
    assert!(
        result.get("files_with_matches").is_some(),
        "Should have files_with_matches field"
    );
    assert!(
        result.get("total_matches").is_some(),
        "Should have total_matches field"
    );
    assert!(
        result.get("truncated").is_some(),
        "Should have truncated field"
    );

    let matches = result.get("matches").unwrap().as_array().unwrap();
    if !matches.is_empty() {
        let first_match = &matches[0];
        assert!(first_match.get("file").is_some(), "Match should have file");
        assert!(
            first_match.get("line_number").is_some(),
            "Match should have line_number"
        );
        assert!(
            first_match.get("column").is_some(),
            "Match should have column"
        );
        assert!(first_match.get("line").is_some(), "Match should have line");
        assert!(
            first_match.get("match_start").is_some(),
            "Match should have match_start"
        );
        assert!(
            first_match.get("match_end").is_some(),
            "Match should have match_end"
        );
    }
}
