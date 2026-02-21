# LocusGraph Observability

Reusable observability utilities for distributed tracing across LocusGraph services.

## Overview

This crate provides OpenTelemetry tracing setup and utilities that can be used across different LocusGraph services (server, agents, CLI tools, etc.).

## Features

- **OpenTelemetry Integration**: OTLP export to Tempo, Jaeger, or other compatible backends
- **Environment-based Configuration**: Easy setup via environment variables
- **Programmatic API**: Full control via Rust API
- **Tracing Utilities**: Macros and functions for common tracing patterns
- **Console Logging**: Integrated with `tracing-subscriber` for local development

## Quick Start

### Basic Usage

```rust
use locusgraph_observability::{ ObservabilityConfig, init };

// Initialize with configuration
let config = ObservabilityConfig::new("my-service")
    .with_otlp_endpoint("http://localhost:4317")
    .with_log_level("info");

init(config)?;

// Use tracing as usual
tracing::info!("Service started");
```

### From Environment Variables

```rust
use locusgraph_observability::init_from_env;

// Reads configuration from environment variables
init_from_env()?;
```

## Configuration

### Programmatic Configuration

```rust
use locusgraph_observability::ObservabilityConfig;

let config = ObservabilityConfig::new("locusgraph-server")
    .with_version("0.1.0")
    .with_otlp_endpoint("http://localhost:4317")
    .with_log_level("debug")
    .with_console(true)
    .with_resource_attribute("environment", "production")
    .with_resource_attribute("region", "us-east-1");
```

### Environment Variables

| Variable | Alternative | Description | Default |
|----------|-------------|-------------|---------|
| `OTEL_SERVICE_NAME` | `SERVICE_NAME` | Service name | `"locusgraph-service"` |
| `OTEL_SERVICE_VERSION` | `SERVICE_VERSION` | Service version | `CARGO_PKG_VERSION` or `"unknown"` |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `TEMPO_ENDPOINT` | OTLP endpoint URL | `"http://localhost:4317"` |
| `OTEL_LOG_LEVEL` | `RUST_LOG` | Log level filter | `"info"` |

## Tracing Utilities

### HTTP Request Spans

```rust
use locusgraph_observability::http_request_span;

let span = http_request_span!("POST", "/v1/events", "req-123");
let _guard = span.enter();
// ... handler code ...
```

### Agent Operation Spans

```rust
use locusgraph_observability::agent_span;

let span = agent_span!("agent-123", "store_event");
let _guard = span.enter();
// ... agent operation ...
```

### Storage Operation Spans

```rust
use locusgraph_observability::storage_span;

let span = storage_span!("rocksdb", "write", "key-123");
let _guard = span.enter();
// ... storage operation ...
```

### Error Recording

```rust
use locusgraph_observability::record_error;

match operation() {
    Ok(result) => result,
    Err(e) => {
        record_error(&e);
        return Err(e);
    }
}
```

### Duration Recording

```rust
use locusgraph_observability::record_duration;
use std::time::Instant;

let start = Instant::now();
// ... operation ...
record_duration("operation.duration_ms", start.elapsed());
```

## Integration Examples

### Axum Server

```rust
use locusgraph_observability::{ init_from_env, shutdown };
use axum::Router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    init_from_env()?;
    
    // ... setup server ...
    
    // On shutdown
    shutdown();
    Ok(())
}
```

### Agent

```rust
use locusgraph_observability::{ ObservabilityConfig, init };
use tracing::instrument;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ObservabilityConfig::new("locusgraph-agent")
        .with_otlp_endpoint("http://localhost:4317");
    init(config)?;
    
    run_agent().await?;
    shutdown();
    Ok(())
}

#[instrument]
async fn run_agent() {
    tracing::info!("Agent running");
}
```

## Graceful Shutdown

Always call `shutdown()` during graceful shutdown to ensure all traces are exported:

```rust
use locusgraph_observability::shutdown;

// During shutdown
shutdown();
```

## Architecture

```
┌─────────────────┐
│  Application    │
│  (tracing!)     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ tracing-subscriber │
│  + OpenTelemetry   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  OTLP Exporter  │
│  (gRPC/Tempo)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Tempo/Jaeger  │
│   (Storage)     │
└─────────────────┘
```

## Development

### Running Tests

```bash
cd crates/observability
cargo test
```

### Example Usage

See `examples/` directory for complete usage examples (if added).

## License

Part of the LocusGraph project.

