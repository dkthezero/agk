use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::llm_provider::LlmProviderStorePort;
use crate::domain::llm_provider::LlmProviderConfig;

/// Add or update an LLM provider configuration.
///
/// Validates the config first (id, endpoint URL, scheme), then upserts it
/// into the persistent store, and finally emits a [`CoreEvent::LlmProviderUpserted`].
pub fn run(
    cfg: LlmProviderConfig,
    store: &dyn LlmProviderStorePort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    cfg.validate()?;
    store.upsert(&cfg)?;
    sink.on_event(CoreEvent::LlmProviderUpserted(cfg));
    Ok(CoreOutcome::Ok)
}
