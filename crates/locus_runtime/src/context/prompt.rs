//! System prompt construction and tool formatting.

use locus_toolbus::ToolInfo;

/// Build the system prompt with tool descriptions.
///
/// Includes the agent identity, capabilities, and safety rules.
pub fn build_system_prompt(tools: &[ToolInfo]) -> String {
    let tools_desc = format_tools(tools);

    format!(
        r#"You are locus.codes, a terminal-native coding agent with persistent memory.

## Role
You help users write, refactor, debug, and understand code. You have access to
tools for file operations, command execution, and code search. You learn from
every interaction via LocusGraph memory.

## Tools Available
{tools_desc}

## Safety Rules
- Never run destructive commands without confirmation
- Never commit secrets to version control
- Always verify file paths before editing
- Use the bash tool with caution - it has full system access
- Never put more than ~8000 characters in a single create_file call (JSON truncation). For larger files, create a small skeleton first, then use multiple edit_file calls to add or replace sections incrementally.

## Memory
You have access to memories from previous sessions. Use them to:
- Maintain consistency with past decisions
- Learn from errors and solutions
- Remember project conventions and patterns
- Track user preferences

## Behavior
- Be concise and direct
- Make autonomous decisions when clear
- Ask for clarification only when truly ambiguous
- Store important decisions and outcomes to memory
"#
    )
}

/// Format tool descriptions for the system prompt.
pub(crate) fn format_tools(tools: &[ToolInfo]) -> String {
    if tools.is_empty() {
        return "No tools available.".to_string();
    }

    tools
        .iter()
        .map(|t| {
            format!(
                "- **{}**: {}",
                t.name,
                t.description.lines().next().unwrap_or("No description")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_system_prompt() {
        let tools = vec![
            ToolInfo {
                name: "bash".to_string(),
                description: "Execute shell commands".to_string(),
                parameters: serde_json::json!({}),
            },
            ToolInfo {
                name: "read".to_string(),
                description: "Read file contents".to_string(),
                parameters: serde_json::json!({}),
            },
        ];

        let prompt = build_system_prompt(&tools);

        assert!(prompt.contains("locus.codes"));
        assert!(prompt.contains("bash"));
        assert!(prompt.contains("read"));
        assert!(prompt.contains("Safety Rules"));
    }

    #[test]
    fn test_format_tools() {
        let tools = vec![ToolInfo {
            name: "bash".to_string(),
            description: "Execute commands".to_string(),
            parameters: serde_json::json!({}),
        }];

        let formatted = format_tools(&tools);

        assert!(formatted.contains("bash"));
        assert!(formatted.contains("Execute commands"));
    }

    #[test]
    fn test_format_tools_empty() {
        let tools: Vec<ToolInfo> = vec![];
        let formatted = format_tools(&tools);

        assert!(formatted.contains("No tools available"));
    }
}
