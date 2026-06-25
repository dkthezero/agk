//! Command-adapter helper for `CoreCommand::UpdateAsset`.
//!
//! Split out of `dispatch_helpers.rs` so that file stays under the
//! 300-LOC ADR-001 §6.4 limit.

use crate::app::core::AgkCore;
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::domain::scope::Scope;

pub(super) fn update_asset_cmd(
    identity: &str,
    scope: Scope,
    provider_filter: Option<&str>,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    sink.on_event(CoreEvent::TaskStarted {
        id: 0,
        name: format!("Updating '{}'", identity),
    });

    let config = match core.store.load(scope) {
        Ok(c) => c,
        Err(e) => {
            sink.on_event(CoreEvent::TaskFailed {
                id: 0,
                error: format!("Failed to load config: {}", e),
            });
            return Ok(CoreOutcome::Ok);
        }
    };

    let mut providers = core.registry.active_providers_from_config(&config);
    if let Some(filter) = provider_filter {
        providers.retain(|p| p.id() == filter);
    }
    if providers.is_empty() {
        sink.on_event(CoreEvent::TaskFailed {
            id: 0,
            error: "No active providers".into(),
        });
        return Ok(CoreOutcome::Ok);
    }

    let pkg = match core.registry.find_package_by_identity(identity) {
        Ok(Some(p)) => p,
        Ok(None) => {
            sink.on_event(CoreEvent::TaskFailed {
                id: 0,
                error: format!("Asset '{}' not found", identity),
            });
            return Ok(CoreOutcome::Ok);
        }
        Err(e) => {
            sink.on_event(CoreEvent::TaskFailed {
                id: 0,
                error: format!("Lookup failed: {}", e),
            });
            return Ok(CoreOutcome::Ok);
        }
    };

    let mut any_failed = false;
    for provider in providers {
        if let Err(e) = super::update::update_asset(scope, &pkg, core.store.as_ref(), provider) {
            sink.on_event(CoreEvent::Error(format!(
                "Provider {}: {}",
                provider.id(),
                e
            )));
            any_failed = true;
        }
    }

    if !any_failed {
        sink.on_event(CoreEvent::AssetUpdated {
            identity: pkg.identity.name.clone(),
        });
    } else {
        sink.on_event(CoreEvent::TaskFailed {
            id: 0,
            error: format!("Failed to update '{}'", identity),
        });
    }
    Ok(CoreOutcome::Ok)
}
