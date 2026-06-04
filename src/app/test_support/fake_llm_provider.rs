//! Fake LLM provider ports for testing.
//!
//! Hand-rolled in-memory fakes (no mocking libraries) that simulate the
//! persistent store, factory, adapter, and health-check layers used by
//! the LLM provider management use-cases.

use crate::app::ports::llm_provider::{
    LlmHealthCheckPort, LlmProviderAdapter, LlmProviderFactoryPort, LlmProviderStorePort,
};
use crate::domain::llm_provider::{LlmHealthStatus, LlmProviderConfig, LlmProviderKind};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

/// In-memory [`LlmProviderStorePort`] backed by a `HashMap` keyed on
/// `LlmProviderConfig::id`.
pub struct FakeLlmProviderStore {
    pub items: Mutex<HashMap<String, LlmProviderConfig>>,
}

impl FakeLlmProviderStore {
    pub fn new() -> Self {
        Self {
            items: Mutex::new(HashMap::new()),
        }
    }

    /// Build a store pre-populated with the given configs (keyed by `id`).
    pub fn seeded(cfgs: Vec<LlmProviderConfig>) -> Self {
        let m: HashMap<_, _> = cfgs.into_iter().map(|c| (c.id.clone(), c)).collect();
        Self {
            items: Mutex::new(m),
        }
    }
}

impl Default for FakeLlmProviderStore {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmProviderStorePort for FakeLlmProviderStore {
    fn list(&self) -> Result<Vec<LlmProviderConfig>> {
        Ok(self.items.lock().unwrap().values().cloned().collect())
    }

    fn get(&self, id: &str) -> Result<Option<LlmProviderConfig>> {
        Ok(self.items.lock().unwrap().get(id).cloned())
    }

    fn upsert(&self, cfg: &LlmProviderConfig) -> Result<()> {
        self.items
            .lock()
            .unwrap()
            .insert(cfg.id.clone(), cfg.clone());
        Ok(())
    }

    fn remove(&self, id: &str) -> Result<()> {
        self.items.lock().unwrap().remove(id);
        Ok(())
    }
}

/// Trivial [`LlmProviderAdapter`] used by [`FakeLlmProviderFactory`].
pub struct FakeAdapter {
    pub kind: LlmProviderKind,
    pub url: String,
}

impl LlmProviderAdapter for FakeAdapter {
    fn kind(&self) -> LlmProviderKind {
        self.kind
    }

    fn health_url(&self) -> String {
        self.url.clone()
    }
}

/// [`LlmProviderFactoryPort`] that returns a [`FakeAdapter`] with a
/// per-kind health URL derived from `cfg.endpoint`.
pub struct FakeLlmProviderFactory;

impl LlmProviderFactoryPort for FakeLlmProviderFactory {
    fn build(&self, cfg: &LlmProviderConfig) -> Result<Box<dyn LlmProviderAdapter>> {
        let trimmed = cfg.endpoint.trim_end_matches('/');
        let url = match cfg.kind {
            LlmProviderKind::Ollama => format!("{trimmed}/api/tags"),
            LlmProviderKind::LmStudio => format!("{trimmed}/v1/models"),
            // Anthropic: use the messages endpoint with an OPTIONS preflight.
            LlmProviderKind::Anthropic => cfg.endpoint.clone(),
            LlmProviderKind::OpenAi => format!("{trimmed}/v1/models"),
        };
        Ok(Box::new(FakeAdapter {
            kind: cfg.kind,
            url,
        }))
    }
}

/// Configurable fake [`LlmHealthCheckPort`].
///
/// Defaults to "reachable, 12ms latency, one model". Override the public
/// fields to test failure paths.
pub struct FakeLlmHealthCheck {
    pub reachable: bool,
    pub latency_ms: u64,
    pub models: Vec<String>,
    pub error: Option<String>,
}

impl Default for FakeLlmHealthCheck {
    fn default() -> Self {
        Self {
            reachable: true,
            latency_ms: 12,
            models: vec!["llama3.2".into()],
            error: None,
        }
    }
}

#[async_trait]
impl LlmHealthCheckPort for FakeLlmHealthCheck {
    async fn check(
        &self,
        _adapter: &dyn LlmProviderAdapter,
        _timeout: Duration,
    ) -> Result<LlmHealthStatus> {
        Ok(LlmHealthStatus {
            reachable: self.reachable,
            latency_ms: if self.reachable {
                Some(self.latency_ms)
            } else {
                None
            },
            models: if self.reachable {
                self.models.clone()
            } else {
                vec![]
            },
            error: self.error.clone(),
        })
    }
}
