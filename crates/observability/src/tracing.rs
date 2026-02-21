//! Utility functions for tracing

/// Create a span with common HTTP request fields
///
/// This is a convenience macro for creating spans with HTTP request metadata.
/// Use this in HTTP handlers to automatically capture request information.
///
/// # Example
///
/// ```rust
/// use locusgraph_observability::http_request_span;
///
/// let span = http_request_span!("GET", "/api/users", "12345");
/// let _guard = span.enter();
/// // ... handler code ...
/// ```
#[macro_export]
macro_rules! http_request_span {
    ($method:expr, $path:expr, $request_id:expr) => {
        tracing::info_span!(
            "http.request",
            http.method = $method,
            http.route = $path,
            http.status_code = tracing::field::Empty,
            request.id = $request_id,
        )
    };
}

/// Create a span for agent operations
///
/// Use this to create spans for agent-specific operations.
///
/// # Example
///
/// ```rust
/// use locusgraph_observability::agent_span;
///
/// let span = agent_span!("agent-123", "store_event");
/// let _guard = span.enter();
/// // ... agent operation ...
/// ```
#[macro_export]
macro_rules! agent_span {
    ($agent_id:expr, $operation:expr) => {
        tracing::info_span!(
            "agent.operation",
            agent.id = $agent_id,
            operation = $operation,
        )
    };
}

/// Create a span for storage operations
///
/// Use this to create spans for storage-related operations (RocksDB, Qdrant, etc.).
///
/// # Example
///
/// ```rust
/// use locusgraph_observability::storage_span;
///
/// let span = storage_span!("rocksdb", "write", "key-123");
/// let _guard = span.enter();
/// // ... storage operation ...
/// ```
#[macro_export]
macro_rules! storage_span {
    ($backend:expr, $operation:expr, $key:expr) => {
        tracing::info_span!(
            "storage.operation",
            storage.backend = $backend,
            storage.operation = $operation,
            storage.key = $key,
        )
    };
}

/// Record an error on the current span
///
/// This is a convenience function that records an error with its message
/// on the current active span.
///
/// # Example
///
/// ```rust
/// use locusgraph_observability::record_error;
///
/// match some_operation() {
///     Ok(result) => result,
///     Err(e) => {
///         record_error(&e);
///         return Err(e);
///     }
/// }
/// ```
pub fn record_error<E: std::error::Error>(error: &E) {
    let span = tracing::Span::current();
    span.record("error", true);
    span.record("error.message", error.to_string());
    tracing::error!(error = %error, "Operation failed");
}

/// Record latency/duration on the current span
///
/// This is a convenience function that records a duration metric
/// on the current active span.
///
/// # Example
///
/// ```rust
/// use locusgraph_observability::record_duration;
/// use std::time::Instant;
///
/// let start = Instant::now();
/// // ... operation ...
/// record_duration("operation.duration_ms", start.elapsed());
/// ```
pub fn record_duration(key: &str, duration: std::time::Duration) {
    let span = tracing::Span::current();
    span.record(key, duration.as_millis() as u64);
}
