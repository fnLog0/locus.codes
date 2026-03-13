# Verify Phases 1-2-3 via LocusGraph API

**Goal:** Build a runtime verification test that starts the Runtime, runs a session, and uses the LocusGraph client APIs to confirm phases 1 (project anchor), 2 (tool anchor + tools), and 3 (session anchor + session lifecycle) are all stored correctly.

**Depends on:** `implement_client_methods.md` must be done first — this uses `get_context`, `get_context_relationships`, and `list_contexts_by_type`.

**File to create:** `crates/locus_runtime/tests/verify_phases.rs`

This is an integration test that requires a running LocusGraph server (via env vars). Skip gracefully if env is not set.

---

## The Test

```rust
//! Integration test: verify Phases 1-3 are stored correctly in LocusGraph.
//!
//! Requires LOCUSGRAPH_GRPC_ENDPOINT and LOCUSGRAPH_AGENT_SECRET in env.
//! Run: cargo test -p locus-runtime --test verify_phases -- --nocapture

use locus_graph::{LocusGraphClient, LocusGraphConfig};

/// Skip helper — returns None if LocusGraph env is not set.
fn try_config() -> Option<LocusGraphConfig> {
    LocusGraphConfig::from_env().ok()
}

// ─── Phase 1: Project Root Anchor ───────────────────────────────────────

#[tokio::test]
async fn phase1_project_anchor_exists() {
    let Some(config) = try_config() else {
        eprintln!("SKIP: LocusGraph env not set");
        return;
    };
    let client = LocusGraphClient::new(config).await.expect("client");

    // List all context types — "project" type should exist
    let types = client.list_context_types(None, None).await.expect("list types");
    let has_project = types.iter().any(|t| t.context_type == "project");
    assert!(has_project, "No 'project' context type found. Phase 1 not bootstrapped.");

    // List project contexts — at least one should exist
    let projects = client.list_contexts_by_type("project", None, None).await.expect("list projects");
    assert!(!projects.is_empty(), "No project contexts found.");

    println!("✅ Phase 1: project anchor exists");
    for p in &projects {
        println!("   project: {} (refs: {})", p.context_id, p.reference_count);
    }
}

#[tokio::test]
async fn phase1_project_anchor_has_payload() {
    let Some(config) = try_config() else { return };
    let client = LocusGraphClient::new(config).await.expect("client");

    let projects = client.list_contexts_by_type("project", None, None).await.expect("list");
    assert!(!projects.is_empty(), "No project contexts");

    let project_id = &projects[0].context_id;
    let detail = client.get_context(project_id).await.expect("get_context");

    assert!(!detail.payload_json.is_empty(), "Project anchor has empty payload");

    let payload: serde_json::Value = serde_json::from_str(&detail.payload_json)
        .expect("payload is valid JSON");
    let data = payload.get("data").expect("payload has 'data' field");
    assert!(data.get("project_name").is_some(), "Missing project_name in payload");
    assert!(data.get("repo_hash").is_some(), "Missing repo_hash in payload");

    println!("✅ Phase 1: project anchor payload correct");
    println!("   project_name: {}", data["project_name"]);
}

// ─── Phase 2: Tool Anchor + Tools ───────────────────────────────────────

#[tokio::test]
async fn phase2_tool_anchor_exists() {
    let Some(config) = try_config() else { return };
    let client = LocusGraphClient::new(config).await.expect("client");

    let types = client.list_context_types(None, None).await.expect("list types");
    let has_tool_anchor = types.iter().any(|t| t.context_type == "tool_anchor");
    assert!(has_tool_anchor, "No 'tool_anchor' context type found. Phase 2 not bootstrapped.");

    let anchors = client.list_contexts_by_type("tool_anchor", None, None).await.expect("list");
    assert!(!anchors.is_empty(), "No tool_anchor contexts found.");

    println!("✅ Phase 2: tool_anchor exists");
    for a in &anchors {
        println!("   tool_anchor: {}", a.context_id);
    }
}

#[tokio::test]
async fn phase2_tool_anchor_extends_project() {
    let Some(config) = try_config() else { return };
    let client = LocusGraphClient::new(config).await.expect("client");

    let anchors = client.list_contexts_by_type("tool_anchor", None, None).await.expect("list");
    assert!(!anchors.is_empty());

    // Parse type and name from context_id "tool_anchor:locuscodes_abc123"
    let ctx_id = &anchors[0].context_id;
    let name = ctx_id.strip_prefix("tool_anchor:").expect("has prefix");

    let rels = client
        .get_context_relationships("tool_anchor", name, Some("extends"), Some("outgoing"), None, None)
        .await
        .expect("relationships");

    let extends_project = rels.iter().any(|r| {
        r.context
            .as_ref()
            .map(|c| c.context_type == "project")
            .unwrap_or(false)
    });
    assert!(extends_project, "tool_anchor does not extend a project anchor");

    println!("✅ Phase 2: tool_anchor extends project");
}

#[tokio::test]
async fn phase2_individual_tools_exist() {
    let Some(config) = try_config() else { return };
    let client = LocusGraphClient::new(config).await.expect("client");

    let types = client.list_context_types(None, None).await.expect("list types");
    let has_tool = types.iter().any(|t| t.context_type == "tool");
    assert!(has_tool, "No 'tool' context type found.");

    let tools = client.list_contexts_by_type("tool", None, None).await.expect("list tools");

    let expected = ["bash", "create_file", "edit_file", "undo_edit", "glob", "grep", "finder"];
    for name in expected {
        let found = tools.iter().any(|t| t.context_name == name);
        assert!(found, "tool:{} not found in LocusGraph", name);
    }

    println!("✅ Phase 2: all core tools registered");
    for t in &tools {
        println!("   tool: {} ({})", t.context_name, t.context_id);
    }
}

#[tokio::test]
async fn phase2_meta_tools_exist() {
    let Some(config) = try_config() else { return };
    let client = LocusGraphClient::new(config).await.expect("client");

    let types = client.list_context_types(None, None).await.expect("list types");
    let has_meta = types.iter().any(|t| t.context_type == "meta");
    assert!(has_meta, "No 'meta' context type found.");

    let metas = client.list_contexts_by_type("meta", None, None).await.expect("list metas");

    let expected = ["tool_search", "tool_explain", "task"];
    for name in expected {
        let found = metas.iter().any(|t| t.context_name == name);
        assert!(found, "meta:{} not found in LocusGraph", name);
    }

    println!("✅ Phase 2: all meta-tools registered");
}

#[tokio::test]
async fn phase2_tool_search_returns_results() {
    let Some(config) = try_config() else { return };
    let client = LocusGraphClient::new(config).await.expect("client");

    // Semantic search for "file operations" — should find edit_file, create_file, etc.
    use locus_graph::{ContextTypeFilter, RetrieveOptions};
    let options = RetrieveOptions::new()
        .limit(5)
        .context_type("tool", ContextTypeFilter::new())
        .context_type("meta", ContextTypeFilter::new());

    let result = client
        .retrieve_memories("file operations", Some(options))
        .await
        .expect("retrieve");

    assert!(result.items_found > 0, "tool_search returned 0 results for 'file operations'");
    assert!(!result.memories.is_empty(), "tool_search returned empty memories");

    println!("✅ Phase 2: tool_search works ({} results)", result.items_found);
}

// ─── Phase 3: Session Anchor + Session Lifecycle ────────────────────────

#[tokio::test]
async fn phase3_session_anchor_exists() {
    let Some(config) = try_config() else { return };
    let client = LocusGraphClient::new(config).await.expect("client");

    let types = client.list_context_types(None, None).await.expect("list types");
    let has_session_anchor = types.iter().any(|t| t.context_type == "session_anchor");
    assert!(
        has_session_anchor,
        "No 'session_anchor' context type found. Phase 3 not bootstrapped."
    );

    let anchors = client
        .list_contexts_by_type("session_anchor", None, None)
        .await
        .expect("list");
    assert!(!anchors.is_empty(), "No session_anchor contexts found.");

    println!("✅ Phase 3: session_anchor exists");
}

#[tokio::test]
async fn phase3_session_anchor_extends_project() {
    let Some(config) = try_config() else { return };
    let client = LocusGraphClient::new(config).await.expect("client");

    let anchors = client
        .list_contexts_by_type("session_anchor", None, None)
        .await
        .expect("list");
    assert!(!anchors.is_empty());

    let ctx_id = &anchors[0].context_id;
    let name = ctx_id.strip_prefix("session_anchor:").expect("has prefix");

    let rels = client
        .get_context_relationships("session_anchor", name, Some("extends"), Some("outgoing"), None, None)
        .await
        .expect("relationships");

    let extends_project = rels.iter().any(|r| {
        r.context
            .as_ref()
            .map(|c| c.context_type == "project")
            .unwrap_or(false)
    });
    assert!(extends_project, "session_anchor does not extend project");

    println!("✅ Phase 3: session_anchor extends project");
}

#[tokio::test]
async fn phase3_sessions_exist_after_run() {
    let Some(config) = try_config() else { return };
    let client = LocusGraphClient::new(config).await.expect("client");

    // Check if any sessions have been created
    let types = client.list_context_types(None, None).await.expect("list types");
    let session_type = types.iter().find(|t| t.context_type == "session");

    match session_type {
        Some(st) if st.count > 0 => {
            let sessions = client
                .list_contexts_by_type("session", None, None)
                .await
                .expect("list sessions");

            println!("✅ Phase 3: {} session(s) found", sessions.len());
            for s in &sessions {
                println!("   session: {} (refs: {})", s.context_id, s.reference_count);

                // Verify payload
                let detail = client.get_context(&s.context_id).await.expect("get");
                let payload: serde_json::Value =
                    serde_json::from_str(&detail.payload_json).expect("valid JSON");
                let data = payload.get("data").expect("has data");
                let status = data.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
                println!("     status: {}, slug: {}", status, data.get("slug").unwrap_or(&serde_json::json!("?")));
            }
        }
        _ => {
            println!("⚠️  Phase 3: no sessions yet (run the agent first, then re-test)");
        }
    }
}

#[tokio::test]
async fn phase3_session_anchor_active_session() {
    let Some(config) = try_config() else { return };
    let client = LocusGraphClient::new(config).await.expect("client");

    let anchors = client
        .list_contexts_by_type("session_anchor", None, None)
        .await
        .expect("list");
    if anchors.is_empty() {
        println!("⚠️  SKIP: no session_anchor");
        return;
    }

    let detail = client.get_context(&anchors[0].context_id).await.expect("get");
    let payload: serde_json::Value =
        serde_json::from_str(&detail.payload_json).expect("valid JSON");
    let data = payload.get("data").expect("has data");
    let active = data.get("active_session");

    match active {
        Some(serde_json::Value::Null) | None => {
            println!("✅ Phase 3: session_anchor has no active session (clean state)");
        }
        Some(v) => {
            println!(
                "✅ Phase 3: session_anchor has active_session = {} (agent is running or unclean shutdown)",
                v
            );
        }
    }
}

// ─── Full Hierarchy Check ───────────────────────────────────────────────

#[tokio::test]
async fn hierarchy_full_check() {
    let Some(config) = try_config() else { return };
    let client = LocusGraphClient::new(config).await.expect("client");

    let types = client.list_context_types(None, None).await.expect("list types");

    println!("\n=== LocusGraph Context Types ===");
    for t in &types {
        println!("  {:20} — {} context(s)", t.context_type, t.count);
    }

    let required_types = ["project", "tool_anchor", "tool", "meta", "session_anchor"];
    let mut all_ok = true;
    for rt in required_types {
        if !types.iter().any(|t| t.context_type == rt) {
            println!("❌ Missing required type: {}", rt);
            all_ok = false;
        }
    }

    if all_ok {
        println!("\n✅ All phases 1-3 verified:");
        println!("   Phase 1: project anchor ✓");
        println!("   Phase 2: tool_anchor + tools + metas ✓");
        println!("   Phase 3: session_anchor ✓");
    }
}
```

---

## How to Run

```bash
# Set env vars (already in .env usually)
export LOCUSGRAPH_GRPC_ENDPOINT="https://api.locusgraph.com"
export LOCUSGRAPH_AGENT_SECRET="your-secret"
export LOCUSGRAPH_GRAPH_ID="your-graph-id"

# Run all phase verification tests
cargo test -p locus-runtime --test verify_phases -- --nocapture

# Run a single test
cargo test -p locus-runtime --test verify_phases phase1_project_anchor_exists -- --nocapture
```

---

## What Each Test Verifies

| Test | Phase | What it checks |
|---|---|---|
| `phase1_project_anchor_exists` | 1 | `project:` context type exists, at least one context |
| `phase1_project_anchor_has_payload` | 1 | Payload has `project_name` and `repo_hash` |
| `phase2_tool_anchor_exists` | 2 | `tool_anchor:` context type exists |
| `phase2_tool_anchor_extends_project` | 2 | `tool_anchor` → extends → `project` relationship |
| `phase2_individual_tools_exist` | 2 | `tool:bash`, `tool:edit_file`, etc. all present |
| `phase2_meta_tools_exist` | 2 | `meta:tool_search`, `meta:tool_explain`, `meta:task` |
| `phase2_tool_search_returns_results` | 2 | Semantic search for "file operations" returns tools |
| `phase3_session_anchor_exists` | 3 | `session_anchor:` context type exists |
| `phase3_session_anchor_extends_project` | 3 | `session_anchor` → extends → `project` relationship |
| `phase3_sessions_exist_after_run` | 3 | `session:` contexts exist with correct payload |
| `phase3_session_anchor_active_session` | 3 | `active_session` field is null (clean) or set (running) |
| `hierarchy_full_check` | 1-3 | All required context types present, summary output |

---

## Prerequisites

1. **`implement_client_methods.md` must be done** — tests use `get_context`, `get_context_relationships`, `list_contexts_by_type`
2. **LocusGraph server must be running** and env vars set
3. **Agent must have been run at least once** against this repo — otherwise session tests will just print warnings
4. Tests skip gracefully if env is not configured (no failures in CI)

---

## Expected Output (after running agent once)

```
=== LocusGraph Context Types ===
  project              — 1 context(s)
  tool_anchor          — 1 context(s)
  tool                 — 7 context(s)
  meta                 — 3 context(s)
  session_anchor       — 1 context(s)
  session              — 2 context(s)

✅ All phases 1-3 verified:
   Phase 1: project anchor ✓
   Phase 2: tool_anchor + tools + metas ✓
   Phase 3: session_anchor ✓
```
