use crate::app::ports::{ConfigStorePort, ProviderPort};
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::config::AssetBucket;
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

/// Register a provider in the scope's config and copy all checked assets into it.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::asset::{AssetKind, ScannedPackage};
    use crate::domain::config::ConfigFile;
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
        installed: Mutex<Vec<String>>,
    }
    impl FakeProvider {
        fn new() -> Self {
            Self {
                installed: Mutex::new(vec![]),
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
            _identity: &AssetIdentity,
            _kind: &AssetKind,
            _scope: Scope,
            _config: Option<&ConfigFile>,
        ) -> Result<()> {
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
}
