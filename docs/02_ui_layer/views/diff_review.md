# Diff Review

PR-style diff approval screen. Shows generated patches with syntax-highlighted diffs.

## Layout

```
┌──────────────────────────────────────────┐
│ Diff Review: Fix auth bug in login.rs    │
│ Files changed: 2                         │
├──────────────────────────────────────────┤
│ ▸ src/auth/login.rs (+12 -3)             │
│ ▸ src/auth/tests.rs (+28 -0)             │
├──────────────────────────────────────────┤
│ src/auth/login.rs                        │
│                                          │
│  42 │   fn validate_token(&self) {       │
│  43 │-    if token.is_empty() {          │
│  43 │+    if token.is_empty() || expired │
│  44 │       return Err(AuthError);       │
│  45 │     }                              │
│                                          │
├──────────────────────────────────────────┤
│ [a]pprove  [r]eject  [e]dit  [n]ext     │
│ > _                                      │
└──────────────────────────────────────────┘
```

## Actions

| Key | Action |
|-----|--------|
| `a` | Approve all changes |
| `r` | Reject all changes |
| `e` | Edit (request modification via prompt) |
| `n` / `p` | Next / previous file |
| `j` / `k` | Scroll down / up within diff |

## Behavior

- File list at top, navigable
- Syntax-highlighted diffs with context lines
- Hunk-level approve/reject possible
- On approve → patches applied, pipeline continues
- On reject → Orchestrator notified, can retry or abort
