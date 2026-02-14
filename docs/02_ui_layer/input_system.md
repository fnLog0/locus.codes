# Input System

## Global Prompt Bar

The prompt bar is always visible at the bottom of the screen, across all views. It is the primary input mechanism.

- Supports multi-line input (Shift+Enter for newline)
- Mode indicator shown: `[Rush]` / `[Smart]` / `[Deep]`
- Submit with Enter
- Input is sent to the Orchestrator

## Input Flow

```
User types prompt → Prompt Bar captures → Orchestrator receives → DAG built
```

## Command Palette

Special commands prefixed with `:` for quick actions:

| Command | Action |
|---------|--------|
| `:mode rush` | Switch to Rush mode |
| `:mode smart` | Switch to Smart mode |
| `:mode deep` | Switch to Deep mode |
| `:view <name>` | Switch to view |
| `:quit` / `:q` | Exit |
| `:cancel` | Cancel current task |
| `:thread save` | Save current thread |
| `:thread list` | List saved threads |

## Input States

- **Ready** — waiting for user input (cursor blinking)
- **Running** — task in progress, prompt shows status, input still available for queue
- **Confirmation** — waiting for approve/reject (Diff Review)
