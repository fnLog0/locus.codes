# Locus TUI Design Plan

This document is the working plan for redesigning the `locus_tui` experience.
The goal is to make the interface feel intentional, legible, and premium without losing terminal speed.

We should review this plan one section at a time before implementation.

---

## 1. Design Goal

The end result should feel like:

- a focused coding workspace, not a generic terminal app
- visually calm at rest, but expressive during activity
- easy to scan when the agent is thinking, using tools, editing files, or failing
- consistent across header, chat body, footer, and special states

Success means:

- users can identify message/state types instantly
- tool activity is understandable without reading every line
- the app feels polished even when idle
- streaming and transitions feel alive but not distracting

---

## 2. Current State Summary

The TUI already has good raw ingredients:

- multiple screens: main, onboarding, debug traces, web automation
- distinct chat item types: user, ai, think, tool, tool group, diff, meta-tool, memory, error, separator
- theme system with dark/light palettes and semantic tokens
- header, input, shortcut, scrollbar, empty state, typing indicator
- shimmer and streaming cursor behavior

What is still weak:

- overall visual hierarchy is uneven
- header/footer chrome is functional but not memorable
- chat blocks do not yet feel like one coherent system
- tool and diff presentation can be clearer and more attractive
- spacing rhythm and density are not fully tuned

---

## 3. Design Principles

We should use these principles to evaluate every UI decision:

1. Clarity first
Every element must communicate purpose quickly.

2. Strong hierarchy
Primary content, secondary metadata, and transient system noise must not compete equally.

3. Consistent rhythm
Spacing, indentation, borders, and alignment should feel systematic.

4. One visual language
Header, chat, tools, diffs, footer, and special states should look like one product.

5. Motion with restraint
Streaming, shimmer, and indicators should support comprehension, not decoration.

6. Terminal-native beauty
The interface should embrace text, contrast, spacing, and line-work rather than mimic web cards badly.

---

## 4. Redesign Scope

The redesign is split into 7 workstreams:

1. Visual foundation
2. App chrome
3. Chat hierarchy
4. Tool and diff experience
5. Streaming and motion
6. Secondary screens and special states
7. Final polish and QA

Each workstream can be discussed and implemented independently.

---

## 5. Workstream 1: Visual Foundation

### Status

Completed for the dark theme. The spacing rhythm and surface hierarchy are implemented; light theme parity is now partially aligned but still needs manual review.

### Goal

Make the whole app feel more deliberate before changing individual components.

### Changes

- refine the dark palette so surfaces, borders, accents, and semantic colors are more balanced
- define a stronger spacing system for vertical rhythm and indentation
- standardize border intensity and when rounded vs straight borders are used
- align typography emphasis rules:
  - primary text
  - muted metadata
  - semantic status text
  - active/streaming emphasis

### Files likely involved

- `crates/locus_tui/src/theme/palette.rs`
- `crates/locus_tui/src/layouts/style.rs`
- `crates/locus_tui/src/utils/constants.rs`
- `crates/locus_tui/src/view.rs`

### Acceptance criteria

- the main view looks more structured even before message redesign
- borders and backgrounds create depth without visual clutter
- muted text is readable but clearly secondary
- spacing feels consistent across screens

### Discussion questions

- Should the TUI stay near-black and minimal, or become slightly richer and more contrasty?
- Should the design lean more editorial, more developer-console, or more “premium terminal”?

---

## 6. Workstream 2: App Chrome

### Status

Completed. Header, input, shortcut line, scrollbar, and growing multiline input are implemented in the main TUI.

### Goal

Upgrade the fixed frame of the app: header, input bar, shortcut line, and scrollbar.

### Header

Proposed changes:

- stronger two-line header presence
- clearer title treatment
- better status badge/dot design
- room for model/provider/session metadata
- more intentional separator/border line

Desired result:

- looks important without stealing attention from the conversation

### Input Bar

Proposed changes:

- cleaner focus treatment
- stronger prompt icon behavior
- better placeholder styling
- more refined cursor and content alignment
- better relationship between input block and shortcut line

Desired result:

- feels like the primary interaction control, not an afterthought

### Shortcut Bar

Proposed changes:

- better visual hierarchy between keys and labels
- more elegant separators
- tighter context-aware hints
- reduced visual noise

### Scrollbar

Proposed changes:

- make the scrollbar more integrated into the chat body
- improve thumb visibility and active state
- tune sizing/positioning so it feels polished

### Files likely involved

- `crates/locus_tui/src/layouts/head.rs`
- `crates/locus_tui/src/layouts/input.rs`
- `crates/locus_tui/src/layouts/shortcut.rs`
- `crates/locus_tui/src/view.rs`
- `crates/locus_tui/src/theme/palette.rs`

### Acceptance criteria

- header, footer, and scrollbar feel like one cohesive chrome system
- status, input, and shortcuts are easier to parse
- the app looks polished even with an empty chat

---

## 7. Workstream 3: Chat Hierarchy

### Status

Completed. User, assistant, thinking, memory, error, and separator treatments now follow one transcript language.

### Goal

Make every message type instantly recognizable and give the transcript a stronger reading rhythm.

### User Messages

Proposed direction:

- stronger visual ownership
- subtle accent treatment
- clearly distinct from assistant output

### Assistant Messages

Proposed direction:

- optimize for readability first
- support markdown and code blocks cleanly
- reduce flat wall-of-text feeling

### Thinking Messages

Proposed direction:

- clearly secondary
- collapsible by default when long
- useful for transparency, not noisy by default

### Memory Messages

Proposed direction:

- visually system-like but not alarming
- communicate “retrieved/stored context” cleanly

### Error Messages

Proposed direction:

- high contrast and unmistakable
- readable enough to debug without overwhelming the transcript

### Separators

Proposed direction:

- stronger session/phase dividers
- better transitions between activity groups

### Files likely involved

- `crates/locus_tui/src/messages/user.rs`
- `crates/locus_tui/src/messages/ai_message.rs`
- `crates/locus_tui/src/messages/ai_think_message.rs`
- `crates/locus_tui/src/messages/error.rs`
- `crates/locus_tui/src/messages/memory.rs`
- `crates/locus_tui/src/messages/markdown.rs`
- `crates/locus_tui/src/view.rs`
- `crates/locus_tui/src/state.rs`

### Acceptance criteria

- users can identify message type at a glance
- long transcripts are easier to scan
- markdown/code rendering feels first-class, not bolted on

---

## 8. Workstream 4: Tool And Diff Experience

### Status

Completed. Tool groups, meta-tools, and edit diffs have dedicated presentation and improved scanability.

### Goal

Make tool execution one of the strongest visual features of the product.

### Tool Calls

Proposed changes:

- cleaner running/done/error states
- stronger group treatment for consecutive tool calls
- better duration and summary presentation
- clearer distinction between normal tools and meta-tools

### Tool Groups

Proposed changes:

- group header that feels intentional
- more compact repeated tool activity
- better scannability in heavy execution turns

### Edit Diffs

Proposed changes:

- stronger dedicated diff block styling
- clearer added/removed/context line treatment
- better pagination indicators
- more confidence that the diff block belongs to the tool that produced it

### Files likely involved

- `crates/locus_tui/src/messages/tool.rs`
- `crates/locus_tui/src/messages/meta_tool.rs`
- `crates/locus_tui/src/messages/edit_diff.rs`
- `crates/locus_tui/src/view.rs`
- `crates/locus_tui/src/state.rs`

### Acceptance criteria

- tool-heavy turns are easy to follow
- diffs look important and readable
- erroring tools stand out immediately

---

## 9. Workstream 5: Streaming And Motion

### Status

Mostly completed. Active-state polish, staged status messaging, and previewable streaming states are implemented; final manual tuning is still part of QA.

### Goal

Make active states feel alive and informative without becoming noisy.

### Proposed changes

- tune shimmer so it is more purposeful
- refine streaming cursor behavior
- improve typing indicator behavior
- make thinking/tool/assistant streaming feel coordinated
- add subtle temporal cues instead of constant visual noise

### Constraints

- motion must remain terminal-safe
- redraw cost must stay low
- no animation should reduce readability

### Files likely involved

- `crates/locus_tui/src/animation/shimmer.rs`
- `crates/locus_tui/src/view.rs`
- `crates/locus_tui/src/run.rs`
- `crates/locus_tui/src/state.rs`

### Acceptance criteria

- active states feel premium
- motion helps explain what is happening
- the interface still feels calm when idle

---

## 10. Workstream 6: Secondary Screens And Special States

### Status

Mostly completed. Onboarding, debug traces, web automation, empty state, and preview mode now share the main visual system.

### Goal

Bring non-main-screen experiences up to the same design quality.

### Areas

- onboarding screen
- debug traces screen
- web automation screen
- empty state
- session separators
- copied/saved/status feedback states

### Proposed changes

- align these screens with the same spacing, border, and hierarchy language
- make onboarding look intentional rather than temporary
- make debug traces utilitarian but still branded
- improve empty state so the product feels complete on first launch

### Files likely involved

- `crates/locus_tui/src/view.rs`
- `crates/locus_tui/src/web_automation/view.rs`
- `crates/locus_tui/src/runtime_events.rs`
- `crates/locus_tui/src/state.rs`

### Acceptance criteria

- switching screens does not feel like entering a different app
- empty and onboarding states feel designed, not placeholder

---

## 11. Workstream 7: Final Polish And QA

### Status

In progress. Core redesign work is shipped, but manual review across terminal sizes, light-theme verification, and style cleanup are still open.

### Goal

Stabilize the redesign and catch regressions.

### Tasks

- tune spacing after the full redesign is visible
- test narrow and wide terminal sizes
- verify scroll behavior across long transcripts
- verify UTF-8 cursor alignment
- verify diff pagination still works
- validate dark/light parity where supported
- clean up style duplication and dead presentation code

### Acceptance criteria

- no major visual regressions across common terminal sizes
- interaction still feels fast
- rendering logic remains maintainable

---

## 12. Recommended Discussion Order

We should review the plan in this order:

1. Visual foundation
2. Header + footer chrome
3. Chat message hierarchy
4. Tool and diff presentation
5. Streaming and motion
6. Secondary screens
7. QA and rollout

This order matters because the later work depends on the earlier visual language.

---

## 13. Recommended Implementation Order

After discussion, implementation should likely happen in this order:

1. theme + spacing tokens
2. header/input/shortcut/scrollbar
3. user + ai + think blocks
4. tool/meta-tool/diff blocks
5. streaming polish
6. empty/onboarding/debug/web automation alignment
7. cleanup + tests

---

## 14. Risks

Main risks:

- over-designing terminal UI and hurting scanability
- too many styles competing in one transcript
- animation/redraw changes causing performance regressions
- making chat visually richer but structurally less clear

Mitigation:

- ship in phases
- review each workstream visually before moving on
- keep one dominant visual hierarchy system throughout

---

## 15. Decision Log Template

Use this section as we discuss each workstream.

### Decision: Visual Direction
- Status: decided
- Notes: Premium terminal workspace. Quiet, precise, transcript-first, and intentionally restrained.

### Decision: Header/Footer Style
- Status: decided
- Notes: Two-line header, elevated input surface, structured shortcut line, and integrated scrollbar.

### Decision: Chat Block Language
- Status: decided
- Notes: Rail-based transcript hierarchy with distinct treatments for user, assistant, think, memory, error, and separators.

### Decision: Tool/Diff Treatment
- Status: decided
- Notes: Grouped tool execution log with explicit state labels and dedicated diff cards attached to tool output.

### Decision: Motion Level
- Status: decided
- Notes: Low-motion by default. Motion is limited to streaming, shimmer, and status progression cues.

---

## 16. Next Step

Start with **Workstream 1: Visual Foundation**.

That is the right first discussion because it defines the palette, contrast, spacing, and overall visual language that every later component should inherit.
