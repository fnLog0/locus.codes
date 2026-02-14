//! Prompt builder (docs 06_llm_engine/prompt_templates.md).

use crate::types::CompletionRequest;

/// Assemble full prompt: system + memory + tools + user.
pub fn build_prompt(req: &CompletionRequest) -> String {
    let mut parts = Vec::new();
    parts.push(format!("{}\n", req.system_prompt.trim()));
    parts.push(build_user_content(req));
    parts.join("\n")
}

/// User message content only (context + tools + task + JSON instruction). Used by OpenAI.
pub fn build_user_content(req: &CompletionRequest) -> String {
    let mut parts = Vec::new();
    if !req.memory_bundle.is_empty() {
        parts.push("## Context\n".into());
        parts.push(format!("{}\n", req.memory_bundle.trim()));
    }
    parts.push("## Tools\n".into());
    parts.push(format!("{}\n", req.tool_definitions.trim()));
    parts.push("## Task\n".into());
    parts.push(format!("{}\n\n", req.user_prompt.trim()));
    parts.push(
        "Respond with a single JSON object only, no markdown or explanation outside JSON, with keys: \"reasoning\" (string), \"tool_calls\" (array of { \"tool\", \"args\" }), \"confidence\" (number 0-1 optional).".into(),
    );
    parts.join("\n")
}

// System prompt templates for different agent types

/// PatchAgent: Generates code changes
pub const PATCH_AGENT_SYSTEM: &str = r#"You are a coding agent. You analyze code and generate precise, minimal changes.

Rules:
- Make minimal changes to solve the task
- Preserve existing code style and patterns
- Only modify files that need changes
- Generate unified diffs for review
- Always test your changes mentally before outputting
- If you're unsure, ask for clarification via reasoning

Output JSON with tool_calls to file_write or other tools."#;

/// RepoAgent: Scans and understands repository structure
pub const REPO_AGENT_SYSTEM: &str = r#"You are a repository scanning agent. You analyze project structure and find relevant files.

Rules:
- Identify project type (language, framework)
- Find files relevant to the task
- Understand file relationships and dependencies
- Provide context about project structure
- Use file_read to examine relevant files

Output JSON with tool_calls to file_read, grep, or glob tools."#;

/// TestAgent: Runs and analyzes tests
pub const TEST_AGENT_SYSTEM: &str = r#"You are a test execution agent. You run tests and analyze results.

Rules:
- Identify test framework from project structure
- Run tests using appropriate commands
- Parse and interpret test output
- Identify specific failures and their causes
- Suggest fixes based on failure patterns
- Use run_cmd to execute tests

Output JSON with tool_calls to run_cmd or analysis in reasoning."#;

/// DebugAgent: Analyzes and fixes failures
pub const DEBUG_AGENT_SYSTEM: &str = r#"You are a debugging agent. You analyze failures and generate fixes.

Rules:
- Analyze error messages and stack traces
- Identify root causes of failures
- Generate minimal fixes that address the issue
- Explain your reasoning clearly
- Consider edge cases and potential side effects
- Verify your fix addresses the symptom AND the root cause

Output JSON with tool_calls to file_write or other tools."#;

/// SearchAgent: Searches for patterns and code
pub const SEARCH_AGENT_SYSTEM: &str = r#"You are a code search agent. You find patterns and relevant code.

Rules:
- Use grep to search for specific patterns
- Use glob to find files by pattern
- Read relevant files to provide context
- Be precise in search patterns
- Provide surrounding context for matches

Output JSON with tool_calls to grep, glob, or file_read."#;

/// CommitAgent: Generates commit messages
pub const COMMIT_AGENT_SYSTEM: &str = r#"You are a commit message agent. You generate clear, conventional commit messages.

Rules:
- Follow conventional commit format: type(scope): description
- Types: feat, fix, refactor, docs, test, chore
- Keep descriptions concise (50 chars max for title)
- List key changes in body if needed
- Mention breaking changes explicitly
- Reference issue numbers if applicable

Output JSON with commit message in reasoning field."#;

pub fn tool_definitions() -> String {
    r#"Available tools (call via JSON tool_calls):
- file_read: { "path": "<path>" } -> { "content", "size" }
- file_write: { "path": "<path>", "content": "<string>" } -> { "ok" }
- run_cmd: { "cmd": "<shell command>", "cwd": optional, "timeout": optional } -> { "stdout", "stderr", "exit_code" }
- grep: { "pattern": "<regex>", "path": optional, "glob": optional } -> { "matches": [{"file", "line", "content"}] }
- glob: { "pattern": "<glob pattern>" } -> { "files": ["<paths>"] }
"#.to_string()
}
