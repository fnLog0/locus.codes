//! locus_llms — Provider-agnostic AI completions SDK with streaming support.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────┐
//! │                  ProviderRegistry                    │
//! │  ┌──────────────────────────────────────────────┐   │
//! │  │  HashMap<String, Arc<dyn Provider>>           │   │
//! │  └──────────────────────────────────────────────┘   │
//! │                       │                              │
//! │          ┌────────────┼────────────┐                │
//! │          ▼            ▼            ▼                │
//! │   ┌───────────┐ ┌──────────┐ ┌──────────┐         │
//! │   │ Anthropic  │ │  Z.AI    │ │ (future) │         │
//! │   │ Provider   │ │ Provider │ │          │         │
//! │   └───────────┘ └──────────┘ └──────────┘         │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use locus_llms::{Provider, ProviderRegistry, AnthropicProvider};
//! use locus_llms::providers::anthropic::AnthropicConfig;
//!
//! let provider = AnthropicProvider::from_env().unwrap();
//! let registry = ProviderRegistry::new()
//!     .register("anthropic", provider);
//! ```

pub mod error;
pub mod provider;
pub mod providers;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export core abstractions
pub use error::{Error, Result};
pub use provider::{Provider, ProviderRegistry};

// Re-export provider implementations
pub use providers::AnthropicProvider;
pub use providers::ZaiProvider;

// Re-export commonly used types
pub use types::{
    GenerateRequest, GenerateResponse, GenerateStream, Message, Role, StreamEvent,
};
