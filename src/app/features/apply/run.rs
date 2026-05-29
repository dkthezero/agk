use crate::app::features::apply::command::ApplyConfigInput;
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::{ConfigStorePort, ContextStorePort};
use crate::domain::context::{ContextConfig, ContextId};
use crate::domain::scope::Scope;

/// Apply a declarative configuration snapshot to the given scope.
///
/// 1. If `context` is provided, upsert the context definition.
/// 2. If `environment` is provided, attach it to the context.
/// 3. Attach all requested vaults (global scope only).
/// 4. Activate all requested providers.
/// 5. Upsert all requested profiles.
///
/// If `dry_run` is true, no filesystem changes are made and only
/// events are emitted describing what *would* happen.
#[allow(clippy::too_many_arguments)]
pub fn run(
    input: ApplyConfigInput,
    scope: Scope,
    environment: Option<crate::domain::context::Environment>,
    context: Option<ContextId>,
    dry_run: bool,
    store: &dyn ConfigStorePort,
    context_store: &dyn ContextStorePort,
    _all_providers: Vec<String>,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    if dry_run {
        sink.on_event(CoreEvent::TaskStarted {
            id: 0,
            name: format!(
                "Dry-run apply {} (env: {:?}, context: {:?})",
                input.source_url, environment, context
            ),
        });
    }

    // Step 1: Upsert context definition if a name was provided.
    if let Some(ctx_id) = &context {
        if !dry_run {
            let mut file = context_store.load_contexts()?;
            let entry = file
                .contexts
                .entry(ctx_id.as_str().to_string())
                .or_insert_with(|| ContextConfig {
                    display_name: Some(ctx_id.as_str().to_string()),
                    ..ContextConfig::default()
                });

            // Merge vaults
            for v in &input.vaults {
                if !entry.vaults.contains(&v.id) {
                    entry.vaults.push(v.id.clone());
                }
            }
            // Merge providers
            for p in &input.providers {
                if !entry.providers.contains(p) {
                    entry.providers.push(p.clone());
                }
            }
            // Merge profile names
            for pr in &input.profiles {
                if !entry.profiles.contains(&pr.name) {
                    entry.profiles.push(pr.name.clone());
                }
            }
            if let Some(env) = environment {
                entry.environment = Some(env);
            }

            let _ = context_store.save_contexts(&file);
        }
        sink.on_event(CoreEvent::TaskCompleted {
            id: 0,
            message: format!("Updated context '{}'", ctx_id.as_str()),
        });
    }

    // Step 2: Attach vaults (global scope only)
    for vault in &input.vaults {
        if scope == Scope::Global {
            if !dry_run {
                if let Err(e) =
                    crate::app::features::asset::sync::attach_vault(vault.id.clone(), vault.config.clone(), store)
                {
                    sink.on_error(format!("Failed to attach vault '{}': {}", vault.id, e));
                    continue;
                }
            }
            sink.on_event(CoreEvent::VaultAttached(vault.id.clone()));
        }
    }

    // Step 3: Activate providers
    let mut config = store.load(scope).unwrap_or_default();
    for pid in &input.providers {
        if !config.providers.contains(pid) {
            if !dry_run {
                config.providers.push(pid.clone());
            }
            sink.on_event(CoreEvent::ProviderActivated(pid.clone()));
        }
    }

    // Step 4: Upsert profiles
    for profile in &input.profiles {
        config.profiles.retain(|p| p.name != profile.name);
        if !dry_run {
            config.profiles.push(profile.clone());
        }
        sink.on_event(CoreEvent::ProfileCreated(
            crate::domain::profile::ProfileId::new(&profile.name),
        ));
    }

    if !dry_run {
        let _ = store.save(scope, &config);
    }

    sink.on_event(CoreEvent::TaskCompleted {
        id: 0,
        message: format!("Applied config from {}", input.source_url),
    });

    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::features::apply::command::ApplyConfigInput;
    use crate::app::outcome::CoreEventSink;
    use crate::app::ports::ConfigStorePort;
    use crate::domain::config::{ConfigFile, Profile, VaultConfig};
    use crate::domain::context::{ContextFile, ContextId};
    use crate::domain::scope::Scope;
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

    struct FakeCtxStore {
        file: Mutex<ContextFile>,
    }

    impl FakeCtxStore {
        fn new() -> Self {
            Self {
                file: Mutex::new(ContextFile::default()),
            }
        }
    }

    impl ContextStorePort for FakeCtxStore {
        fn load_contexts(&self) -> anyhow::Result<ContextFile> {
            Ok(self.file.lock().unwrap().clone())
        }
        fn save_contexts(&self, file: &ContextFile) -> anyhow::Result<()> {
            *self.file.lock().unwrap() = file.clone();
            Ok(())
        }
        fn current_context(&self) -> anyhow::Result<ContextId> {
            Ok(self.file.lock().unwrap().current_id())
        }
        fn switch_context(&self, id: &ContextId) -> anyhow::Result<()> {
            if !self.file.lock().unwrap().contexts.contains_key(id.as_str()) {
                anyhow::bail!("missing");
            }
            self.file.lock().unwrap().current_context = id.as_str().to_string();
            Ok(())
        }
    }

    struct CollectingSink {
        events: Vec<CoreEvent>,
        errors: Vec<String>,
    }

    impl CollectingSink {
        fn new() -> Self {
            Self {
                events: Vec::new(),
                errors: Vec::new(),
            }
        }
    }

    impl CoreEventSink for CollectingSink {
        fn on_event(&mut self, event: CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, error: String) {
            self.errors.push(error);
        }
    }

    #[test]
    fn apply_adds_vaults_and_providers() {
        let store = FakeStore::new();
        let ctx_store = FakeCtxStore::new();
        let input = ApplyConfigInput::from_url("https://example.com/team.yaml")
            .with_vault(
                "team",
                VaultConfig::Local(crate::domain::config::LocalVaultSource {
                    path: "/tmp".into(),
                }),
            )
            .with_provider("opencode");
        let mut sink = CollectingSink::new();
        let result = run(
            input,
            Scope::Global,
            None,
            None,
            false,
            &store,
            &ctx_store,
            vec![],
            &mut sink,
        );
        assert!(result.is_ok());

        let config = store.load(Scope::Global).unwrap();
        assert_eq!(config.vaults, vec!["team"]);
        assert_eq!(config.providers, vec!["opencode"]);
        assert!(sink.events.iter().any(|e| matches!(
            e,
            CoreEvent::VaultAttached(id) if id == "team"
        )));
        assert!(sink.events.iter().any(|e| matches!(
            e,
            CoreEvent::ProviderActivated(id) if id == "opencode"
        )));
    }

    #[test]
    fn apply_dry_run_does_not_modify_config() {
        let store = FakeStore::new();
        let ctx_store = FakeCtxStore::new();
        let input =
            ApplyConfigInput::from_url("https://example.com/team.yaml").with_provider("opencode");
        let mut sink = CollectingSink::new();
        let result = run(
            input,
            Scope::Workspace,
            None,
            None,
            true,
            &store,
            &ctx_store,
            vec![],
            &mut sink,
        );
        assert!(result.is_ok());

        let config = store.load(Scope::Workspace).unwrap();
        assert!(config.providers.is_empty());
        assert!(sink.events.iter().any(|e| matches!(
            e,
            CoreEvent::ProviderActivated(id) if id == "opencode"
        )));
    }

    #[test]
    fn apply_with_context_updates_contexts() {
        let store = FakeStore::new();
        let ctx_store = FakeCtxStore::new();
        let input = ApplyConfigInput::from_url("https://example.com/team.yaml")
            .with_vault(
                "team",
                VaultConfig::Local(crate::domain::config::LocalVaultSource {
                    path: "/tmp".into(),
                }),
            )
            .with_provider("opencode");
        let mut sink = CollectingSink::new();
        let result = run(
            input,
            Scope::Global,
            Some(crate::domain::context::Environment::Prod),
            Some(ContextId::new("company-x")),
            false,
            &store,
            &ctx_store,
            vec![],
            &mut sink,
        );
        assert!(result.is_ok());

        let file = ctx_store.load_contexts().unwrap();
        assert!(file.contexts.contains_key("company-x"));
        let ctx = file.contexts.get("company-x").unwrap();
        assert_eq!(
            ctx.environment,
            Some(crate::domain::context::Environment::Prod)
        );
        assert!(ctx.vaults.contains(&"team".to_string()));
        assert!(ctx.providers.contains(&"opencode".to_string()));
    }

    #[test]
    fn apply_upserts_profiles() {
        let store = FakeStore::new();
        let ctx_store = FakeCtxStore::new();
        let profile = Profile {
            name: "backend".into(),
            provider_id: "opencode".into(),
            skills: vec!["rust".into()],
            mcps: vec![],
        };
        let input = ApplyConfigInput::from_url("https://example.com/team.yaml")
            .with_profile(profile.clone());
        let mut sink = CollectingSink::new();
        let result = run(
            input,
            Scope::Workspace,
            None,
            None,
            false,
            &store,
            &ctx_store,
            vec![],
            &mut sink,
        );
        assert!(result.is_ok());

        let config = store.load(Scope::Workspace).unwrap();
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.profiles[0].name, "backend");
    }
}
