use crate::app::ports::{ConfigStorePort, ProviderPort};
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::config::{AssetBucket, ConfigFile, VaultConfig};
use crate::domain::identity::AssetIdentity;
use crate::domain::scope::Scope;
use anyhow::{bail, Result};

/// Install a scanned package into the active provider for the given scope.
/// Returns Err if no provider is configured for that scope.
pub fn install_asset(
    scope: Scope,
    pkg: &ScannedPackage,
    store: &dyn ConfigStorePort,
    provider: &dyn ProviderPort,
) -> Result<()> {
    let mut config = store.load(scope)?;
    if config.providers.is_empty() {
        bail!("No provider configured for {:?} scope", scope);
    }
    provider.install(pkg, scope, Some(&config), pkg.include_evals)?;
    let section = config.vault_defs.entry(pkg.vault_id.clone()).or_default();
    let identity_str = pkg.identity.to_config_string();
    match pkg.kind {
        AssetKind::Skill => {
            let bucket = section.skills.get_or_insert_with(AssetBucket::default);
            if !bucket.items.contains(&identity_str) {
                bucket.items.push(identity_str);
            }
        }
        AssetKind::Instruction => {
            let bucket = section
                .instructions
                .get_or_insert_with(AssetBucket::default);
            if !bucket.items.contains(&identity_str) {
                bucket.items.push(identity_str);
            }
        }
        AssetKind::McpServer => {}
    }
    store.save(scope, &config)
}

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
            &AssetKind::McpServer => {}
        }
    }
    prune_empty_vault_defs(&mut config);
    store.save(scope, &config)
}

/// Attach a vault to the global config.
pub fn attach_vault(
    vault_id: String,
    vault_config: VaultConfig,
    store: &dyn ConfigStorePort,
) -> Result<()> {
    let mut config = store.load(Scope::Global)?;
    if !config.vaults.contains(&vault_id) {
        config.vaults.push(vault_id.clone());
    }
    let section = config.vault_defs.entry(vault_id).or_default();
    section.vault = Some(vault_config);
    store.save(Scope::Global, &config)
}

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
            AssetKind::McpServer => {}
        }
    }
    store.save(scope, &config)?;
    install_asset(scope, pkg, store, provider)
}

/// Register a provider in the scope's config and copy all checked assets into it.
#[allow(dead_code)]
pub fn install_provider(
    scope: Scope,
    provider_id: &str,
    checked_pkgs: &[ScannedPackage],
    store: &dyn ConfigStorePort,
    provider: &dyn ProviderPort,
) -> Result<()> {
    let mut config = store.load(scope)?;
    if !config.providers.contains(&provider_id.to_string()) {
        config.providers.push(provider_id.to_string());
    }
    store.save(scope, &config)?;
    for pkg in checked_pkgs {
        install_asset(scope, pkg, store, provider)?;
    }
    Ok(())
}

/// Remove a provider from the scope's config.
#[allow(dead_code)]
pub fn remove_provider(scope: Scope, provider_id: &str, store: &dyn ConfigStorePort) -> Result<()> {
    let mut config = store.load(scope)?;
    config.providers.retain(|p| p != provider_id);
    store.save(scope, &config)
}

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
    use crate::domain::asset::AssetKind;
    use crate::domain::config::{AssetBucket, ConfigFile, VaultSection};
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
    impl ProviderPort for FakeProvider {
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
        let mut config = ConfigFile::default();
        config.providers = vec!["fake".to_string()];
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
        let mut config = ConfigFile::default();
        config.providers = vec!["fake".to_string()];
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
    fn attach_vault_adds_to_vaults_list() {
        let store = FakeStore::default();
        attach_vault(
            "my-vault".to_string(),
            VaultConfig::Local(crate::domain::config::LocalVaultSource { path: ".".into() }),
            &store,
        )
        .unwrap();
        let config = store.load(Scope::Global).unwrap();
        assert_eq!(config.vaults, vec!["my-vault"]);
        assert!(config.vault_defs.contains_key("my-vault"));
    }

    #[test]
    fn update_asset_replaces_identity_in_config() {
        let store = FakeStore::default();
        let provider = FakeProvider::new();

        let mut config = ConfigFile::default();
        config.providers = vec!["fake".to_string()];
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
        let mut config = ConfigFile::default();
        config.vaults = vec!["workspace".to_string()];
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
        let mut config = ConfigFile::default();
        config.vaults = vec!["workspace".to_string()];
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
        let mut config = ConfigFile::default();
        config.providers = vec!["fake".to_string()];
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
                vault: Some(VaultConfig::Local(
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
