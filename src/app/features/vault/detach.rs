use crate::app::ports::ConfigStorePort;
use crate::domain::scope::Scope;
use anyhow::Result;

/// Detach a vault from the global config. Removes from active vaults list.
/// Only removes the vault definition if no installed assets reference it
/// in either global or workspace scope.
pub fn detach_vault(vault_id: &str, store: &dyn ConfigStorePort) -> Result<()> {
    let mut config = store.load(Scope::Global)?;
    config.vaults.retain(|v| v != vault_id);

    let mut has_assets = config.has_installed_assets(vault_id);
    if let Ok(ws_config) = store.load(Scope::Workspace) {
        has_assets = has_assets || ws_config.has_installed_assets(vault_id);
    }
    if !has_assets {
        config.vault_defs.remove(vault_id);
    }

    store.save(Scope::Global, &config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{AssetBucket, ConfigFile, VaultSection};
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeStore(Mutex<HashMap<String, ConfigFile>>);

    impl ConfigStorePort for FakeStore {
        fn load(&self, scope: Scope) -> Result<ConfigFile> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .get(&format!("{:?}", scope))
                .cloned()
                .unwrap_or_default())
        }
        fn save(&self, scope: Scope, config: &ConfigFile) -> Result<()> {
            self.0
                .lock()
                .unwrap()
                .insert(format!("{:?}", scope), config.clone());
            Ok(())
        }
    }

    #[test]
    fn detach_vault_removes_from_vaults_list() {
        let store = FakeStore::default();
        let mut config = ConfigFile {
            vaults: vec!["workspace".to_string()],
            ..Default::default()
        };
        config.vault_defs.insert(
            "workspace".to_string(),
            VaultSection {
                vault: None,
                skills: None,
                instructions: None,
                mcps: None,
                profiles: None,
            },
        );
        store.save(Scope::Global, &config).unwrap();

        detach_vault("workspace", &store).unwrap();

        let loaded = store.load(Scope::Global).unwrap();
        assert!(loaded.vaults.is_empty());
        assert!(loaded.vault_defs.is_empty());
    }

    #[test]
    fn detach_vault_preserves_defs_when_assets_installed() {
        let store = FakeStore::default();
        let mut config = ConfigFile {
            vaults: vec!["workspace".to_string()],
            ..Default::default()
        };
        config.vault_defs.insert(
            "workspace".to_string(),
            VaultSection {
                vault: None,
                skills: Some(AssetBucket {
                    items: vec!["[x:--:0000000000]".to_string()],
                }),
                instructions: None,
                mcps: None,
                profiles: None,
            },
        );
        store.save(Scope::Global, &config).unwrap();

        detach_vault("workspace", &store).unwrap();

        let loaded = store.load(Scope::Global).unwrap();
        assert!(loaded.vaults.is_empty());
        // vault_defs preserved because assets are still installed
        assert!(loaded.vault_defs.contains_key("workspace"));
    }
}
