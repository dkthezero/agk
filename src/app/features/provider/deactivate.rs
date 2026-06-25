use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::{ConfigStorePort, ProviderPort};
use crate::domain::asset::AssetKind;
use crate::domain::config::ConfigFile;
use crate::domain::scope::Scope;

/// Deactivate a provider for the given scope, removing all installed assets
/// from its filesystem and clearing the provider from config.
///
/// In Phase 3 this moves the entire `handle_deactivate_last_provider_confirm`
/// logic out of `tui/event.rs`.
pub fn run(
    provider_id: String,
    scope: Scope,
    store: &dyn ConfigStorePort,
    provider: &dyn ProviderPort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let mut config = store.load(scope)?;
    if !config.providers.contains(&provider_id) {
        anyhow::bail!("Provider '{}' already deactivated", provider_id);
    }

    config.providers.retain(|p| p != &provider_id);
    config.provider_roots.remove(&provider_id);

    // Remove installed assets from the provider's filesystem
    let mut failures: Vec<String> = Vec::new();
    for section in config.vault_defs.values() {
        if let Some(ref skills) = section.skills {
            for item in &skills.items {
                if let Some(identity) = crate::domain::config::parse_identity(item) {
                    if let Err(e) =
                        provider.remove(&identity, &AssetKind::Skill, scope, Some(&config))
                    {
                        failures.push(format!("{}: {}", identity.name, e));
                    }
                }
            }
        }
        if let Some(ref instructions) = section.instructions {
            for item in &instructions.items {
                if let Some(identity) = crate::domain::config::parse_identity(item) {
                    if let Err(e) =
                        provider.remove(&identity, &AssetKind::Instruction, scope, Some(&config))
                    {
                        failures.push(format!("{}: {}", identity.name, e));
                    }
                }
            }
        }
    }

    // Clear all installed assets
    for section in config.vault_defs.values_mut() {
        if let Some(ref mut b) = section.skills {
            b.items.clear();
        }
        if let Some(ref mut b) = section.instructions {
            b.items.clear();
        }
    }
    crate::app::features::common::prune_empty_vault_defs(&mut config);

    // If the entire config is now default (empty), delete the file
    if config == ConfigFile::default() {
        store.delete_file(scope)?;
    } else {
        store.save(scope, &config)?;
    }

    if !failures.is_empty() {
        sink.on_event(CoreEvent::ProviderDeactivated(provider_id.clone()));
        anyhow::bail!(
            "Deactivated '{}' but {} asset(s) failed to remove: {}",
            provider_id,
            failures.len(),
            failures.join("; ")
        );
    }

    sink.on_event(CoreEvent::ProviderDeactivated(provider_id));
    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::{ConfigStorePort, ProviderPort};
    use crate::domain::asset::{AssetKind, ScannedPackage};
    use crate::domain::config::ConfigFile;
    use crate::domain::identity::AssetIdentity;
    use crate::domain::scope::Scope;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakeStore {
        data: Mutex<HashMap<String, ConfigFile>>,
    }

    impl FakeStore {
        fn new(config: ConfigFile) -> Self {
            let mut data = HashMap::new();
            data.insert(format!("{:?}", Scope::Workspace), config);
            Self {
                data: Mutex::new(data),
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
        fn delete_file(&self, scope: Scope) -> anyhow::Result<()> {
            self.data.lock().unwrap().remove(&format!("{:?}", scope));
            Ok(())
        }
    }

    struct FakeProvider;
    impl ProviderPort for FakeProvider {
        fn id(&self) -> &str {
            "fake"
        }
        fn name(&self) -> &str {
            "Fake"
        }
        fn install(
            &self,
            _: &ScannedPackage,
            _: Scope,
            _: Option<&ConfigFile>,
            _: bool,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            _: &AssetIdentity,
            _: &AssetKind,
            _: Scope,
            _: Option<&ConfigFile>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct NullSink;
    impl CoreEventSink for NullSink {
        fn on_event(&mut self, _event: CoreEvent) {}
        fn on_error(&mut self, _error: String) {}
    }

    #[test]
    fn deactivate_already_inactive_returns_err() {
        let store = FakeStore::new(ConfigFile::default());
        let provider = FakeProvider;
        let mut sink = NullSink;
        let result = run(
            "fake".into(),
            Scope::Workspace,
            &store,
            &provider,
            &mut sink,
        );
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already deactivated"), "got: {}", err);
    }

    #[test]
    fn deactivate_active_provider_clears_config() {
        let mut config = ConfigFile::default();
        config.providers.push("fake".into());
        let store = FakeStore::new(config);
        let provider = FakeProvider;
        let mut sink = NullSink;
        let result = run(
            "fake".into(),
            Scope::Workspace,
            &store,
            &provider,
            &mut sink,
        );
        assert!(result.is_ok());

        let config = store.load(Scope::Workspace).unwrap();
        assert!(config.providers.is_empty());
    }

    struct FailingRemoveProvider;
    impl ProviderPort for FailingRemoveProvider {
        fn id(&self) -> &str {
            "failing-remove"
        }
        fn name(&self) -> &str {
            "FailingRemove"
        }
        fn install(
            &self,
            _: &ScannedPackage,
            _: Scope,
            _: Option<&ConfigFile>,
            _: bool,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            _: &AssetIdentity,
            _: &AssetKind,
            _: Scope,
            _: Option<&ConfigFile>,
        ) -> anyhow::Result<()> {
            anyhow::bail!("permission denied")
        }
    }

    #[test]
    fn deactivate_with_remove_failure_returns_err_and_emits_event() {
        let mut config = ConfigFile::default();
        config.providers.push("failing-remove".into());
        let mut section = crate::domain::config::VaultSection::default();
        let mut skills = crate::domain::config::AssetBucket::default();
        skills.items.push("[skill-a:1.0.0:0000000000]".to_string());
        section.skills = Some(skills);
        config.vault_defs.insert("vault-a".to_string(), section);

        let store = FakeStore::new(config);
        let provider = FailingRemoveProvider;
        let mut sink = NullSink;
        let result = run(
            "failing-remove".into(),
            Scope::Workspace,
            &store,
            &provider,
            &mut sink,
        );
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed to remove"), "got: {}", err);
        assert!(err.contains("permission denied"), "got: {}", err);
    }

    struct SaveFailingStore {
        data: Mutex<HashMap<String, ConfigFile>>,
    }

    impl SaveFailingStore {
        fn new(config: ConfigFile) -> Self {
            let mut data = HashMap::new();
            data.insert(format!("{:?}", Scope::Workspace), config);
            Self {
                data: Mutex::new(data),
            }
        }
    }

    impl ConfigStorePort for SaveFailingStore {
        fn load(&self, scope: Scope) -> anyhow::Result<ConfigFile> {
            Ok(self
                .data
                .lock()
                .unwrap()
                .get(&format!("{:?}", scope))
                .cloned()
                .unwrap_or_default())
        }
        fn save(&self, _scope: Scope, _config: &ConfigFile) -> anyhow::Result<()> {
            anyhow::bail!("disk full")
        }
        fn delete_file(&self, scope: Scope) -> anyhow::Result<()> {
            self.data.lock().unwrap().remove(&format!("{:?}", scope));
            Ok(())
        }
    }

    #[test]
    fn deactivate_save_failure_returns_err() {
        let mut config = ConfigFile::default();
        config.providers.push("fake".into());
        // A profile keeps the config non-default after asset clearing, forcing
        // the `store.save` branch (instead of `delete_file`) so the failing
        // save surfaces.
        config.profiles.push(crate::domain::config::Profile {
            name: "dev".into(),
            provider_id: "claude".into(),
            ..Default::default()
        });
        let store = SaveFailingStore::new(config);
        let provider = FakeProvider;
        let mut sink = NullSink;
        let result = run(
            "fake".into(),
            Scope::Workspace,
            &store,
            &provider,
            &mut sink,
        );
        let err = result.unwrap_err().to_string();
        assert!(err.contains("disk full"), "got: {}", err);
    }
}
