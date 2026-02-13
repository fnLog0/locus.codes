# locus.codes

**locus.codes** is a frontier coding agent (terminal + editor) with [LocusGraph](https://locusgraph.com) as implicit memory. No AGENTS.md, no Skills — the agent learns from every interaction.

This repo is **product and code only** (no deployment/infra). We work here on the locus.codes app and landing.

## Repo structure

| Path | Description |
|------|-------------|
| **`landing/`** | Landing page (React, TypeScript, Vite, Oat, Geist Pixel, Buttondown) |
| **`.cursor/`** | Cursor rules (LocusGraph updates, locus.codes scope) |

## Quick start

```bash
cd landing
npm install
npm run dev
```

Open [http://localhost:5173](http://localhost:5173). See **[landing/README.md](./landing/README.md)** for full docs.

## Scope

- **In scope:** locus.codes product, architecture, landing, and code.
- **Out of scope:** Deployment, LocusGraph backend, EKS, infra.
