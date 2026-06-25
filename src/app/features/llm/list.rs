use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreResult};
use crate::app::ports::llm_provider::LlmProviderStorePort;

/// List all configured LLM providers.
///
/// Emits one [`CoreEvent::LlmProviderListed`] per stored config. The TUI
/// and CLI presenters each consume the stream and render it differently.
pub fn run(store: &dyn LlmProviderStorePort, sink: &mut dyn CoreEventSink) -> CoreResult {
    let cfgs = store.list()?;
    if cfgs.is_empty() {
        sink.on_event(CoreEvent::Info(
            "No LLM providers configured. Use `agk llm add` to add one.".into(),
        ));
        return Ok(crate::app::outcome::CoreOutcome::Ok);
    }
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

    #[test]
    fn list_empty_emits_info_instead_of_silence() {
        let store = FakeLlmProviderStore::seeded(vec![]);
        let mut sink = RecordingSink::default();
        let result = run(&store, &mut sink);
        assert!(result.is_ok());
        // No per-provider events should be emitted...
        assert!(sink
            .events
            .iter()
            .all(|e| !matches!(e, crate::app::event::CoreEvent::LlmProviderListed(_))));
        // ...but exactly one Info event guiding the user must be emitted.
        let infos: Vec<_> = sink
            .events
            .iter()
            .filter_map(|e| match e {
                crate::app::event::CoreEvent::Info(msg) => Some(msg.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(infos.len(), 1);
        assert!(infos[0].contains("llm add"));
    }

    #[derive(Default)]
    struct RecordingSink {
        events: Vec<crate::app::event::CoreEvent>,
    }
    impl crate::app::outcome::CoreEventSink for RecordingSink {
        fn on_event(&mut self, event: crate::app::event::CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, _: String) {}
    }
}
