use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::registry::Registry;
use crate::domain::asset::AssetKind;
use crate::domain::scope::Scope;

/// Validate installed assets against source vaults.
///
/// Emits [`CoreEvent::ValidationReport`] with a summary message.
pub fn run(
    scope: Scope,
    registry: &Registry,
    store: &dyn crate::app::ports::ConfigStorePort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let config = store.load(scope)?;
    let providers = registry.active_providers_from_config(&config);

    let mut all_passed = true;
    let mut messages: Vec<String> = vec![];

    let all_vault_ids: Vec<String> = config.vault_defs.keys().cloned().collect();

    for vault_id in &all_vault_ids {
        for identity in config.installed_skills(vault_id) {
            let latest = registry.find_package_by_identity(&identity.name)?;
            let sha10_match = latest
                .as_ref()
                .map(|p| p.identity.sha10 == identity.sha10)
                .unwrap_or(false);
            let parse_ok = latest.is_some();
            if !sha10_match || !parse_ok {
                all_passed = false;
            }

            let mut provider_checks = vec![];
            for provider in &providers {
                let path = provider.install_path_for(&identity, &AssetKind::Skill, scope);
                provider_checks.push(format!(
                    "{}: {}",
                    provider.name(),
                    if path.as_ref().map(|p| p.exists()).unwrap_or(false) {
                        "ok"
                    } else {
                        "missing"
                    }
                ));
            }

            messages.push(format!(
                "{} (vault={}): sha10_match={}, parse_ok={}, providers=[{}]",
                identity.name,
                vault_id,
                sha10_match,
                parse_ok,
                provider_checks.join(", ")
            ));
        }

        for identity in config.installed_instructions(vault_id) {
            let latest = registry.find_package_by_identity(&identity.name)?;
            let sha10_match = latest
                .as_ref()
                .map(|p| p.identity.sha10 == identity.sha10)
                .unwrap_or(false);
            let parse_ok = latest.is_some();
            if !sha10_match || !parse_ok {
                all_passed = false;
            }

            let mut provider_checks = vec![];
            for provider in &providers {
                let path = provider.install_path_for(&identity, &AssetKind::Instruction, scope);
                provider_checks.push(format!(
                    "{}: {}",
                    provider.name(),
                    if path.as_ref().map(|p| p.exists()).unwrap_or(false) {
                        "ok"
                    } else {
                        "missing"
                    }
                ));
            }

            messages.push(format!(
                "{} (vault={}): sha10_match={}, parse_ok={}, providers=[{}]",
                identity.name,
                vault_id,
                sha10_match,
                parse_ok,
                provider_checks.join(", ")
            ));
        }
    }

    let summary = if all_passed {
        format!("All {} assets are valid.", messages.len())
    } else {
        format!(
            "Validation failed for some assets:\n{}",
            messages
                .iter()
                .filter(|m| m.contains("sha10_match=false") || m.contains("parse_ok=false"))
                .map(|m| format!("  - {}", m))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    sink.on_event(CoreEvent::ValidationReport {
        passed: all_passed,
        message: summary,
    });

    Ok(CoreOutcome::Ok)
}
