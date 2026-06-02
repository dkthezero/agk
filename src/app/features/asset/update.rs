use crate::app::ports::{ConfigStorePort, ProviderPort};
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::scope::Scope;
use anyhow::Result;

/// Update an installed asset: remove old identity, reinstall from scanned package.
pub fn update_asset(
    scope: Scope,
    pkg: &ScannedPackage,
    store: &dyn ConfigStorePort,
    provider: &dyn ProviderPort,
) -> Result<()> {
    let mut config = store.load(scope)?;
    if let Some(section) = config.vault_defs.get_mut(&pkg.vault_id) {
        let name = &pkg.identity.name;
        match pkg.kind {
            AssetKind::Skill => {
                if let Some(bucket) = section.skills.as_mut() {
                    bucket.items.retain(|s| {
                        crate::domain::config::parse_identity(s)
                            .map(|id| id.name != *name)
                            .unwrap_or(true)
                    });
                }
            }
            AssetKind::Instruction => {
                if let Some(bucket) = section.instructions.as_mut() {
                    bucket.items.retain(|s| {
                        crate::domain::config::parse_identity(s)
                            .map(|id| id.name != *name)
                            .unwrap_or(true)
                    });
                }
            }
            AssetKind::McpServer => {
                if let Some(bucket) = section.mcps.as_mut() {
                    bucket.items.retain(|s| {
                        crate::domain::config::parse_identity(s)
                            .map(|id| id.name != *name)
                            .unwrap_or(true)
                    });
                }
            }
            AssetKind::Profile => {
                if let Some(bucket) = section.profiles.as_mut() {
                    bucket.items.retain(|s| {
                        crate::domain::config::parse_identity(s)
                            .map(|id| id.name != *name)
                            .unwrap_or(true)
                    });
                }
            }
        }
    }
    store.save(scope, &config)?;
    crate::app::features::asset::install::install_asset(scope, pkg, store, provider)
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
        ) -> Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            _identity: &AssetIdentity,
            _kind: &AssetKind,
            _scope: Scope,
            _config: Option<&ConfigFile>,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn update_asset_replaces_identity_in_config() {
        let store = FakeStore::default();
        let provider = FakeProvider;

        let mut config = ConfigFile {
            providers: vec!["fake".to_string()],
            ..Default::default()
        };
        config.vault_defs.insert(
            "workspace".to_string(),
            VaultSection {
                vault: None,
                skills: Some(AssetBucket {
                    items: vec!["[my-skill:--:old_sha_old]".to_string()],
                    source: None,
                }),
                instructions: None,
                mcps: None,
                profiles: None,
            },
        );
        store.save(Scope::Workspace, &config).unwrap();

        let pkg = ScannedPackage {
            identity: AssetIdentity::new("my-skill", None, "new_sha_new"),
            path: std::path::PathBuf::from("/fake"),
            vault_id: "workspace".to_string(),
            kind: AssetKind::Skill,
            is_remote: false,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
            include_evals: false,
        };
        update_asset(Scope::Workspace, &pkg, &store, &provider).unwrap();

        let loaded = store.load(Scope::Workspace).unwrap();
        let skills = loaded.installed_skills("workspace");
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].sha10, "new_sha_new");
    }
}
