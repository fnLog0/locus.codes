# locus.codes

**locus.codes** is a frontier coding agent (terminal + editor) with [LocusGraph](https://locusgraph.com) as implicit memory. No AGENTS.md, no Skills — the agent learns from every interaction.

This repo is **product and code only** (no deployment/infra). We work here on the locus.codes app and landing.

## Repo structure

| Path | Description |
|------|-------------|
| **`landing/`** | Landing page (React, TypeScript, Vite, Oat, Geist Pixel, Buttondown) |
| **`crates/`** | Rust workspace: locus-cli (TUI), locus-runtime, locus-toolbus, locus-llm, etc. |
| **`.cursor/`** | Cursor rules (LocusGraph updates, locus.codes scope) |
