//! Provider trait and registry

mod trait_def;

pub use trait_def::Provider;

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{Error, Result};

/// Registry of provider implementations, keyed by provider ID.
#[derive(Default, Clone)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a provider under the given ID. Returns `self` for chaining.
    pub fn register<P: Provider + 'static>(mut self, id: impl Into<String>, provider: P) -> Self {
        self.providers.insert(id.into(), Arc::new(provider));
        self
    }

    /// Look up a provider by ID.
    pub fn get_provider(&self, id: &str) -> Result<Arc<dyn Provider>> {
        self.providers
            .get(id)
            .cloned()
            .ok_or_else(|| Error::ProviderNotFound(id.to_string()))
    }

    /// List all registered provider IDs.
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}
