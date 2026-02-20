# Z.AI API Error 1210 Debug Notes

## Problem
CLI runtime fails with Z.AI API error 400 (code 1210: "Invalid API parameter") when using tools.

## API Documentation Reference
- Tool Message Format: https://docs.z.ai/api-reference/llm/chat-completion#tool-message

## Test Results

| Test | Tools | Stream | Result |
|------|-------|--------|--------|
| Simple "Say hello" | No | No | ✅ 200 OK |
| With format: "uri" | 1 | No | ✅ 200 OK |
| Multiple tools | 5 | Yes | ✅ 200 OK |
| 10 tools with enums | 10 | Yes | ✅ 200 OK |
| CLI "Say hello" | 10 | Yes | ❌ Intermittent |
| CLI "read readme.md" | 10 | Yes | ❌ Error 1210 |

## Key Findings
1. Test program with 10 tools + streaming + complex schemas WORKS
2. CLI with similar configuration FAILS
3. CLI request size: ~19KB

## Hypotheses to Investigate
1. **Request size limit** - CLI sends ~19KB, test program sends smaller
2. **System message length** - CLI has long system prompt
3. **Specific tool schema fields** - `additionalProperties`, `default` values
4. **Rate limiting** - transient API issues

## Commands to Reproduce

```bash
# Test program (WORKS)
cd /tmp/zai_test && ZAI_API_KEY=b142622ee7d64bbb9ecda1f4fa9becd9.XLxwdAoPvbWWTXwK cargo run

# CLI simple (INTERMITTENT)
cd /Users/nasimakhtar/Projects/fnlog0/locuscodes && ZAI_API_KEY=b142622ee7d64bbb9ecda1f4fa9becd9.XLxwdAoPvbWWTXwK cargo run -p locus-cli -- run --provider zai --model glm-5 -p "Say hello"

# CLI with tools (FAILS)
cd /Users/nasimakhtar/Projects/fnlog0/locuscodes && ZAI_API_KEY=b142622ee7d64bbb9ecda1f4fa9becd9.XLxwdAoPvbWWTXwK cargo run -p locus-cli -- run --provider zai --model glm-5 -p "read readme.md and summarize it"
```

## Key Files

| File | Purpose |
|------|---------|
| `crates/locus_llms/src/providers/zai/types.rs` | ZaiMessage, ZaiRequest structs |
| `crates/locus_llms/src/providers/zai/convert.rs` | to_zai_request() builds tools |
| `crates/locus_llms/src/providers/zai/provider.rs` | Debug logging at line 105 |
| `crates/locus_llms/src/providers/zai/stream.rs` | Streaming response handling |

## Debug Logging
Line 105 in `provider.rs` has debug output:
```rust
eprintln!("[DEBUG ZAI REQUEST]\n{}", serde_json::to_string_pretty(&zai_request).unwrap_or_default());
```

Remove after fix confirmed.

## Z.AI API Constraints (from docs)

| Parameter | Constraint |
|-----------|------------|
| model | Required (glm-5, glm-4.7, etc.) |
| messages | Required, min 1 |
| max_tokens | 1 to 131,072 |
| temperature | 0.0 to 1.0 |
| top_p | 0.01 to 1.0 |
| tools | Max 128 functions |
| tool_choice | Only "auto" supported |
| stream | boolean |
| stop | Max array length 1 |

## Tool Message Format
```json
{
  "role": "tool",
  "content": "The result of the tool execution",
  "tool_call_id": "call_abc123"
}
```

## Tools Array Format
```json
"tools": [
  {
    "type": "function",
    "function": {
      "name": "string",
      "description": "string",
      "parameters": {
        "type": "object",
        "properties": { ... },
        "required": [ ... ]
      }
    }
  }
]
```

## Next Steps
1. Test with full CLI system message in isolation
2. Test with progressively larger requests to find size limit
3. Compare exact tool schemas between CLI and test program
4. Check if `additionalProperties` or `default` values cause issues

## Test Program Location
`/tmp/zai_test/` - Minimal test program to isolate API behavior
