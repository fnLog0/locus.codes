# Multi-Model Engine

Layer E — Routes tasks to different self-hosted models based on mode.

## Model Routing

| Mode | Model Profile | Characteristics |
|------|--------------|-----------------|
| **Rush** | Cheap, fast | Low latency, small context, minimal reasoning |
| **Smart** | Balanced SOTA | Standard latency, full context, good reasoning |
| **Deep** | Strongest available | Higher latency acceptable, max context, extended thinking |

## Model Selection

The Mode Controller sets the active mode. The Model Router selects the appropriate model:

```
Mode Controller → active mode → Model Router → select model → send request
```

## Properties

- **Self-hosted**: all models run on own infrastructure, no external API calls
- **Replaceable**: models can be swapped without code changes (config-driven)
- **No vendor lock-in**: model interface is abstracted
- **Privacy**: all data stays on own servers

## Model Interface

All models implement the same interface:

```
Input:  system_prompt + memory_bundle + user_prompt + tool_definitions
Output: tool_calls[] + reasoning + confidence
```

## Fallback

If the selected model is unavailable:
- Rush → fall back to any available model
- Smart → fall back to Rush model with warning
- Deep → fail with error (no compromise on quality)
