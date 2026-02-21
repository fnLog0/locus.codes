//! Configuration for observability/telemetry

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Sink for runtime log lines (e.g. TUI debug traces). Called from the tracing layer; must not block.
pub type LogSink = Arc<dyn Fn(String) + Send + Sync>;

/// Observability configuration
#[derive(Clone)]
pub struct ObservabilityConfig {
    /// Service name for traces (e.g., "locusgraph-server", "locusgraph-agent")
    pub service_name: String,

    /// Service version (optional, defaults to "unknown")
    pub service_version: Option<String>,

    /// OTLP endpoint for trace export (e.g., "http://localhost:4317")
    pub otlp_endpoint: Option<String>,

    /// Enable console/log output in addition to OTLP export
    pub enable_console: bool,

    /// Log level filter (e.g., "info", "debug", "trace")
    /// Defaults to "info" if not set
    pub log_level: Option<String>,

    /// Additional resource attributes (key-value pairs)
    pub resource_attributes: Vec<(String, String)>,

    /// Optional sink for each formatted log line (e.g. TUI debug traces). Not serialized.
    pub log_sink: Option<LogSink>,
}

// Serde doesn't support Arc<dyn Fn>, so we don't derive Serialize/Deserialize for the whole struct.
// We use a separate impl and skip log_sink.
impl Serialize for ObservabilityConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ObservabilityConfig", 6)?;
        s.serialize_field("service_name", &self.service_name)?;
        s.serialize_field("service_version", &self.service_version)?;
        s.serialize_field("otlp_endpoint", &self.otlp_endpoint)?;
        s.serialize_field("enable_console", &self.enable_console)?;
        s.serialize_field("log_level", &self.log_level)?;
        s.serialize_field("resource_attributes", &self.resource_attributes)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for ObservabilityConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ObservabilityConfigDe {
            #[serde(default = "default_service_name")]
            service_name: String,
            service_version: Option<String>,
            otlp_endpoint: Option<String>,
            #[serde(default)]
            enable_console: bool,
            log_level: Option<String>,
            #[serde(default)]
            resource_attributes: Vec<(String, String)>,
        }
        fn default_service_name() -> String {
            "locusgraph-service".to_string()
        }
        let de = ObservabilityConfigDe::deserialize(deserializer)?;
        Ok(ObservabilityConfig {
            service_name: de.service_name,
            service_version: de.service_version,
            otlp_endpoint: de.otlp_endpoint,
            enable_console: de.enable_console,
            log_level: de.log_level,
            resource_attributes: de.resource_attributes,
            log_sink: None,
        })
    }
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: "locusgraph-service".to_string(),
            service_version: None,
            otlp_endpoint: None,
            enable_console: true,
            log_level: None,
            resource_attributes: Vec::new(),
            log_sink: None,
        }
    }
}

impl std::fmt::Debug for ObservabilityConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObservabilityConfig")
            .field("service_name", &self.service_name)
            .field("service_version", &self.service_version)
            .field("otlp_endpoint", &self.otlp_endpoint)
            .field("enable_console", &self.enable_console)
            .field("log_level", &self.log_level)
            .field("resource_attributes", &self.resource_attributes)
            .field("log_sink", &self.log_sink.as_ref().map(|_| "Some(LogSink)"))
            .finish()
    }
}

impl ObservabilityConfig {
    /// Create a new configuration with service name
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            ..Default::default()
        }
    }

    /// Set service version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = Some(version.into());
        self
    }

    /// Set OTLP endpoint
    pub fn with_otlp_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.otlp_endpoint = Some(endpoint.into());
        self
    }

    /// Enable or disable console output
    pub fn with_console(mut self, enable: bool) -> Self {
        self.enable_console = enable;
        self
    }

    /// Set log level
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = Some(level.into());
        self
    }

    /// Add resource attribute
    pub fn with_resource_attribute(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.resource_attributes.push((key.into(), value.into()));
        self
    }

    /// Sink for runtime log lines (e.g. TUI debug traces). Called from the tracing layer; must not block.
    pub fn with_log_sink(mut self, sink: LogSink) -> Self {
        self.log_sink = Some(sink);
        self
    }

    /// Build from environment variables
    ///
    /// Reads:
    /// - `OTEL_SERVICE_NAME` or `SERVICE_NAME` → service_name
    /// - `OTEL_SERVICE_VERSION` or `SERVICE_VERSION` → service_version
    /// - `OTEL_EXPORTER_OTLP_ENDPOINT` or `TEMPO_ENDPOINT` → otlp_endpoint
    /// - `OTEL_LOG_LEVEL` or `RUST_LOG` → log_level
    pub fn from_env() -> Self {
        let service_name = std::env::var("OTEL_SERVICE_NAME")
            .or_else(|_| std::env::var("SERVICE_NAME"))
            .unwrap_or_else(|_| "locusgraph-service".to_string());

        let service_version = std::env::var("OTEL_SERVICE_VERSION")
            .or_else(|_| std::env::var("SERVICE_VERSION"))
            .ok();

        // Only enable OTLP when explicitly set; otherwise console-only (avoids connection-refused noise).
        let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .or_else(|_| std::env::var("TEMPO_ENDPOINT"))
            .ok();

        let log_level = std::env::var("OTEL_LOG_LEVEL")
            .or_else(|_| std::env::var("RUST_LOG"))
            .ok();

        Self {
            service_name,
            service_version,
            otlp_endpoint,
            enable_console: true,
            log_level,
            resource_attributes: Vec::new(),
            log_sink: None,
        }
    }
}
