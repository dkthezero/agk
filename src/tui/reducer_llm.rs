//! Status-line rendering for LLM-provider `CoreEvent`s, split out of
//! `core_event_reducer.rs` to keep that file within the ADR-001 §6.4 budget.

use crate::app::event::CoreEvent;

/// Produce the status-line string for an LLM-provider event. Returns an empty
/// string for non-LLM events (the reducer only routes LLM variants here).
pub(crate) fn llm_status_line(event: &CoreEvent) -> String {
    match event {
        CoreEvent::LlmProviderListed(cfg) => {
            format!("LLM provider '{}' listed ({})", cfg.id, cfg.kind.as_str())
        }
        CoreEvent::LlmProviderUpserted(cfg) => {
            format!("LLM provider '{}' saved ({})", cfg.id, cfg.kind.as_str())
        }
        CoreEvent::LlmProviderRemoved(id) => format!("LLM provider '{}' removed", id),
        CoreEvent::LlmProviderHealth { id, status } => {
            let latency = status
                .latency_ms
                .map(|ms| format!(" ({} ms)", ms))
                .unwrap_or_default();
            let detail = if status.reachable {
                format!("reachable{}", latency)
            } else {
                format!(
                    "unreachable: {}",
                    status.error.as_deref().unwrap_or("unknown")
                )
            };
            format!("LLM provider '{}' health: {}", id, detail)
        }
        _ => String::new(),
    }
}
