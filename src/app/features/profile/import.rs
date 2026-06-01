use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ConfigStorePort;
use crate::domain::profile::{check_version_compatibility, ExportedProfile, ProfileId};
use crate::domain::scope::Scope;

/// Import a profile from a portable JSON file.
///
/// 1. Read and deserialize JSON from `json_path`.
/// 2. Check version compatibility.
/// 3. Determine target name (override or from export).
/// 4. Check collision in target scope.
/// 5. Replace missing vaults with "auto" + warnings.
/// 6. Create profile entry in config.
/// 7. Write agent markdown to `.agk/profiles/<name>/agent.md`.
pub fn run(
    json_path: &std::path::Path,
    target_name: Option<String>,
    target_scope: Scope,
    workspace: &std::path::Path,
    store: &dyn ConfigStorePort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    // 1. Read and deserialize JSON
    let content = std::fs::read_to_string(json_path).map_err(|e| {
        anyhow::anyhow!("Failed to read import file '{}': {}", json_path.display(), e)
    })?;
    let exported: ExportedProfile =
        serde_json::from_str(&content).map_err(|e| {
            anyhow::anyhow!("Failed to parse import file: {}", e)
        })?;

    // 2. Check version compatibility
    let current_version = env!("CARGO_PKG_VERSION");
    if let Err(e) = check_version_compatibility(&exported.agk_version, current_version) {
        anyhow::bail!("{}", e);
    }
    if exported.agk_version != current_version {
        sink.on_event(CoreEvent::Info(format!(
            "Warning: import version {} differs from current {} — some features may not work",
            exported.agk_version, current_version
        )));
    }

    // 3. Determine target name
    let name = target_name.unwrap_or_else(|| exported.profile.name.clone());
    let profile_id = ProfileId::new(&name);
    crate::domain::profile::validate_profile_id(&profile_id)?;

    // 4. Check collision
    let mut config = store.load(target_scope)?;
    if config.profiles.iter().any(|p| p.name == name) {
        anyhow::bail!(
            "Profile '{}' already exists in {:?} scope",
            name,
            target_scope
        );
    }

    // 5. Replace missing vaults with "auto" + warnings
    let skills = resolve_missing_vaults(&exported.profile.skills, &config, sink);
    let mcps = resolve_missing_vaults(&exported.profile.mcps, &config, sink);
    let instructions = resolve_missing_vaults(&exported.profile.instructions, &config, sink);

    // 6. Create profile entry in config
    let profile_entry = crate::domain::config::Profile {
        name: name.clone(),
        provider_id: exported.profile.provider_id.clone(),
        scope: target_scope.to_string().to_lowercase(),
        skills,
        mcps,
        instructions,
        tool_refs: exported.profile.tools.clone(),
        permission_mode: exported.profile.permission_mode.clone(),
        prompt_overlay_path: None,
    };
    config.profiles.push(profile_entry);
    store.save(target_scope, &config)?;

    // 7. Write agent markdown
    if !exported.profile.agent_markdown.is_empty() {
        let profile_dir = workspace.join(".agk").join("profiles").join(&name);
        std::fs::create_dir_all(&profile_dir)?;
        let agent_md = profile_dir.join("agent.md");
        std::fs::write(&agent_md, &exported.profile.agent_markdown)?;
    }

    sink.on_event(CoreEvent::ProfileImported {
        profile_name: name.clone(),
    });
    sink.on_event(CoreEvent::Info(format!(
        "Profile '{}' imported from {}",
        name,
        json_path.display()
    )));

    Ok(CoreOutcome::Ok)
}

/// Replace vault references that don't exist in the target config with "auto".
/// Emits a warning for each replaced vault.
fn resolve_missing_vaults(
    refs: &[crate::domain::profile::ProfileAssetRef],
    config: &crate::domain::config::ConfigFile,
    sink: &mut dyn CoreEventSink,
) -> Vec<crate::domain::profile::ProfileAssetRef> {
    refs.iter()
        .map(|r| {
            if r.vault != "auto" && !config.vaults.contains(&r.vault) {
                sink.on_event(CoreEvent::Info(format!(
                    "Warning: vault '{}' not found in target config, replacing with 'auto' for asset '{}'",
                    r.vault, r.name
                )));
                crate::domain::profile::ProfileAssetRef::new(&r.name, "auto")
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
    use crate::domain::profile::{ExportPayload, ProfileAssetRef};
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
        errors: Vec<String>,
    }

    impl CoreEventSink for CollectingSink {
        fn on_event(&mut self, event: CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, error: String) {
            self.errors.push(error);
        }
    }

    fn make_exported_profile(name: &str) -> ExportedProfile {
        ExportedProfile {
            agk_version: env!("CARGO_PKG_VERSION").to_string(),
            exported_at: "2026-06-01T00:00:00Z".to_string(),
            profile: ExportPayload {
                name: name.to_string(),
                provider_id: "opencode".to_string(),
                scope: "workspace".to_string(),
                structured_answers: None,
                skills: vec![ProfileAssetRef::new("rust", "auto")],
                mcps: vec![],
                instructions: vec![],
                tools: vec![],
                permission_mode: None,
                agent_markdown: "# Agent\nYou are an agent.".to_string(),
            },
        }
    }

    #[test]
    fn import_profile_creates_entry() {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeStore::new();
        let mut sink = CollectingSink {
            events: vec![],
            errors: vec![],
        };

        // Write a temp JSON file
        let exported = make_exported_profile("imported-dev");
        let json_path = dir.path().join("import.agk.json");
        std::fs::write(&json_path, serde_json::to_string_pretty(&exported).unwrap()).unwrap();

        let result = run(
            &json_path,
            None,
            Scope::Workspace,
            dir.path(),
            &store,
            &mut sink,
        );
        assert!(result.is_ok());

        let config = store.load(Scope::Workspace).unwrap();
        assert!(config.profiles.iter().any(|p| p.name == "imported-dev"));

        // Check agent.md was written
        let agent_md = dir
            .path()
            .join(".agk")
            .join("profiles")
            .join("imported-dev")
            .join("agent.md");
        assert!(agent_md.exists());
        assert!(std::fs::read_to_string(&agent_md)
            .unwrap()
            .contains("# Agent"));
    }

    #[test]
    fn import_profile_with_name_override() {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeStore::new();
        let mut sink = CollectingSink {
            events: vec![],
            errors: vec![],
        };

        let exported = make_exported_profile("original");
        let json_path = dir.path().join("import.agk.json");
        std::fs::write(&json_path, serde_json::to_string_pretty(&exported).unwrap()).unwrap();

        let result = run(
            &json_path,
            Some("overridden".to_string()),
            Scope::Workspace,
            dir.path(),
            &store,
            &mut sink,
        );
        assert!(result.is_ok());

        let config = store.load(Scope::Workspace).unwrap();
        assert!(config.profiles.iter().any(|p| p.name == "overridden"));
    }

    #[test]
    fn import_collision_fails() {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeStore::new();
        // Pre-populate with a profile named "dev"
        let mut config = ConfigFile::default();
        config.profiles.push(Profile {
            name: "dev".to_string(),
            provider_id: "opencode".to_string(),
            scope: "workspace".to_string(),
            skills: vec![],
            mcps: vec![],
            instructions: vec![],
            tool_refs: vec![],
            permission_mode: None,
            prompt_overlay_path: None,
        });
        store.save(Scope::Workspace, &config).unwrap();

        let mut sink = CollectingSink {
            events: vec![],
            errors: vec![],
        };

        let exported = make_exported_profile("dev");
        let json_path = dir.path().join("import.agk.json");
        std::fs::write(&json_path, serde_json::to_string_pretty(&exported).unwrap()).unwrap();

        let result = run(
            &json_path,
            None,
            Scope::Workspace,
            dir.path(),
            &store,
            &mut sink,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn import_major_version_mismatch_fails() {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeStore::new();
        let mut sink = CollectingSink {
            events: vec![],
            errors: vec![],
        };

        let mut exported = make_exported_profile("version-test");
        exported.agk_version = "1.0.0".to_string();
        let json_path = dir.path().join("import.agk.json");
        std::fs::write(&json_path, serde_json::to_string_pretty(&exported).unwrap()).unwrap();

        let result = run(
            &json_path,
            None,
            Scope::Workspace,
            dir.path(),
            &store,
            &mut sink,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Major version mismatch"));
    }

    #[test]
    fn import_missing_file_fails() {
        let store = FakeStore::new();
        let mut sink = CollectingSink {
            events: vec![],
            errors: vec![],
        };

        let result = run(
            std::path::Path::new("/nonexistent/file.json"),
            None,
            Scope::Workspace,
            std::path::Path::new("."),
            &store,
            &mut sink,
        );
        assert!(result.is_err());
    }
}