pub mod install;
pub mod pack;
pub mod remove;
pub mod search_remote;
pub mod sync;
pub mod update;
pub mod validate;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::domain::scope::Scope;

/// Dispatch asset-related [`CoreCommand`] variants.
/// Returns `Some(result)` if the command was handled, `None` otherwise.
pub fn dispatch(
    cmd: &CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::SearchRemoteVault { vault_id, query } => Some(search_remote::run(
            vault_id.clone(),
            query.clone(),
            core.vault_search.as_ref(),
            sink,
        )),
        CoreCommand::ValidateAssets { scope } => Some(validate::run(
            *scope,
            core.registry.as_ref(),
            core.store.as_ref(),
            sink,
        )),
        CoreCommand::PackAsset {
            identity,
            target,
            stdout,
            scope,
        } => Some(pack::run(
            identity,
            *target,
            *stdout,
            *scope,
            core.registry.as_ref(),
            &core.workspace_root,
            sink,
        )),
        CoreCommand::InstallAsset {
            identity,
            scope,
            provider_filter,
            include_evals,
            dry_run,
        } => Some(install_asset_cmd(
            identity,
            *scope,
            provider_filter.as_deref(),
            *include_evals,
            *dry_run,
            core,
            sink,
        )),
        CoreCommand::RemoveAsset {
            identity,
            scope,
            provider_filter,
        } => Some(remove_asset_cmd(
            identity,
            *scope,
            provider_filter.as_deref(),
            core,
            sink,
        )),
        CoreCommand::UpdateAsset {
            identity,
            scope,
            provider_filter,
        } => Some(update_asset_cmd(
            identity,
            *scope,
            provider_filter.as_deref(),
            core,
            sink,
        )),
        CoreCommand::SyncAssets { scope, dry_run } => {
            Some(sync_assets_cmd(*scope, *dry_run, core, sink))
        }
        _ => None,
    }
}

fn install_asset_cmd(
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
            // Attempt remote fetch via ClawHub for simple slugs.
            if !identity.contains('/') {
                if let Err(e) = crate::infra::vault::clawhub::cli_install(identity) {
                    sink.on_event(CoreEvent::TaskFailed {
                        id: 0,
                        error: format!("Fetch failed: {}", e),
                    });
                    return Ok(CoreOutcome::Ok);
                }
                match core.registry.find_package_by_identity(identity) {
                    Ok(Some(p)) => p,
                    _ => {
                        sink.on_event(CoreEvent::TaskFailed {
                            id: 0,
                            error: format!("Asset '{}' not found after remote fetch", identity),
                        });
                        return Ok(CoreOutcome::Ok);
                    }
                }
            } else {
                sink.on_event(CoreEvent::TaskFailed {
                    id: 0,
                    error: format!("Asset '{}' not found in any vault", identity),
                });
                return Ok(CoreOutcome::Ok);
            }
        }
        Err(e) => {
            sink.on_event(CoreEvent::TaskFailed {
                id: 0,
                error: format!("Lookup failed: {}", e),
            });
            return Ok(CoreOutcome::Ok);
        }
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
        if let Err(e) = install::install_asset(scope, &pkg, core.store.as_ref(), provider) {
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
    } else {
        sink.on_event(CoreEvent::TaskFailed {
            id: 0,
            error: format!("Failed to install '{}'", identity),
        });
    }
    Ok(CoreOutcome::Ok)
}

fn remove_asset_cmd(
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
        if let Err(e) = remove::remove_asset(
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
    } else {
        sink.on_event(CoreEvent::TaskFailed {
            id: 0,
            error: format!("Failed to remove '{}'", identity),
        });
    }
    Ok(CoreOutcome::Ok)
}

fn update_asset_cmd(
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
        if let Err(e) = update::update_asset(scope, &pkg, core.store.as_ref(), provider) {
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

fn sync_assets_cmd(
    scope: Scope,
    dry_run: bool,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    sink.on_event(CoreEvent::TaskStarted {
        id: 0,
        name: "Syncing assets".into(),
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

    let provider_ids: Vec<String> = core
        .registry
        .active_providers_from_config(&config)
        .iter()
        .map(|p| p.id().to_string())
        .collect();

    if provider_ids.is_empty() {
        sink.on_event(CoreEvent::TaskFailed {
            id: 0,
            error: "No active providers".into(),
        });
        return Ok(CoreOutcome::Ok);
    }

    let mut updated = Vec::new();
    let mut skipped = Vec::new();
    let mut errors = Vec::new();

    for section in config.vault_defs.values() {
        let identities: Vec<String> = section
            .skills
            .as_ref()
            .map(|b| b.items.clone())
            .unwrap_or_default()
            .into_iter()
            .chain(
                section
                    .instructions
                    .as_ref()
                    .map(|b| b.items.clone())
                    .unwrap_or_default(),
            )
            .collect();

        for identity_str in identities {
            let name = identity_str
                .split(':')
                .next()
                .unwrap_or(&identity_str)
                .trim_start_matches('[');
            match core.registry.find_package_by_identity(name) {
                Ok(Some(pkg)) => {
                    if dry_run {
                        updated.push(pkg.identity.name.clone());
                        continue;
                    }
                    let mut provider_success = false;
                    for provider_id in &provider_ids {
                        let provider = match core.registry.get_provider(provider_id) {
                            Ok(p) => p,
                            Err(e) => {
                                errors.push(format!(
                                    "{}: provider lookup failed: {}",
                                    pkg.identity.name, e
                                ));
                                continue;
                            }
                        };
                        if let Err(e) =
                            update::update_asset(scope, &pkg, core.store.as_ref(), provider)
                        {
                            errors.push(format!(
                                "{}@{}: update failed: {}",
                                pkg.identity.name, provider_id, e
                            ));
                        } else {
                            provider_success = true;
                        }
                    }
                    if provider_success {
                        updated.push(pkg.identity.name.clone());
                    } else {
                        errors.push(format!("{}: all providers failed", pkg.identity.name));
                    }
                }
                Ok(None) => {
                    skipped.push(name.to_string());
                }
                Err(e) => {
                    errors.push(format!("{}: lookup failed: {}", name, e));
                }
            }
        }
    }

    sink.on_event(CoreEvent::SyncComplete {
        updated,
        skipped,
        errors,
    });
    Ok(CoreOutcome::Ok)
}
