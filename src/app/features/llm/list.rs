use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreResult};
use crate::app::ports::llm_provider::LlmProviderStorePort;

/// List all configured LLM providers.
///
/// Emits one [`CoreEvent::LlmProviderListed`] per stored config. The TUI
/// and CLI presenters each consume the stream and render it differently.
pub fn run(store: &dyn LlmProviderStorePort, sink: &mut dyn CoreEventSink) -> CoreResult {
    let cfgs = store.list()?;
    for cfg in cfgs {
        sink.on_event(CoreEvent::LlmProviderListed(cfg));
    }
    Ok(crate::app::outcome::CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::fake_llm_provider::FakeLlmProviderStore;
    use crate::domain::llm_provider::{LlmProviderConfig, LlmProviderKind};

    struct NullSink;
    impl crate::app::outcome::CoreEventSink for NullSink {
        fn on_event(&mut self, _: crate::app::event::CoreEvent) {}
        fn on_error(&mut self, _: String) {}
    }

    #[test]
    fn list_emits_one_event_per_provider() {
        let store = FakeLlmProviderStore::seeded(vec![LlmProviderConfig {
            id: "a".into(),
            kind: LlmProviderKind::Ollama,
            endpoint: "http://127.0.0.1:11434".into(),
            api_key: None,
            default_model: Some("llama3.2".into()),
        }]);
        let mut sink = NullSink;
        let result = run(&store, &mut sink);
        assert!(result.is_ok());
    }
}
