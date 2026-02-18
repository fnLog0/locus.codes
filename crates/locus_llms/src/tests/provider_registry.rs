use crate::error::Error;
use crate::provider::{Provider, ProviderRegistry};
use crate::types::{GenerateRequest, GenerateResponse, GenerateStream, Headers};
use async_trait::async_trait;

/// Mock provider for testing
struct MockProvider {
    id: &'static str,
}

#[async_trait]
impl Provider for MockProvider {
    fn provider_id(&self) -> &str {
        self.id
    }

    fn build_headers(&self, _custom_headers: Option<&Headers>) -> Headers {
        Headers::new()
    }

    async fn generate(&self, _request: GenerateRequest) -> crate::error::Result<GenerateResponse> {
        Err(Error::Other("mock".to_string()))
    }

    async fn stream(&self, _request: GenerateRequest) -> crate::error::Result<GenerateStream> {
        Err(Error::Other("mock".to_string()))
    }
}

#[test]
fn test_register_and_get_provider() {
    let registry = ProviderRegistry::new()
        .register("test", MockProvider { id: "test" });

    let provider = registry.get_provider("test");
    assert!(provider.is_ok());
    assert_eq!(provider.unwrap().provider_id(), "test");
}

#[test]
fn test_provider_not_found() {
    let registry = ProviderRegistry::new();
    let result = registry.get_provider("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_list_providers() {
    let registry = ProviderRegistry::new()
        .register("alpha", MockProvider { id: "alpha" })
        .register("beta", MockProvider { id: "beta" });

    let mut ids = registry.list_providers();
    ids.sort();
    assert_eq!(ids, vec!["alpha", "beta"]);
}
