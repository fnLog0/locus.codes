# Tool UI Spec — Per-Tool TUI Rendering

Visual design for every tool in the locus.codes terminal. Each layout shows exact character positions, colors (RGB from `LocusPalette::locus_dark()`), and which spans map to which palette role.

## Design Principles

1. **One line by default** — most tools are a single status line. Preview only when it adds real value.
2. **Preview only for user-facing output** — bash stdout/stderr, edit_file diffs. Everything else goes to the LLM — the user doesn't need to see it.
3. **Max 3 preview lines** — never more. `+N more` as last line when truncated.
4. **bash is special** — `$` prompt in blue, full command in white. Only tool with highlighted summary.
5. **edit_file is special** — gets a bordered diff block. Stats (`+3 -1`) live in the diff header, not the status line.
6. **No redundant info** — don't repeat the path in both status line and diff header. Don't show file size. Don't show content previews for created files.

---

## Color Legend

Every mockup below uses these tags to indicate color:

| Tag | Palette Role | RGB (dark) | What it colors |
|---|---|---|---|
| `[TEXT]` | `text` | `(214,220,238)` | Tool names (bold), primary content |
| `[MUTED]` | `text_muted` | `(104,114,145)` | Paths, durations, detail lines, rail `┊` |
| `[ACCENT]` | `accent` | `(103,155,255)` | Spinner `⠋`, running rail, diff label |
| `[SUCCESS]` | `success` | `(126,224,158)` | `✓` icon, success rail, `+` diff lines |
| `[DANGER]` | `danger` | `(255,110,128)` | `✗` icon, error rail, `-` diff lines, stderr |
| `[WARNING]` | `warning` | `(244,188,108)` | Caution labels |
| `[INFO]` | `info` | `(102,208,255)` | Memory icons |

Background is always `(7,8,11)` — the app background. No tool renders its own background.

---

## Grid Anatomy

Every tool status line follows this grid:

```
  ┊ ✓ bash          $ cargo test  4.1s
│ │ │ │              │ │           │
│ │ │ │              │ │           └── [MUTED] duration
│ │ │ │              │ └── [TEXT] command (full, not truncated)
│ │ │ │              └── [ACCENT] $ prompt marker
│ │ │ └── [TEXT+BOLD] tool name, padded to 12 chars
│ │ └── [SUCCESS/DANGER/ACCENT] status icon
│ └── [MUTED/SUCCESS/DANGER/ACCENT] rail "┊ "
└── 2-space LEFT_PADDING (no color)
```

Bash is special — the summary shows `$ command` where `$` is `[ACCENT]` blue `(103,155,255)` and the command text is `[TEXT]` white `(214,220,238)`. All other tools show summary in `[MUTED]`.

Preview lines below the status line:

```
  ┊     Compiling locus-core v0.1.0
│ │     │
│ │     └── [MUTED or DANGER] preview content
│ └── [MUTED] rail "┊ "
└── 2-space LEFT_PADDING
```

---

## bash — Shell Command

### Running

```
  ┊ ⠋ bash          $ cargo build  3.2s
```

| Position | Content | Color |
|---|---|---|
| col 0-1 | `  ` | none (padding) |
| col 2-3 | `┊ ` | `[ACCENT]` (running rail is accent-colored) |
| col 4-5 | `⠋ ` | `[ACCENT]` spinner |
| col 6-17 | `bash          ` | `[TEXT]` bold, padded to 12 |
| col 18 | `$ ` | `[ACCENT]` prompt marker `(103,155,255)` |
| col 20+ | `cargo build` | `[TEXT]` full command `(214,220,238)` |
| trailing | `3.2s` | `[MUTED]` elapsed (live-updating) |

### Success

```
  ┊ ✓ bash          $ cargo test  4.1s
  ┊     test memory::tests::test_simple_hash … ok
  ┊     test runtime::tests::test_slugify … ok
  ┊     3 passed, 0 failed
```

| Line | Content | Color |
|---|---|---|
| 1: rail | `┊ ` | `[SUCCESS]` |
| 1: icon | `✓ ` | `[SUCCESS]` |
| 1: name | `bash          ` | `[TEXT]` bold |
| 1: `$` | `$ ` | `[ACCENT]` prompt marker |
| 1: command | `cargo test` | `[TEXT]` full command, not truncated |
| 1: duration | `4.1s` | `[MUTED]` |
| 2-4: rail | `┊ ` | `[MUTED]` |
| 2-4: indent | `    ` | none |
| 2-4: content | stdout tail | `[MUTED]` |

### Success (long output, truncated)

```
  ┊ ✓ bash          $ cargo test  12.5s
  ┊     … 28 lines above
  ┊     test tool_handler::tests::test_extract … ok
  ┊     test tool_handler::tests::test_confirm … ok
  ┊     31 passed, 0 failed
```

| Line | Content | Color |
|---|---|---|
| 2: truncation notice | `… 28 lines above` | `[MUTED]` |
| 3-5: content | last 3 stdout lines | `[MUTED]` |

### Success (multiline command — shows first line only)

```
  ┊ ✓ bash          $ echo hello && echo world  1.0s
  ┊     hello
  ┊     world
```

The full command string is shown as-is (first line if multiline). No truncation — the terminal wraps naturally.

### Failure

```
  ┊ ✗ bash          $ cargo build  2.3s  failed
  ┊     error[E0433]: failed to resolve: could no…
  ┊     --> src/main.rs:12:5
  ┊     … 5 more lines
```

| Line | Content | Color |
|---|---|---|
| 1: rail | `┊ ` | `[DANGER]` |
| 1: icon | `✗ ` | `[DANGER]` |
| 1: name | `bash          ` | `[TEXT]` bold |
| 1: `$` | `$ ` | `[ACCENT]` prompt marker (stays accent even on failure) |
| 1: command | `cargo build` | `[TEXT]` full command |
| 1: `failed` | `failed` | `[DANGER]` |
| 1: duration | `2.3s` | `[MUTED]` |
| 2-3: rail | `┊ ` | `[MUTED]` |
| 2-3: stderr | first 3 lines | `[DANGER]` |
| 4: more notice | `… 5 more lines` | `[DANGER]` |

### Error (tool itself crashed)

```
  ┊ ✗ bash          $ xyz --unknown
  ┊     command not found: xyz
```

| Line | Content | Color |
|---|---|---|
| 1: rail | `┊ ` | `[DANGER]` |
| 1: icon | `✗ ` | `[DANGER]` |
| 1: name | `bash` | `[TEXT]` bold |
| 1: `$` | `$ ` | `[ACCENT]` |
| 1: command | `xyz --unknown` | `[TEXT]` |
| 2: rail | `┊ ` | `[MUTED]` |
| 2: error | `command not found: xyz` | `[DANGER]` |

---

## edit_file — File Edit

### Success (with diff block)

Status line is compact — just path and duration. Stats move into the diff header.

```
  ┊ ✓ edit_file     src/main.rs  120ms
  ╭─ diff  src/main.rs  +3 -1  L12-L15
  │  12  -  let x = old_value;
  │  12  +  let x = new_value;
  │  13  +  let y = extra;
  │  14     unchanged_line
  ╰─
```

| Line | Content | Color |
|---|---|---|
| 1: rail | `┊ ` | `[SUCCESS]` |
| 1: icon | `✓ ` | `[SUCCESS]` |
| 1: name | `edit_file     ` | `[TEXT]` bold |
| 1: path | `src/main.rs` | `[MUTED]` |
| 1: duration | `120ms` | `[MUTED]` |
| 2: top border | `╭─ ` | `[MUTED]` |
| 2: label | `diff` | `[ACCENT]` |
| 2: path | `src/main.rs` | `[TEXT]` |
| 2: stats | `+3 -1` | `[SUCCESS]` for +, `[DANGER]` for - |
| 2: range | `L12-L15` | `[MUTED]` |
| 3: border | `│ ` | `[MUTED]` |
| 3: line num | ` 12  ` | `[MUTED]` |
| 3: marker | `-` | `[DANGER]` |
| 3: old content | `let x = old_value;` | `[DANGER]` |
| 4: marker | `+` | `[SUCCESS]` |
| 4: new content | `let x = new_value;` | `[SUCCESS]` |
| 5: marker | `+` | `[SUCCESS]` |
| 5: new content | `let y = extra;` | `[SUCCESS]` |
| 6: line num | ` 14  ` | `[MUTED]` |
| 6: content | `unchanged_line` | `[TEXT]` |
| 7: bottom border | `╰─` | `[MUTED]` |

No `showing X of Y` meta line — saves a line. Page count only appears when there are more lines than fit (press `d` to page).

### Large diff (omitted)

```
  ┊ ✓ edit_file     src/main.rs  250ms
  ╭─ diff  src/main.rs  +150 -80
  │ diff preview omitted
  ╰─
```

---

## create_file — File Creation

### Success

One line. No content preview — the file was just created, the user knows what's in it.

```
  ┊ ✓ create_file   src/new_mod.rs  42 lines  80ms
```

| Content | Color |
|---|---|
| icon | `[SUCCESS]` |
| name | `[TEXT]` bold |
| path | `[MUTED]` |
| `42 lines` | `[MUTED]` |
| duration | `[MUTED]` |

---

## undo_edit — Undo Last Edit

### Success

```
  ┊ ✓ undo_edit     src/main.rs  restored  90ms
```

| Content | Color |
|---|---|
| icon | `[SUCCESS]` |
| name | `[TEXT]` bold |
| path | `[MUTED]` |
| `restored` | `[MUTED]` |
| duration | `[MUTED]` |

Minimal — one line, no preview. The restored state is the "before" of the edit that was undone.

---

## read — File Read

### File

```
  ┊ ✓ read          src/lib.rs  [1-50]  50 lines  30ms
```

### Directory

```
  ┊ ✓ read          crates/  12 entries  15ms
```

| Content | Color |
|---|---|
| icon | `[SUCCESS]` |
| name | `[TEXT]` bold |
| path | `[MUTED]` |
| range `[1-50]` | `[MUTED]` |
| `50 lines` or `12 entries` | `[MUTED]` |
| duration | `[MUTED]` |

No preview lines — the content goes to the LLM, not the user.

---

## glob — File Pattern Matching

### Success (≤ 5 matches — no preview, count is enough)

```
  ┊ ✓ glob          **/*.toml  3 matches  15ms
```

### Success (> 5 matches — show first 3)

```
  ┊ ✓ glob          **/*.rs  23 matches  45ms
  ┊     src/main.rs
  ┊     src/lib.rs
  ┊     src/tools/mod.rs
  ┊     +20 more
```

| Line | Content | Color |
|---|---|---|
| 1: icon | `✓ ` | `[SUCCESS]` |
| 1: name | `glob          ` | `[TEXT]` bold |
| 1: pattern | `**/*.rs` | `[MUTED]` |
| 1: count | `23 matches` | `[MUTED]` |
| 1: duration | `45ms` | `[MUTED]` |
| 2-4: rail | `┊ ` | `[MUTED]` |
| 2-4: paths | first 3 matches | `[MUTED]` |
| 5: more | `+20 more` | `[MUTED]` |

Rule: preview only when > 5 matches. Max 3 preview lines. `+N more` always last.

---

## grep — Text Search

### Success

```
  ┊ ✓ grep          "TODO:"  src/  8 in 4 files  60ms
  ┊     src/main.rs:12    // TODO: fix this
  ┊     src/lib.rs:45     // TODO: add tests
  ┊     +6 more
```

| Line | Content | Color |
|---|---|---|
| 1: icon | `✓ ` | `[SUCCESS]` |
| 1: name | `grep          ` | `[TEXT]` bold |
| 1: pattern | `"TODO:"` | `[MUTED]` |
| 1: scope | `src/` | `[MUTED]` |
| 1: stats | `8 in 4 files` | `[MUTED]` |
| 1: duration | `60ms` | `[MUTED]` |
| 2-3: rail | `┊ ` | `[MUTED]` |
| 2-3: file:line | `src/main.rs:12` | `[MUTED]` |
| 2-3: match text | `// TODO: fix this` | `[MUTED]` |
| 4: more | `+6 more` | `[MUTED]` |

Max 2 preview lines. `+N more` always last. Shorter stats: `8 in 4 files` not `8 matches in 4 files`.

### No matches

```
  ┊ ✓ grep          "nonexistent"  src/  0 matches  20ms
```

---

## finder — Semantic Code Search

### Success

No preview — finder results go to the LLM, not the user. Count is enough.

```
  ┊ ✓ finder        "JWT validation logic"  5 results  120ms
```

| Content | Color |
|---|---|
| icon | `[SUCCESS]` |
| name | `[TEXT]` bold |
| query | `[MUTED]` |
| count | `[MUTED]` |
| duration | `[MUTED]` |

---

## handoff — Sub-Agent Handoff

### Running

```
  ┊ ⠋ handoff       Fix auth middleware and add tes…  2.1s
```

### Success

```
  ┊ ✓ handoff       Fix auth middleware and add tes…  45s
  ┊     Modified 3 files, added 2 tests
```

| Line | Content | Color |
|---|---|---|
| 1: rail | `┊ ` | `[SUCCESS]` |
| 1: icon | `✓ ` | `[SUCCESS]` |
| 1: name | `handoff       ` | `[TEXT]` bold |
| 1: goal | truncated to 40 chars | `[MUTED]` |
| 1: duration | `45s` | `[MUTED]` |
| 2: rail | `┊ ` | `[MUTED]` |
| 2: result | sub-agent summary | `[MUTED]` |

---

## task_list — Task Management

### Success

One line — the task list content goes to the LLM. User sees the action and count.

```
  ┊ ✓ task_list     list  3 tasks (1 active)  20ms
```

| Content | Color |
|---|---|
| icon | `[SUCCESS]` |
| name | `[TEXT]` bold |
| action | `[MUTED]` |
| count | `[MUTED]` |
| active hint | `[MUTED]` |
| duration | `[MUTED]` |

---

## web_automation — Web Browsing / Search

### URL fetch

```
  ┊ ✓ web           docs.rs/tokio  200ms
```

Path is the URL with protocol stripped. One line — content goes to LLM.

### Web search

```
  ┊ ✓ web_search    "rust async channels"  5 results  800ms
```

| Content | Color |
|---|---|
| name | `[TEXT]` bold |
| query or URL | `[MUTED]` |
| count | `[MUTED]` |
| duration | `[MUTED]` |

One line each. No result preview — the user doesn't need to see search titles.

---

## Meta-Tools (`messages/meta-tools/`)

Meta-tools use `╎` (broken bar) instead of `┊` (dotted bar) to visually distinguish them.

### search.rs — tool_search

```
  ╎ ✓ Search tools   "file operations"  4 tools  150ms
```

| Content | Color |
|---|---|
| rail | `[SUCCESS]` |
| icon | `[SUCCESS]` |
| label | `[TEXT]` (padded to 13) |
| query | `[MUTED]` |
| count | `[MUTED]` |
| duration | `[MUTED]` |

One line. The tool list goes to the LLM — the user doesn't pick tools manually.

### explain.rs — tool_explain

```
  ╎ ✓ Explain tool   edit_file  5 params  80ms
  ╎     Make edits to a text file. Replaces old_str with new_str.
```

| Content | Color |
|---|---|
| label | `[TEXT]` |
| tool name | `[MUTED]` |
| param count | `[MUTED]` |
| description | `[MUTED]` |

### tasks.rs — task (sub-agent)

### Running

```
  ╎ ⠋ Task           "Add tests for auth module"  12.5s
```

| Content | Color |
|---|---|
| rail | `[ACCENT]` |
| spinner | `[ACCENT]` |
| label | `[TEXT]` |
| description | `[MUTED]` |
| elapsed | `[MUTED]` (live-updating) |

### Done

```
  ╎ ✓ Task           "Add tests for auth module"  completed  45s
  ╎     Created tests/auth_test.rs (3 test functions)
```

| Line | Content | Color |
|---|---|---|
| 1: rail | `[SUCCESS]` |
| 1: icon | `[SUCCESS]` |
| 1: label | `[TEXT]` |
| 1: description | `[MUTED]` |
| 1: `completed` | `[SUCCESS]` |
| 1: duration | `[MUTED]` |
| 2: rail | `[MUTED]` |
| 2: result summary | `[MUTED]` |

---

## Tool Group Header

When multiple tools run in the same LLM turn, they're grouped under a header:

```
  ╭─ tools  3  ✓   4.1s
  ┊ ✓ read          src/main.rs  [1-50]  50 lines  30ms
  ┊ ✓ edit_file     src/main.rs  [L12-L15]  +3 -1  120ms
  ╭─ diff  src/main.rs
  │ showing 1-3 of 3 lines
  │  12  -  old line
  │  12  +  new line
  ╰─
  ┊ ✓ bash          $ cargo check  3.8s
  ┊     Finished `dev` profile
```

| Line | Content | Color |
|---|---|---|
| header: `╭─ ` | border | `[SUCCESS]` (all done) / `[ACCENT]` (some running) / `[DANGER]` (any failed) |
| header: `tools` | label | `[TEXT]` |
| header: `3` | tool count | `[MUTED]` |
| header: `✓` | aggregate icon | `[SUCCESS]` |
| header: `4.1s` | max duration | `[MUTED]` |

Header color logic:
- All done + all success → `[SUCCESS]` rail + `✓`
- Any still running → `[ACCENT]` rail + `⠋` + running count
- Any failed → `[DANGER]` rail + `✗` + failed count

---

## Full Conversation Example

What a complete turn looks like — clean, compact, scannable:

```
                                                        Background: (7,8,11)

  fix the JWT refresh bug in auth.rs                    [TEXT_ACCENT] (124,174,255)

  · ◎ Memory recall  fix JWT refresh  3 memories        [INFO] icon, [MUTED] text

  I'll fix the JWT refresh token issue. Let me           [TEXT] (214,220,238)
  read the file first.

  ╭─ tools  3  ✓   4.1s                                 [SUCCESS] header
  ┊ ✓ read          src/auth.rs  [1-200]  200 lines  …  one line
  ┊ ✓ edit_file     src/auth.rs  120ms                   one line
  ╭─ diff  src/auth.rs  +5 -3  L45-L52                  stats in header
  │  45  -  let token = jwt::decode(old);                [DANGER]
  │  45  +  let token = jwt::decode_with_refresh(new);   [SUCCESS]
  │  46  +  if token.is_expired() {                      [SUCCESS]
  ╰─                                                     [MUTED]
  ┊ ✓ bash          $ cargo test  3.2s                   [ACCENT] $, [TEXT] cmd
  ┊     test auth::tests::test_refresh … ok              [MUTED]
  ┊     1 passed, 0 failed                               [MUTED]

  Fixed the JWT refresh bug. The token now               [TEXT] (214,220,238)
  automatically refreshes when expired.

  ─── Turn complete · 1,234 tokens (800↑ 434↓) ───      [MUTED] separator
```

**Before optimization:** ~18 tool lines (status + previews + meta).
**After optimization:** ~12 tool lines. Same information density, 33% less vertical space.

Only bash and edit_file get multi-line output. Everything else is one line.
