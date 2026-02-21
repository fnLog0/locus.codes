//! LocusGraph Observability - Reusable observability utilities for distributed tracing
//!
//! This crate provides OpenTelemetry tracing setup and utilities that can be
//! used across different LocusGraph services (server, agents, etc.).
//!
//! # Features
//!
//! - OpenTelemetry integration with OTLP export
//! - Configurable via environment variables or programmatic API
//! - Reusable tracing utilities (HTTP, agent, storage spans)
//! - Console logging integration
//!
//! # Quick Start
//!
//! ```no_run
//! use locusgraph_observability::{ ObservabilityConfig, init };
//!
//! // Initialize with configuration
//! let config = ObservabilityConfig::new("my-service")
//!     .with_otlp_endpoint("http://localhost:4317")
//!     .with_log_level("info");
//!
//! init(config)?;
//!
//! // Or initialize from environment variables
//! locusgraph_observability::init_from_env()?;
//!
//! // Use tracing as usual
//! tracing::info!("Service started");
//! ```
//!
//! # Environment Variables
//!
//! - `OTEL_SERVICE_NAME` or `SERVICE_NAME` - Service name
//! - `OTEL_SERVICE_VERSION` or `SERVICE_VERSION` - Service version
//! - `OTEL_EXPORTER_OTLP_ENDPOINT` or `TEMPO_ENDPOINT` - OTLP endpoint
//! - `OTEL_LOG_LEVEL` or `RUST_LOG` - Log level filter
//!
//! # Examples
//!
//! See the examples directory for usage examples.

pub mod config;
pub mod error;
pub mod telemetry;
pub mod tui_log_layer;
pub mod tracing;

pub use config::ObservabilityConfig;
pub use error::ObservabilityError;
pub use telemetry::{init, init_from_env, shutdown};
pub use tracing::{record_duration, record_error};

// Macros are automatically exported via #[macro_export] and available
// as locusgraph_observability::agent_span!(), etc.
