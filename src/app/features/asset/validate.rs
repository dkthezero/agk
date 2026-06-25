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
        message: summary.clone(),
    });

    Ok(CoreOutcome::ValidationReport {
        passed: all_passed,
        message: summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::registry::Registry;
    use crate::app::test_support::{CollectingSink, FakeStore};
    use crate::domain::config::{AssetBucket, AssetSource, ConfigFile, VaultSection};
    use std::collections::HashMap;

    /// When an installed skill has no matching package in the registry the
    /// validation must report `passed: false` AND return it via the
    /// `CoreOutcome::ValidationReport` outcome so the CLI dispatcher can map
    /// a failing validation onto a non-zero exit code (regression: `agk
    /// validate` used to exit 0 even when it printed "Validation failed").
    #[test]
    fn failing_validation_returns_failed_outcome() {
        let store = FakeStore::new();
        let mut config = ConfigFile::default();
        config.vault_defs.insert(
            "ghost-vault".to_string(),
            VaultSection {
                vault: None,
                skills: Some(AssetBucket {
                    items: vec!["[ghost:1.0.0:deadbeef00]".to_string()],
                    source: Some(AssetSource::Personal),
                }),
                instructions: None,
                mcps: None,
                profiles: None,
            },
        );
        store.seed(Scope::Workspace, config);

        // Empty registry -> find_package_by_identity returns Ok(None) ->
        // parse_ok=false -> all_passed=false.
        let registry = Registry::new();
        let mut sink = CollectingSink::new();

        let outcome = run(
            Scope::Workspace,
            &registry,
            &store as &dyn crate::app::ports::ConfigStorePort,
            &mut sink,
        )
        .expect("validate use case should not error");

        // Outcome carries the failure so the dispatcher can set a non-zero
        // exit code.
        match outcome {
            CoreOutcome::ValidationReport { passed, .. } => assert!(
                !passed,
                "a failed validation must surface passed=false in the outcome"
            ),
            other => panic!("expected ValidationReport outcome, got {:?}", other),
        }

        // The human/JSON renderers are driven by the event, so it must still
        // be emitted exactly once with passed=false.
        let reports: Vec<_> = sink
            .events
            .iter()
            .filter(|e| matches!(e, CoreEvent::ValidationReport { .. }))
            .collect();
        assert_eq!(reports.len(), 1, "exactly one ValidationReport event");
        if let CoreEvent::ValidationReport { passed, message } = &reports[0] {
            assert!(!*passed);
            assert!(message.contains("ghost"), "message should name the asset");
        }
    }

    /// When every installed asset resolves in the registry the validation
    /// must report `passed: true` via both the event and the outcome.
    #[test]
    fn passing_validation_returns_passed_outcome() {
        let store = FakeStore::new();
        let config = ConfigFile {
            vault_defs: HashMap::new(),
            ..ConfigFile::default()
        };
        store.seed(Scope::Workspace, config);

        let registry = Registry::new();
        let mut sink = CollectingSink::new();

        let outcome = run(
            Scope::Workspace,
            &registry,
            &store as &dyn crate::app::ports::ConfigStorePort,
            &mut sink,
        )
        .expect("validate use case should not error");

        match outcome {
            CoreOutcome::ValidationReport { passed, message } => {
                assert!(passed, "no assets => nothing to fail => passed=true");
                assert!(message.contains("valid"), "summary should affirm validity");
            }
            other => panic!("expected ValidationReport outcome, got {:?}", other),
        }
        if let CoreEvent::ValidationReport { passed, .. } = &sink.events[0] {
            assert!(*passed);
        }
    }
}
