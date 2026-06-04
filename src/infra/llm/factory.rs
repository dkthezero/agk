//! Concrete [`LlmProviderFactoryPort`] implementation that returns a real
//! adapter for the configured provider kind.  Feature-gated so the slim
//! build (no `llm-*` features) does not pull in `reqwest`-dependent
//! adapter crates.

use crate::app::ports::llm_provider::{LlmProviderAdapter, LlmProviderFactoryPort};
use crate::domain::llm_provider::{LlmProviderConfig, LlmProviderKind};
use anyhow::Result;

/// [`LlmProviderFactoryPort`] that dispatches to the per-kind adapter
/// implementation.  Always available (no feature gate on the type itself)
/// — the per-kind adapters live behind their own `#[cfg]` blocks.
pub struct InfraLlmProviderFactory;

impl InfraLlmProviderFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InfraLlmProviderFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmProviderFactoryPort for InfraLlmProviderFactory {
    fn build(&self, cfg: &LlmProviderConfig) -> Result<Box<dyn LlmProviderAdapter>> {
        match cfg.kind {
            #[cfg(feature = "llm-ollama")]
            LlmProviderKind::Ollama => Ok(Box::new(super::ollama::OllamaProvider::new(
                cfg.endpoint.clone(),
            ))),
            #[cfg(feature = "llm-lmstudio")]
            LlmProviderKind::LmStudio => Ok(Box::new(super::lmstudio::LmStudioProvider::new(
                cfg.endpoint.clone(),
            ))),
            #[cfg(feature = "llm-anthropic")]
            LlmProviderKind::Anthropic => Ok(Box::new(super::anthropic::AnthropicProvider::new(
                cfg.endpoint.clone(),
                cfg.api_key.clone(),
            ))),
            #[cfg(feature = "llm-openai")]
            LlmProviderKind::OpenAi => Ok(Box::new(super::openai::OpenAiProvider::new(
                cfg.endpoint.clone(),
                cfg.api_key.clone(),
            ))),
            // Fall back to a generic adapter that passes the endpoint through
            // when the feature is not active.  Health checks will return an
            // error because the `LlmHealthCheckPort` itself is feature-gated
            // to a real HTTP implementation; fakes work fine in tests.
            #[allow(unreachable_patterns)]
            _ => Ok(Box::new(GenericLlmProvider::new_with_key(
                cfg.kind,
                cfg.endpoint.clone(),
                cfg.api_key.clone(),
            ))),
        }
    }
}

/// Generic adapter used when no per-kind feature is active.  It echoes the
/// kind and endpoint unchanged so the store, factory, and health check can
/// be exercised end-to-end in slim builds.
pub struct GenericLlmProvider {
    kind: LlmProviderKind,
    endpoint: String,
    api_key: Option<String>,
}

impl GenericLlmProvider {
    pub fn new(kind: LlmProviderKind, endpoint: impl Into<String>) -> Self {
        Self {
            kind,
            endpoint: endpoint.into(),
            api_key: None,
        }
    }

    /// Constructor that also plumbs an API key so the generic adapter can
    /// emit auth headers the same way the real per-kind adapters do.
    pub fn new_with_key(
        kind: LlmProviderKind,
        endpoint: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            kind,
            endpoint: endpoint.into(),
            api_key,
        }
    }
}

impl LlmProviderAdapter for GenericLlmProvider {
    fn kind(&self) -> LlmProviderKind {
        self.kind
    }
    fn health_url(&self) -> String {
        let trimmed = self.endpoint.trim_end_matches('/');
        match self.kind {
            LlmProviderKind::Ollama => format!("{trimmed}/api/tags"),
            LlmProviderKind::LmStudio | LlmProviderKind::OpenAi => {
                format!("{trimmed}/v1/models")
            }
            LlmProviderKind::Anthropic => trimmed.to_string(),
        }
    }
    fn auth_header(&self) -> Option<(&'static str, String)> {
        match (self.kind, self.api_key.as_deref()) {
            (LlmProviderKind::Anthropic, Some(k)) => Some(("x-api-key", k.to_string())),
            (LlmProviderKind::OpenAi, Some(k)) => Some(("Authorization", format!("Bearer {k}"))),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm_provider::LlmProviderConfig;

    fn cfg(kind: LlmProviderKind, endpoint: &str) -> LlmProviderConfig {
        LlmProviderConfig {
            id: "id".into(),
            kind,
            endpoint: endpoint.into(),
            api_key: None,
            default_model: None,
        }
    }

    #[test]
    fn factory_returns_adapter_with_matching_kind() {
        let f = InfraLlmProviderFactory::new();
        let a = f.build(&cfg(LlmProviderKind::Ollama, "http://x")).unwrap();
        assert_eq!(a.kind(), LlmProviderKind::Ollama);
    }

    #[test]
    fn generic_provider_builds_health_url_per_kind() {
        let p = GenericLlmProvider::new(LlmProviderKind::Ollama, "http://h/");
        assert_eq!(p.health_url(), "http://h/api/tags");
        let p = GenericLlmProvider::new(LlmProviderKind::LmStudio, "http://h");
        assert_eq!(p.health_url(), "http://h/v1/models");
        let p = GenericLlmProvider::new(LlmProviderKind::Anthropic, "http://h/");
        assert_eq!(p.health_url(), "http://h");
    }
}
