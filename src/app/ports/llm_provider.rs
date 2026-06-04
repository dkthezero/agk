//! Ports for LLM provider management.
//!
//! - [`LlmProviderStorePort`]: persistent store (TOML in config file).
//! - [`LlmProviderFactoryPort`]: produces an `LlmProviderAdapter` for a given
//!   `LlmProviderConfig` so the use-case can call health checks.
//! - [`LlmProviderAdapter`]: provider-specific behaviour (kind, default health URL).
//! - [`LlmHealthCheckPort`]: separate trait so fakes and real HTTP impls can be
//!   swapped in tests without needing the full adapter stack.

use crate::domain::llm_provider::{LlmHealthStatus, LlmProviderConfig, LlmProviderKind};
use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;

pub trait LlmProviderStorePort: Send + Sync {
    fn list(&self) -> Result<Vec<LlmProviderConfig>>;
    fn get(&self, id: &str) -> Result<Option<LlmProviderConfig>>;
    fn upsert(&self, cfg: &LlmProviderConfig) -> Result<()>;
    fn remove(&self, id: &str) -> Result<()>;
}

/// Factory that turns a stored `LlmProviderConfig` into a live adapter for
/// the duration of a health check. Always available (no feature gate on the
/// trait itself) so use-case code can call it from any build.
pub trait LlmProviderFactoryPort: Send + Sync {
    fn build(&self, cfg: &LlmProviderConfig) -> Result<Box<dyn LlmProviderAdapter>>;
}

/// Per-provider adapter: answers what kind it is and what URL/headers to
/// probe. Real impls live in `infra/llm/` and are feature-gated.
pub trait LlmProviderAdapter: Send + Sync {
    fn kind(&self) -> LlmProviderKind;
    /// URL the health check should hit. Implementations should pick the
    /// cheapest call that exercises the server (see spec section 8).
    fn health_url(&self) -> String;
    /// Default model advertised by the server. May be `None` if not known
    /// until the health check runs.
    fn default_model_hint(&self) -> Option<String> {
        None
    }
    /// Optional `(name, value)` for an HTTP header that the health check
    /// should send on every probe (e.g. `Authorization: Bearer …` for
    /// OpenAI, `x-api-key: …` for Anthropic). Providers that do not
    /// require auth (Ollama, LM Studio) return `None`.
    fn auth_header(&self) -> Option<(&'static str, String)> {
        None
    }
}

#[async_trait]
pub trait LlmHealthCheckPort: Send + Sync {
    async fn check(
        &self,
        adapter: &dyn LlmProviderAdapter,
        timeout: Duration,
    ) -> Result<LlmHealthStatus>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm_provider::{LlmProviderConfig, LlmProviderKind};

    struct InMemoryStore {
        items: std::sync::Mutex<Vec<LlmProviderConfig>>,
    }

    impl LlmProviderStorePort for InMemoryStore {
        fn list(&self) -> Result<Vec<LlmProviderConfig>> {
            Ok(self.items.lock().unwrap().clone())
        }
        fn get(&self, id: &str) -> Result<Option<LlmProviderConfig>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|c| c.id == id)
                .cloned())
        }
        fn upsert(&self, cfg: &LlmProviderConfig) -> Result<()> {
            let mut g = self.items.lock().unwrap();
            if let Some(existing) = g.iter_mut().find(|c| c.id == cfg.id) {
                *existing = cfg.clone();
            } else {
                g.push(cfg.clone());
            }
            Ok(())
        }
        fn remove(&self, id: &str) -> Result<()> {
            self.items.lock().unwrap().retain(|c| c.id != id);
            Ok(())
        }
    }

    #[test]
    fn in_memory_store_upsert_replaces() {
        let s = InMemoryStore {
            items: std::sync::Mutex::new(vec![]),
        };
        s.upsert(&LlmProviderConfig {
            id: "a".into(),
            kind: LlmProviderKind::Ollama,
            endpoint: "http://x".into(),
            api_key: None,
            default_model: None,
        })
        .unwrap();
        s.upsert(&LlmProviderConfig {
            id: "a".into(),
            kind: LlmProviderKind::Ollama,
            endpoint: "http://y".into(),
            api_key: None,
            default_model: Some("llama3".into()),
        })
        .unwrap();
        let items = s.list().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].endpoint, "http://y");
    }
}
