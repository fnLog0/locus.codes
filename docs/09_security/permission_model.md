# Permission Model

ToolBus enforces permissions on all tool calls.

## Permission Levels

| Level | Default | Configurable | Tools |
|-------|---------|-------------|-------|
| **read** | Always allowed | No | file_read, grep, glob, git_status, git_diff |
| **write** | Ask | Yes (ask / auto) | file_write, git_add |
| **execute** | Ask | Yes (ask / auto) | run_cmd |
| **git_write** | Ask | Partially | git_commit (configurable), git_push (always ask) |

## Dangerous Operations (Always Blocked or Always Ask)

| Operation | Policy |
|-----------|--------|
| `git push --force` | Blocked by default |
| `rm -rf` | Blocked |
| `sudo` | Blocked |
| `git push` | Always ask |
| Writes outside repo root | Blocked |

## Configuration

```yaml
permissions:
  write: auto           # auto | ask
  execute: ask           # auto | ask
  git_commit: ask        # auto | ask
  git_push: ask          # always ask (override ignored)
  
  allowed_commands:      # allowlist for run_cmd
    - cargo test
    - cargo build
    - npm test
    - npm run lint
    
  blocked_commands:      # blocklist (overrides allowlist)
    - rm -rf
    - sudo
    - curl
    - wget
```

## Permission Prompt

When permission is required, the UI shows a confirmation in the prompt bar:

```
Allow file_write to src/auth/login.rs? [y/n/always]
```

- `y` — allow this once
- `n` — deny
- `always` — auto-approve this tool for the session
