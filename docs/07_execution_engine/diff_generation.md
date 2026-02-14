# Diff Generation

How diffs are created from patches and presented for review.

## Format

Unified diff format with:
- File headers (`--- a/path` / `+++ b/path`)
- Hunk headers (`@@ -line,count +line,count @@`)
- Context lines (3 lines before/after by default)
- Added lines (`+`)
- Removed lines (`-`)

## Generation

1. PatchAgent outputs new file content
2. Diff engine compares original file ↔ new content
3. Produces unified diff with hunks
4. Syntax highlighting applied per language

## Granularity

| Level | Description |
|-------|-------------|
| **File-level** | Entire file diff (add/modify/delete) |
| **Hunk-level** | Individual change blocks within a file |
| **Line-level** | Added/removed/context lines |

## Presentation

Diffs are sent to the Diff Review view:
- Syntax-highlighted (language-aware)
- Line numbers shown
- Added lines in green, removed in red
- Context lines in default color
- File navigation for multi-file diffs
