use crate::app::ports::llm_provider::{LlmHealthCheckPort, LlmProviderAdapter};
use crate::domain::llm_provider::{LlmHealthStatus, LlmProviderKind};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Method};
use std::str::FromStr;
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

    /// Build the header map the health check should send. Combines any
    /// per-provider auth header (e.g. `Authorization: Bearer …` for OpenAI,
    /// `x-api-key: …` for Anthropic) with the optional Anthropic
    /// `anthropic-version: 2023-06-01` so the `/v1/messages` OPTIONS
    /// preflight is realistic. Providers without auth (Ollama, LM Studio)
    /// get an empty map.
    fn build_headers(adapter: &dyn LlmProviderAdapter) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        if let Some((name, value)) = adapter.auth_header() {
            let header_name = HeaderName::from_str(name)
                .with_context(|| format!("invalid auth header name: {name}"))?;
            let header_value = HeaderValue::from_str(&value)
                .with_context(|| format!("invalid auth header value for {name}"))?;
            headers.insert(header_name, header_value);
        }
        if matches!(adapter.kind(), LlmProviderKind::Anthropic) {
            // Anthropic recommends the version header for `/v1/messages`
            // and accepts it on the OPTIONS preflight. Without it some
            // edge proxies reject the request.
            headers.insert(
                HeaderName::from_static("anthropic-version"),
                HeaderValue::from_static("2023-06-01"),
            );
        }
        Ok(headers)
    }

    /// Issue a single HTTP probe and translate the result into an
    /// [`LlmHealthStatus`]. Network errors collapse into
    /// `reachable=false, error=<message>` so the caller doesn't have to
    /// distinguish transport failure from a non-2xx response.
    async fn probe(
        &self,
        method: Method,
        url: &str,
        headers: &HeaderMap,
        timeout: Duration,
    ) -> LlmHealthStatus {
        let start = Instant::now();
        let req = match self
            .client
            .request(method, url)
            .timeout(timeout)
            .headers(headers.clone())
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                return LlmHealthStatus {
                    reachable: false,
                    latency_ms: None,
                    models: vec![],
                    error: Some(e.to_string()),
                }
            }
        };
        let latency = start.elapsed().as_millis() as u64;
        match self.client.execute(req).await {
            Ok(resp) if resp.status().is_success() => LlmHealthStatus {
                reachable: true,
                latency_ms: Some(latency),
                models: vec![],
                error: None,
            },
            Ok(resp) => LlmHealthStatus {
                reachable: false,
                latency_ms: Some(latency),
                models: vec![],
                error: Some(format!("HTTP {}", resp.status())),
            },
            Err(e) => LlmHealthStatus {
                reachable: false,
                latency_ms: None,
                models: vec![],
                error: Some(e.to_string()),
            },
        }
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
        let headers = Self::build_headers(adapter)?;

        // Anthropic uses an `OPTIONS` preflight on `/v1/messages` so we do
        // not consume API quota. If the preflight is blocked by a proxy
        // (i.e. returns 4xx — typically 403/405), fall back to a `GET /`
        // on the configured endpoint. Anthropic returns 404 on the root,
        // but that's still proof the server is responding (per spec 8.4).
        // The fallback is gated on Anthropic only — other providers get a
        // single probe.
        if matches!(adapter.kind(), LlmProviderKind::Anthropic) {
            let preflight = self
                .probe(Method::OPTIONS, &url, &headers, timeout)
                .await;
            match &preflight {
                LlmHealthStatus {
                    reachable: true, ..
                } => return Ok(preflight),
                LlmHealthStatus {
                    error: Some(err), ..
                } if err.starts_with("HTTP 4") => {
                    // 4xx on OPTIONS → proxy/edge is blocking preflight.
                    // Retry with `GET /` (the endpoint root, already
                    // stripped of trailing slashes by `health_url`).
                    // Per spec 8.4: any 4xx on the fallback is "reachable,
                    // server is responding, just doesn't allow OPTIONS";
                    // only 5xx is "unreachable".
                    let mut fallback = self
                        .probe(Method::GET, &url, &headers, timeout)
                        .await;
                    if let Some(err) = &fallback.error {
                        if err.starts_with("HTTP 4") {
                            fallback.reachable = true;
                            // Replace the raw "HTTP 4xx" error with a
                            // user-friendly note that the OPTIONS preflight
                            // was blocked but the server is reachable.
                            fallback.error = Some(format!(
                                "Anthropic OPTIONS preflight blocked; server reachable (root returned {err})"
                            ));
                        }
                    }
                    return Ok(fallback);
                }
                _ => return Ok(preflight),
            }
        }

        // Default: single GET probe.
        let method = Method::GET;
        Ok(self.probe(method, &url, &headers, timeout).await)
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

    #[test]
    fn build_headers_omits_auth_for_providers_without_key() {
        // StubAdapter (Ollama) has no `auth_header` impl → empty.
        let headers = HttpLlmHealthCheck::build_headers(&StubAdapter).unwrap();
        assert!(headers.is_empty());
    }

    struct AnthropicAdapterNoKey;
    impl LlmProviderAdapter for AnthropicAdapterNoKey {
        fn kind(&self) -> LlmProviderKind {
            LlmProviderKind::Anthropic
        }
        fn health_url(&self) -> String {
            "https://api.anthropic.com".into()
        }
    }
    struct AnthropicAdapterWithKey;
    impl LlmProviderAdapter for AnthropicAdapterWithKey {
        fn kind(&self) -> LlmProviderKind {
            LlmProviderKind::Anthropic
        }
        fn health_url(&self) -> String {
            "https://api.anthropic.com".into()
        }
        fn auth_header(&self) -> Option<(&'static str, String)> {
            Some(("x-api-key", "sk-test".to_string()))
        }
    }

    #[test]
    fn build_headers_for_anthropic_with_key_includes_api_key_and_version() {
        let headers =
            HttpLlmHealthCheck::build_headers(&AnthropicAdapterWithKey).unwrap();
        assert_eq!(headers.get("x-api-key").unwrap(), "sk-test");
        assert_eq!(headers.get("anthropic-version").unwrap(), "2023-06-01");
    }

    #[test]
    fn build_headers_for_anthropic_without_key_still_has_version() {
        let headers =
            HttpLlmHealthCheck::build_headers(&AnthropicAdapterNoKey).unwrap();
        assert!(headers.get("x-api-key").is_none());
        assert_eq!(headers.get("anthropic-version").unwrap(), "2023-06-01");
    }
}
