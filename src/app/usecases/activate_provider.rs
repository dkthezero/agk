use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ConfigStorePort;
use crate::domain::scope::Scope;

/// Activate a provider for the given scope.
///
/// Adds the provider ID to the scope's config.  Asset installation into
/// the newly-activated provider is the responsibility of a separate
/// `SyncAssets` or `InstallAsset` command so that activation stays atomic.
///
/// If the provider is already active this is a no-op (emits an error).
pub fn run(
    provider_id: String,
    scope: Scope,
    store: &dyn ConfigStorePort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let mut config = store.load(scope)?;
    if config.providers.contains(&provider_id) {
        sink.on_error(format!("Provider '{}' already activated", provider_id));
        return Ok(CoreOutcome::Ok);
    }

    config.providers.push(provider_id.clone());
    store.save(scope, &config)?;

    sink.on_event(CoreEvent::ProviderActivated(provider_id));
    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::ConfigStorePort;
    use crate::domain::config::ConfigFile;
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
        let mut sink = NullSink;
        let result = run("fake".into(), Scope::Workspace, &store, &mut sink);
        assert!(result.is_ok());
    }

    #[test]
    fn activate_inactive_provider_adds_to_config() {
        let store = FakeStore::new(ConfigFile::default());
        let mut sink = NullSink;
        let result = run("fake".into(), Scope::Workspace, &store, &mut sink);
        assert!(result.is_ok());

        let config = store.load(Scope::Workspace).unwrap();
        assert_eq!(config.providers, vec!["fake"]);
    }
}
