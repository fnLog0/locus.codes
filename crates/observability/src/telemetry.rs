//! OpenTelemetry telemetry initialization for version 0.31.0
//!
//! Implements OTLP export to Tempo/Grafana with console logging fallback.
//!
//! Based on OpenTelemetry Rust 0.31 API patterns from:
//! https://github.com/open-telemetry/opentelemetry-rust
//! https://docs.rs/opentelemetry-otlp/0.31.0/opentelemetry_otlp/

use once_cell::sync::OnceCell;
use opentelemetry::{global, trace::TracerProvider, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Registry};

use crate::config::ObservabilityConfig;
use crate::error::ObservabilityError;
use crate::tui_log_layer;

// Store the tracer provider for proper shutdown
static TRACER_PROVIDER: OnceCell<SdkTracerProvider> = OnceCell::new();

/// Initialize OpenTelemetry tracing with the given configuration
///
/// Uses OpenTelemetry 0.31.0 API with OTLP export to Tempo/Grafana.
///
/// # Arguments
///
/// * `config` - Observability configuration
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if initialization fails
pub fn init(config: ObservabilityConfig) -> Result<(), ObservabilityError> {
    let env_filter = config
        .log_level
        .as_ref()
        .map(|level| tracing_subscriber::EnvFilter::new(level.as_str()))
        .unwrap_or_else(|| {
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        });

    // Build resource using Resource::new() pattern (0.31 API)
    let mut attributes = vec![KeyValue::new("service.name", config.service_name.clone())];

    if let Some(version) = &config.service_version {
        attributes.push(KeyValue::new("service.version", version.clone()));
    }

    // Add custom resource attributes
    for (key, value) in &config.resource_attributes {
        attributes.push(KeyValue::new(key.clone(), value.clone()));
    }

    // Build resource using Resource::builder() pattern (0.31 API)
    // In 0.31, Resource constructors are private; ResourceBuilder is the public API
    let resource = Resource::builder().with_attributes(attributes).build();

    // Build layers first (build separately, then compose once to avoid type mismatch)
    let fmt_layer = config
        .enable_console
        .then_some(tracing_subscriber::fmt::layer());

    // Build OTLP layer if endpoint is configured
    let otel_layer = if let Some(endpoint) = &config.otlp_endpoint {
        match build_otlp_tracer_provider(&config.service_name, endpoint, resource.clone()) {
            Ok((tracer, provider)) => {
                // Set as global provider BEFORE creating layer (important ordering)
                global::set_tracer_provider(provider.clone());

                // Store provider for shutdown
                let _ = TRACER_PROVIDER.set(provider);

                tracing::info!(
                    service.name = %config.service_name,
                    otlp.endpoint = %endpoint,
                    "OTLP tracing enabled"
                );

                Some(OpenTelemetryLayer::new(tracer))
            }
            Err(e) => {
                tracing::warn!(
                    service.name = %config.service_name,
                    endpoint = %endpoint,
                    error = %e,
                    "Failed to initialize OTLP export, falling back to console-only tracing"
                );
                None
            }
        }
    } else {
        tracing::info!(
            service.name = %config.service_name,
            "Tracing initialized (console only, no OTLP endpoint configured)"
        );
        None
    };

    // Optional TUI log sink (runtime logs for debug traces screen)
    let tui_layer = tui_log_layer::tui_log_layer(config.log_sink.clone());

    // Compose subscriber once (no mutation, avoids type mismatch)
    let subscriber = Registry::default()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_layer)
        .with(tui_layer);

    // Initialize subscriber
    subscriber.init();

    Ok(())
}

/// Build OTLP tracer provider for OpenTelemetry 0.31
///
/// Creates a tracer provider with OTLP exporter for spans (e.g., Tempo).
/// Uses the correct 0.31 API pattern: SpanExporter::builder() + SdkTracerProvider::builder().
///
/// # Arguments
///
/// * `service_name` - Service name for resource attributes
/// * `endpoint` - OTLP endpoint URL (e.g., "http://localhost:4317")
/// * `resource` - OpenTelemetry resource with attributes
///
/// # Returns
///
/// Returns (tracer, provider) on success, or an error if initialization fails
fn build_otlp_tracer_provider(
    service_name: &str,
    endpoint: &str,
    resource: Resource,
) -> Result<(opentelemetry_sdk::trace::SdkTracer, SdkTracerProvider), ObservabilityError> {
    // Build the OTLP span exporter using 0.31 builder pattern
    // Pattern: SpanExporter::builder().with_tonic().with_endpoint().build()
    // Note: with_endpoint() requires String, not &str
    let span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint.to_string())
        .build()
        .map_err(|e| ObservabilityError::InitFailed(e.to_string()))?;

    // Create SDK tracer provider with batch exporter
    // In 0.31, with_batch_exporter() takes only the exporter (runtime handled via rt-tokio feature)
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(span_exporter)
        .with_resource(resource)
        .build();

    // Get tracer from provider (0.31 API: provider.tracer() returns SdkTracer)
    // Note: tracer() requires 'static lifetime, so we pass owned String
    let tracer = provider.tracer(service_name.to_string());

    Ok((tracer, provider))
}

/// Shutdown OpenTelemetry tracer provider
///
/// Call this during graceful shutdown to ensure all traces are exported.
/// Uses the correct 0.31 pattern: call shutdown() on the provider instance.
pub fn shutdown() {
    if let Some(provider) = TRACER_PROVIDER.get() {
        // In 0.31, shutdown() returns Result but errors are logged internally
        let _ = provider.shutdown();
        tracing::debug!("OpenTelemetry tracer provider shut down");
    } else {
        tracing::debug!("No OpenTelemetry tracer provider to shutdown");
    }
}

/// Initialize with default configuration from environment variables
pub fn init_from_env() -> Result<(), ObservabilityError> {
    let config = ObservabilityConfig::from_env();
    init(config)
}
