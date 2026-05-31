use crate::app::ports::{ConfigStorePort, ProviderPort};
use crate::domain::asset::AssetKind;
use crate::domain::identity::AssetIdentity;
use crate::domain::scope::Scope;
use anyhow::Result;

/// Remove an installed asset from the provider and config for the given scope.
pub fn remove_asset(
    scope: Scope,
    identity: &AssetIdentity,
    kind: &AssetKind,
    vault_id: &str,
    store: &dyn ConfigStorePort,
    provider: &dyn ProviderPort,
) -> Result<()> {
    let mut config = store.load(scope)?;
    provider.remove(identity, kind, scope, Some(&config))?;
    if let Some(section) = config.vault_defs.get_mut(vault_id) {
        let identity_str = identity.to_config_string();
        match kind {
            AssetKind::Skill => {
                if let Some(bucket) = section.skills.as_mut() {
                    bucket.items.retain(|s| s != &identity_str);
                }
            }
            AssetKind::Instruction => {
                if let Some(bucket) = section.instructions.as_mut() {
                    bucket.items.retain(|s| s != &identity_str);
                }
            }
            &AssetKind::McpServer => {
                if let Some(bucket) = section.mcps.as_mut() {
                    bucket.items.retain(|s| s != &identity_str);
                }
            }
            &AssetKind::Profile => {
                if let Some(bucket) = section.profiles.as_mut() {
                    bucket.items.retain(|s| s != &identity_str);
                }
            }
        }
    }
    crate::app::features::common::prune_empty_vault_defs(&mut config);
    store.save(scope, &config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::asset::{AssetKind, ScannedPackage};
    use crate::domain::config::{AssetBucket, ConfigFile, VaultSection};
    use crate::domain::identity::AssetIdentity;
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

    struct FakeProvider {
        removed: Mutex<Vec<String>>,
    }
    impl FakeProvider {
        fn new() -> Self {
            Self {
                removed: Mutex::new(vec![]),
            }
        }
    }
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
        ) -> Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            identity: &AssetIdentity,
            _kind: &AssetKind,
            _scope: Scope,
            _config: Option<&ConfigFile>,
        ) -> Result<()> {
            self.removed.lock().unwrap().push(identity.name.clone());
            Ok(())
        }
    }

    #[test]
    fn remove_asset_removes_from_config_and_calls_provider() {
        let store = FakeStore::default();
        let provider = FakeProvider::new();
        let mut config = ConfigFile {
            providers: vec!["fake".to_string()],
            ..Default::default()
        };
        config.vault_defs.insert(
            "workspace".to_string(),
            VaultSection {
                vault: None,
                skills: Some(AssetBucket {
                    items: vec!["[my-skill:--:0000000000]".to_string()],
                }),
                instructions: None,
                mcps: None,
                profiles: None,
            },
        );
        store.save(Scope::Workspace, &config).unwrap();

        let identity = AssetIdentity::new("my-skill", None, "0000000000");
        remove_asset(
            Scope::Workspace,
            &identity,
            &AssetKind::Skill,
            "workspace",
            &store,
            &provider,
        )
        .unwrap();

        assert!(provider
            .removed
            .lock()
            .unwrap()
            .contains(&"my-skill".to_string()));
        let loaded = store.load(Scope::Workspace).unwrap();
        assert!(!loaded.is_skill_installed("workspace", "my-skill"));
    }

    #[test]
    fn remove_asset_prunes_empty_section() {
        let store = FakeStore::default();
        let provider = FakeProvider::new();
        let mut config = ConfigFile {
            providers: vec!["fake".to_string()],
            ..Default::default()
        };
        config.vault_defs.insert(
            "workspace".to_string(),
            VaultSection {
                vault: None,
                skills: Some(AssetBucket {
                    items: vec!["[my-skill:--:0000000000]".to_string()],
                }),
                instructions: None,
                mcps: None,
                profiles: None,
            },
        );
        store.save(Scope::Workspace, &config).unwrap();

        let identity = AssetIdentity::new("my-skill", None, "0000000000");
        remove_asset(
            Scope::Workspace,
            &identity,
            &AssetKind::Skill,
            "workspace",
            &store,
            &provider,
        )
        .unwrap();

        let loaded = store.load(Scope::Workspace).unwrap();
        assert!(loaded.vault_defs.is_empty());
    }
}
