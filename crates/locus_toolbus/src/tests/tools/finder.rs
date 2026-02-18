use crate::tools::{Finder, Tool};
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }
}
"#,
    )
    .unwrap();

    fs::create_dir(dir.path().join("src/utils")).unwrap();
    fs::write(
        dir.path().join("src/utils/helper.rs"),
        r#"pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

pub fn farewell(name: &str) -> String {
    format!("Goodbye, {}!", name)
}
"#,
    )
    .unwrap();

    fs::write(
        dir.path().join("config.json"),
        r#"{
    "name": "test-project",
    "version": "1.0.0",
    "debug": true
}
"#,
    )
    .unwrap();

    dir
}

#[tokio::test]
async fn test_basic_search() {
    let dir = create_test_repo();
    let finder = Finder::new(dir.path().to_path_buf());

    let result = finder
        .execute(json!({
            "query": "println"
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();
    assert!(!matches.is_empty(), "Should find println matches");
}

#[tokio::test]
async fn test_case_insensitive_search() {
    let dir = create_test_repo();
    let finder = Finder::new(dir.path().to_path_buf());

    let result = finder
        .execute(json!({
            "query": "HELLO",
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
async fn test_case_sensitive_search() {
    let dir = create_test_repo();
    let finder = Finder::new(dir.path().to_path_buf());

    let result = finder
        .execute(json!({
            "query": "HELLO",
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
async fn test_file_type_filter() {
    let dir = create_test_repo();
    let finder = Finder::new(dir.path().to_path_buf());

    let result = finder
        .execute(json!({
            "query": "name",
            "file_type": "json"
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();

    for m in matches {
        let file = m.get("file").unwrap().as_str().unwrap();
        assert!(file.ends_with(".json"), "Should only match json files");
    }
}

#[tokio::test]
async fn test_path_filter() {
    let dir = create_test_repo();
    let finder = Finder::new(dir.path().to_path_buf());

    let result = finder
        .execute(json!({
            "query": "fn",
            "path": "src/utils"
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();

    for m in matches {
        let file = m.get("file").unwrap().as_str().unwrap();
        assert!(
            file.contains("utils"),
            "Should only match files in utils path"
        );
    }
}

#[tokio::test]
async fn test_regex_search() {
    let dir = create_test_repo();
    let finder = Finder::new(dir.path().to_path_buf());

    let result = finder
        .execute(json!({
            "query": r#"fn \w+\("#,
            "regex": true,
            "file_type": "rust"
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
async fn test_context_lines() {
    let dir = create_test_repo();
    let finder = Finder::new(dir.path().to_path_buf());

    let result = finder
        .execute(json!({
            "query": "assert_eq",
            "context_lines": 2
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();
    assert!(!matches.is_empty(), "Should find assert_eq");

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
async fn test_max_results() {
    let dir = create_test_repo();
    let finder = Finder::new(dir.path().to_path_buf());

    let result = finder
        .execute(json!({
            "query": "fn",
            "max_results": 2,
            "file_type": "rust"
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();
    assert!(matches.len() <= 2, "Should respect max_results limit");
}

#[tokio::test]
async fn test_exclude_patterns() {
    let dir = create_test_repo();
    let finder = Finder::new(dir.path().to_path_buf());

    let result = finder
        .execute(json!({
            "query": "fn",
            "exclude": ["utils"],
            "file_type": "rust"
        }))
        .await
        .unwrap();

    let matches = result.get("matches").unwrap().as_array().unwrap();

    for m in matches {
        let file = m.get("file").unwrap().as_str().unwrap();
        assert!(!file.contains("utils"), "Should exclude files in utils");
    }
}

#[tokio::test]
async fn test_empty_query_error() {
    let dir = create_test_repo();
    let finder = Finder::new(dir.path().to_path_buf());

    let result = finder
        .execute(json!({
            "query": ""
        }))
        .await;

    assert!(result.is_err(), "Should return error for empty query");
}

#[tokio::test]
async fn test_no_matches() {
    let dir = create_test_repo();
    let finder = Finder::new(dir.path().to_path_buf());

    let result = finder
        .execute(json!({
            "query": "nonexistent_pattern_xyz123"
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
async fn test_search_result_structure() {
    let dir = create_test_repo();
    let finder = Finder::new(dir.path().to_path_buf());

    let result = finder
        .execute(json!({
            "query": "fn main"
        }))
        .await
        .unwrap();

    assert!(result.get("query").is_some(), "Should have query field");
    assert!(result.get("matches").is_some(), "Should have matches field");
    assert!(
        result.get("files_searched").is_some(),
        "Should have files_searched field"
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
