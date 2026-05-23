use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::config::ConfigFile;
use crate::domain::identity::AssetIdentity;
use crate::domain::mcp::McpServer;
use crate::domain::scope::Scope;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub trait FeatureSetPort: Send + Sync {
    fn kind_name(&self) -> &str;
    fn display_name(&self) -> &str;
    fn scan_root(&self) -> &str;
    fn asset_kind(&self) -> AssetKind;
    fn is_package(&self, path: &Path) -> bool;
    fn hash_files(&self, path: &Path) -> Vec<PathBuf>;

    fn extract_version(&self, _path: &Path) -> Option<String> {
        None
    }

    /// Override to return `true` for placeholder tabs not yet implemented.
    fn is_stub(&self) -> bool {
        false
    }
}

#[async_trait::async_trait]
pub trait VaultPort: Send + Sync {
    fn id(&self) -> &str;
    #[allow(dead_code)]
    fn kind_name(&self) -> &str;

    async fn refresh(&self) -> Result<()> {
        Ok(())
    }

    fn list_packages(&self, feature: &dyn FeatureSetPort) -> Result<Vec<ScannedPackage>>;
}

pub trait ConfigStorePort: Send + Sync {
    fn load(&self, scope: Scope) -> Result<ConfigFile>;
    fn save(&self, scope: Scope, config: &ConfigFile) -> Result<()>;
    /// Delete the backing file for the scope if it exists.
    /// Default no-op for stores that don't have a file.
    fn delete_file(&self, _scope: Scope) -> Result<()> {
        Ok(())
    }
}

pub trait ProviderPort: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn install(
        &self,
        pkg: &ScannedPackage,
        scope: Scope,
        config: Option<&ConfigFile>,
    ) -> Result<()>;
    fn remove(
        &self,
        identity: &AssetIdentity,
        kind: &AssetKind,
        scope: Scope,
        config: Option<&ConfigFile>,
    ) -> Result<()>;

    /// Return the expected on-disk install path for the given asset, if known.
    /// Defaults to `None` for providers where the path convention is not exposed.
    fn install_path_for(
        &self,
        _identity: &AssetIdentity,
        _kind: &AssetKind,
        _scope: Scope,
    ) -> Option<PathBuf> {
        None
    }

    /// Return a list of alternative config root folder names this provider
    /// supports. Each entry is (folder_name, description).
    /// Default empty vec means the provider has a single hardcoded root.
    fn available_config_roots(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// Extension trait for providers that support MCP configuration.
pub trait McpProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn supports_mcp(&self) -> bool;
    #[allow(dead_code)]
    fn mcp_config_path(&self, scope: Scope) -> Option<PathBuf>;
    fn write_mcp_server(&self, server: &McpServer, scope: Scope) -> Result<()>;
    fn remove_mcp_server(&self, name: &str, scope: Scope) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestFeatureSet;
    impl FeatureSetPort for TestFeatureSet {
        fn kind_name(&self) -> &str {
            "test"
        }
        fn display_name(&self) -> &str {
            "Test"
        }
        fn scan_root(&self) -> &str {
            "test_root"
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
    fn feature_set_port_default_not_stub() {
        let f = TestFeatureSet;
        assert!(!f.is_stub());
    }

    #[test]
    fn feature_set_port_kind_name() {
        let f = TestFeatureSet;
        assert_eq!(f.kind_name(), "test");
    }

    struct DummyProvider;
    impl ProviderPort for DummyProvider {
        fn id(&self) -> &str {
            "dummy"
        }
        fn name(&self) -> &str {
            "Dummy"
        }
        fn install(&self, _: &ScannedPackage, _: Scope, _: Option<&ConfigFile>) -> Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            _: &AssetIdentity,
            _: &AssetKind,
            _: Scope,
            _: Option<&ConfigFile>,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn provider_port_default_available_roots_empty() {
        let p = DummyProvider;
        assert!(p.available_config_roots().is_empty());
    }
}
