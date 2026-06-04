//! JSON serialization for LLM-provider `CoreEvent`s, split out of
//! `presenter_json.rs` to keep that file within the ADR-001 §6.4 budget.

use crate::app::event::CoreEvent;

/// Serialize an LLM-provider event to JSON. Returns `null` for non-LLM events
/// (`event_to_json` only routes LLM variants here).
pub(crate) fn llm_event_to_json(event: &CoreEvent) -> serde_json::Value {
    match event {
        CoreEvent::LlmProviderListed(cfg) => serde_json::json!({
            "type": "LlmProviderListed",
            "id": cfg.id,
            "kind": cfg.kind.as_str(),
            "endpoint": cfg.endpoint,
            "default_model": cfg.default_model,
        }),
        CoreEvent::LlmProviderUpserted(cfg) => serde_json::json!({
            "type": "LlmProviderUpserted",
            "id": cfg.id,
            "kind": cfg.kind.as_str(),
            "endpoint": cfg.endpoint,
            "default_model": cfg.default_model,
        }),
        CoreEvent::LlmProviderRemoved(id) => {
            serde_json::json!({ "type": "LlmProviderRemoved", "id": id })
        }
        CoreEvent::LlmProviderHealth { id, status } => serde_json::json!({
            "type": "LlmProviderHealth",
            "id": id,
            "reachable": status.reachable,
            "latency_ms": status.latency_ms,
            "models": status.models,
            "error": status.error,
        }),
        _ => serde_json::Value::Null,
    }
}
