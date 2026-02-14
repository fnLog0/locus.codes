//! Tool trait and implementations (plan §0.6, 08_protocols/toolbus_api.md).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Common response envelope: tool name, success, result or error, duration.
#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub tool: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Async tool: call with JSON args, return JSON result.
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value>;
}

/// file_read — Input: { path }, Output: { content, size }
#[derive(Debug)]
pub struct FileReadTool {
    pub repo_root: PathBuf,
}

#[async_trait::async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &'static str {
        "file_read"
    }
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        #[derive(Deserialize)]
        struct Input {
            path: String,
        }
        let input: Input = serde_json::from_value(args)?;
        let path = self.repo_root.join(path_clean(input.path));
        if !path.starts_with(&self.repo_root) {
            anyhow::bail!("path outside repo root");
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let size = content.len();
        Ok(serde_json::json!({ "content": content, "size": size }))
    }
}

/// file_write — Input: { path, content }, Output: { ok }
#[derive(Debug)]
pub struct FileWriteTool {
    pub repo_root: PathBuf,
}

#[derive(Deserialize)]
struct FileWriteInput {
    path: String,
    content: String,
}

#[async_trait::async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &'static str {
        "file_write"
    }
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let input: FileWriteInput = serde_json::from_value(args)?;
        let path = self.repo_root.join(path_clean(input.path));
        if !path.starts_with(&self.repo_root) {
            anyhow::bail!("path outside repo root");
        }
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, input.content).await?;
        Ok(serde_json::json!({ "ok": true }))
    }
}

/// run_cmd — Input: { cmd, cwd?, timeout? }, Output: { stdout, stderr, exit_code }
#[derive(Debug)]
pub struct RunCmdTool {
    pub repo_root: PathBuf,
}

/// Blocked commands (dangerous operations)
const BLOCKED_COMMANDS: &[&str] = &[
    "rm -rf",
    "sudo",
    "curl",
    "wget",
];

/// Allowed commands (safe test/build commands)
const ALLOWED_COMMANDS: &[&str] = &[
    "cargo test",
    "cargo build",
    "cargo check",
    "cargo fmt",
    "cargo clippy",
    "npm test",
    "npm run test",
    "npm run build",
    "pytest",
    "python -m pytest",
    "go test",
    "go build",
    "make test",
    "make build",
];

/// Check if command is blocked or allowed
fn check_command_allowed(cmd: &str) -> Result<()> {
    // Check blocked commands first
    for blocked in BLOCKED_COMMANDS {
        if cmd.starts_with(blocked) || cmd.contains(blocked) {
            anyhow::bail!("Blocked command: '{}' is not allowed", blocked);
        }
    }

    // Check if command is in allowlist or starts with an allowed prefix
    let is_allowed = ALLOWED_COMMANDS.iter().any(|allowed| cmd.starts_with(allowed));

    if !is_allowed {
        // For now, be permissive during Phase 0 - just warn but allow
        // In production, we might want to enforce the allowlist more strictly
        eprintln!("Warning: '{}' is not in the command allowlist", cmd);
    }

    Ok(())
}

#[derive(Deserialize)]
struct RunCmdInput {
    cmd: String,
    cwd: Option<String>,
    timeout: Option<u64>,
}

#[async_trait::async_trait]
impl Tool for RunCmdTool {
    fn name(&self) -> &'static str {
        "run_cmd"
    }
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let input: RunCmdInput = serde_json::from_value(args)?;

        // Check if command is allowed
        check_command_allowed(&input.cmd)?;

        let cwd = input
            .cwd
            .map(|s| self.repo_root.join(path_clean(s)))
            .unwrap_or_else(|| self.repo_root.clone());
        if !cwd.starts_with(&self.repo_root) {
            anyhow::bail!("cwd outside repo root");
        }
        let _timeout_secs = input.timeout.unwrap_or(60);
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&input.cmd)
            .current_dir(&cwd)
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let exit_code = output.status.code().unwrap_or(-1);
        Ok(serde_json::json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code
        }))
    }
}

/// grep — Input: { pattern, path?, glob?, case_sensitive? }, Output: { matches[] }
#[derive(Debug)]
pub struct GrepTool {
    pub repo_root: PathBuf,
}

#[derive(Deserialize)]
struct GrepInput {
    pattern: String,
    path: Option<String>,
    case_sensitive: Option<bool>,
}

#[async_trait::async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let input: GrepInput = serde_json::from_value(args)?;
        let search_path = input
            .path
            .map(|s| self.repo_root.join(path_clean(s)))
            .unwrap_or_else(|| self.repo_root.clone());
        if !search_path.starts_with(&self.repo_root) {
            anyhow::bail!("path outside repo root");
        }

        let pattern = if input.case_sensitive.unwrap_or(false) {
            input.pattern
        } else {
            format!("(?i){}", input.pattern)
        };

        let cmd = format!(
            "grep -r -n '{}' {}",
            pattern,
            search_path.display()
        );

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .current_dir(&self.repo_root)
            .output()
            .await?;

        let matches: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(serde_json::json!({ "matches": matches }))
    }
}

/// glob — Input: { pattern }, Output: { files[] }
#[derive(Debug)]
pub struct GlobTool {
    pub repo_root: PathBuf,
}

#[derive(Deserialize)]
struct GlobInput {
    pattern: String,
}

#[async_trait::async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "glob"
    }
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let input: GlobInput = serde_json::from_value(args)?;

        let cmd = format!("find . -name '{}'", input.pattern.trim_start_matches("**/"));
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .current_dir(&self.repo_root)
            .output()
            .await?;

        let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.trim_start_matches("./").to_string())
            .collect();

        Ok(serde_json::json!({ "files": files }))
    }
}

/// git_status — Input: {}, Output: { status, branch, clean }
#[derive(Debug)]
pub struct GitStatusTool {
    pub repo_root: PathBuf,
}

#[async_trait::async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &'static str {
        "git_status"
    }
    async fn call(&self, _args: serde_json::Value) -> Result<serde_json::Value> {
        let output = tokio::process::Command::new("git")
            .args(["status", "--porcelain=v1", "-b"])
            .current_dir(&self.repo_root)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut lines = stdout.lines();

        let branch = lines
            .next()
            .and_then(|line| line.strip_prefix("## "))
            .unwrap_or("unknown")
            .to_string();

        let status: Vec<String> = lines.map(|s| s.to_string()).collect();
        let clean = status.is_empty();

        Ok(serde_json::json!({
            "status": status,
            "branch": branch,
            "clean": clean
        }))
    }
}

/// git_diff — Input: { path?, staged? }, Output: { diff }
#[derive(Debug)]
pub struct GitDiffTool {
    pub repo_root: PathBuf,
}

#[derive(Deserialize)]
struct GitDiffInput {
    path: Option<String>,
    staged: Option<bool>,
}

#[async_trait::async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &'static str {
        "git_diff"
    }
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let input: GitDiffInput = serde_json::from_value(args)?;

        let mut cmd = tokio::process::Command::new("git");
        cmd.arg("diff");

        if input.staged.unwrap_or(false) {
            cmd.arg("--staged");
        }

        if let Some(path) = input.path {
            cmd.arg(path_clean(path));
        }

        cmd.current_dir(&self.repo_root);

        let output = cmd.output().await?;
        let diff = String::from_utf8_lossy(&output.stdout).into_owned();

        Ok(serde_json::json!({ "diff": diff }))
    }
}

/// git_add — Input: { paths[] }, Output: { ok }
#[derive(Debug)]
pub struct GitAddTool {
    pub repo_root: PathBuf,
}

#[derive(Deserialize)]
struct GitAddInput {
    paths: Vec<String>,
}

#[async_trait::async_trait]
impl Tool for GitAddTool {
    fn name(&self) -> &'static str {
        "git_add"
    }
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let input: GitAddInput = serde_json::from_value(args)?;

        let mut cmd = tokio::process::Command::new("git");
        cmd.arg("add");

        for path in input.paths {
            cmd.arg(path_clean(path));
        }

        cmd.current_dir(&self.repo_root);
        cmd.output().await?;

        Ok(serde_json::json!({ "ok": true }))
    }
}

/// git_commit — Input: { message }, Output: { hash }
#[derive(Debug)]
pub struct GitCommitTool {
    pub repo_root: PathBuf,
}

#[derive(Deserialize)]
struct GitCommitInput {
    message: String,
}

#[async_trait::async_trait]
impl Tool for GitCommitTool {
    fn name(&self) -> &'static str {
        "git_commit"
    }
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let input: GitCommitInput = serde_json::from_value(args)?;

        let output = tokio::process::Command::new("git")
            .args(["commit", "-m", &input.message])
            .current_dir(&self.repo_root)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let hash = stdout
            .lines()
            .find(|line| line.starts_with('['))
            .and_then(|line| line.split(' ').nth(1))
            .map(|s| s.trim_end_matches(']').to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(serde_json::json!({ "hash": hash }))
    }
}

/// git_push — Input: { force? }, Output: { ok }
#[derive(Debug)]
pub struct GitPushTool {
    pub repo_root: PathBuf,
}

#[derive(Deserialize)]
struct GitPushInput {
    force: Option<bool>,
}

#[async_trait::async_trait]
impl Tool for GitPushTool {
    fn name(&self) -> &'static str {
        "git_push"
    }
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let input: GitPushInput = serde_json::from_value(args)?;

        let mut cmd = tokio::process::Command::new("git");
        cmd.arg("push");

        if input.force.unwrap_or(false) {
            cmd.arg("--force");
        }

        cmd.current_dir(&self.repo_root);
        cmd.output().await?;

        Ok(serde_json::json!({ "ok": true }))
    }
}

fn path_clean(p: String) -> String {
    let p = p.trim_start_matches('/');
    if p.is_empty() {
        ".".to_string()
    } else {
        p.to_string()
    }
}
