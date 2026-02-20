//! Integration tests for LocusGraph client.
//!
//! These tests require a running LocusGraph gRPC server at localhost:50051.
//! Run with: cargo test -p locus-graph --test integration -- --test-threads=1

mod common;

use common::{test_client, unique_context_id, GRAPH_ID};
use locus_graph::{ContextTypeFilter, CreateEventRequest, EventKind, InsightsOptions, RetrieveOptions};
use serde_json::json;

#[tokio::test]
async fn test_client_connects() {
    let client = test_client().await;
    assert_eq!(client.graph_id(), GRAPH_ID);
}

#[tokio::test]
async fn test_store_event_fact() {
    let client = test_client().await;

    let context_id = unique_context_id("fact");
    let event = CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "technical_fact",
            "data": {
                "topic": "testing",
                "value": "we use Rust for the agent"
            }
        }),
    )
    .context_id(&context_id)
    .source("agent");

    let result = client.store_event_result(event).await;
    assert!(result.is_ok(), "Failed to store event: {:?}", result.err());
    let event_id = result.unwrap();
    assert!(!event_id.is_empty(), "Event ID should not be empty");
}

#[tokio::test]
async fn test_store_event_action() {
    let client = test_client().await;

    let context_id = unique_context_id("action");
    let event = CreateEventRequest::new(
        EventKind::Action,
        json!({
            "kind": "tool_run",
            "data": {
                "tool": "bash",
                "command": "cargo test",
                "success": true
            }
        }),
    )
    .context_id(&context_id)
    .source("agent")
    .related_to(vec!["fact:testing".to_string()]);

    let result = client.store_event_result(event).await;
    assert!(result.is_ok(), "Failed to store action event: {:?}", result.err());
}

#[tokio::test]
async fn test_store_event_decision() {
    let client = test_client().await;

    let context_id = unique_context_id("decision");
    let event = CreateEventRequest::new(
        EventKind::Decision,
        json!({
            "kind": "architecture_decision",
            "data": {
                "summary": "Use gRPC for LocusGraph communication",
                "reasoning": "Better performance and type safety than REST",
                "alternatives": ["REST API", "GraphQL"]
            }
        }),
    )
    .context_id(&context_id)
    .source("agent");

    let result = client.store_event_result(event).await;
    assert!(result.is_ok(), "Failed to store decision: {:?}", result.err());
}

#[tokio::test]
async fn test_store_event_observation() {
    let client = test_client().await;

    let context_id = unique_context_id("observation");
    let event = CreateEventRequest::new(
        EventKind::Observation,
        json!({
            "kind": "error",
            "data": {
                "context": "test_execution",
                "error_message": "Connection timeout",
                "recoverable": true
            }
        }),
    )
    .context_id(&context_id)
    .source("agent");

    let result = client.store_event_result(event).await;
    assert!(result.is_ok(), "Failed to store observation: {:?}", result.err());
}

#[tokio::test]
async fn test_store_event_feedback() {
    let client = test_client().await;

    let context_id = unique_context_id("feedback");
    let event = CreateEventRequest::new(
        EventKind::Feedback,
        json!({
            "kind": "user_feedback",
            "data": {
                "rating": 5,
                "comment": "Great implementation!",
                "feature": "memory_storage"
            }
        }),
    )
    .context_id(&context_id)
    .source("user");

    let result = client.store_event_result(event).await;
    assert!(result.is_ok(), "Failed to store feedback: {:?}", result.err());
}

#[tokio::test]
async fn test_store_event_with_extends() {
    let client = test_client().await;

    let base_context = unique_context_id("base");
    let extended_context = unique_context_id("extended");

    // Store base event
    let base_event = CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "base_fact",
            "data": {"value": "base information"}
        }),
    )
    .context_id(&base_context)
    .source("agent");

    client.store_event_result(base_event).await.unwrap();

    // Store extended event
    let extended_event = CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "extended_fact",
            "data": {"value": "additional information"}
        }),
    )
    .context_id(&extended_context)
    .source("agent")
    .extends(vec![base_context]);

    let result = client.store_event_result(extended_event).await;
    assert!(result.is_ok(), "Failed to store extended event: {:?}", result.err());
}

#[tokio::test]
async fn test_retrieve_memories_basic() {
    let client = test_client().await;

    // First store some test data
    let context_id = unique_context_id("retrieve_test");
    let event = CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "test_data",
            "data": {
                "topic": "retrieval_testing",
                "value": "This is test data for retrieval"
            }
        }),
    )
    .context_id(&context_id)
    .source("agent");

    client.store_event_result(event).await.unwrap();

    // Give the server a moment to index
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Now retrieve
    let result = client
        .retrieve_memories("retrieval testing", None)
        .await;

    assert!(result.is_ok(), "Failed to retrieve memories: {:?}", result.err());
    let _context = result.unwrap();
    // Note: items_found may be 0 if the embedding hasn't been indexed yet
    // but the call should succeed
}

#[tokio::test]
async fn test_retrieve_memories_with_options() {
    let client = test_client().await;

    let context_id = unique_context_id("retrieve_options_test");

    // Store test event
    let event = CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "filtered_data",
            "data": {
                "topic": "filtered_retrieval",
                "value": "This data should be filtered"
            }
        }),
    )
    .context_id(&context_id)
    .source("agent");

    client.store_event_result(event).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Retrieve with options
    let options = RetrieveOptions::new()
        .limit(5)
        .context_id(&context_id);

    let result = client
        .retrieve_memories("filtered", Some(options))
        .await;

    assert!(result.is_ok(), "Failed with options: {:?}", result.err());
}

#[tokio::test]
async fn test_retrieve_memories_with_context_types() {
    let client = test_client().await;

    let options = RetrieveOptions::new()
        .limit(10)
        .context_type("fact", ContextTypeFilter::new());

    let result = client
        .retrieve_memories("test query", Some(options))
        .await;

    assert!(result.is_ok(), "Failed with context_types: {:?}", result.err());
}

#[tokio::test]
async fn test_generate_insights_basic() {
    let client = test_client().await;

    // Store some context for insights
    let context_id = unique_context_id("insight_test");
    let event = CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "project_info",
            "data": {
                "topic": "testing",
                "value": "We use integration tests for verifying gRPC communication"
            }
        }),
    )
    .context_id(&context_id)
    .source("agent");

    client.store_event_result(event).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Generate insights
    let result = client
        .generate_insights("How do we test gRPC communication?", None)
        .await;

    assert!(result.is_ok(), "Failed to generate insights: {:?}", result.err());
    let insight = result.unwrap();
    assert!(!insight.insight.is_empty() || !insight.recommendation.is_empty());
    assert!(insight.confidence >= 0.0 && insight.confidence <= 1.0);
}

#[tokio::test]
async fn test_generate_insights_with_options() {
    let client = test_client().await;

    let options = InsightsOptions::new()
        .limit(5)
        .locus_query("testing")
        .context_type("fact", ContextTypeFilter::new());

    let result = client
        .generate_insights("What testing approach do we use?", Some(options))
        .await;

    assert!(result.is_ok(), "Failed with options: {:?}", result.err());
}

#[tokio::test]
async fn test_list_context_types() {
    let client = test_client().await;

    let result = client
        .list_context_types(None, Some(10))
        .await;

    assert!(result.is_ok(), "Failed to list context types: {:?}", result.err());
    let types = result.unwrap();
    // Should have at least some context types from our test data
    println!("Found {} context types", types.len());
    for ct in &types {
        println!("  - {}: {} contexts", ct.context_type, ct.count);
    }
}

#[tokio::test]
async fn test_list_context_types_with_pagination() {
    let client = test_client().await;

    // First page
    let result = client
        .list_context_types(Some(0), Some(5))
        .await;

    assert!(result.is_ok(), "Failed with pagination: {:?}", result.err());
}

#[tokio::test]
async fn test_list_contexts_by_type() {
    let client = test_client().await;

    // Store an event with a known type
    let context_id = unique_context_id("list_test");
    let event = CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "listable_fact",
            "data": {"topic": "listing", "value": "test"}
        }),
    )
    .context_id(&context_id)
    .source("agent");

    client.store_event_result(event).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // List contexts of type "fact"
    let result = client
        .list_contexts_by_type("fact", None, Some(10))
        .await;

    assert!(result.is_ok(), "Failed to list contexts: {:?}", result.err());
    let contexts = result.unwrap();
    println!("Found {} fact contexts", contexts.len());
}

#[tokio::test]
async fn test_search_contexts() {
    let client = test_client().await;

    // Store an event with a searchable name
    let context_id = unique_context_id("searchable_unique_name_12345");
    let event = CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "searchable_data",
            "data": {"topic": "search", "value": "test"}
        }),
    )
    .context_id(&context_id)
    .source("agent");

    client.store_event_result(event).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Search for the context
    let result = client
        .search_contexts("searchable_unique_name", None, None, Some(10))
        .await;

    assert!(result.is_ok(), "Failed to search contexts: {:?}", result.err());
    let contexts = result.unwrap();
    println!("Found {} matching contexts", contexts.len());
}

#[tokio::test]
async fn test_search_contexts_with_type_filter() {
    let client = test_client().await;

    let result = client
        .search_contexts("test", Some("fact"), Some(0), Some(10))
        .await;

    assert!(result.is_ok(), "Failed with type filter: {:?}", result.err());
}

#[tokio::test]
async fn test_fire_and_forget_store() {
    let client = test_client().await;

    let context_id = unique_context_id("fire_forget");
    let event = CreateEventRequest::new(
        EventKind::Action,
        json!({
            "kind": "background_action",
            "data": {"action": "async_store"}
        }),
    )
    .context_id(&context_id)
    .source("agent");

    // This should not block or return an error
    client.store_event(event).await;
}

#[tokio::test]
async fn test_queued_events_count() {
    let client = test_client().await;

    let result = client.queued_events_count();
    assert!(result.is_ok(), "Failed to get queue count: {:?}", result.err());
    let count = result.unwrap();
    println!("Queued events: {}", count);
}
