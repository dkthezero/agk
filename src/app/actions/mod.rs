pub mod install;
pub mod remove;
pub mod sync;

pub use install::{install_asset, update_asset};
pub use remove::{detach_vault, remove_asset};
pub use sync::attach_vault;

use crate::domain::config::ConfigFile;

/// Remove empty vault sections / asset buckets so the TOML stays clean.
pub fn prune_empty_vault_defs(config: &mut ConfigFile) {
    config.vault_defs.retain(|_id, section| {
        let has_vault = section.vault.is_some();
        let has_skills = section
            .skills
            .as_ref()
            .map(|b| !b.items.is_empty())
            .unwrap_or(false);
        let has_instructions = section
            .instructions
            .as_ref()
            .map(|b| !b.items.is_empty())
            .unwrap_or(false);
        has_vault || has_skills || has_instructions
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::ConfigStorePort;
    use crate::domain::asset::{AssetKind, ScannedPackage};
    use crate::domain::config::{AssetBucket, ConfigFile, VaultSection};
    use crate::domain::identity::AssetIdentity;
    use crate::domain::scope::Scope;
    use anyhow::Result;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // --- Fake store ---
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

    // --- Fake provider ---
    struct FakeProvider {
        installed: Mutex<Vec<String>>,
        removed: Mutex<Vec<String>>,
    }
    impl FakeProvider {
        fn new() -> Self {
            Self {
                installed: Mutex::new(vec![]),
                removed: Mutex::new(vec![]),
            }
        }
    }
    impl crate::app::ports::ProviderPort for FakeProvider {
        fn id(&self) -> &str {
            "fake"
        }
        fn name(&self) -> &str {
            "Fake"
        }
        fn install(
            &self,
            pkg: &ScannedPackage,
            _scope: Scope,
            _config: Option<&ConfigFile>,
            _include_evals: bool,
        ) -> Result<()> {
            self.installed
                .lock()
                .unwrap()
                .push(pkg.identity.name.clone());
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

    fn make_pkg(name: &str, kind: AssetKind) -> ScannedPackage {
        ScannedPackage {
            identity: AssetIdentity::new(name, None, "0000000000"),
            path: std::path::PathBuf::from("/fake"),
            vault_id: "workspace".to_string(),
            kind,
            is_remote: false,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
            include_evals: false,
        }
    }

    #[test]
    fn install_asset_fails_without_provider() {
        let store = FakeStore::default();
        let provider = FakeProvider::new();
        let pkg = make_pkg("my-skill", AssetKind::Skill);
        let result = install_asset(Scope::Workspace, &pkg, &store, &provider);
        assert!(result.is_err());
    }

    #[test]
    fn install_asset_writes_to_config_and_calls_provider() {
        let store = FakeStore::default();
        let provider = FakeProvider::new();
        let config = ConfigFile {
            providers: vec!["fake".to_string()],
            ..ConfigFile::default()
        };
        store.save(Scope::Workspace, &config).unwrap();

        let pkg = make_pkg("my-skill", AssetKind::Skill);
        install_asset(Scope::Workspace, &pkg, &store, &provider).unwrap();

        assert!(provider
            .installed
            .lock()
            .unwrap()
            .contains(&"my-skill".to_string()));
        let loaded = store.load(Scope::Workspace).unwrap();
        assert!(loaded.is_skill_installed("workspace", "my-skill"));
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
    fn update_asset_replaces_identity_in_config() {
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
                    items: vec!["[my-skill:--:old_sha_old]".to_string()],
                }),
                instructions: None,
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
            },
        );
        store.save(Scope::Global, &config).unwrap();

        detach_vault("workspace", &store).unwrap();

        let loaded = store.load(Scope::Global).unwrap();
        assert!(loaded.vaults.is_empty());
        // vault_defs preserved because assets are still installed
        assert!(loaded.vault_defs.contains_key("workspace"));
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

    #[test]
    fn prune_empty_vault_defs_keeps_nonempty() {
        let mut config = ConfigFile::default();
        config.vault_defs.insert(
            "a".to_string(),
            VaultSection {
                vault: Some(crate::domain::config::VaultConfig::Local(
                    crate::domain::config::LocalVaultSource {
                        path: "/tmp".into(),
                    },
                )),
                skills: None,
                instructions: None,
            },
        );
        config.vault_defs.insert(
            "b".to_string(),
            VaultSection {
                vault: None,
                skills: Some(AssetBucket { items: vec![] }),
                instructions: None,
            },
        );
        config.vault_defs.insert(
            "c".to_string(),
            VaultSection {
                vault: None,
                skills: None,
                instructions: Some(AssetBucket {
                    items: vec!["[i:--:0000000000]".to_string()],
                }),
            },
        );

        prune_empty_vault_defs(&mut config);

        assert!(config.vault_defs.contains_key("a"));
        assert!(!config.vault_defs.contains_key("b"));
        assert!(config.vault_defs.contains_key("c"));
    }
}
