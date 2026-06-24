use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::llm_provider::{
    LlmHealthCheckPort, LlmProviderFactoryPort, LlmProviderStorePort,
};
use std::time::Duration;

/// Probe an LLM provider's reachability and report a [`CoreEvent::LlmProviderHealth`].
///
/// Emits the `LlmProviderHealth` event (carrying the reachability status +
/// models/error) for renderers, and returns `Err` when the probe reported
/// the provider unreachable so the CLI dispatcher maps it to a non-zero
/// exit code.  Returning `Ok(CoreOutcome::Ok)` on an unreachable probe
/// would make `agk llm health <id>` exit 0 despite printing "<id>
/// unreachable: ..." — a false-success (the `TaskFailed`-then-`Ok`
/// anti-pattern documented in AGENTS.md).
pub async fn run(
    id: &str,
    store: &dyn LlmProviderStorePort,
    factory: &dyn LlmProviderFactoryPort,
    health: &dyn LlmHealthCheckPort,
    timeout: Duration,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let cfg = store
        .get(id)?
        .ok_or_else(|| anyhow::anyhow!("LLM provider '{}' not configured", id))?;
    let adapter = factory.build(&cfg)?;
    let status = health.check(adapter.as_ref(), timeout).await?;
    sink.on_event(CoreEvent::LlmProviderHealth {
        id: id.into(),
        status: status.clone(),
    });
    if status.reachable {
        Ok(CoreOutcome::Ok)
    } else {
        let reason = status
            .error
            .clone()
            .unwrap_or_else(|| "unknown error".to_string());
        Err(anyhow::anyhow!(
            "LLM provider '{}' unreachable: {}",
            id,
            reason
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::outcome::NullSink;
    use crate::app::test_support::collecting_sink::CollectingSink;
    use crate::app::test_support::fake_llm_provider::{
        FakeLlmHealthCheck, FakeLlmProviderFactory, FakeLlmProviderStore,
    };
    use crate::domain::llm_provider::{LlmProviderConfig, LlmProviderKind};

    fn cfg(id: &str) -> LlmProviderConfig {
        LlmProviderConfig {
            id: id.into(),
            kind: LlmProviderKind::Ollama,
            endpoint: "http://127.0.0.1:11434".into(),
            api_key: None,
            default_model: Some("llama3.2".into()),
        }
    }

    #[tokio::test]
    async fn run_reachable_emits_event_and_returns_ok() {
        let mut sink = CollectingSink::new();
        let store = FakeLlmProviderStore::seeded(vec![cfg("local")]);
        let factory = FakeLlmProviderFactory;
        let health = FakeLlmHealthCheck::default();
        let result = run(
            "local",
            &store,
            &factory,
            &health,
            Duration::from_secs(1),
            &mut sink,
        )
        .await;
        assert!(result.is_ok(), "reachable probe must return Ok");
        assert_eq!(sink.events.len(), 1);
        match &sink.events[0] {
            CoreEvent::LlmProviderHealth { id, status } => {
                assert_eq!(id, "local");
                assert!(status.reachable);
            }
            other => panic!("expected LlmProviderHealth, got {:?}", other),
        }
    }

    /// Regression: `llm health <id>` must return `Err` (so the CLI exits
    /// non-zero) when the probe reports the provider unreachable, while
    /// still emitting the `LlmProviderHealth { reachable: false }` event
    /// for renderers.  Previously it returned `Ok(CoreOutcome::Ok)`,
    /// making `agk llm health <id>` exit 0 despite printing
    /// "<id> unreachable: ..." — a false success (the `TaskFailed`-then-`Ok`
    /// anti-pattern documented in AGENTS.md).
    #[tokio::test]
    async fn run_unreachable_emits_event_and_returns_err() {
        let mut sink = CollectingSink::new();
        let store = FakeLlmProviderStore::seeded(vec![cfg("dead")]);
        let factory = FakeLlmProviderFactory;
        let health = FakeLlmHealthCheck {
            reachable: false,
            error: Some("connection refused".into()),
            ..FakeLlmHealthCheck::default()
        };
        let result = run(
            "dead",
            &store,
            &factory,
            &health,
            Duration::from_secs(1),
            &mut sink,
        )
        .await;
        assert!(result.is_err(), "unreachable probe must return Err");
        assert_eq!(
            sink.events.len(),
            1,
            "the unreachable event must still emit for renderers"
        );
        match &sink.events[0] {
            CoreEvent::LlmProviderHealth { id, status } => {
                assert_eq!(id, "dead");
                assert!(!status.reachable);
                assert_eq!(status.error.as_deref(), Some("connection refused"));
            }
            other => panic!("expected LlmProviderHealth, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn run_unreachable_with_null_sink_still_returns_err() {
        let mut sink = NullSink;
        let store = FakeLlmProviderStore::seeded(vec![cfg("dead")]);
        let factory = FakeLlmProviderFactory;
        let health = FakeLlmHealthCheck {
            reachable: false,
            ..FakeLlmHealthCheck::default()
        };
        assert!(run(
            "dead",
            &store,
            &factory,
            &health,
            Duration::from_secs(1),
            &mut sink
        )
        .await
        .is_err());
    }

    #[tokio::test]
    async fn run_missing_provider_returns_err_without_event() {
        let mut sink = CollectingSink::new();
        let store = FakeLlmProviderStore::new();
        let factory = FakeLlmProviderFactory;
        let health = FakeLlmHealthCheck::default();
        let result = run(
            "ghost",
            &store,
            &factory,
            &health,
            Duration::from_secs(1),
            &mut sink,
        )
        .await;
        assert!(result.is_err(), "missing provider must return Err");
        assert!(
            sink.events.is_empty(),
            "no event should emit for a missing provider"
        );
    }
}
