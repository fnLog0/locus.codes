# Implement Missing LocusGraphClient Methods

**Goal:** Add 7 missing methods to `crates/locus_graph/src/client.rs` and their types to `crates/locus_graph/src/types.rs`. All proxy methods already exist — this is just the higher-level wrapper.

**Files to change:**
- `crates/locus_graph/src/types.rs` — add new types
- `crates/locus_graph/src/client.rs` — add new methods + imports
- `crates/locus_graph/src/lib.rs` — update re-exports

**Reference (already implemented):**
- Proxy client: `/Users/nasimakhtar/Projects/hyperbola-network/locusgraph/apps/locus_proxy/src/client.rs`
- Proto: `/Users/nasimakhtar/Projects/hyperbola-network/locusgraph/apps/locus_proxy/proto/locusgraph/v1/`
- TS client: `/Users/nasimakhtar/Projects/hyperbola-network/locusgraph/apps/bindings/typescript/src/client.ts`

---

## Step 1: Add types to `types.rs`

Append these after `TurnSummary` (line ~313):

```rust
/// Detailed context with payload, returned by get_context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextDetail {
    /// Full context_id (e.g. "fact:auth")
    pub context_id: String,
    /// Context metadata (may be None if not found)
    pub context: Option<Context>,
    /// Resolved locus_id
    pub locus_id: String,
    /// Latest payload as JSON string
    pub payload_json: String,
}

/// Result from batch_get_context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchContextResult {
    /// Found contexts with payloads
    pub contexts: Vec<ContextDetail>,
    /// Context IDs that were not found
    pub not_found: Vec<String>,
}

/// A relationship between two contexts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRelationship {
    /// The related context
    pub context: Option<Context>,
    /// Link type: "related_to", "extends", "reinforces", "contradicts"
    pub link_type: String,
    /// Direction: "outgoing", "incoming"
    pub direction: String,
}

/// Result from resolve operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveResult {
    pub context_id: String,
    pub locus_id: String,
    pub links_resolved: u64,
    pub success: bool,
}

/// Result from batch_resolve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResolveResult {
    pub results: Vec<ResolveResult>,
    pub total_resolved: u64,
}

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

---

## Step 2: Update imports in `client.rs`

Current import block (line ~7):

```rust
use crate::types::{
    Context, ContextResult, ContextType, CreateEventRequest, InsightResult, InsightsOptions,
    RetrieveOptions,
};
```

Change to:

```rust
use crate::types::{
    BatchContextResult, BatchResolveResult, Context, ContextDetail, ContextRelationship,
    ContextResult, ContextType, CreateEventRequest, InsightResult, InsightsOptions, LinkInfo,
    ResolveResult, RetrieveOptions, UnresolvedContextStats, UnresolvedLinks, UnresolvedOverview,
};
```

Current proxy import block (line ~11):

```rust
use locus_proxy::{
    ContextTypeFilter, GenerateInsightsRequest, ListContextTypesRequest, ListContextsByTypeRequest,
    ListContextsResponse, RetrieveContextRequest, SearchContextsRequest, StoreEventRequest,
};
```

Change to:

```rust
use locus_proxy::{
    BatchGetContextRequest, BatchResolveRequest, ContextTypeFilter, GenerateInsightsRequest,
    GetContextByNameRequest, GetContextRelationshipsRequest, GetContextRequest,
    GetUnresolvedLinksRequest, GetUnresolvedOverviewRequest, ListContextTypesRequest,
    ListContextsByTypeRequest, ListContextsResponse, ResolveItem,
    ResolveRequest as ProxyResolveRequest, RetrieveContextRequest, SearchContextsRequest,
    StoreEventRequest,
};
```

Note: `ResolveRequest` is aliased to `ProxyResolveRequest` to avoid collision with any future wrapper type.

---

## Step 3: Add methods to `client.rs`

Add these methods inside `impl LocusGraphClient` (before `parse_contexts_response`):

### 3a: `get_context`

```rust
/// Get a single context by context_id.
/// Returns context metadata and latest payload.
pub async fn get_context(&self, context_id: &str) -> Result<ContextDetail> {
    let request = GetContextRequest {
        graph_id: self.config.graph_id.clone(),
        context_id: context_id.to_string(),
    };
    let response = self.proxy.get_context(request).await?;
    Ok(self.parse_context_detail(response))
}
```

### 3b: `get_context_by_name`

```rust
/// Get a single context by exact name, optionally scoped to a type.
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
    Ok(self.parse_context_detail(response))
}
```

### 3c: `batch_get_context`

```rust
/// Batch get multiple contexts by context_id.
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
        contexts: response.contexts.into_iter().map(|r| self.parse_context_detail(r)).collect(),
        not_found: response.not_found,
    })
}
```

### 3d: `get_context_relationships`

```rust
/// Get relationships for a context.
/// Filter by link_type ("related_to", "extends", "reinforces", "contradicts")
/// and direction ("outgoing", "incoming", "both").
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
    Ok(response
        .relationships
        .into_iter()
        .map(|r| ContextRelationship {
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
        })
        .collect())
}
```

### 3e: `resolve`

```rust
/// Resolve a context_id to a locus_id.
pub async fn resolve(
    &self,
    context_id: &str,
    locus_id: &str,
) -> Result<ResolveResult> {
    let request = ProxyResolveRequest {
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

### 3f: `batch_resolve`

```rust
/// Batch resolve multiple context_id → locus_id mappings.
pub async fn batch_resolve(
    &self,
    resolutions: Vec<(String, String)>,
) -> Result<BatchResolveResult> {
    let request = BatchResolveRequest {
        graph_id: self.config.graph_id.clone(),
        resolutions: resolutions
            .into_iter()
            .map(|(ctx, loc)| ResolveItem {
                context_id: ctx,
                locus_id: loc,
            })
            .collect(),
    };
    let response = self.proxy.batch_resolve(request).await?;
    Ok(BatchResolveResult {
        results: response
            .results
            .into_iter()
            .map(|r| ResolveResult {
                context_id: r.context_id,
                locus_id: r.locus_id,
                links_resolved: r.links_resolved,
                success: r.success,
            })
            .collect(),
        total_resolved: response.total_resolved,
    })
}
```

### 3g: `get_unresolved_overview`

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
        context_ids: response
            .context_ids
            .into_iter()
            .map(|s| UnresolvedContextStats {
                context_id: s.context_id,
                link_count: s.link_count,
                oldest_link_age_hours: s.oldest_link_age_hours,
            })
            .collect(),
    })
}
```

### 3h: `get_unresolved_links`

```rust
/// Get unresolved links for a specific context.
pub async fn get_unresolved_links(&self, context_id: &str) -> Result<UnresolvedLinks> {
    let request = GetUnresolvedLinksRequest {
        graph_id: self.config.graph_id.clone(),
        context_id: context_id.to_string(),
    };
    let response = self.proxy.get_unresolved_links(request).await?;
    Ok(UnresolvedLinks {
        context_id: response.context_id,
        links_count: response.links_count,
        links: response
            .links
            .into_iter()
            .map(|l| LinkInfo {
                from: l.from,
                to: l.to,
                link_type: l.link_type,
            })
            .collect(),
    })
}
```

---

## Step 4: Add `parse_context_detail` helper

Add this private method inside `impl LocusGraphClient` (next to `parse_contexts_response`):

```rust
fn parse_context_detail(&self, response: locus_proxy::GetContextResponse) -> ContextDetail {
    ContextDetail {
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
    }
}
```

---

## Step 5: Update `lib.rs` re-exports

Change the `pub use types::` block (line ~61):

```rust
pub use types::{
    BatchContextResult, BatchResolveResult, Context, ContextDetail, ContextRelationship,
    ContextResult, ContextType, ContextTypeFilter, CreateEventRequest, EventKind, EventLinks,
    InsightResult, InsightsOptions, LinkInfo, ResolveResult, RetrieveOptions, TurnSummary,
    UnresolvedContextStats, UnresolvedLinks, UnresolvedOverview,
};
```

---

## Verification

```bash
cargo check -p locus-graph
cargo test -p locus-graph
cargo clippy -p locus-graph
# Also verify downstream compiles:
cargo check -p locus-runtime
```

All must pass with zero errors.
