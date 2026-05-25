use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ConfigStorePort;
use crate::domain::config::VaultConfig;
use crate::domain::scope::Scope;
use anyhow::Result;

/// Attach a vault definition to the global config.
///
/// Phase 3: Uses [`ConfigStorePort`] only — no direct filesystem access.
pub fn run(
    vault_id: String,
    vault_config: VaultConfig,
    store: &dyn ConfigStorePort,
    _sink: &mut dyn CoreEventSink,
) -> Result<()> {
    let mut config = store.load(Scope::Global)?;
    if !config.vaults.contains(&vault_id) {
        config.vaults.push(vault_id.clone());
    }
    let section = config.vault_defs.entry(vault_id.clone()).or_default();
    section.vault = Some(vault_config);
    store.save(Scope::Global, &config)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::ConfigStorePort;
    use crate::domain::config::{ConfigFile, VaultConfig};
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
        fn load(&self, scope: Scope) -> Result<ConfigFile> {
            Ok(self
                .data
                .lock()
                .unwrap()
                .get(&format!("{:?}", scope))
                .cloned()
                .unwrap_or_default())
        }
        fn save(&self, scope: Scope, config: &ConfigFile) -> Result<()> {
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
    fn attach_vault_adds_to_config() {
        let store = FakeStore::new();
        let mut sink = NullSink;
        let vault_config = VaultConfig::Local(crate::domain::config::LocalVaultSource {
            path: "/tmp/vault".into(),
        });
        run("my-vault".into(), vault_config, &store, &mut sink).unwrap();

        let config = store.load(Scope::Global).unwrap();
        assert!(config.vaults.contains(&"my-vault".to_string()));
        assert!(config.vault_defs.contains_key("my-vault"));
    }
}
