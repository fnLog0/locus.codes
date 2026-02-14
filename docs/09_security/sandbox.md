# Sandbox

Command execution is sandboxed for safety.

## Restrictions

| Restriction | Description |
|-------------|-------------|
| **Filesystem** | Access limited to project directory only |
| **Network** | No network access by default |
| **Resources** | CPU and memory limits per command |
| **Timeout** | Configurable per command (default: 60s) |
| **Process** | No spawning background processes |
| **Environment** | Filtered env vars (secrets stripped) |

## Filesystem Isolation

- Commands can only read/write within the repo root
- Symlinks outside repo root are blocked
- `/tmp` access allowed (for test artifacts)
- Home directory access blocked

## Network Isolation

- No outbound network by default
- Can be relaxed per-session for specific use cases (e.g. `npm install`)
- Network access requires explicit permission

## Resource Limits

| Resource | Default Limit |
|----------|--------------|
| CPU time | 120s |
| Memory | 512MB |
| File size | 50MB |
| Open files | 256 |
| Processes | 32 |

## Environment

Before executing a command:
1. Copy current environment
2. Remove sensitive variables (API keys, tokens, passwords)
3. Set `HOME` to a sandbox directory
4. Set `PATH` to minimal required set
