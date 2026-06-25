use crate::app::event::CoreEvent;
use crate::app::features::apply::command::ApplyConfigInput;
use crate::app::features::apply::source::resolve_source;
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
    // Resolve the source (read + parse a local file, reject unsupported URL
    // sources, surface missing/unreadable files) BEFORE applying anything,
    // so a bad source surfaces as a clear error instead of a silent
    // false-success that reports "Applied config from <source>" while doing
    // nothing. The `context://` scheme short-circuits (context create path).
    let input = resolve_source(input)?;

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

            context_store.save_contexts(&file)?;
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
                crate::app::features::asset::sync::attach_vault(
                    vault.id.clone(),
                    vault.config.clone(),
                    store,
                )
                .map_err(|e| anyhow::anyhow!("Failed to attach vault '{}': {}", vault.id, e))?;
            }
            sink.on_event(CoreEvent::VaultAttached(vault.id.clone()));
        }
    }

    // Step 3: Activate providers
    let mut config = store.load(scope)?;
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
        store.save(scope, &config)?;
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
            scope: "workspace".to_string(),
            skills: vec![crate::domain::profile::ProfileAssetRef::new("rust", "auto")],
            mcps: vec![],
            instructions: vec![],
            tool_refs: vec![],
            permission_mode: None,
            prompt_overlay_path: None,
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

    /// A config store whose `save` always fails — used to verify that
    /// `apply::run` propagates save errors instead of swallowing them.
    struct SaveFailingStore;

    impl ConfigStorePort for SaveFailingStore {
        fn load(&self, _scope: Scope) -> anyhow::Result<ConfigFile> {
            Ok(ConfigFile::default())
        }
        fn save(&self, _scope: Scope, _config: &ConfigFile) -> anyhow::Result<()> {
            anyhow::bail!("disk full")
        }
    }

    /// A config store whose `load` always fails — used to verify that
    /// `apply::run` surfaces a malformed/missing config instead of
    /// defaulting silently.
    struct LoadFailingStore;

    impl ConfigStorePort for LoadFailingStore {
        fn load(&self, _scope: Scope) -> anyhow::Result<ConfigFile> {
            anyhow::bail!("config file is malformed")
        }
        fn save(&self, _scope: Scope, _config: &ConfigFile) -> anyhow::Result<()> {
            Ok(())
        }
    }

    /// A context store whose `save_contexts` always fails — used to verify
    /// that `apply::run` propagates context-save errors instead of ignoring
    /// them via `let _ =`.
    struct SaveFailingCtxStore;

    impl ContextStorePort for SaveFailingCtxStore {
        fn load_contexts(&self) -> anyhow::Result<ContextFile> {
            Ok(ContextFile::default())
        }
        fn save_contexts(&self, _file: &ContextFile) -> anyhow::Result<()> {
            anyhow::bail!("context store write failed")
        }
        fn current_context(&self) -> anyhow::Result<ContextId> {
            Ok(ContextId::new("default"))
        }
        fn switch_context(&self, _id: &ContextId) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn apply_vault_attach_failure_returns_err_not_false_success() {
        let store = SaveFailingStore;
        let ctx_store = FakeCtxStore::new();
        let input = ApplyConfigInput::from_url("https://example.com/team.yaml").with_vault(
            "team",
            VaultConfig::Local(crate::domain::config::LocalVaultSource {
                path: "/tmp".into(),
            }),
        );
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
        // Previously this returned Ok while printing the error via on_error —
        // a false-success. Now the attach failure must propagate as Err.
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Failed to attach vault 'team'"));
        // No VaultAttached success event should fire on failure.
        assert!(!sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::VaultAttached(_))));
    }

    #[test]
    fn apply_config_load_failure_surfaces_error_not_default() {
        let store = LoadFailingStore;
        let ctx_store = FakeCtxStore::new();
        // A provider is required to reach the `store.load` line; a vault in
        // global scope would hit attach_vault's load first, which also fails.
        let input =
            ApplyConfigInput::from_url("https://example.com/team.yaml").with_provider("opencode");
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
        // Previously `store.load(scope).unwrap_or_default()` masked this.
        let err = result.unwrap_err();
        assert!(err.to_string().contains("malformed"));
    }

    #[test]
    fn apply_config_save_failure_returns_err_not_false_success() {
        let store = SaveFailingStore;
        let ctx_store = FakeCtxStore::new();
        let input =
            ApplyConfigInput::from_url("https://example.com/team.yaml").with_provider("opencode");
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
        // Previously `let _ = store.save(scope, &config)` swallowed this and
        // returned Ok — a false-success. Now the save error propagates.
        let err = result.unwrap_err();
        assert!(err.to_string().contains("disk full"));
    }

    #[test]
    fn apply_context_save_failure_returns_err_not_false_success() {
        let store = FakeStore::new();
        let ctx_store = SaveFailingCtxStore;
        let input =
            ApplyConfigInput::from_url("https://example.com/team.yaml").with_provider("opencode");
        let mut sink = CollectingSink::new();
        let result = run(
            input,
            Scope::Global,
            None,
            Some(ContextId::new("company-x")),
            false,
            &store,
            &ctx_store,
            vec![],
            &mut sink,
        );
        // Previously `let _ = context_store.save_contexts(&file)` swallowed
        // this and returned Ok — a false-success. Now it propagates.
        let err = result.unwrap_err();
        assert!(err.to_string().contains("context store write failed"));
    }
}
