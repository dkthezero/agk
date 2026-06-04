use crate::app::ports::llm_provider::{LlmHealthCheckPort, LlmProviderAdapter};
use crate::domain::llm_provider::{LlmHealthStatus, LlmProviderKind};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use reqwest::{Client, Method};
use std::time::{Duration, Instant};

pub struct HttpLlmHealthCheck {
    pub client: Client,
}

impl HttpLlmHealthCheck {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("reqwest client");
        Self { client }
    }
}

impl Default for HttpLlmHealthCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmHealthCheckPort for HttpLlmHealthCheck {
    async fn check(
        &self,
        adapter: &dyn LlmProviderAdapter,
        timeout: Duration,
    ) -> Result<LlmHealthStatus> {
        let url = adapter.health_url();
        let method = match adapter.kind() {
            LlmProviderKind::Anthropic => Method::OPTIONS,
            _ => Method::GET,
        };
        let _headers = HeaderMap::new();
        let start = Instant::now();
        let req = self
            .client
            .request(method.clone(), &url)
            .timeout(timeout)
            .headers(HeaderMap::new())
            .build()?;
        let result = self.client.execute(req).await;
        let latency = start.elapsed().as_millis() as u64;
        match result {
            Ok(resp) if resp.status().is_success() => Ok(LlmHealthStatus {
                reachable: true,
                latency_ms: Some(latency),
                models: vec![],
                error: None,
            }),
            Ok(resp) => Ok(LlmHealthStatus {
                reachable: false,
                latency_ms: Some(latency),
                models: vec![],
                error: Some(format!("HTTP {}", resp.status())),
            }),
            Err(e) => Ok(LlmHealthStatus {
                reachable: false,
                latency_ms: None,
                models: vec![],
                error: Some(e.to_string()),
            }),
        }
    }
}

#[cfg(all(test, feature = "llm-ollama"))]
mod tests {
    use super::*;
    use crate::app::ports::llm_provider::{LlmHealthCheckPort, LlmProviderAdapter};
    use crate::domain::llm_provider::LlmProviderKind;
    use std::time::Duration;

    struct StubAdapter;
    impl LlmProviderAdapter for StubAdapter {
        fn kind(&self) -> LlmProviderKind {
            LlmProviderKind::Ollama
        }
        fn health_url(&self) -> String {
            "http://127.0.0.1:1/api/tags".into()
        } // unreachable
    }

    #[tokio::test]
    async fn health_check_marks_unreachable_when_refused() {
        let hc = HttpLlmHealthCheck::new();
        let status = hc
            .check(&StubAdapter, Duration::from_millis(500))
            .await
            .unwrap();
        assert!(!status.reachable);
        assert!(status.error.is_some());
    }
}
