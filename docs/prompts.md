# Test prompts for Locus

Use these prompts to test the agent and file-writing behavior. For CLI options and configuration, run `locus --help` and `locus config --help`.

---

## Small file (direct write)

Creates a file in one tool call (content ≤ 8k characters).

```
Create a file at src/hello.rs with this content:

fn main() {
    println!("Hello from locus");
}
```

---

## Large file (chunked write)

Triggers the chunked write flow (content > 8k characters). The runtime writes to a temp file in chunks, then finalizes to the final path.

```
Create a file named test_large.txt in the project root and write exactly this content (do not shorten or summarize): a single line of the letter 'a' repeated 10,000 times, then a newline, then the line "END".
```

---

## Alternative large-file prompt

```
Create a file docs/sample.md with a long markdown document: first a # Title line, then 20 paragraphs of lorem ipsum style placeholder text (each paragraph at least 5 sentences). Write the full content, do not truncate.
```

---

## Quick 8k check

```
Create a file big.txt containing exactly 9000 copies of the character 'x' (no newlines). Write the complete content.
```

---

## CLI reference

| Command | Description |
|--------|-------------|
| `locus --help` | All commands and global options |
| `locus tui [--workdir DIR] [--provider PROVIDER] [--model MODEL] [--onboarding]` | Run interactive TUI. Use `--onboarding` to show the config screen first (e.g. when no API key is set). |
| `locus config api [--provider PROVIDER]` | Configure LLM API key (anthropic, zai, tinyfish) |
| `locus config graph [--url URL] [--graph-id ID]` | Configure LocusGraph server and graph |
| `locus providers list` | List LLM providers |
| `locus providers test PROVIDER` | Test provider connectivity |
| `locus toolbus list` | List ToolBus tools |
| `locus run [--prompt PROMPT] ...` | Non-interactive run with optional initial prompt |
| `locus graph clean` | Remove LocusGraph cache and event queue (fresh start) |
| `locus graph clear-queue` | Same as `graph clean` |

**LocusGraph cache path:** `LOCUSGRAPH_DB_PATH` env, or `~/.locus/locus_graph_cache.db`, or `$TMPDIR/locus_graph_cache.db`. To use a project-local cache, set e.g. `LOCUSGRAPH_DB_PATH=.locus/locus_graph_cache.db`.

Configuration is stored in `~/.locus/env`. Source it after changing: `source ~/.locus/env` (or restart the shell).
