# Plan: Polished Onboarding UI

## Goal

Upgrade the setup wizard from plain text-in-a-box to a polished, branded onboarding experience that feels like a proper first-run screen — using only what ratatui provides (no images, no web).

---

## Current Problems

| Issue | Detail |
|---|---|
| **No visual hierarchy** | Every step looks the same — title in border, flat text list |
| **No progress indicator** | User has no idea how many steps remain |
| **No branding** | Welcome screen is just plain text, no logo or identity |
| **Input field is inline text** | No visual distinction between the input area and surrounding text |
| **No visual feedback on selection** | Selected item just has `›` prefix, no background highlight |
| **No spacing/breathing room** | Content is crammed; lines run together |
| **No transition feel** | Steps snap instantly; no visual continuity between them |
| **Confirm screen is sparse** | Just key-value pairs; doesn't feel like a summary card |
| **Done screen is underwhelming** | Single green line, then "press Enter" |

---

## Design Principles

1. **Use the palette** — every element uses semantic colors from `LocusPalette` (accent, surface, elevated_surface, element_hover, etc.)
2. **ratatui only** — no external deps; use `Block`, `Paragraph`, `Layout`, `Span` styling, unicode box chars
3. **Consistent structure** — every step has: header bar, progress dots, content area, footer hints
4. **Breathing room** — generous vertical spacing, padded content
5. **Dark-first** — designed for the dark palette (which is default), works on light too

---

## Layout Structure (Every Step)

```
┌──────────────────────────────────────────────────────────┐
│  locus.codes                          Step 2 of 5  ● ● ◉ ○ ○ │  ← Header bar (surface bg)
├──────────────────────────────────────────────────────────┤
│                                                          │
│                                                          │
│     [Step Title — accent, bold]                          │
│                                                          │
│     [Description — text_muted]                           │
│                                                          │
│     ┌─────────────────────────────────┐                  │  ← Content (input/selection)
│     │  [content area]                 │                  │
│     └─────────────────────────────────┘                  │
│                                                          │
│     [Error message — danger, if any]                     │
│                                                          │
│                                                          │
├──────────────────────────────────────────────────────────┤
│  Enter confirm  │  Esc back  │  ↑↓ select               │  ← Footer hints (muted)
└──────────────────────────────────────────────────────────┘
```

Three vertical zones inside the centered box:
1. **Header bar** (2 lines): brand name left, step progress right — `surface_background` bg
2. **Content area** (flexible): step title + description + interactive content — `background` bg
3. **Footer bar** (1 line): context-sensitive key hints — `surface_background` bg

---

## Implementation Steps

### Step 1: Three-Zone Layout

**File**: `crates/locus_tui/src/layouts/setup.rs`

Replace the current single-Paragraph approach with a three-zone layout:

```rust
fn setup_zones(area: Rect) -> (Rect, Rect, Rect) {
    // Centered card (max 70 wide, max 22 tall)
    let card = centered_rect(area);
    let zones = Layout::vertical([
        Constraint::Length(2),   // header
        Constraint::Min(10),    // content
        Constraint::Length(1),  // footer
    ]).split(card);
    (zones[0], zones[1], zones[2])
}
```

**Header bar**: Draw with `surface_background` bg, border bottom. Left: "locus.codes" (bold, text color). Right: progress dots.

**Footer bar**: Draw with `surface_background` bg, border top. Show context-sensitive hints as `key` (accent) + `action` (muted) pairs separated by ` │ `.

---

### Step 2: Progress Indicator

Show step progress as dots in the header. Map the 9 `SetupStep` variants to 5 logical steps (LocusGraph sub-steps count as one):

```
Step mapping:
  Welcome         → step 1 of 5
  SelectProvider   → step 2 of 5
  EnterApiKey      → step 3 of 5
  LocusGraphChoice → step 4 of 5
  LocusGraphUrl    → step 4 of 5
  LocusGraphSecret → step 4 of 5
  LocusGraphId     → step 4 of 5
  Confirm          → step 5 of 5
  Done             → step 5 of 5 (filled)
```

Render: filled circle `●` (accent) for completed, current ring `◉` (accent), empty `○` (text_muted) for remaining.

```rust
fn progress_dots(current: usize, total: usize, palette: &LocusPalette) -> Vec<Span<'static>> {
    (0..total).map(|i| {
        if i < current {
            Span::styled("● ", text_style(palette.accent))
        } else if i == current {
            Span::styled("◉ ", text_style(palette.accent))
        } else {
            Span::styled("○ ", text_muted_style(palette.text_muted))
        }
    }).collect()
}
```

---

### Step 3: ASCII Logo on Welcome

Replace the plain "Welcome to locus.codes." with a small ASCII art logo:

```
    ╭──╮
    │  │  locus.codes
    ╰──╯
    Terminal-native coding agent
```

Or a simpler styled approach using unicode block elements:

```
    ▐█▌  locus.codes
         Terminal-native coding agent with memory
```

Keep it 2-3 lines max. Logo char in `accent`, product name in `text` + bold, tagline in `text_muted`.

Below that, the welcome text and "Press Enter to begin →".

---

### Step 4: Styled Selection List

Replace the bare `›`/`  ` prefix with highlighted rows using background color on the selected item:

```rust
fn selection_item(
    label: &str,
    description: &str,
    selected: bool,
    palette: &LocusPalette,
    width: usize,
) -> Line<'static> {
    let bg = if selected {
        background_style(palette.element_selected)  // Rgb(36, 40, 59) in dark
    } else {
        Style::default()
    };
    let prefix = if selected { "› " } else { "  " };
    let prefix_style = if selected {
        text_style(palette.accent)
    } else {
        text_muted_style(palette.text_muted)
    };

    // Pad the line to full width so background fills the row
    let content = format!("{}{:<pad$}", prefix, label, pad = width - 2);
    // ... build spans with bg applied to all
}
```

This gives selected items a visible highlight band across the row (like a menu).

---

### Step 5: Bordered Input Field

Replace the inline `> ****▌` with a bordered input box:

```
    ┌─ API Key ──────────────────────────────┐
    │  ************************************▌ │
    └────────────────────────────────────────┘
```

Use `Block::default().borders(Borders::ALL).title(label)` with `border_focused` color, and render the masked text inside with padding.

```rust
fn draw_input_field(
    frame: &mut Frame,
    area: Rect,           // allocated rect for the input block (3 lines tall)
    label: &str,
    value: &str,
    is_secret: bool,
    cursor_visible: bool,
    palette: &LocusPalette,
) {
    let display = if is_secret { masked_secret(value) } else { value.to_string() };
    let block = Block::default()
        .title(format!(" {} ", label))
        .borders(Borders::ALL)
        .border_style(border_focused_style(palette.border_focused))
        .style(background_style(palette.elevated_surface_background));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut spans = vec![
        Span::styled("  ".to_string(), text_style(palette.text)),
        Span::styled(display, text_style(palette.text)),
    ];
    if cursor_visible {
        spans.push(Span::styled("▌", text_style(palette.accent)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), inner);
}
```

---

### Step 6: Summary Card on Confirm

Replace the flat key-value lines with a bordered card:

```
    ┌─ Configuration ───────────────────────┐
    │                                        │
    │  Provider     Anthropic                │
    │  API Key      sk-a...xY4z              │
    │  LocusGraph   ✓ Configured             │
    │  URL          grpc-dev.locusgraph.com   │
    │  Graph ID     locus-agent              │
    │                                        │
    └────────────────────────────────────────┘
```

Use `elevated_surface_background` for the card bg, `border_variant` for the border. Labels in `text_muted`, values in `text`. LocusGraph status uses `success` color for `✓` or `text_muted` for "Skipped".

---

### Step 7: Done Screen with Check Animation

On the Done step, show a large checkmark with success color and a brief shimmer animation on the "saved" text:

```
        ✓

    Configuration saved.
    Press Enter to start chatting.
```

The `✓` uses `success` color, large (centered). Use the existing `Shimmer` animation on "Configuration saved" text for a brief visual highlight before it settles to static `success` color.

Add to `SetupState`:
```rust
pub done_shimmer: Option<Shimmer>,  // created when entering Done step
```

Tick the shimmer on each frame while on Done screen. After ~2 seconds (shimmer cycles once), stop and show static.

---

### Step 8: Context-Sensitive Footer Hints

Each step shows different key hints in the footer. Render as styled key badges:

```rust
fn footer_hints(step: SetupStep, palette: &LocusPalette) -> Line<'static> {
    let hints = match step {
        SetupStep::Welcome => vec![("Enter", "begin")],
        SetupStep::SelectProvider => vec![("↑↓", "select"), ("Enter", "confirm")],
        SetupStep::EnterApiKey => vec![("Enter", "continue"), ("Esc", "back")],
        SetupStep::LocusGraphChoice => vec![("↑↓", "select"), ("Enter", "confirm"), ("Esc", "back")],
        SetupStep::LocusGraphUrl
        | SetupStep::LocusGraphSecret
        | SetupStep::LocusGraphId => vec![("Enter", "continue"), ("Esc", "back")],
        SetupStep::Confirm => vec![("Enter", "save & start"), ("Esc", "back")],
        SetupStep::Done => vec![("Enter", "start chatting")],
    };
    // Render: "Enter" in accent, "begin" in muted, " │ " separator
}
```

Key name in `accent` + bold, action in `text_muted`, separator `│` in `border` color.

---

### Step 9: Step Title + Description Pattern

Every content step follows the same visual pattern:

```rust
fn step_header_lines(title: &str, description: &str, palette: &LocusPalette) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(title.to_string(), text_style(palette.text).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(description.to_string(), text_muted_style(palette.text_muted)),
        ]),
        Line::from(""),
    ]
}
```

Titles and descriptions per step:

| Step | Title | Description |
|---|---|---|
| Welcome | (logo instead) | Terminal-native coding agent with memory |
| SelectProvider | Choose your LLM provider | Which provider's models should the agent use? |
| EnterApiKey | Enter your {Provider} API key | Paste your key — it's stored locally in ~/.locus |
| LocusGraphChoice | Memory | LocusGraph gives the agent memory across sessions |
| LocusGraphUrl | LocusGraph server URL | The gRPC endpoint for your LocusGraph instance |
| LocusGraphSecret | LocusGraph secret | Your agent authentication secret |
| LocusGraphId | Graph ID | Namespace for this agent's memory (e.g. locus-agent) |
| Confirm | Review your configuration | Everything looks good? |
| Done | (checkmark instead) | Configuration saved |

---

## File Changes

| File | Change |
|---|---|
| `crates/locus_tui/src/layouts/setup.rs` | **Rewrite**: three-zone layout, header bar, progress dots, styled selections, bordered inputs, summary card, footer hints |
| `crates/locus_tui/src/state.rs` | Add `done_shimmer: Option<Shimmer>` to `SetupState` |
| `crates/locus_tui/src/setup.rs` | Initialize `done_shimmer` on Done transition |
| `crates/locus_tui/src/run.rs` | Tick `done_shimmer` in render loop when on Setup/Done |

No new files. No new dependencies.

---

## Visual Reference (Dark Theme)

### Welcome
```
┌──────────────────────────────────────────────────────────────────┐
│  locus.codes                                       ◉ ○ ○ ○ ○   │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│                                                                  │
│       ▐█▌  locus.codes                                           │
│            Terminal-native coding agent with memory               │
│                                                                  │
│                                                                  │
│       This wizard sets up the minimum config to get started.     │
│       You'll need an API key for at least one LLM provider.      │
│                                                                  │
│                                                                  │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│  Enter begin                                                     │
└──────────────────────────────────────────────────────────────────┘
```

### Select Provider
```
┌──────────────────────────────────────────────────────────────────┐
│  locus.codes                                       ● ◉ ○ ○ ○   │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│       Choose your LLM provider                                   │
│       Which provider's models should the agent use?              │
│                                                                  │
│       ┃█ Anthropic    Claude models (sonnet, opus, haiku)  ██████│  ← selected row bg
│       ┃  ZAI          GLM models (glm-5, glm-4-plus)            │
│                                                                  │
│                                                                  │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│  ↑↓ select  │  Enter confirm                                    │
└──────────────────────────────────────────────────────────────────┘
```

### Enter API Key
```
┌──────────────────────────────────────────────────────────────────┐
│  locus.codes                                       ● ● ◉ ○ ○   │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│       Enter your Anthropic API key                               │
│       Paste your key — it's stored locally in ~/.locus           │
│                                                                  │
│       ┌─ ANTHROPIC_API_KEY ────────────────────────────┐         │
│       │  **************************************▌       │         │
│       └────────────────────────────────────────────────┘         │
│                                                                  │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│  Enter continue  │  Esc back                                     │
└──────────────────────────────────────────────────────────────────┘
```

### Confirm
```
┌──────────────────────────────────────────────────────────────────┐
│  locus.codes                                       ● ● ● ● ◉   │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│       Review your configuration                                  │
│       Everything looks good?                                     │
│                                                                  │
│       ┌─ Configuration ───────────────────────────────┐          │
│       │                                                │          │
│       │  Provider      Anthropic                       │          │
│       │  API Key       sk-a...xY4z                     │          │
│       │  LocusGraph    ✓ Configured                    │          │
│       │  URL           grpc-dev.locusgraph.com         │          │
│       │  Graph ID      locus-agent                     │          │
│       │                                                │          │
│       └────────────────────────────────────────────────┘          │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│  Enter save & start  │  Esc back                                 │
└──────────────────────────────────────────────────────────────────┘
```

### Done
```
┌──────────────────────────────────────────────────────────────────┐
│  locus.codes                                       ● ● ● ● ●   │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│                                                                  │
│                                                                  │
│                          ✓                                       │
│                                                                  │
│               Configuration saved.                               │
│                                                                  │
│           Press Enter to start chatting.                          │
│                                                                  │
│                                                                  │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│  Enter start chatting                                            │
└──────────────────────────────────────────────────────────────────┘
```

---

## Testing

1. `cargo build` — compiles
2. `cargo run -- config reset && cargo run` — full wizard flow
3. `cargo run -- tui --onboarding` — opens wizard with keys present
4. Resize terminal small (< 80 cols) — layout should degrade gracefully (min constraints)
5. Light theme test (if toggle exists) — all elements visible
6. Walk every step forward and backward — Esc goes back correctly
7. Enter empty API key — error shown in danger color below input
8. Done screen shimmer plays briefly, then settles

---

## Summary

The key changes are structural, not functional:
- **Three-zone layout** (header + content + footer) instead of flat paragraph
- **Progress dots** in header for orientation
- **Bordered input fields** instead of inline `>`
- **Highlighted selection rows** with background color
- **Summary card** with elevated surface background
- **Branded welcome** with mini logo
- **Animated done** screen with checkmark
- **Footer key hints** that change per step

All using existing palette colors, existing `Shimmer` animation, and ratatui primitives. No new crates.
