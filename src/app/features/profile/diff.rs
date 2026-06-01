use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ConfigStorePort;
use crate::domain::profile::ProfileAssetRef;
use crate::domain::profile::ProfileId;
use crate::domain::profile_diff::compute_diff;
use crate::domain::scope::Scope;

/// Compare a local profile against its vault source and emit the diff.
pub fn run(
    id: &ProfileId,
    scope: Scope,
    store: &dyn ConfigStorePort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let config = store.load(scope)?;

    let local_profile = config
        .profiles
        .iter()
        .find(|p| p.name == id.as_str())
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found in {:?} scope", id.as_str(), scope))?;

    // Search vault definitions for a matching vault profile
    let mut found_in_vault = false;

    for (_vault_id, section) in &config.vault_defs {
        if let Some(ref profiles_bucket) = section.profiles {
            for identity_str in &profiles_bucket.items {
                let trimmed = identity_str.trim_start_matches('[').trim_end_matches(']');
                let name = trimmed.split(':').next().unwrap_or(trimmed);
                if name == id.as_str() {
                    found_in_vault = true;
                    break;
                }
            }
        }
        if found_in_vault {
            break;
        }
    }

    // Vault source refs — currently empty until vault profile resolution is implemented
    let vault_skills: Vec<ProfileAssetRef> = Vec::new();
    let vault_mcps: Vec<ProfileAssetRef> = Vec::new();
    let vault_instructions: Vec<ProfileAssetRef> = Vec::new();
    let vault_tools: Vec<String> = Vec::new();
    let vault_permission_mode: Option<&str> = None;

    let diff = compute_diff(
        &local_profile.skills,
        &vault_skills,
        &local_profile.mcps,
        &vault_mcps,
        &local_profile.instructions,
        &vault_instructions,
        &local_profile.tool_refs,
        &vault_tools,
        local_profile.permission_mode.as_deref(),
        vault_permission_mode,
    );

    sink.on_event(CoreEvent::ProfileDiffResult {
        profile_name: id.as_str().to_string(),
        diff: diff.clone(),
    });

    if !found_in_vault {
        sink.on_event(CoreEvent::TaskCompleted {
            id: 0,
            message: format!(
                "Profile '{}' has no vault source — all local refs shown as additions.",
                id.as_str()
            ),
        });
    } else if diff.has_drift() {
        sink.on_event(CoreEvent::TaskCompleted {
            id: 0,
            message: format!("Profile '{}' has drifted from vault source.", id.as_str()),
        });
    } else {
        sink.on_event(CoreEvent::TaskCompleted {
            id: 0,
            message: format!("Profile '{}' matches vault source — no drift.", id.as_str()),
        });
    }

    Ok(CoreOutcome::Ok)
}