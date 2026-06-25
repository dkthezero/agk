use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::llm_provider::LlmProviderStorePort;
use anyhow::anyhow;

/// Remove a configured LLM provider by id and emit [`CoreEvent::LlmProviderRemoved`].
///
/// If no provider with the given id exists, surface a clear error instead of
/// silently reporting success — a no-op "removed" message is misleading UX.
/// We return `Err` (rather than calling `sink.on_error`) so the caller renders
/// the error exactly once; this mirrors the `health::run` "not configured"
/// path and avoids double-reporting on the CLI.
pub fn run(id: &str, store: &dyn LlmProviderStorePort, sink: &mut dyn CoreEventSink) -> CoreResult {
    let existing = store.get(id)?;
    if existing.is_none() {
        return Err(anyhow!("LLM provider '{}' not configured", id));
    }
    store.remove(id)?;
    sink.on_event(CoreEvent::LlmProviderRemoved(id.into()));
    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::fake_llm_provider::FakeLlmProviderStore;
    use crate::domain::llm_provider::{LlmProviderConfig, LlmProviderKind};

    #[derive(Default)]
    struct RecordingSink {
        events: Vec<crate::app::event::CoreEvent>,
        errors: Vec<String>,
    }
    impl crate::app::outcome::CoreEventSink for RecordingSink {
        fn on_event(&mut self, event: crate::app::event::CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, error: String) {
            self.errors.push(error);
        }
    }

    fn cfg(id: &str) -> LlmProviderConfig {
        LlmProviderConfig {
            id: id.into(),
            kind: LlmProviderKind::Ollama,
            endpoint: "http://127.0.0.1:11434".into(),
            api_key: None,
            default_model: None,
        }
    }

    #[test]
    fn remove_existing_emits_removed_event() {
        let store = FakeLlmProviderStore::seeded(vec![cfg("a")]);
        let mut sink = RecordingSink::default();
        let result = run("a", &store, &mut sink);
        assert!(result.is_ok());
        assert!(sink
            .events
            .iter()
            .any(|e| matches!(e, crate::app::event::CoreEvent::LlmProviderRemoved(_))));
        assert!(sink.errors.is_empty());
    }

    #[test]
    fn remove_missing_returns_error_and_emits_nothing() {
        let store = FakeLlmProviderStore::new();
        let mut sink = RecordingSink::default();
        let result = run("ghost", &store, &mut sink);
        let err = result.expect_err("expected error for missing provider");
        assert!(err.to_string().contains("ghost"));
        // No events and no on_error should be emitted — the error is
        // surfaced solely via the `Result` so the caller prints it once.
        assert!(sink.events.is_empty());
        assert!(sink.errors.is_empty());
    }
}
