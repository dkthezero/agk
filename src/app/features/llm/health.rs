use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreResult};
use crate::app::ports::llm_provider::{
    LlmHealthCheckPort, LlmProviderFactoryPort, LlmProviderStorePort,
};
use std::time::Duration;

/// Probe an LLM provider's reachability and report a [`CoreEvent::LlmProviderHealth`].
///
/// This is the only async use case in the LLM feature because the health
/// check performs an HTTP request via [`LlmHealthCheckPort`].
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
        status,
    });
    Ok(crate::app::outcome::CoreOutcome::Ok)
}
