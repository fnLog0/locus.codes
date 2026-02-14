# ToolBus

Layer D — Execution Gateway. **All actions must go through ToolBus.** This is where safety and determinism lives.

## Tools

| Tool | Input | Output | Permission |
|------|-------|--------|------------|
| `file_read` | path | content | read |
| `file_write` | path, content | ok/err | write |
| `run_cmd` | command, cwd | stdout, stderr, exit code | execute |
| `grep` | pattern, path/glob | matches (file, line, text) | read |
| `glob` | pattern | file paths | read |
| `git_status` | — | status output | read |
| `git_diff` | optional path | diff output | read |
| `git_add` | paths | ok | write |
| `git_commit` | message | commit hash | git_write |
| `git_push` | — | ok/err | git_write |

## Guarantees

- **Logging**: Every tool call is logged as an event (tool, args, result, duration)
- **Permission**: Every call is permission-checked before execution
- **Determinism**: Same input → same permission check → same execution path
- **Isolation**: Commands run in project directory only
- **Timeout**: All operations have configurable timeouts

## Event Emission

Every ToolBus call emits a `ToolCalled` event on the Event Bus:

```
ToolCalled {
    tool: "run_cmd",
    args: { cmd: "cargo test", cwd: "/project" },
    agent: "TestAgent",
    timestamp: 1739500000,
}
```

Followed by `ToolResult` on completion.
