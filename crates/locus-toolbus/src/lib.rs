//! locus-toolbus — execution gateway; all file/cmd/git ops go through here (plan §0.6).

mod tools;

pub use tools::{FileReadTool, FileWriteTool, RunCmdTool, GrepTool, GlobTool, GitStatusTool, GitDiffTool, GitAddTool, GitCommitTool, GitPushTool, Tool, ToolResult};

use anyhow::Result;
use std::path::PathBuf;

/// Permission level for tool operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// Always allowed
    Read,
    /// Ask user before executing (for write operations)
    Write,
    /// Ask user before executing (for command execution)
    Execute,
    /// Always ask (for git write operations like push, force commit)
    GitWrite,
}

/// ToolBus holds repo root and dispatches to tools.
pub struct ToolBus {
    repo_root: PathBuf,
    /// Write operations require confirmation
    pub require_write_confirmation: bool,
    /// Execute operations require confirmation
    pub require_execute_confirmation: bool,
}

impl ToolBus {
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            repo_root,
            require_write_confirmation: true,
            require_execute_confirmation: true,
        }
    }

    /// Check if a tool's permission allows the operation
    pub fn check_permission(&self, tool: &str) -> (Permission, bool) {
        let permission = match tool {
            "file_read" | "grep" | "glob" | "git_status" | "git_diff" => Permission::Read,
            "file_write" | "git_add" | "git_commit" => Permission::Write,
            "run_cmd" => Permission::Execute,
            "git_push" => Permission::GitWrite,
            _ => Permission::Execute,
        };

        let allowed = match permission {
            Permission::Read => true,
            Permission::Write => !self.require_write_confirmation,
            Permission::Execute => !self.require_execute_confirmation,
            Permission::GitWrite => false, // Always ask
        };

        (permission, allowed)
    }

    /// Call a tool by name with JSON args. Returns (result, duration_ms).
    pub async fn call(
        &self,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<(serde_json::Value, u64)> {
        let start = std::time::Instant::now();
        let result = match tool {
            "file_read" => {
                let t = FileReadTool { repo_root: self.repo_root.clone() };
                t.call(args).await?
            }
            "file_write" => {
                let t = FileWriteTool { repo_root: self.repo_root.clone() };
                t.call(args).await?
            }
            "run_cmd" => {
                let t = RunCmdTool { repo_root: self.repo_root.clone() };
                t.call(args).await?
            }
            "grep" => {
                let t = GrepTool { repo_root: self.repo_root.clone() };
                t.call(args).await?
            }
            "glob" => {
                let t = GlobTool { repo_root: self.repo_root.clone() };
                t.call(args).await?
            }
            "git_status" => {
                let t = GitStatusTool { repo_root: self.repo_root.clone() };
                t.call(args).await?
            }
            "git_diff" => {
                let t = GitDiffTool { repo_root: self.repo_root.clone() };
                t.call(args).await?
            }
            "git_add" => {
                let t = GitAddTool { repo_root: self.repo_root.clone() };
                t.call(args).await?
            }
            "git_commit" => {
                let t = GitCommitTool { repo_root: self.repo_root.clone() };
                t.call(args).await?
            }
            "git_push" => {
                let t = GitPushTool { repo_root: self.repo_root.clone() };
                t.call(args).await?
            }
            _ => anyhow::bail!("unknown tool: {}", tool),
        };
        let duration_ms = start.elapsed().as_millis() as u64;
        Ok((result, duration_ms))
    }
}
