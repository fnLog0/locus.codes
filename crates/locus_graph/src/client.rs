//! LocusGraph client — wraps locus-proxy with higher-level API.
//!
//! Provides semantic search, memory retrieval, and event storage.

use crate::config::LocusGraphConfig;
use crate::error::Result;
use crate::types::{
    Context, ContextResult, ContextType, CreateEventRequest, InsightResult, InsightsOptions,
    RetrieveOptions,
};
use locus_proxy::{
    ContextTypeFilter, GenerateInsightsRequest, ListContextTypesRequest,
    ListContextsByTypeRequest, ListContextsResponse, RetrieveContextRequest,
    SearchContextsRequest, StoreEventRequest,
};
use std::sync::Arc;
use tracing::{debug, warn};

/// Backend allows only lowercase, digits, underscore, hyphen, colon. Enforce type:name (one colon).
fn sanitize_context_id(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return String::new();
    }
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    let (type_part, name_part) = match parts.as_slice() {
        [t, n] if !t.is_empty() && !n.is_empty() => (*t, *n),
        _ => return s.to_lowercase().chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_' || *c == '-').collect(),
    };
    let type_ok: String = type_part
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>()
        .to_lowercase();
    let name_ok: String = name_part
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>()
        .to_lowercase();
    if type_ok.is_empty() || name_ok.is_empty() {
        return "fact:unknown".to_string();
    }
    format!("{}:{}", type_ok, name_ok)
}

fn sanitize_context_id_list(list: Vec<String>) -> Vec<String> {
    list.into_iter()
        .map(|s| sanitize_context_id(&s))
        .filter(|s| s.contains(':') && !s.is_empty())
        .collect()
}

/// LocusGraph client for storing and retrieving memories.
///
/// Wraps the locus-proxy gRPC client with:
/// - Higher-level types
/// - Automatic graph_id injection
/// - Fire-and-forget storage
/// - Memory retrieval before LLM calls
#[derive(Clone)]
pub struct LocusGraphClient {
    proxy: Arc<locus_proxy::LocusProxyClient>,
    config: LocusGraphConfig,
}

impl LocusGraphClient {
    /// Create a new client with the given configuration.
    /// Ensures the parent directory of the DB path exists (e.g. project .locus).
    pub async fn new(config: LocusGraphConfig) -> Result<Self> {
        if let Some(parent) = config.db_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let proxy_config = locus_proxy::LocusProxyConfig::new(
            config.grpc_endpoint.clone(),
            config.agent_secret.clone(),
            config.db_path.clone(),
        )
        .cache_reads(config.cache_reads)
        .queue_stores(config.queue_stores);

        let proxy = locus_proxy::LocusProxyClient::new(proxy_config).await?;

        Ok(Self {
            proxy: Arc::new(proxy),
            config,
        })
    }

    /// Get the graph ID for this client.
    pub fn graph_id(&self) -> &str {
        &self.config.graph_id
    }

    /// Store a memory event (fire-and-forget — failures are logged but don't block).
    ///
    /// Returns `true` if the event was stored/queued successfully, `false` on failure.
    pub async fn store_event(&self, event: CreateEventRequest) -> bool {
        let body = serde_json::to_string_pretty(&event).unwrap_or_else(|_| format!("{:?}", event));
        tracing::debug!(
            target: "locus.trace",
            message = %format!("LocusGraph store_event\n{}", body)
        );
        let request = self.build_store_request(event);
        match self.proxy.store_event(request).await {
            Ok(_) => {
                debug!("Event stored successfully");
                true
            }
            Err(e) => {
                warn!("Failed to store event: {}", e);
                false
            }
        }
    }

    /// Store a memory event and return the result.
    pub async fn store_event_result(&self, event: CreateEventRequest) -> Result<String> {
        let request = self.build_store_request(event);
        let response = self.proxy.store_event(request).await?;
        Ok(response.event_id)
    }

    fn build_store_request(&self, event: CreateEventRequest) -> StoreEventRequest {
        StoreEventRequest {
            graph_id: self.config.graph_id.clone(),
            event_kind: event.event_kind.as_str().to_string(),
            context_id: event.context_id.as_ref().map(|s| sanitize_context_id(s)),
            source: event.source,
            payload_json: serde_json::to_string(&event.payload).unwrap_or_default(),
            related_to: sanitize_context_id_list(event.related_to.unwrap_or_default()),
            extends: sanitize_context_id_list(event.extends.unwrap_or_default()),
            reinforces: sanitize_context_id_list(event.reinforces.unwrap_or_default()),
            contradicts: sanitize_context_id_list(event.contradicts.unwrap_or_default()),
            timestamp: event.timestamp,
        }
    }

    /// Semantic search — returns memories relevant to a query.
    ///
    /// Called BEFORE every LLM call to inject context.
    /// On failure, returns empty context (agent works without memory).
    pub async fn retrieve_memories(
        &self,
        query: &str,
        options: Option<RetrieveOptions>,
    ) -> Result<ContextResult> {
        let opts = options.unwrap_or_default();

        let request = RetrieveContextRequest {
            graph_id: self.config.graph_id.clone(),
            query: query.to_string(),
            limit: opts.limit,
            context_ids: opts.context_ids,
            context_types: opts
                .context_types
                .into_iter()
                .map(|(k, v)| (k, ContextTypeFilter { names: v.names }))
                .collect(),
        };

        match self.proxy.retrieve_context(request).await {
            Ok(response) => Ok(ContextResult {
                memories: response.memories,
                items_found: response.items_found,
                degraded: false,
            }),
            Err(e) => {
                warn!("Failed to retrieve memories: {}", e);
                Ok(ContextResult {
                    memories: String::new(),
                    items_found: 0,
                    degraded: true,
                })
            }
        }
    }

    /// Reason over stored memories for a task.
    pub async fn generate_insights(
        &self,
        task: &str,
        options: Option<InsightsOptions>,
    ) -> Result<InsightResult> {
        let opts = options.unwrap_or_default();

        let request = GenerateInsightsRequest {
            graph_id: self.config.graph_id.clone(),
            task: task.to_string(),
            locus_query: opts.locus_query,
            limit: opts.limit,
            context_ids: opts.context_ids,
            context_types: opts
                .context_types
                .into_iter()
                .map(|(k, v)| (k, ContextTypeFilter { names: v.names }))
                .collect(),
        };

        let response = self.proxy.generate_insights(request).await?;

        // Parse confidence from string
        let confidence = response
            .confidence
            .parse::<f64>()
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);

        Ok(InsightResult {
            insight: response.insight,
            recommendation: response.recommendation,
            confidence,
        })
    }

    /// List available context types in the graph.
    pub async fn list_context_types(
        &self,
        page: Option<u64>,
        page_size: Option<u64>,
    ) -> Result<Vec<ContextType>> {
        let request = ListContextTypesRequest {
            graph_id: self.config.graph_id.clone(),
            page,
            page_size,
        };

        let response = self.proxy.list_context_types(request).await?;

        Ok(response
            .context_types
            .into_iter()
            .map(|ct| ContextType {
                context_type: ct.context_type,
                count: ct.count,
            })
            .collect())
    }

    /// List contexts by type.
    pub async fn list_contexts_by_type(
        &self,
        context_type: &str,
        page: Option<u64>,
        page_size: Option<u64>,
    ) -> Result<Vec<Context>> {
        let request = ListContextsByTypeRequest {
            graph_id: self.config.graph_id.clone(),
            context_type: context_type.to_string(),
            page,
            page_size,
        };

        let response = self.proxy.list_contexts_by_type(request).await?;

        Ok(self.parse_contexts_response(response))
    }

    /// Search contexts by name.
    pub async fn search_contexts(
        &self,
        query: &str,
        context_type: Option<&str>,
        page: Option<u64>,
        page_size: Option<u64>,
    ) -> Result<Vec<Context>> {
        let request = SearchContextsRequest {
            graph_id: self.config.graph_id.clone(),
            q: query.to_string(),
            context_type: context_type.map(|s| s.to_string()),
            min_count: None,
            page,
            page_size,
        };

        let response = self.proxy.search_contexts(request).await?;

        Ok(self.parse_contexts_response(response))
    }

    /// Get the number of queued events waiting to be sent.
    pub fn queued_events_count(&self) -> Result<usize> {
        self.proxy.queued_events_count().map_err(Into::into)
    }

    fn parse_contexts_response(&self, response: ListContextsResponse) -> Vec<Context> {
        response
            .contexts
            .into_iter()
            .map(|c| Context {
                context_id: c.context_id,
                context_type: c.context_type,
                context_name: c.context_name,
                created_at: c.created_at,
                updated_at: c.updated_at,
                reference_count: c.reference_count,
            })
            .collect()
    }
}
