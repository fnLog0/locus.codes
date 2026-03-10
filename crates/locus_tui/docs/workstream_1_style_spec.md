# Workstream 1 Style Spec

This is the concrete proposal for the Locus TUI visual foundation.
It is intentionally specific so we can approve or change it before touching implementation.

Status: proposed

---

## 1. Core Direction

The TUI should feel like a **premium terminal workspace**.

Not this:

- retro hacker neon
- web-app cards forced into a terminal
- chat app bubbles
- over-decorated dashboard chrome

Yes to this:

- quiet, dark, precise
- transcript-first
- strong hierarchy with restrained color
- minimal chrome with deliberate spacing
- activity that feels alive without becoming noisy

Three target adjectives:

- quiet
- sharp
- trustworthy

---

## 2. Overall Feel

### At rest

The app should feel calm and professional.
The user should see a stable workspace, not a busy interface.

### During activity

The app should feel responsive and intelligent.
Streaming, tool execution, and memory events should be visible immediately, but still feel controlled.

### In heavy turns

The interface should compress noise and preserve signal.
Tool groups, diffs, and errors should remain readable without turning the transcript into visual chaos.

---

## 3. Palette Character

### Palette philosophy

Use a **near-black base with cool contrast**.
Color should communicate meaning, not decorate empty space.

### Visual model

- background: deep charcoal black
- surfaces: graphite layers, only slightly lifted
- accent: cool electric blue
- success: mint-green
- warning: warm amber
- danger: coral-red
- info: cold cyan

### Behavioral rules

- one dominant accent color only
- semantic colors appear only when meaning changes
- borders should separate, not shout
- muted text must still be readable
- bright colors should never dominate the screen in idle state

---

## 4. Target Color Behavior

These are behavior targets, not a final locked palette file yet.

### Base layers

- `background`
  - nearly black
  - should visually recede behind all content
- `surface_background`
  - visible but subtle lift from background
  - used for stable structural regions
- `elevated_surface_background`
  - one step brighter than surface
  - reserved for emphasis, grouped UI, or code/diff framing

### Borders

- `border`
  - subtle separator only
- `border_variant`
  - quieter than main border
  - used for secondary framing
- `border_focused`
  - only for active input or clearly active region
- `border_selected`
  - used sparingly for selected state, not general emphasis

### Text

- `text`
  - soft bright, not stark white
- `text_muted`
  - clearly secondary, still readable in long sessions
- `text_placeholder`
  - dimmer than muted, but not invisible
- `text_disabled`
  - rare use only
- `text_accent`
  - reserved for inline high-value metadata or emphasis

### Semantic colors

- `accent`
  - focus, active state, primary app identity
- `success`
  - completion, healthy status, successful tools
- `warning`
  - active/streaming/waiting states
- `danger`
  - errors, blocked states, failed tools
- `info`
  - memory/system/helpful secondary feedback

### Chrome

- scrollbar should use low-contrast track + controlled thumb
- focused panel accents should not be brighter than the input border
- editor/diff surfaces should feel slightly denser than normal transcript blocks

---

## 5. Recommended Palette Mood

This is the recommended mood to implement unless we decide otherwise.

- background: very dark, nearly black
- surfaces: cool graphite
- accent blue: slightly saturated, not neon
- green: clean and fresh, not terminal-lime
- red: warm coral, not pure alarm red
- amber: soft but readable
- cyan: sharp and minimal

Net effect:

- the app feels modern and premium
- the transcript stays dominant
- color remains meaningful under heavy activity

---

## 6. Spacing Rhythm

### Philosophy

Spacing should create hierarchy before borders do.
The UI should feel structured because of rhythm, not because everything is boxed.

### Base rhythm

Use a simple scale:

- `1` unit: tight internal spacing
- `2` units: default inset / content padding
- `4` units: nested content or subordinate details

### Transcript spacing rules

- between major chat blocks: `1` blank line
- between grouped tools: `0` blank lines
- between group header and grouped items: compact, no extra spacer
- before major state transitions: `1` blank line
- diff block internal rows: dense

### Horizontal rhythm

- normal content inset: `2` chars
- nested metadata inset: `4` chars
- diff and error detail inset: `4` or `6` chars depending on density
- avoid deep indentation unless it carries meaning

### Vertical regions

- header: compact but breathing
- body: transcript-first, largest usable area
- footer: anchored and deliberate, not cramped

---

## 7. Typography And Emphasis Rules

Terminal UI has no real type scale, so emphasis must come from:

- weight
- color
- spacing
- line treatment

### Hierarchy

- primary content
  - normal text color
  - largest visual weight through contrast
- secondary metadata
  - muted text
  - never competes with message body
- semantic state text
  - semantic color only on the most important words/symbols
- structural labels
  - subtle, consistent, low-noise

### Bold usage

Use bold sparingly for:

- app title
- key block titles
- selected/high-value labels

Do not use bold broadly in transcript content.

---

## 8. Component Mood Spec

This does not define final component layout yet. It defines the mood each component should project.

### Header

Mood:

- stable
- premium
- slightly authoritative

Should feel like:

- the control rail of the workspace

Should not feel like:

- a giant banner
- a decorative toolbar

### Chat Body

Mood:

- clean
- readable
- transcript-first

Should feel like:

- a serious work log

Should not feel like:

- a messaging app timeline

### Input Bar

Mood:

- active
- precise
- clearly interactive

Should feel like:

- the command center of the interface

### Shortcut Line

Mood:

- quiet
- helpful
- secondary

Should never visually compete with input or transcript.

### Tool Blocks

Mood:

- operational
- compact
- trustworthy

Should feel like:

- real machine activity

### Diff Blocks

Mood:

- technical
- high-value
- grounded

Should feel more substantial than a normal message, but not louder than an error.

### Error Blocks

Mood:

- unmistakable
- serious
- readable

Should draw attention quickly, then get out of the way.

---

## 9. Motion Level

Recommended motion level: **restrained premium**

### Allowed

- subtle shimmer on active tool names
- blinking streaming cursor
- lightweight typing indicator
- status color shifts for active/error/success states

### Avoid

- continuous aggressive shimmer everywhere
- too many independently animated regions
- flashy transitions that obscure text

Rule:

Motion should confirm activity, not become activity.

---

## 10. Design Do / Do Not

### Do

- keep the screen dark and calm
- let spacing and contrast do most of the work
- use semantic color intentionally
- make transcript scanning easy
- compress repeated tool activity

### Do not

- overuse boxes
- overuse color
- make every message type equally loud
- make the header or footer visually heavier than the transcript
- turn the UI into a dashboard

---

## 11. Proposed Default Decisions

Unless we explicitly change them, this is the proposal:

- visual direction: premium terminal workspace
- palette character: near-black, cool, restrained, semantic
- spacing rhythm: compact but breathable, transcript-first
- motion level: restrained premium
- transcript style: clean work log, not chat bubble UI

---

## 12. Questions To Resolve

These are the remaining style questions before implementation:

1. Do we want the dark theme to stay nearly monochrome, or allow slightly richer surfaces?
2. Should user messages be only subtly distinguished, or visibly owned with stronger accent treatment?
3. Should the header feel more minimal, or more “instrument panel”?
4. Should diffs feel embedded in the transcript, or like dedicated technical panels?

---

## 13. Next Step

If this spec looks right, the next design discussion should be:

**App chrome**

That means:

- header feel
- input bar feel
- shortcut line treatment
- scrollbar treatment
