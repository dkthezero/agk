use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::{ConfigStorePort, ContextStorePort};
use crate::domain::context::ContextId;
use crate::domain::scope::Scope;

/// Switch the active context and optionally activate its vaults / providers.
///
/// 1. Validate the target context exists.
/// 2. Write the new current-context marker.
/// 3. (Not dry-run) Update the scope config with context's vaults / providers.
pub fn run(
    id: &ContextId,
    dry_run: bool,
    context_store: &dyn ContextStorePort,
    sink: &mut dyn CoreEventSink,
    store: &dyn ConfigStorePort,
) -> CoreResult {
    if dry_run {
        sink.on_event(CoreEvent::TaskStarted {
            id: 0,
            name: format!("Dry-run switch context '{}'", id.as_str()),
        });
    }

    let file = context_store.load_contexts()?;
    let ctx = match file.get(id) {
        Some(ctx) => ctx,
        None => {
            return Err(anyhow::anyhow!("Context '{}' does not exist", id.as_str()));
        }
    };

    if !dry_run {
        if let Err(e) = context_store.switch_context(id) {
            return Err(anyhow::anyhow!("Failed to switch context: {}", e));
        }
    }

    sink.on_event(CoreEvent::TaskCompleted {
        id: 0,
        message: format!("Switched to context '{}'", id.as_str()),
    });

    // Merge context defaults into active config so vaults / providers are immediately visible.
    if !dry_run {
        let mut config = store.load(Scope::Global)?;
        let mut changed = false;

        for vault_id in &ctx.vaults {
            if !config.vaults.contains(vault_id) {
                config.vaults.push(vault_id.clone());
                changed = true;
            }
        }

        for pid in &ctx.providers {
            if !config.providers.contains(pid) {
                config.providers.push(pid.clone());
                changed = true;
            }
        }

        if changed {
            store.save(Scope::Global, &config)?;
        }
    }

    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::outcome::CoreEventSink;
    use crate::app::ports::{ConfigStorePort, ContextStorePort};
    use crate::domain::config::ConfigFile;
    use crate::domain::context::{ContextConfig, ContextFile, ContextId};
    use crate::domain::scope::Scope;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakeStore {
        config: Mutex<HashMap<String, ConfigFile>>,
    }
    impl FakeStore {
        fn new() -> Self {
            Self {
                config: Mutex::new(HashMap::new()),
            }
        }
    }
    impl ConfigStorePort for FakeStore {
        fn load(&self, scope: Scope) -> anyhow::Result<ConfigFile> {
            Ok(self
                .config
                .lock()
                .unwrap()
                .get(&format!("{:?}", scope))
                .cloned()
                .unwrap_or_default())
        }
        fn save(&self, scope: Scope, config: &ConfigFile) -> anyhow::Result<()> {
            self.config
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
        fn with(file: ContextFile) -> Self {
            Self {
                file: Mutex::new(file),
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

    struct CollectingSink;
    impl CoreEventSink for CollectingSink {
        fn on_event(&mut self, _event: CoreEvent) {}
        fn on_error(&mut self, _error: String) {}
    }

    #[test]
    fn switch_context_updates_current() {
        let mut file = ContextFile::default();
        file.ensure_default();
        file.contexts.insert(
            "company-x".to_string(),
            ContextConfig {
                display_name: Some("Company X".to_string()),
                vaults: vec!["team".to_string()],
                providers: vec!["opencode".to_string()],
                ..ContextConfig::default()
            },
        );
        let ctx_store = FakeCtxStore::with(file);
        let config_store = FakeStore::new();
        let mut sink = CollectingSink;
        let result = run(
            &ContextId::new("company-x"),
            false,
            &ctx_store,
            &mut sink,
            &config_store,
        );
        assert!(result.is_ok());
        assert_eq!(ctx_store.current_context().unwrap().as_str(), "company-x");

        let config = config_store.load(Scope::Global).unwrap();
        assert!(config.vaults.contains(&"team".to_string()));
        assert!(config.providers.contains(&"opencode".to_string()));
    }

    #[test]
    fn switch_unknown_context_errors() {
        let file = ContextFile::default();
        let ctx_store = FakeCtxStore::with(file);
        let config_store = FakeStore::new();
        let mut sink = CollectingSink;
        let result = run(
            &ContextId::new("missing"),
            false,
            &ctx_store,
            &mut sink,
            &config_store,
        );
        assert!(result.is_err(), "switching to a missing context must error");
        // Context remains default
        assert_eq!(ctx_store.current_context().unwrap().as_str(), "default");
    }

    #[test]
    fn switch_context_dry_run_no_change() {
        let mut file = ContextFile::default();
        file.ensure_default();
        file.contexts.insert(
            "team".to_string(),
            ContextConfig {
                display_name: Some("Team".to_string()),
                ..ContextConfig::default()
            },
        );
        let ctx_store = FakeCtxStore::with(file);
        let config_store = FakeStore::new();
        let mut sink = CollectingSink;
        let result = run(
            &ContextId::new("team"),
            true,
            &ctx_store,
            &mut sink,
            &config_store,
        );
        assert!(result.is_ok());
        assert_eq!(ctx_store.current_context().unwrap().as_str(), "default");
    }

    #[test]
    fn switch_context_save_failure_returns_err() {
        // Regression: a config-save failure during `agk context switch` must
        // propagate as `Err` (exit non-zero) instead of being silently dropped
        // via `let _ = store.save(...)` while `Switched to context ...` is
        // reported as success.
        let mut file = ContextFile::default();
        file.ensure_default();
        file.contexts.insert(
            "company-x".to_string(),
            ContextConfig {
                display_name: Some("Company X".to_string()),
                vaults: vec!["team".to_string()],
                providers: vec!["opencode".to_string()],
                ..ContextConfig::default()
            },
        );
        let ctx_store = FakeCtxStore::with(file);
        // Reuse FakeStore but flip save to fail. We wrap it in a store whose
        // `save` always errors.
        struct FailingSaveStore;
        impl ConfigStorePort for FailingSaveStore {
            fn load(&self, _scope: Scope) -> anyhow::Result<ConfigFile> {
                Ok(ConfigFile::default())
            }
            fn save(&self, _scope: Scope, _config: &ConfigFile) -> anyhow::Result<()> {
                anyhow::bail!("disk full")
            }
        }
        let config_store = FailingSaveStore;
        let mut sink = CollectingSink;
        let result = run(
            &ContextId::new("company-x"),
            false,
            &ctx_store,
            &mut sink,
            &config_store,
        );
        assert!(result.is_err(), "a save failure must surface as an error");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("disk full"),
            "error should carry the underlying save error, got: {msg}"
        );
    }
}
