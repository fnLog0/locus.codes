# UI Layer Overview

Built with **ratatui + crossterm**. Terminal-native. The UI is mission control, not a text editor.

## Layout

```
┌──────────────────────────────────────────┐
│  Nav Bar  [mode: Smart]  [view: Tasks]   │
├──────────────────────────────────────────┤
│                                          │
│           Main Content Area              │
│       (switches between 6 views)         │
│                                          │
│                                          │
├──────────────────────────────────────────┤
│  > prompt input bar (always visible)     │
└──────────────────────────────────────────┘
```

## Views

| View | Purpose | Default |
|------|---------|---------|
| Task Board | Prompt history, task status, queue | ✓ (home) |
| Plan View | Execution DAG visualization | |
| Agents View | Active subagent cards | |
| Diff Review | PR-style patch approval | |
| Logs View | Command output | |
| Memory Trace | LocusGraph debug (optional) | |

## Key Properties

- **Prompt bar is global** — always visible at the bottom, across all views
- **Event-driven** — UI updates via Event Bus from runtime
- **Responsive** — adapts to terminal size
- **Theme support** — respects terminal colors
- **Keyboard-first** — all actions available via keybindings
