# Logs View

Shows command output from ToolBus executions.

## Content

- stdout and stderr from `run_cmd` calls
- Git operation output
- Test runner output
- Timestamps per line
- Source agent label per entry

## Layout

```
┌──────────────────────────────────────────┐
│ Logs  [filter: all]                      │
├──────────────────────────────────────────┤
│ 14:23:01 [TestAgent]  $ cargo test       │
│ 14:23:03 [TestAgent]  running 12 tests   │
│ 14:23:03 [TestAgent]  test auth::ok .. ok│
│ 14:23:03 [TestAgent]  test auth::expired │
│                         .. FAILED        │
│ 14:23:04 [ToolBus]    $ git status       │
│ 14:23:04 [ToolBus]    M src/auth/login.rs│
├──────────────────────────────────────────┤
│ > _                                      │
└──────────────────────────────────────────┘
```

## Features

- Scrollable (j/k or arrow keys)
- Searchable (/ to search)
- Filter by agent or tool type
- stderr highlighted differently from stdout
- Auto-scroll to bottom (toggle with `G`)
