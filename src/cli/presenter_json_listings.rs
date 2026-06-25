//! JSON serialization for list-type `CoreEvent`s, split out of
//! `presenter_json.rs` to keep that file within the ADR-001 §6.4 budget.

use crate::app::event::CoreEvent;

/// Serialize a list-type event to JSON. Returns `null` for non-list events
/// (`event_to_json` only routes list variants here).
pub(crate) fn listing_event_to_json(event: &CoreEvent) -> serde_json::Value {
    match event {
        CoreEvent::ProfileListed(entries) => {
            serde_json::json!({
                "type": "ProfileListed",
                "profiles": entries.iter().map(|e| {
                    serde_json::json!({
                        "name": e.name,
                        "provider_id": e.provider_id,
                        "skills": e.skills.iter().map(|s| &s.name).collect::<Vec<_>>(),
                        "mcps": e.mcps.iter().map(|m| &m.name).collect::<Vec<_>>(),
                        "has_drift": e.has_drift,
                    })
                }).collect::<Vec<_>>()
            })
        }
        CoreEvent::ContextListed(entries) => {
            serde_json::json!({
                "type": "ContextListed",
                "contexts": entries.iter().map(|e| {
                    serde_json::json!({
                        "name": e.name,
                        "display_name": e.display_name,
                        "is_active": e.is_active,
                        "environment": e.environment,
                        "vaults": e.vaults,
                        "profiles": e.profiles,
                        "providers": e.providers,
                    })
                }).collect::<Vec<_>>()
            })
        }
        _ => serde_json::Value::Null,
    }
}
