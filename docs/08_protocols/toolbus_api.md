# ToolBus API

Complete API specification for all ToolBus tools.

## Tools

### file_read
```
Input:  { path: string }
Output: { content: string, size: usize }
Permission: read
```

### file_write
```
Input:  { path: string, content: string }
Output: { ok: bool }
Permission: write
```

### run_cmd
```
Input:  { cmd: string, cwd: Option<string>, timeout: Option<u64> }
Output: { stdout: string, stderr: string, exit_code: i32 }
Permission: execute
```

### grep
```
Input:  { pattern: string, path: Option<string>, glob: Option<string>, case_sensitive: Option<bool> }
Output: { matches: [{ file: string, line: u32, text: string }] }
Permission: read
```

### glob
```
Input:  { pattern: string }
Output: { files: [string] }
Permission: read
```

### git_status
```
Input:  {}
Output: { status: string, branch: string, clean: bool }
Permission: read
```

### git_diff
```
Input:  { path: Option<string>, staged: Option<bool> }
Output: { diff: string }
Permission: read
```

### git_add
```
Input:  { paths: [string] }
Output: { ok: bool }
Permission: write
```

### git_commit
```
Input:  { message: string }
Output: { hash: string }
Permission: git_write
```

### git_push
```
Input:  { force: Option<bool> }
Output: { ok: bool }
Permission: git_write (force blocked by default)
```

## Common Response Envelope

```json
{
  "tool": "tool_name",
  "success": true,
  "result": { ... },
  "duration_ms": 42
}
```

On error:
```json
{
  "tool": "tool_name",
  "success": false,
  "error": "permission denied: write requires approval",
  "duration_ms": 0
}
```
