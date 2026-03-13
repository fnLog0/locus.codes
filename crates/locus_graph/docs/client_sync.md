# Client Sync — Add Missing LocusGraphClient Methods

**Goal:** Bring `locus_graph::LocusGraphClient` in sync with `locus_proxy`. The proxy already implements all 14 gRPC RPCs with caching and queue support. The client wrapper is missing 7 methods.

**Priority:** Medium. Not needed for Phase 4 (turns). Required starting Phase 5 (`get_context`) and Phase 9 (`get_context_relationships`).

---

## Current State

| Method | Proto RPC | locus_proxy | locus_graph client | Status |
|---|---|---|---|---|
| StoreEvent | ✅ | ✅ | ✅ `store_event` | Done |
| RetrieveContext | ✅ | ✅ | ✅ `retrieve_memories` | Done |
| ListContextTypes | ✅ | ✅ | ✅ `list_context_types` | Done |
| ListContextsByType | ✅ | ✅ | ✅ `list_contexts_by_type` | Done |
| SearchContexts | ✅ | ✅ | ✅ `search_contexts` | Done |
| GenerateInsights | ✅ | ✅ | ✅ `generate_insights` | Done |
| GetContext | ✅ | ✅ | ❌ | **Add** |
| GetContextByName | ✅ | ✅ | ❌ | **Add** |
| BatchGetContext | ✅ | ✅ | ❌ | **Add** |
| GetContextRelationships | ✅ | ✅ | ❌ | **Add** |
| Resolve | ✅ | ✅ | ❌ | **Add** |
| BatchResolve | ✅ | ✅ | ❌ | **Add** |
| GetUnresolvedOverview | ✅ | ✅ | ❌ | **Add** |
| GetUnresolvedLinks | ✅ | ✅ | ❌ | **Add** |

---

## Reference Files

- **Proto definitions:** `/Users/nasimakhtar/Projects/hyperbola-network/locusgraph/apps/locus_proxy/proto/locusgraph/v1/`
- **Proxy client (Rust):** `/Users/nasimakhtar/Projects/hyperbola-network/locusgraph/apps/locus_proxy/src/client.rs`
- **Proxy lib.rs (re-exports):** `/Users/nasimakhtar/Projects/hyperbola-network/locusgraph/apps/locus_proxy/src/lib.rs`
- **TS client (reference API):** `/Users/nasimakhtar/Projects/hyperbola-network/locusgraph/apps/bindings/typescript/src/client.ts`
- **Target file:** `crates/locus_graph/src/client.rs`
- **Types file:** `crates/locus_graph/src/types.rs`
- **Lib re-exports:** `crates/locus_graph/src/lib.rs`

---

## Tasks

### Task 1: Add `get_context`

**File:** `crates/locus_graph/src/client.rs`

Wraps `proxy.get_context(GetContextRequest)`. Returns the context metadata + latest payload.

```rust
/// Get a single context by context_id.
/// Returns context metadata and latest locus payload.
pub async fn get_context(&self, context_id: &str) -> Result<ContextDetail> {
    let request = GetContextRequest {
        graph_id: self.config.graph_id.clone(),
        context_id: context_id.to_string(),
    };

    let response = self.proxy.get_context(request).await?;

    Ok(ContextDetail {
        context_id: response.context_id,
        context: response.context.map(|c| Context {
            context_id: c.context_id,
            context_type: c.context_type,
            context_name: c.context_name,
            created_at: c.created_at,
            updated_at: c.updated_at,
            reference_count: c.reference_count,
        }),
        locus_id: response.locus_id,
        payload_json: response.payload_json,
    })
}
```

**Add to `types.rs`:**

```rust
/// Detailed context with payload, returned by get_context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextDetail {
    pub context_id: String,
    pub context: Option<Context>,
    pub locus_id: String,
    pub payload_json: String,
}
```

**Import in `client.rs`:** `use locus_proxy::GetContextRequest;`

### Task 2: Add `get_context_by_name`

**File:** `crates/locus_graph/src/client.rs`

```rust
/// Get a single context by exact name (optionally scoped to a type).
pub async fn get_context_by_name(
    &self,
    context_name: &str,
    context_type: Option<&str>,
) -> Result<ContextDetail> {
    let request = GetContextByNameRequest {
        graph_id: self.config.graph_id.clone(),
        context_name: context_name.to_string(),
        context_type: context_type.map(|s| s.to_string()),
    };

    let response = self.proxy.get_context_by_name(request).await?;

    Ok(ContextDetail {
        context_id: response.context_id,
        context: response.context.map(|c| Context {
            context_id: c.context_id,
            context_type: c.context_type,
            context_name: c.context_name,
            created_at: c.created_at,
            updated_at: c.updated_at,
            reference_count: c.reference_count,
        }),
        locus_id: response.locus_id,
        payload_json: response.payload_json,
    })
}
```

**Import:** `use locus_proxy::GetContextByNameRequest;`

### Task 3: Add `batch_get_context`

**File:** `crates/locus_graph/src/client.rs`

```rust
/// Batch get multiple contexts by context_id.
/// Returns found contexts and list of not-found IDs.
pub async fn batch_get_context(
    &self,
    context_ids: Vec<String>,
) -> Result<BatchContextResult> {
    let request = BatchGetContextRequest {
        graph_id: self.config.graph_id.clone(),
        context_ids,
    };

    let response = self.proxy.batch_get_context(request).await?;

    Ok(BatchContextResult {
        contexts: response.contexts.into_iter().map(|r| ContextDetail {
            context_id: r.context_id,
            context: r.context.map(|c| Context {
                context_id: c.context_id,
                context_type: c.context_type,
                context_name: c.context_name,
                created_at: c.created_at,
                updated_at: c.updated_at,
                reference_count: c.reference_count,
            }),
            locus_id: r.locus_id,
            payload_json: r.payload_json,
        }).collect(),
        not_found: response.not_found,
    })
}
```

**Add to `types.rs`:**

```rust
/// Result from batch_get_context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchContextResult {
    pub contexts: Vec<ContextDetail>,
    pub not_found: Vec<String>,
}
```

**Import:** `use locus_proxy::BatchGetContextRequest;`

### Task 4: Add `get_context_relationships`

**File:** `crates/locus_graph/src/client.rs`

```rust
/// Get relationships for a context.
/// Filter by link_type (related_to, extends, reinforces, contradicts)
/// and direction (outgoing, incoming, both).
pub async fn get_context_relationships(
    &self,
    context_type: &str,
    context_name: &str,
    link_type: Option<&str>,
    direction: Option<&str>,
    page: Option<u64>,
    page_size: Option<u64>,
) -> Result<Vec<ContextRelationship>> {
    let request = GetContextRelationshipsRequest {
        graph_id: self.config.graph_id.clone(),
        context_type: context_type.to_string(),
        context_name: context_name.to_string(),
        link_type: link_type.map(|s| s.to_string()),
        direction: direction.map(|s| s.to_string()),
        page,
        page_size,
    };

    let response = self.proxy.get_context_relationships(request).await?;

    Ok(response.relationships.into_iter().map(|r| {
        ContextRelationship {
            context: r.context.map(|c| Context {
                context_id: c.context_id,
                context_type: c.context_type,
                context_name: c.context_name,
                created_at: c.created_at,
                updated_at: c.updated_at,
                reference_count: c.reference_count,
            }),
            link_type: r.link_type,
            direction: r.direction,
        }
    }).collect())
}
```

**Add to `types.rs`:**

```rust
/// A relationship between contexts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRelationship {
    pub context: Option<Context>,
    pub link_type: String,
    pub direction: String,
}
```

**Import:** `use locus_proxy::GetContextRelationshipsRequest;`

Note: The proxy re-exports `locus_proxy::ContextRelationship` as a gRPC struct — the new `types::ContextRelationship` is the higher-level wrapper. Rename import if there's a collision:

```rust
use locus_proxy::ContextRelationship as ProxyContextRelationship;
```

Or just access the fields directly from the response without importing the proxy type.

### Task 5: Add `resolve`

**File:** `crates/locus_graph/src/client.rs`

```rust
/// Resolve a context_id to a locus_id.
pub async fn resolve(
    &self,
    context_id: &str,
    locus_id: &str,
) -> Result<ResolveResult> {
    let request = ResolveRequest {
        graph_id: self.config.graph_id.clone(),
        context_id: context_id.to_string(),
        locus_id: locus_id.to_string(),
    };

    let response = self.proxy.resolve(request).await?;

    Ok(ResolveResult {
        context_id: response.context_id,
        locus_id: response.locus_id,
        links_resolved: response.links_resolved,
        success: response.success,
    })
}
```

**Add to `types.rs`:**

```rust
/// Result from resolve operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveResult {
    pub context_id: String,
    pub locus_id: String,
    pub links_resolved: u64,
    pub success: bool,
}
```

**Import:** The proxy re-exports `locus_proxy::ResolveRequest` — there will be a name collision with the new `ResolveResult` type but that's fine since it's a different name. However, `locus_proxy::ResolveResponse` must be used carefully. Import as:

```rust
use locus_proxy::ResolveRequest as ProxyResolveRequest;
```

### Task 6: Add `batch_resolve`

**File:** `crates/locus_graph/src/client.rs`

```rust
/// Batch resolve multiple context_id → locus_id mappings.
pub async fn batch_resolve(
    &self,
    resolutions: Vec<(String, String)>,
) -> Result<BatchResolveResult> {
    use locus_proxy::ResolveItem;

    let request = BatchResolveRequest {
        graph_id: self.config.graph_id.clone(),
        resolutions: resolutions.into_iter().map(|(ctx, loc)| ResolveItem {
            context_id: ctx,
            locus_id: loc,
        }).collect(),
    };

    let response = self.proxy.batch_resolve(request).await?;

    Ok(BatchResolveResult {
        results: response.results.into_iter().map(|r| ResolveResult {
            context_id: r.context_id,
            locus_id: r.locus_id,
            links_resolved: r.links_resolved,
            success: r.success,
        }).collect(),
        total_resolved: response.total_resolved,
    })
}
```

**Add to `types.rs`:**

```rust
/// Result from batch_resolve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResolveResult {
    pub results: Vec<ResolveResult>,
    pub total_resolved: u64,
}
```

**Import:** `use locus_proxy::{BatchResolveRequest, ResolveItem};`

### Task 7: Add `get_unresolved_overview`

**File:** `crates/locus_graph/src/client.rs`

```rust
/// Get overview of all unresolved links in the graph.
pub async fn get_unresolved_overview(&self) -> Result<UnresolvedOverview> {
    let request = GetUnresolvedOverviewRequest {
        graph_id: self.config.graph_id.clone(),
    };

    let response = self.proxy.get_unresolved_overview(request).await?;

    Ok(UnresolvedOverview {
        total_unresolved_links: response.total_unresolved_links,
        unique_context_ids: response.unique_context_ids,
        context_ids: response.context_ids.into_iter().map(|s| UnresolvedContextStats {
            context_id: s.context_id,
            link_count: s.link_count,
            oldest_link_age_hours: s.oldest_link_age_hours,
        }).collect(),
    })
}
```

**Add to `types.rs`:**

```rust
/// Overview of unresolved links in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedOverview {
    pub total_unresolved_links: u64,
    pub unique_context_ids: u64,
    pub context_ids: Vec<UnresolvedContextStats>,
}

/// Stats for a single unresolved context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedContextStats {
    pub context_id: String,
    pub link_count: u64,
    pub oldest_link_age_hours: u64,
}
```

**Import:** `use locus_proxy::GetUnresolvedOverviewRequest;`

### Task 8: Add `get_unresolved_links`

**File:** `crates/locus_graph/src/client.rs`

```rust
/// Get unresolved links for a specific context.
pub async fn get_unresolved_links(
    &self,
    context_id: &str,
) -> Result<UnresolvedLinks> {
    let request = GetUnresolvedLinksRequest {
        graph_id: self.config.graph_id.clone(),
        context_id: context_id.to_string(),
    };

    let response = self.proxy.get_unresolved_links(request).await?;

    Ok(UnresolvedLinks {
        context_id: response.context_id,
        links_count: response.links_count,
        links: response.links.into_iter().map(|l| LinkInfo {
            from: l.from,
            to: l.to,
            link_type: l.link_type,
        }).collect(),
    })
}
```

**Add to `types.rs`:**

```rust
/// Unresolved links for a context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedLinks {
    pub context_id: String,
    pub links_count: u64,
    pub links: Vec<LinkInfo>,
}

/// A single link between contexts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInfo {
    pub from: String,
    pub to: String,
    pub link_type: String,
}
```

**Import:** `use locus_proxy::GetUnresolvedLinksRequest;`

### Task 9: Update lib.rs re-exports

**File:** `crates/locus_graph/src/lib.rs`

Add new types to the `pub use types::` block:

```rust
pub use types::{
    // existing
    Context, ContextResult, ContextType, ContextTypeFilter, CreateEventRequest, EventKind,
    EventLinks, InsightResult, InsightsOptions, RetrieveOptions, TurnSummary,
    // new
    BatchContextResult, BatchResolveResult, ContextDetail, ContextRelationship,
    LinkInfo, ResolveResult, UnresolvedContextStats, UnresolvedLinks, UnresolvedOverview,
};
```

### Task 10: Handle import collisions

The proxy re-exports some types with the same names as the new wrapper types. In `client.rs`, use qualified imports or rename:

```rust
// At the top of client.rs, the existing proxy imports need to be checked.
// locus_proxy re-exports: Context, ContextRelationship, ResolveRequest, etc.
// Our types.rs defines: Context, ContextRelationship, ResolveResult, etc.
//
// Use locus_proxy:: prefix for proxy types that collide:
//   locus_proxy::Context → converted to types::Context
//   locus_proxy::ContextRelationship → converted to types::ContextRelationship
//   locus_proxy::ResolveRequest → used as request, no collision
```

The existing code already imports `locus_proxy::ContextTypeFilter` directly. The new methods should follow the same pattern — import the request types from `locus_proxy`, convert response types to `crate::types`.

---

## Verification

```bash
cargo check -p locus-graph
cargo test -p locus-graph
cargo clippy -p locus-graph
cargo doc -p locus-graph --no-deps
```

All must pass. Doc should show all new methods on `LocusGraphClient`.

### Quick smoke test

```rust
#[test]
fn test_context_detail_serialization() {
    let detail = ContextDetail {
        context_id: "fact:auth".to_string(),
        context: Some(Context {
            context_id: "fact:auth".to_string(),
            context_type: "fact".to_string(),
            context_name: "auth".to_string(),
            created_at: 0,
            updated_at: 0,
            reference_count: 1,
        }),
        locus_id: "locus-123".to_string(),
        payload_json: r#"{"topic":"auth"}"#.to_string(),
    };
    let json = serde_json::to_string(&detail).unwrap();
    assert!(json.contains("fact:auth"));
}
```

---

## Files Changed (summary)

| File | Changes |
|---|---|
| `crates/locus_graph/src/client.rs` | Add 8 new methods: `get_context`, `get_context_by_name`, `batch_get_context`, `get_context_relationships`, `resolve`, `batch_resolve`, `get_unresolved_overview`, `get_unresolved_links` |
| `crates/locus_graph/src/types.rs` | Add 8 new types: `ContextDetail`, `BatchContextResult`, `ContextRelationship`, `ResolveResult`, `BatchResolveResult`, `UnresolvedOverview`, `UnresolvedContextStats`, `UnresolvedLinks`, `LinkInfo` |
| `crates/locus_graph/src/lib.rs` | Update `pub use types::` to re-export new types |

**Do NOT change:**
- `locus_proxy` — it's already complete
- `hooks.rs` — no changes needed
- `config.rs` — no changes needed
- Any runtime crate files — these methods are not called yet

---

## When Each Method Is Needed

| Method | First needed | Phase |
|---|---|---|
| `get_context` | Check if fact/knowledge exists before storing | Phase 5 |
| `get_context_by_name` | Look up specific anchors by name | Phase 5 |
| `batch_get_context` | Load safety cache (rules + constraints) | Phase 6 |
| `get_context_relationships` | Traverse learning graph, find children | Phase 9 |
| `resolve` / `batch_resolve` | MCP/ACP tool registration | Phase 10 |
| `get_unresolved_overview/links` | Debug/admin tooling | Phase 10+ |
