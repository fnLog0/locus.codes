# Permissions

ToolBus enforces a permission model on all tool calls.

## Permission Levels

| Level | Behavior | Tools |
|-------|----------|-------|
| **read** | Always allowed | file_read, grep, glob, git_status, git_diff |
| **write** | Configurable: ask / auto-approve | file_write |
| **execute** | Sandboxed, configurable | run_cmd |
| **git_write** | Always ask for push/force-push | git_commit, git_push |

## Configuration

Per-session permission config:

```
permissions:
  write: auto        # auto | ask
  execute: ask        # auto | ask
  git_write: ask      # always ask (not configurable for push)
  allowed_commands:
    - cargo test
    - npm test
    - pytest
  blocked_commands:
    - rm -rf
    - sudo
```

## Rules

- **Dangerous operations always require confirmation**: force-push, delete, sudo
- **Allowlists/blocklists** for command execution
- **Project directory only**: no filesystem access outside repo root
- **No network access by default** for executed commands
- Permission prompts shown in UI prompt bar
