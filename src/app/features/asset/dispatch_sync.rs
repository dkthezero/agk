//! `sync_assets_cmd` adapter — extracted from `dispatch_helpers.rs` so both
//! files stay under the ADR-001 §6.4 limit.

use crate::app::core::AgkCore;
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::domain::scope::Scope;

pub(super) fn sync_assets_cmd(
    scope: Scope,
    dry_run: bool,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    sink.on_event(CoreEvent::TaskStarted {
        id: 0,
        name: "Syncing assets".into(),
    });

    // --- Team-aware sync: if team.toml is present, merge team vaults ---
    let _team_sync_result = {
        let team_config = match core.team_config_store.load(scope) {
            Ok(c) => c,
            Err(e) => {
                // If team.toml exists but is malformed, surface the error instead
                // of silently proceeding as if no team config is present.
                if core.team_config_store.exists(scope) {
                    return Err(e);
                }
                // File simply doesn't exist — proceed without team config.
                Default::default()
            }
        };
        if !team_config.name.is_empty() || !team_config.vaults.is_empty() {
            let mut config = core.store.load(scope)?;
            let result = super::sync_team::sync_team_config(&team_config, &mut config, dry_run);
            if !dry_run {
                core.store.save(scope, &config)?;
            }
            // Emit team sync result event
            sink.on_event(CoreEvent::TeamSyncComplete {
                vaults_attached: result.vaults_attached.clone(),
                skills_installed: result.skills_installed.clone(),
                skills_updated: result.skills_updated.clone(),
                skills_removed_from_team: result.skills_removed_from_team.clone(),
                errors: result.errors.clone(),
            });
            Some(result)
        } else {
            None
        }
    };

    // --- Regular sync ---
    let config = core.store.load(scope)?;

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
                            super::update::update_asset(scope, &pkg, core.store.as_ref(), provider)
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