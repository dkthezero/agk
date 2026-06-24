//! Private command-adapter helpers for `asset::dispatch`.
//!
//! Each `*_cmd` function converts the parsed `CoreCommand` payload into a
//! call to the matching use-case in `install.rs` / `remove.rs` /
//! `update.rs` / `sync.rs`, fans out per provider, and emits the
//! corresponding `CoreEvent` sequence.
//!
//! Extracted from `mod.rs` so the dispatcher itself stays under the
//! 300-LOC ADR-001 §6.4 limit.

use super::vault_dispatch;
use crate::app::core::AgkCore;
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::domain::scope::Scope;

/// Emit a `TaskFailed` event (so the TUI clears its spinner and the
/// human-readable `[0] Failed:` line renders) AND return an `Err` carrying
/// the same message, so the CLI dispatcher maps the failure to a non-zero
/// exit code.
///
/// This replaces the previous `TaskFailed`-then-`Ok(CoreOutcome::Ok)`
/// anti-pattern (documented in AGENTS.md) which printed a failure message
/// but exited 0.
fn fail(sink: &mut dyn CoreEventSink, error: impl Into<String>) -> CoreResult {
    let error = error.into();
    sink.on_event(CoreEvent::TaskFailed {
        id: 0,
        error: error.clone(),
    });
    Err(anyhow::anyhow!(error))
}

pub(super) fn install_asset_cmd(
    identity: &str,
    scope: Scope,
    provider_filter: Option<&str>,
    include_evals: bool,
    dry_run: bool,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    sink.on_event(CoreEvent::TaskStarted {
        id: 0,
        name: format!("Installing '{}'", identity),
    });

    if vault_dispatch::is_vault_workspace(core) {
        return vault_dispatch::install_asset(identity, core, sink);
    }

    let config = match core.store.load(scope) {
        Ok(c) => c,
        Err(e) => return fail(sink, format!("Failed to load config: {}", e)),
    };

    let mut providers = core.registry.active_providers_from_config(&config);
    if let Some(filter) = provider_filter {
        providers.retain(|p| p.id() == filter);
    }
    if providers.is_empty() {
        return fail(sink, "No active providers");
    }

    let pkg = match core.registry.find_package_by_identity(identity) {
        Ok(Some(p)) => p,
        Ok(None) => {
            // Attempt remote fetch via ClawHub for simple slugs.
            if !identity.contains('/') {
                if let Err(e) = core.clawhub.cli_install(identity) {
                    return fail(sink, format!("Fetch failed: {}", e));
                }
                match core.registry.find_package_by_identity(identity) {
                    Ok(Some(p)) => p,
                    _ => {
                        return fail(
                            sink,
                            format!("Asset '{}' not found after remote fetch", identity),
                        );
                    }
                }
            } else {
                return fail(sink, format!("Asset '{}' not found in any vault", identity));
            }
        }
        Err(e) => return fail(sink, format!("Lookup failed: {}", e)),
    };

    if dry_run {
        sink.on_event(CoreEvent::Info(format!(
            "Dry run: would install '{}' to {} provider(s)",
            pkg.identity.name,
            providers.len()
        )));
        return Ok(CoreOutcome::Ok);
    }

    let mut installed_providers = Vec::new();
    let mut any_failed = false;
    let mut pkg = pkg;
    pkg.include_evals = include_evals;
    for provider in providers {
        if let Err(e) = super::install::install_asset(scope, &pkg, core.store.as_ref(), provider) {
            sink.on_event(CoreEvent::Error(format!(
                "Provider {}: {}",
                provider.id(),
                e
            )));
            any_failed = true;
        } else {
            installed_providers.push(provider.id().to_string());
        }
    }

    if !installed_providers.is_empty() && !any_failed {
        sink.on_event(CoreEvent::AssetInstalled {
            identity: pkg.identity.name.clone(),
            providers: installed_providers,
        });
        Ok(CoreOutcome::Ok)
    } else {
        fail(sink, format!("Failed to install '{}'", identity))
    }
}

pub(super) fn remove_asset_cmd(
    identity: &str,
    scope: Scope,
    provider_filter: Option<&str>,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    sink.on_event(CoreEvent::TaskStarted {
        id: 0,
        name: format!("Removing '{}'", identity),
    });

    if vault_dispatch::is_vault_workspace(core) {
        return vault_dispatch::remove_asset(identity, core, sink);
    }

    let config = match core.store.load(scope) {
        Ok(c) => c,
        Err(e) => return fail(sink, format!("Failed to load config: {}", e)),
    };

    let mut providers = core.registry.active_providers_from_config(&config);
    if let Some(filter) = provider_filter {
        providers.retain(|p| p.id() == filter);
    }
    if providers.is_empty() {
        return fail(sink, "No active providers");
    }

    let pkg = match core.registry.find_package_by_identity(identity) {
        Ok(Some(p)) => p,
        Ok(None) => return fail(sink, format!("Asset '{}' not found", identity)),
        Err(e) => return fail(sink, format!("Lookup failed: {}", e)),
    };

    let mut any_failed = false;
    for provider in providers {
        if let Err(e) = super::remove::remove_asset(
            scope,
            &pkg.identity,
            &pkg.kind,
            &pkg.vault_id,
            core.store.as_ref(),
            provider,
        ) {
            sink.on_event(CoreEvent::Error(format!(
                "Provider {}: {}",
                provider.id(),
                e
            )));
            any_failed = true;
        }
    }

    if !any_failed {
        sink.on_event(CoreEvent::AssetRemoved {
            identity: pkg.identity.name.clone(),
        });
        Ok(CoreOutcome::Ok)
    } else {
        fail(sink, format!("Failed to remove '{}'", identity))
    }
}

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
        Err(e) => return fail(sink, format!("Failed to load config: {}", e)),
    };

    let mut providers = core.registry.active_providers_from_config(&config);
    if let Some(filter) = provider_filter {
        providers.retain(|p| p.id() == filter);
    }
    if providers.is_empty() {
        return fail(sink, "No active providers");
    }

    let pkg = match core.registry.find_package_by_identity(identity) {
        Ok(Some(p)) => p,
        Ok(None) => return fail(sink, format!("Asset '{}' not found", identity)),
        Err(e) => return fail(sink, format!("Lookup failed: {}", e)),
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
        Ok(CoreOutcome::Ok)
    } else {
        fail(sink, format!("Failed to update '{}'", identity))
    }
}
