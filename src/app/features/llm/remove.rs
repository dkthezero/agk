use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::llm_provider::LlmProviderStorePort;

/// Remove a configured LLM provider by id and emit [`CoreEvent::LlmProviderRemoved`].
pub fn run(id: &str, store: &dyn LlmProviderStorePort, sink: &mut dyn CoreEventSink) -> CoreResult {
    store.remove(id)?;
    sink.on_event(CoreEvent::LlmProviderRemoved(id.into()));
    Ok(CoreOutcome::Ok)
}
