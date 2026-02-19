# locus-llms

A **provider-agnostic AI completions SDK** for locus.codes. It provides a unified interface for multiple LLM providers with streaming support. Every AI call goes through the `ProviderRegistry` to ensure consistent request/response types across providers.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                   ProviderRegistry                       │
│  ┌──────────────────────────────────────────────────┐   │
│  │  HashMap<String, Arc<dyn Provider>>               │   │
│  └──────────────────────────────────────────────────┘   │
│                          │                               │
│         ┌────────────────┼────────────────┐             │
│         ▼                ▼                ▼             │
│   ┌───────────┐    ┌──────────┐    ┌──────────┐        │
│   │ Anthropic  │    │   Z.AI   │    │  Custom  │        │
│   │ Provider   │    │ Provider │    │ Provider │        │
│   └───────────┘    └──────────┘    └──────────┘        │
└──────────────────────────────────────────────────────────┘
```

## Core Concepts

### Provider Trait

All providers implement the `Provider` trait:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn build_headers(&self, custom_headers: Option<&Headers>) -> Headers;
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse>;
    async fn stream(&self, request: GenerateRequest) -> Result<GenerateStream>;
    async fn list_models(&self) -> Result<Vec<String>> { Ok(vec![]) }
}
```

### ProviderRegistry

The `ProviderRegistry` manages provider registration and lookup:

```rust
use locus_llms::{ProviderRegistry, AnthropicProvider, ZaiProvider};
use locus_llms::providers::anthropic::AnthropicConfig;
use locus_llms::providers::zai::ZaiConfig;

let registry = ProviderRegistry::new()
    .register("anthropic", AnthropicProvider::new(AnthropicConfig::new("sk-ant-...")).unwrap())
    .register("zai", ZaiProvider::new(ZaiConfig::new("your-key")).unwrap());

// Look up a provider
let provider = registry.get_provider("anthropic")?;

// List registered providers
let ids = registry.list_providers();
```

### Unified Types

Standardized request/response types across all providers:

```rust
// Request
let request = GenerateRequest::new(
    "claude-sonnet-4-20250514",
    vec![Message::new(Role::User, "Hello!")],
);

// Non-streaming
let response = provider.generate(request.clone()).await?;
println!("{}", response.text());

// Streaming
let mut stream = provider.stream(request).await?;
while let Some(event) = stream.next().await {
    match event? {
        StreamEvent::TextDelta { delta, .. } => print!("{}", delta),
        StreamEvent::Finish { usage, .. } => println!("\nTokens: {}", usage.total_tokens),
        _ => {}
    }
}
```

## Directory Structure

```
src/
├── lib.rs              # Public API, re-exports
├── error.rs            # Error enum, Result alias
├── provider/
│   ├── mod.rs          # Provider trait + ProviderRegistry
│   └── trait_def.rs    # Provider trait definition
├── providers/
│   ├── mod.rs          # Re-exports all providers
│   ├── anthropic/      # Anthropic (Claude) implementation
│   │   ├── mod.rs      # Module exports
│   │   ├── provider.rs # AnthropicProvider + Provider impl
│   │   ├── convert.rs  # Unified ↔ Anthropic type conversion
│   │   ├── stream.rs   # SSE streaming (Anthropic format)
│   │   └── types.rs    # AnthropicConfig, request/response types
│   └── zai/            # Z.AI (GLM) implementation
│       ├── mod.rs      # Module exports
│       ├── provider.rs # ZaiProvider + Provider impl
│       ├── convert.rs  # Unified ↔ Z.AI type conversion
│       ├── stream.rs   # SSE streaming (OpenAI-compatible format)
│       └── types.rs    # ZaiConfig, request/response types
├── types/
│   ├── mod.rs          # Re-exports all types
│   ├── message.rs      # Message, Role, ContentPart
│   ├── request.rs      # GenerateRequest, ProviderOptions
│   ├── response.rs     # GenerateResponse, Usage, FinishReason
│   ├── stream.rs       # GenerateStream, StreamEvent
│   ├── options.rs      # GenerateOptions, Tool, ToolChoice
│   ├── headers.rs      # Headers helper
│   ├── cache.rs        # CacheControl, PromptCacheRetention
│   └── cache_validator.rs  # Cache breakpoint validation
└── tests/
    ├── mod.rs
    └── provider_registry.rs  # Registry unit tests
```

---

## Guidelines for Adding New Providers

### Step 1: Research the Provider API

**CRITICAL**: Before implementing, read the provider's API documentation thoroughly. Understand:
- Authentication method (Bearer token, API key header, etc.)
- Request/response JSON schema
- Streaming format (SSE event structure, `[DONE]` signal)
- Tool calling format
- Rate limits and error codes

### Step 2: Create Provider Directory

```
src/providers/
└── your_provider/
    ├── mod.rs       # Module exports
    ├── provider.rs  # Provider struct + Provider trait impl
    ├── convert.rs   # Unified ↔ provider type conversion
    ├── stream.rs    # SSE streaming implementation
    └── types.rs     # Config, request/response types
```

### Step 3: Implementation Pattern

#### types.rs
```rust
use serde::{Deserialize, Serialize};

/// Configuration for YourProvider
#[derive(Debug, Clone)]
pub struct YourConfig {
    pub api_key: String,
    pub base_url: String,
}

impl YourConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.example.com/v1/".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

/// Provider-specific request type
#[derive(Debug, Serialize)]
pub struct YourRequest {
    pub model: String,
    pub messages: Vec<YourMessage>,
    // ...
}

/// Provider-specific response type
#[derive(Debug, Deserialize)]
pub struct YourResponse {
    pub id: String,
    pub choices: Vec<YourChoice>,
    pub usage: YourUsage,
}
```

#### convert.rs
```rust
use crate::error::Result;
use crate::types::{GenerateRequest, GenerateResponse, FinishReason, FinishReasonKind};

/// Convert unified request to provider-specific request
pub fn to_your_request(req: &GenerateRequest, stream: bool) -> Result<YourRequest> {
    // Map messages, tools, options to provider format
}

/// Convert provider-specific response to unified response
pub fn from_your_response(resp: YourResponse) -> Result<GenerateResponse> {
    // Map content, usage, finish_reason to unified types
}

/// Parse provider-specific finish reason to unified
pub fn parse_finish_reason(reason: &Option<String>) -> FinishReason {
    match reason.as_deref() {
        Some("stop") => FinishReason::with_raw(FinishReasonKind::Stop, "stop"),
        Some("length") => FinishReason::with_raw(FinishReasonKind::Length, "length"),
        Some("tool_calls") => FinishReason::with_raw(FinishReasonKind::ToolCalls, "tool_calls"),
        Some(raw) => FinishReason::with_raw(FinishReasonKind::Other, raw),
        None => FinishReason::other(),
    }
}
```

#### stream.rs
```rust
use crate::error::{Error, Result};
use crate::types::{GenerateStream, StreamEvent};
use reqwest_eventsource::{Event, EventSource};
use futures::stream::StreamExt;

pub async fn create_stream(mut event_source: EventSource) -> Result<GenerateStream> {
    let stream = async_stream::stream! {
        while let Some(event) = event_source.next().await {
            match event {
                Ok(Event::Open) => continue,
                Ok(Event::Message(message)) => {
                    if message.data == "[DONE]" { break; }
                    // Parse and yield StreamEvent variants
                }
                Err(reqwest_eventsource::Error::StreamEnded) => break,
                Err(e) => {
                    yield Err(Error::stream_error(format!("Stream error: {}", e)));
                    break;
                }
            }
        }
        event_source.close();
    };
    Ok(GenerateStream::new(Box::pin(stream)))
}
```

#### provider.rs
```rust
use crate::provider::Provider;
use crate::error::{Error, Result};
use crate::types::{GenerateRequest, GenerateResponse, GenerateStream, Headers};
use async_trait::async_trait;
use reqwest::Client;

pub struct YourProvider {
    config: YourConfig,
    client: Client,
}

impl YourProvider {
    pub const API_KEY_ENV: &'static str = "YOUR_API_KEY";

    pub fn new(config: YourConfig) -> Result<Self> {
        if config.api_key.is_empty() {
            return Err(Error::MissingApiKey("your_provider".to_string()));
        }
        Ok(Self { config, client: Client::new() })
    }

    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var(Self::API_KEY_ENV)
            .map_err(|_| Error::MissingApiKey("your_provider".to_string()))?;
        Self::new(YourConfig::new(api_key))
    }
}

#[async_trait]
impl Provider for YourProvider {
    fn provider_id(&self) -> &str { "your_provider" }
    fn build_headers(&self, custom_headers: Option<&Headers>) -> Headers { /* ... */ }
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse> { /* ... */ }
    async fn stream(&self, request: GenerateRequest) -> Result<GenerateStream> { /* ... */ }
}
```

### Step 4: Register in Module Hierarchy

Update `src/providers/mod.rs`:

```rust
pub mod anthropic;
pub mod zai;
pub mod your_provider;

pub use anthropic::AnthropicProvider;
pub use zai::ZaiProvider;
pub use your_provider::YourProvider;
```

Update `src/lib.rs`:

```rust
pub use providers::YourProvider;
```

### Step 5: Code Quality

```bash
cargo check -p locus-llms
cargo clippy -p locus-llms
cargo fmt -- --check
```

All code must be formatted with `cargo fmt`.

### Modular Design Principles

1. **Single Responsibility**: Each file handles one concern
   - `types.rs` → Config + provider-specific request/response types
   - `convert.rs` → Unified ↔ provider type mapping only
   - `stream.rs` → SSE parsing and StreamEvent emission only
   - `provider.rs` → HTTP calls + Provider trait impl

2. **Builder Pattern**: Use for configuration
   ```rust
   let config = ZaiConfig::new("api-key")
       .with_base_url("https://custom.api.com/v1/");
   ```

3. **Error Propagation**: Use `?` operator, never panic in library code
   ```rust
   let response = self.client.post(&url).send().await?;
   if !response.status().is_success() {
       return Err(Error::provider_error(format!("API error {}", status)));
   }
   ```

4. **Async Consistency**: All Provider methods are async, use `reqwest` + `tokio`

5. **Unified Type Mapping**: Always convert to/from the unified types in `types/`
   - Never expose provider-specific types in the `Provider` trait
   - Map finish reasons, usage stats, and content to unified enums

---

## Testing Guidelines

### Test File Location

Tests live inline in each module:

```
src/providers/anthropic/convert.rs   # #[cfg(test)] mod tests { ... }
src/providers/anthropic/stream.rs    # #[cfg(test)] mod tests { ... }
src/providers/zai/convert.rs         # #[cfg(test)] mod tests { ... }
src/providers/zai/stream.rs          # #[cfg(test)] mod tests { ... }
src/tests/
├── mod.rs
└── provider_registry.rs             # Registry unit tests
```

### Test Coverage Requirements

Every provider must have tests covering:

1. **Type Conversion**: Unified → provider and provider → unified
2. **Finish Reason Parsing**: All finish reason variants
3. **Stream Processing**: Text deltas, reasoning deltas, tool call flow, finish events, errors
4. **Tool Call Streaming**: Start → delta → end with accumulated arguments
5. **Edge Cases**: Empty content, missing fields, multiple tool calls

### Running Tests

```bash
# All tests
cargo test -p locus-llms

# Specific provider tests
cargo test -p locus-llms providers::zai

# With output
cargo test -p locus-llms -- --nocapture

# Doctests only
cargo test -p locus-llms --doc
```

---

## Quick Reference

### Adding a New Provider Checklist

- [ ] Read provider API documentation thoroughly
- [ ] Create `src/providers/your_provider/` directory
- [ ] Create `types.rs` with Config, request/response structs
- [ ] Create `convert.rs` with unified ↔ provider conversion
- [ ] Create `stream.rs` with SSE parsing
- [ ] Create `provider.rs` with Provider trait implementation
- [ ] Create `mod.rs` with module exports
- [ ] Export from `src/providers/mod.rs`
- [ ] Re-export from `src/lib.rs`
- [ ] Add inline tests in `convert.rs` and `stream.rs`
- [ ] Run `cargo test -p locus-llms`
- [ ] Run `cargo clippy -p locus-llms`
- [ ] Run `cargo fmt`

### Registered Providers

| Provider | Provider ID | Config Env Var | API Base URL |
|----------|-------------|----------------|--------------|
| Anthropic | `anthropic` | `ANTHROPIC_API_KEY` | `https://api.anthropic.com/v1/` |
| Z.AI | `zai` | `ZAI_API_KEY` | `https://api.z.ai/api/paas/v4/` |

---

## API Reference

### ProviderRegistry

| Method | Description |
|--------|-------------|
| `new()` | Create empty registry |
| `register(id, provider)` | Register a provider (chainable) |
| `get_provider(id)` | Look up provider by ID |
| `list_providers()` | List all registered provider IDs |

### Provider Trait

| Method | Return Type | Description |
|--------|-------------|-------------|
| `provider_id()` | `&str` | Provider identifier |
| `build_headers(custom)` | `Headers` | Build auth + custom headers |
| `generate(request)` | `Result<GenerateResponse>` | Non-streaming generation |
| `stream(request)` | `Result<GenerateStream>` | Streaming generation |
| `list_models()` | `Result<Vec<String>>` | List available models (optional) |

### Key Types

| Type | Description |
|------|-------------|
| `GenerateRequest` | Unified request (model, messages, options, tools) |
| `GenerateResponse` | Unified response (content, usage, finish_reason) |
| `GenerateStream` | Async stream of `StreamEvent` |
| `StreamEvent` | `TextDelta`, `ReasoningDelta`, `ToolCallStart/Delta/End`, `Finish`, `Error` |
| `Message` | Conversation message (role, content parts) |
| `Usage` | Token usage (prompt, completion, total, cache details) |
| `FinishReason` | Why generation stopped (Stop, Length, ToolCalls, etc.) |
| `Error` | SDK error enum (HttpError, ProviderError, StreamError, etc.) |
