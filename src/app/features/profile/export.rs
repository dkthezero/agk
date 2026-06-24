use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ConfigStorePort;
use crate::domain::profile::{ExportPayload, ExportedProfile, ProfileId};
use crate::domain::scope::Scope;

/// Export a profile to a portable JSON structure.
///
/// 1. Load profile from config via [`ConfigStorePort`].
/// 2. Read agent markdown from `.agk/profiles/<name>/agent.md`.
/// 3. Build [`ExportPayload`] from profile data.
/// 4. If `resolve_vaults`, replace "auto" vault refs with actual vault names.
/// 5. Return [`ExportedProfile`] with `agk_version` from `CARGO_PKG_VERSION`.
pub fn run(
    profile_id: &ProfileId,
    scope: Scope,
    resolve_vaults: bool,
    output_path: Option<&str>,
    workspace: &std::path::Path,
    store: &dyn ConfigStorePort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let config = store.load(scope)?;
    let profile = config
        .find_profile(profile_id.as_str())
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Profile '{}' not found in {:?} config",
                profile_id.as_str(),
                scope
            )
        })?;

    // Read agent markdown
    let agent_md_path = workspace
        .join(".agk")
        .join("profiles")
        .join(&profile.name)
        .join("agent.md");
    let agent_markdown = std::fs::read_to_string(&agent_md_path).unwrap_or_default();

    // Build skill refs, optionally resolving "auto" vault names
    let skills = if resolve_vaults {
        resolve_vault_refs(&profile.skills, &config)
    } else {
        profile.skills.clone()
    };
    let mcps = if resolve_vaults {
        resolve_vault_refs(&profile.mcps, &config)
    } else {
        profile.mcps.clone()
    };
    let instructions = if resolve_vaults {
        resolve_vault_refs(&profile.instructions, &config)
    } else {
        profile.instructions.clone()
    };

    let payload = ExportPayload {
        name: profile.name.clone(),
        provider_id: profile.provider_id.clone(),
        scope: profile.scope.clone(),
        structured_answers: None,
        skills,
        mcps,
        instructions,
        tools: profile.tool_refs.clone(),
        permission_mode: profile.permission_mode.clone(),
        agent_markdown,
    };

    let exported = ExportedProfile {
        agk_version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        profile: payload,
    };

    let json = serde_json::to_string_pretty(&exported)?;

    if let Some(path) = output_path {
        std::fs::write(path, &json)
            .map_err(|e| anyhow::anyhow!("Failed to write export file '{}': {}", path, e))?;
    }

    sink.on_event(CoreEvent::ProfileExported {
        profile_name: profile.name.clone(),
        content: json,
        output_path: output_path.map(|s| s.to_string()),
    });

    Ok(CoreOutcome::Ok)
}

/// Replace "auto" vault refs with the first vault name from config that
/// contains the referenced skill/MCP/instruction.
fn resolve_vault_refs(
    refs: &[crate::domain::profile::ProfileAssetRef],
    config: &crate::domain::config::ConfigFile,
) -> Vec<crate::domain::profile::ProfileAssetRef> {
    refs.iter()
        .map(|r| {
            if r.vault == "auto" {
                let resolved_vault = config
                    .vaults
                    .iter()
                    .find(|v| {
                        config.is_skill_installed(v.as_str(), &r.name)
                            || config.is_mcp_installed(v.as_str(), &r.name)
                            || config.is_instruction_installed(v.as_str(), &r.name)
                    })
                    .cloned()
                    .unwrap_or_else(|| "auto".to_string());
                crate::domain::profile::ProfileAssetRef::new(&r.name, &resolved_vault)
            } else {
                r.clone()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{ConfigFile, Profile};
    use crate::domain::profile::ProfileAssetRef;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakeStore {
        data: Mutex<HashMap<String, ConfigFile>>,
    }

    impl FakeStore {
        fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    impl ConfigStorePort for FakeStore {
        fn load(&self, scope: Scope) -> anyhow::Result<ConfigFile> {
            Ok(self
                .data
                .lock()
                .unwrap()
                .get(&format!("{:?}", scope))
                .cloned()
                .unwrap_or_default())
        }
        fn save(&self, scope: Scope, config: &ConfigFile) -> anyhow::Result<()> {
            self.data
                .lock()
                .unwrap()
                .insert(format!("{:?}", scope), config.clone());
            Ok(())
        }
    }

    struct CollectingSink {
        events: Vec<CoreEvent>,
    }

    impl CoreEventSink for CollectingSink {
        fn on_event(&mut self, event: CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, _error: String) {}
    }

    fn make_config_with_profile() -> ConfigFile {
        let mut config = ConfigFile::default();
        config.profiles.push(Profile {
            name: "dev".to_string(),
            provider_id: "opencode".to_string(),
            scope: "workspace".to_string(),
            skills: vec![
                ProfileAssetRef::new("rust-patterns", "auto"),
                ProfileAssetRef::new("docker", "clawhub"),
            ],
            mcps: vec![ProfileAssetRef::new("filesystem", "auto")],
            instructions: vec![],
            tool_refs: vec!["Read".to_string(), "Glob".to_string()],
            permission_mode: Some("auto".to_string()),
            prompt_overlay_path: None,
        });
        config
    }

    #[test]
    fn export_profile_found() {
        let store = FakeStore::new();
        store
            .save(Scope::Workspace, &make_config_with_profile())
            .unwrap();
        let mut sink = CollectingSink { events: vec![] };
        let result = run(
            &ProfileId::new("dev"),
            Scope::Workspace,
            false,
            None,
            std::path::Path::new("."),
            &store,
            &mut sink,
        );
        assert!(result.is_ok());
        assert!(sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::ProfileExported { .. })));
    }

    #[test]
    fn export_profile_not_found_fails() {
        let store = FakeStore::new();
        let mut sink = CollectingSink { events: vec![] };
        let result = run(
            &ProfileId::new("missing"),
            Scope::Workspace,
            false,
            None,
            std::path::Path::new("."),
            &store,
            &mut sink,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn export_profile_bad_output_path_returns_err_and_no_event() {
        let store = FakeStore::new();
        store
            .save(Scope::Workspace, &make_config_with_profile())
            .unwrap();
        let mut sink = CollectingSink { events: vec![] };
        let result = run(
            &ProfileId::new("dev"),
            Scope::Workspace,
            false,
            Some("/nonexistent_agk_dir/prof.json"),
            std::path::Path::new("."),
            &store,
            &mut sink,
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to write export file"));
        assert!(
            sink.events.is_empty(),
            "no event should be emitted on write failure"
        );
    }

    #[test]
    fn resolve_vault_refs_auto_stays_auto_when_no_match() {
        let config = ConfigFile::default();
        let refs = vec![ProfileAssetRef::new("nonexistent", "auto")];
        let resolved = resolve_vault_refs(&refs, &config);
        assert_eq!(resolved[0].vault, "auto");
    }

    #[test]
    fn resolve_vault_refs_explicit_vault_preserved() {
        let config = ConfigFile::default();
        let refs = vec![ProfileAssetRef::new("skill-a", "clawhub")];
        let resolved = resolve_vault_refs(&refs, &config);
        assert_eq!(resolved[0].vault, "clawhub");
    }
}
