use crate::app::outcome::CoreEventSink;
use crate::app::ports::{ClawHubPort, ConfigStorePort};
use crate::domain::config::VaultConfig;
use crate::domain::scope::Scope;
use anyhow::Result;

/// Attach a vault definition to the global config.
///
/// Phase 3: Uses [`ConfigStorePort`] only — no direct filesystem access.
/// For ClawHub vaults, verifies the CLI is available and attempts
/// Homebrew installation when missing (via [`ClawHubPort`]).
pub fn run(
    vault_id: String,
    vault_config: VaultConfig,
    store: &dyn ConfigStorePort,
    clawhub: &dyn ClawHubPort,
    sink: &mut dyn CoreEventSink,
) -> Result<()> {
    // ClawHub auto-install: if the CLI is missing and Homebrew is present,
    // install automatically before attaching.
    if matches!(vault_config, VaultConfig::Clawhub(_)) && !clawhub.is_cli_available() {
        if clawhub.is_homebrew_available() {
            sink.on_event(crate::app::event::CoreEvent::Info(
                "ClawHub CLI not found — installing via Homebrew...".to_string(),
            ));
            if let Err(e) = clawhub.install_cli() {
                anyhow::bail!(
                    "ClawHub CLI is required but installation failed: {}. \
                     Install manually from https://clawhub.ai",
                    e
                );
            }
        } else {
            anyhow::bail!(
                "ClawHub CLI not found and Homebrew is unavailable. \
                 Install manually from https://clawhub.ai"
            );
        }
    }

    let mut config = store.load(Scope::Global)?;
    if !config.vaults.contains(&vault_id) {
        config.vaults.push(vault_id.clone());
    }
    let section = config.vault_defs.entry(vault_id.clone()).or_default();
    section.vault = Some(vault_config);
    store.save(Scope::Global, &config)?;
    sink.on_event(crate::app::event::CoreEvent::VaultAttached(vault_id));
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
        let clawhub = crate::app::test_support::FakeClawHub::new();
        let vault_config = VaultConfig::Local(crate::domain::config::LocalVaultSource {
            path: "/tmp/vault".into(),
        });
        run("my-vault".into(), vault_config, &store, &clawhub, &mut sink).unwrap();

        let config = store.load(Scope::Global).unwrap();
        assert!(config.vaults.contains(&"my-vault".to_string()));
        assert!(config.vault_defs.contains_key("my-vault"));
    }
}
