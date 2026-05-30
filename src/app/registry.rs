use crate::app::ports::{FeatureSetPort, ProviderPort, VaultPort};

pub struct Registry {
    pub feature_sets: Vec<Box<dyn FeatureSetPort>>,
    pub vaults: Vec<Box<dyn VaultPort>>,
    pub providers: Vec<Box<dyn ProviderPort>>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    pub fn new() -> Self {
        Self {
            feature_sets: Vec::new(),
            vaults: Vec::new(),
            providers: Vec::new(),
        }
    }

    pub fn register_feature_set(&mut self, fs: Box<dyn FeatureSetPort>) {
        self.feature_sets.push(fs);
    }

    pub fn register_provider(&mut self, provider: Box<dyn ProviderPort>) {
        self.providers.push(provider);
    }

    pub fn register_vault(&mut self, vault: Box<dyn VaultPort>) {
        self.vaults.push(vault);
    }

    pub fn get_provider(&self, id: &str) -> anyhow::Result<&dyn ProviderPort> {
        for p in &self.providers {
            if p.id() == id {
                return Ok(p.as_ref());
            }
        }
        anyhow::bail!("Provider {} not found", id)
    }

    /// Find a package by its full identity string (e.g. "vault/name:version" or "name").
    pub fn find_package_by_identity(
        &self,
        identity_str: &str,
    ) -> anyhow::Result<Option<crate::domain::asset::ScannedPackage>> {
        let parts: Vec<&str> = identity_str.split('/').collect();
        let (vault_hint, name_part) = if parts.len() == 2 {
            (Some(parts[0]), parts[1])
        } else {
            (None, identity_str)
        };

        let name = name_part.split(':').next().unwrap_or(name_part);

        for vault in &self.vaults {
            if let Some(hint) = vault_hint {
                if vault.id() != hint {
                    continue;
                }
            }
            for feature in &self.feature_sets {
                let pkgs = vault.list_packages(feature.as_ref())?;
                for pkg in pkgs {
                    if pkg.identity.name == name {
                        return Ok(Some(pkg));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Return all active providers that are listed in the given config.
    pub fn active_providers_from_config(
        &self,
        config: &crate::domain::config::ConfigFile,
    ) -> Vec<&dyn ProviderPort> {
        self.providers
            .iter()
            .filter(|p| config.providers.contains(&p.id().to_string()))
            .map(|p| p.as_ref())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::FeatureSetPort;
    use crate::domain::asset::AssetKind;
    use std::path::{Path, PathBuf};

    struct FakeFeature(&'static str);
    impl FeatureSetPort for FakeFeature {
        fn kind_name(&self) -> &str {
            self.0
        }
        fn display_name(&self) -> &str {
            self.0
        }
        fn scan_root(&self) -> &str {
            ""
        }
        fn asset_kind(&self) -> AssetKind {
            AssetKind::Skill
        }
        fn is_package(&self, _: &Path) -> bool {
            false
        }
        fn hash_files(&self, _: &Path) -> Vec<PathBuf> {
            vec![]
        }
    }

    #[test]
    fn registry_starts_empty() {
        let r = Registry::new();
        assert!(r.feature_sets.is_empty());
        assert!(r.vaults.is_empty());
        assert!(r.providers.is_empty());
    }

    #[test]
    fn registry_register_feature_set() {
        let mut r = Registry::new();
        r.register_feature_set(Box::new(FakeFeature("skill")));
        r.register_feature_set(Box::new(FakeFeature("instruction")));
        assert_eq!(r.feature_sets.len(), 2);
        assert_eq!(r.feature_sets[0].kind_name(), "skill");
    }
}
