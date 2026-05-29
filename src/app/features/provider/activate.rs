use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::{ConfigStorePort, ProviderPort};
use crate::app::registry::Registry;
use crate::domain::scope::Scope;

/// Activate a provider for the given scope and install all currently tracked
/// assets into it.
///
/// If the provider is already active this is a no-op (emits an error).
pub fn run(
    provider_id: String,
    scope: Scope,
    store: &dyn ConfigStorePort,
    provider: &dyn ProviderPort,
    registry: &Registry,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let mut config = store.load(scope)?;
    if config.providers.contains(&provider_id) {
        sink.on_error(format!("Provider '{}' already activated", provider_id));
        return Ok(CoreOutcome::Ok);
    }

    config.providers.push(provider_id.clone());
    store.save(scope, &config)?;

    sink.on_event(CoreEvent::TaskStarted {
        id: 0,
        name: format!("Activating '{}'", provider_id),
    });

    let mut total = 0usize;
    for section in config.vault_defs.values() {
        total += section.skills.as_ref().map(|b| b.items.len()).unwrap_or(0);
        total += section
            .instructions
            .as_ref()
            .map(|b| b.items.len())
            .unwrap_or(0);
    }

    let mut current = 0usize;
    for (vault_id, section) in &config.vault_defs {
        if let Some(ref skills) = section.skills {
            for item in &skills.items {
                current += 1;
                if let Some(identity) = crate::domain::config::parse_identity(item) {
                    let hint = format!("{}/{}", vault_id, identity.name);
                    if let Ok(Some(pkg)) = registry.find_package_by_identity(&hint) {
                        let _ = crate::app::features::asset::install::install_asset(
                            scope, &pkg, store, provider,
                        );
                    }
                }
                let percent = ((current as f32 / total.max(1) as f32) * 100.0) as u8;
                sink.on_event(CoreEvent::TaskProgress { id: 0, percent });
            }
        }
        if let Some(ref instructions) = section.instructions {
            for item in &instructions.items {
                current += 1;
                if let Some(identity) = crate::domain::config::parse_identity(item) {
                    let hint = format!("{}/{}", vault_id, identity.name);
                    if let Ok(Some(pkg)) = registry.find_package_by_identity(&hint) {
                        let _ = crate::app::features::asset::install::install_asset(
                            scope, &pkg, store, provider,
                        );
                    }
                }
                let percent = ((current as f32 / total.max(1) as f32) * 100.0) as u8;
                sink.on_event(CoreEvent::TaskProgress { id: 0, percent });
            }
        }
    }

    sink.on_event(CoreEvent::ProviderActivated(provider_id));
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
            _pkg: &ScannedPackage,
            _scope: Scope,
            _config: Option<&ConfigFile>,
            _include_evals: bool,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            _identity: &AssetIdentity,
            _kind: &AssetKind,
            _scope: Scope,
            _config: Option<&ConfigFile>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct NullSink;
    impl crate::app::outcome::CoreEventSink for NullSink {
        fn on_event(&mut self, _event: crate::app::event::CoreEvent) {}
        fn on_error(&mut self, _error: String) {}
    }

    #[test]
    fn activate_already_active_is_noop() {
        let mut config = ConfigFile::default();
        config.providers.push("fake".into());
        let store = FakeStore::new(config);
        let mut registry = Registry::new();
        registry.register_provider(Box::new(FakeProvider));
        let mut sink = NullSink;
        let result = run(
            "fake".into(),
            Scope::Workspace,
            &store,
            &FakeProvider,
            &registry,
            &mut sink,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn activate_inactive_provider_adds_to_config() {
        let store = FakeStore::new(ConfigFile::default());
        let mut registry = Registry::new();
        registry.register_provider(Box::new(FakeProvider));
        let mut sink = NullSink;
        let result = run(
            "fake".into(),
            Scope::Workspace,
            &store,
            &FakeProvider,
            &registry,
            &mut sink,
        );
        assert!(result.is_ok());

        let config = store.load(Scope::Workspace).unwrap();
        assert_eq!(config.providers, vec!["fake"]);
    }
}
