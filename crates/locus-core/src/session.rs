//! Session state: repo root, branch, mode

use crate::mode::Mode;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Walk up from `start` until a directory contains `.git`.
pub fn detect_repo_root(start: Option<PathBuf>) -> Result<PathBuf> {
    let cwd = start.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let mut current = cwd.canonicalize().context("canonicalize start path")?;
    loop {
        if current.join(".git").exists() {
            return Ok(current);
        }
        if !current.pop() {
            anyhow::bail!("No .git found (not a git repo)")
        }
    }
}

/// Repo metadata detected on startup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMetadata {
    /// Primary language detected (Rust, TypeScript, Python, etc.)
    pub language: String,
    /// Frameworks detected (React, Flask, etc.)
    pub frameworks: Vec<String>,
    /// Test framework (cargo test, pytest, etc.)
    pub test_framework: Option<String>,
    /// Project structure summary
    pub structure: String,
}

/// Detect basic repo metadata by scanning files
fn detect_repo_metadata(repo_root: &Path) -> RepoMetadata {
    let mut language = "Unknown".to_string();
    let mut frameworks: Vec<String> = Vec::new();
    let mut test_framework = None;

    // Language detection
    if repo_root.join("Cargo.toml").exists() {
        language = "Rust".to_string();
        test_framework = Some("cargo test".to_string());
    } else if repo_root.join("package.json").exists() {
        language = "TypeScript/JavaScript".to_string();
        test_framework = Some("npm test".to_string());
        if repo_root.join("tsconfig.json").exists() {
            frameworks.push("TypeScript".to_string());
        }
    } else if repo_root.join("requirements.txt").exists()
        || repo_root.join("pyproject.toml").exists()
        || repo_root.join("setup.py").exists()
    {
        language = "Python".to_string();
        test_framework = Some("pytest".to_string());
    } else if repo_root.join("go.mod").exists() {
        language = "Go".to_string();
        test_framework = Some("go test ./...".to_string());
    }

    // Framework detection
    let src_dir = repo_root.join("src");
    if src_dir.exists() {
        // Check for common framework indicators
        if src_dir.join("main.rs").exists() {
            frameworks.push("Rust CLI".to_string());
        }
        if src_dir.join("lib.rs").exists() {
            frameworks.push("Rust Library".to_string());
        }
    }

    let structure = format!(
        "Language: {}, Frameworks: {}, Test: {}",
        language,
        if frameworks.is_empty() {
            "None".to_string()
        } else {
            frameworks.join(", ")
        },
        test_framework.as_deref().unwrap_or("None")
    );

    RepoMetadata {
        language,
        frameworks,
        test_framework,
        structure,
    }
}

/// Task result for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub prompt: String,
    pub status: String,
    pub summary: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub repo_root: PathBuf,
    pub branch: String,
    pub working_dir: PathBuf,
    pub mode: Mode,
    pub metadata: RepoMetadata,
    /// Prompt history (stored locally, not in LocusGraph)
    pub prompt_history: Vec<String>,
    /// Task results (stored locally, not in LocusGraph)
    pub task_results: Vec<TaskResult>,
}

impl SessionState {
    pub fn new(repo_root: PathBuf, mode: Mode) -> Self {
        let branch = git_branch(&repo_root).unwrap_or_else(|_| "unknown".to_string());
        let working_dir = std::env::current_dir().unwrap_or(repo_root.clone());
        let metadata = detect_repo_metadata(&repo_root);
        Self {
            repo_root,
            branch,
            working_dir,
            mode,
            metadata,
            prompt_history: Vec::new(),
            task_results: Vec::new(),
        }
    }

    /// Add a prompt to history
    pub fn add_prompt(&mut self, prompt: String) {
        self.prompt_history.push(prompt);
        // Keep only last 100 prompts
        if self.prompt_history.len() > 100 {
            self.prompt_history.remove(0);
        }
    }

    /// Add a task result
    pub fn add_task_result(&mut self, result: TaskResult) {
        self.task_results.push(result);
        // Keep only last 100 task results
        if self.task_results.len() > 100 {
            self.task_results.remove(0);
        }
    }
}

fn git_branch(repo_root: &std::path::Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_root)
        .output()
        .context("run git rev-parse")?;
    if !output.status.success() {
        anyhow::bail!("git rev-parse failed");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
