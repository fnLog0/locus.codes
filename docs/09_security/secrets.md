# Secrets Management

Secrets are never exposed to the LLM or stored in LocusGraph.

## Rules

1. **Never in prompts**: environment variables filtered before LLM context injection
2. **Never in events**: Event Extractor redacts secrets before writing to LocusGraph
3. **Never in logs**: ToolBus output sanitized before display and storage
4. **Never in diffs**: patches scanned for accidental secret inclusion

## Detection Patterns

The secret detector scans for:
- API keys (common prefixes: `sk-`, `pk-`, `AKIA`, etc.)
- Tokens (`token`, `bearer`, `jwt`)
- Passwords (assignment patterns: `password = "..."`)
- Connection strings (`redis://`, `mongodb://`)
- Private keys (`-----BEGIN`)
- Base64-encoded secrets (high-entropy strings)

## Redaction

When a secret is detected:
- Replace with `[REDACTED]` in output
- Log a warning (without the secret value)
- Block the action if in a patch (prevent committing secrets)

## Storage

- Secrets used by locus.codes itself (model API keys, DB credentials) stored in environment variables or OS keychain
- Never stored in config files within the repo
- Never stored in LocusGraph
