# View Routing

## Router

The View Router manages which view is displayed in the main content area. Only one view is active at a time. Views are pushed/popped on a stack.

## Default View

**Task Board** is the default (home) view on startup.

## Navigation

| Method | Description |
|--------|-------------|
| Keyboard shortcuts | Direct jump to any view (see keybindings) |
| Command palette | `:view <name>` |
| Automatic | Diff Review auto-opens when patches ready |

## View Stack

Views can be pushed onto a stack for contextual navigation:

```
Task Board → Plan View → Agents View → (back) → Plan View → (back) → Task Board
```

- `Esc` pops the current view (returns to previous)
- Direct shortcuts replace the stack top

## Automatic View Switching

The runtime can trigger view changes via Event Bus:

| Event | View Opened |
|-------|-------------|
| Patches generated | Diff Review |
| Tests failing | Logs View |
| Debug loop started | Agents View |
