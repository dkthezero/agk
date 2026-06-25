use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ConfigStorePort;
use crate::domain::config;
use crate::domain::profile::ProfileAssetRef;
use crate::domain::profile::ProfileId;
use crate::domain::profile_diff::compute_diff;
use crate::domain::scope::Scope;

/// Compare a local profile against its vault source and emit the diff.
///
/// **Limitation:** Vault profile resolution is currently a stub — vault-side
/// skill/MCP/instruction/tool refs are always empty. This means every local
/// asset appears as an "addition" in the diff. Once `VaultPort` can fetch
/// actual vault profile contents, this use-case should be updated to pass
/// real vault refs into `compute_diff`.
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
        .ok_or_else(|| {
            anyhow::anyhow!("Profile '{}' not found in {:?} scope", id.as_str(), scope)
        })?;

    // Search vault definitions for a matching vault profile
    let mut found_in_vault = false;

    for section in config.vault_defs.values() {
        if let Some(ref profiles_bucket) = section.profiles {
            for identity_str in &profiles_bucket.items {
                if config::parse_identity(identity_str)
                    .map(|parsed| parsed.name == id.as_str())
                    .unwrap_or(false)
                {
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

    let has_drift = diff.has_drift();

    sink.on_event(CoreEvent::ProfileDiffResult {
        profile_name: id.as_str().to_string(),
        diff,
    });

    if !found_in_vault {
        sink.on_event(CoreEvent::Info(format!(
            "Profile '{}' has no vault source — all local refs shown as additions.",
            id.as_str()
        )));
    } else if has_drift {
        sink.on_event(CoreEvent::Info(format!(
            "Profile '{}' has drifted from vault source.",
            id.as_str()
        )));
    } else {
        sink.on_event(CoreEvent::Info(format!(
            "Profile '{}' matches vault source — no drift.",
            id.as_str()
        )));
    }

    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{ConfigFile, Profile};
    use crate::domain::profile::ProfileId;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct CollectingSink {
        events: Vec<CoreEvent>,
    }
    impl CoreEventSink for CollectingSink {
        fn on_event(&mut self, event: CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, _error: String) {}
    }

    #[derive(Default)]
    struct FakeStore(Mutex<HashMap<String, ConfigFile>>);
    impl ConfigStorePort for FakeStore {
        fn load(&self, scope: Scope) -> anyhow::Result<ConfigFile> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .get(&format!("{:?}", scope))
                .cloned()
                .unwrap_or_default())
        }
        fn save(&self, scope: Scope, config: &ConfigFile) -> anyhow::Result<()> {
            self.0
                .lock()
                .unwrap()
                .insert(format!("{:?}", scope), config.clone());
            Ok(())
        }
    }

    /// The diff summary line must be emitted as a clean `CoreEvent::Info`
    /// (rendering the raw message in text mode) rather than a misleading
    /// `TaskCompleted { id: 0, message }` that renders as
    /// `[0] Completed: ...` — a listing/summary is not a task completion.
    #[test]
    fn diff_summary_uses_info_not_task_completed() {
        let store = FakeStore::default();
        let mut config = ConfigFile::default();
        config.profiles.push(Profile {
            name: "dev".into(),
            provider_id: "opencode".into(),
            scope: String::new(),
            skills: vec![],
            mcps: vec![],
            instructions: vec![],
            tool_refs: vec![],
            permission_mode: None,
            prompt_overlay_path: None,
        });
        store
            .0
            .lock()
            .unwrap()
            .insert("Workspace".to_string(), config);
        let mut sink = CollectingSink { events: vec![] };
        let result = run(&ProfileId::new("dev"), Scope::Workspace, &store, &mut sink);
        assert!(result.is_ok());
        // No TaskCompleted should be emitted for the summary.
        assert!(!sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::TaskCompleted { .. })));
        // The summary is carried as Info, rendered cleanly in text/JSON/TUI.
        assert!(sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::Info(msg) if msg.contains("no vault source"))));
    }
}
