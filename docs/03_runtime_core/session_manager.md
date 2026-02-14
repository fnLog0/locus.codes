# Session Manager

Owns the working context for the current locus.codes session.

## State

| Field | Description |
|-------|-------------|
| **repo_root** | Absolute path to repository root |
| **branch** | Current git branch |
| **working_dir** | Current working directory |
| **git_state** | Clean / dirty / merge / rebase |
| **mode** | Current mode (Rush / Smart / Deep) |
| **thread** | Current thread (interaction sequence) |
| **config** | Session-level settings (permissions, limits) |

## Initialization

On startup:
1. Detect repo root (walk up to `.git`)
2. Read current branch and git state
3. Load session config (if exists)
4. Warm memory cache (load high-confidence locuses for this repo)

## Threads

- Each interaction sequence is a **thread**
- Threads can be saved, resumed, and shared
- Thread state: prompt history, task results, events generated
- Stored locally (not in LocusGraph)

## Repo Metadata

Session Manager provides repo metadata to all agents:
- Language/framework detection
- Test framework detection
- Project structure summary
- `.gitignore` patterns
