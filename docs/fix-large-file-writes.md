# Fix: Large File Writes

## The Problem

When the agent used a single `create_file` call with very large content (40k+ chars):
1. **Tool call arguments were truncated** — JSON payload cut off by transport limits.
2. **JSON incomplete: EOF while parsing** — Truncation mid-string → invalid JSON.
3. **Content too large** — Tool calls are a control plane, not a data pipe.

## How Others Solve This

| Tool | Approach |
|------|----------|
| **Claude Code** | No chunked protocol. `write_to_file` + `edit_file` (search/replace). Tool descriptions tell the LLM to work incrementally. Errors if too big. |
| **Amp** | Same. `create_file` + `edit_file`. Prompt instructions say "keep changes small". No temp files. |
| **Cursor** | LLM generates a plan/sketch. A separate fine-tuned "fast apply" model rewrites the full file at ~1000 tok/s. No chunked protocol. |

**None of them use a chunked write protocol.** They all rely on prompt instructions + incremental edits.

## Our Solution

**Instruct the LLM via tool descriptions to build files incrementally. No special protocol needed.**

The `create_file` tool description tells the LLM:
> "IMPORTANT: Never put more than ~8000 characters of content in a single call — the JSON payload will be truncated and the call will fail. For larger files, create a small skeleton first, then use multiple edit_file calls to insert or replace sections incrementally."

### What was removed

| Component | Status |
|-----------|--------|
| `finalize_file` tool (3 files) | **Deleted** |
| `edit_file` append mode | **Deleted** |
| `run_chunked_create_file()` in runtime | **Deleted** |
| `SAFE_TOOL_PAYLOAD` constant | **Deleted** |
| `CHUNKED_TEMP_SUFFIX` constant | **Deleted** |
| `create_file` description | **Updated** — tells LLM to build incrementally |

### What stays

- `edit_file` overwrite mode (`old_string: ""`) — handles full-content writes
- `edit_file` find/replace — the natural way to build up a file incrementally
- `edit_file` multiedit — batch multiple edits in one call
