use crate::app::ports::VaultManifestStorePort;
use crate::domain::vault_manifest::VaultManifest;
use anyhow::Result;
use std::path::PathBuf;

pub struct VaultManifestTomlStore {
    lock: std::sync::Mutex<()>,
}

impl VaultManifestTomlStore {
    pub fn new() -> Self {
        Self {
            lock: std::sync::Mutex::new(()),
        }
    }
}

impl VaultManifestStorePort for VaultManifestTomlStore {
    fn load(&self, path: &PathBuf) -> Result<VaultManifest> {
        let _guard = self
            .lock
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "Vault manifest not found at {}",
                path.display()
            ));
        }
        let content = std::fs::read_to_string(path)?;
        let manifest: VaultManifest = toml::from_str(&content)?;
        Ok(manifest)
    }

    fn save(&self, path: &PathBuf, manifest: &VaultManifest) -> Result<()> {
        let _guard = self
            .lock
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(manifest)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vault_manifest::VaultDependency;

    #[test]
    fn load_missing_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".agk").join("vault.toml");
        let store = VaultManifestTomlStore::new();
        let result = store.load(&path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Vault manifest not found"));
    }

    #[test]
    fn round_trip_minimal_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".agk").join("vault.toml");
        let store = VaultManifestTomlStore::new();

        let manifest = VaultManifest {
            name: "my-vault".to_string(),
            description: None,
            version: None,
            dependencies: vec![],
        };
        store.save(&path, &manifest).unwrap();
        let loaded = store.load(&path).unwrap();
        assert_eq!(loaded, manifest);
    }

    #[test]
    fn round_trip_full_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".agk").join("vault.toml");
        let store = VaultManifestTomlStore::new();

        let manifest = VaultManifest {
            name: "enterprise-vault".to_string(),
            description: Some("Internal enterprise skill vault".to_string()),
            version: Some("2.3.1".to_string()),
            dependencies: vec![
                VaultDependency {
                    identity: "core-lib".to_string(),
                    dep_type: "github".to_string(),
                    url: "https://github.com/acme/core-lib".to_string(),
                },
                VaultDependency {
                    identity: "security-base".to_string(),
                    dep_type: "github".to_string(),
                    url: "https://github.com/acme/security-base".to_string(),
                },
            ],
        };
        store.save(&path, &manifest).unwrap();
        let loaded = store.load(&path).unwrap();
        assert_eq!(loaded, manifest);
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("vault.toml");
        let store = VaultManifestTomlStore::new();

        let manifest = VaultManifest {
            name: "test-vault".to_string(),
            description: None,
            version: None,
            dependencies: vec![],
        };
        store.save(&path, &manifest).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn overwrite_existing_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.toml");
        let store = VaultManifestTomlStore::new();

        let v1 = VaultManifest {
            name: "v1".to_string(),
            description: None,
            version: Some("1.0.0".to_string()),
            dependencies: vec![],
        };
        store.save(&path, &v1).unwrap();

        let v2 = VaultManifest {
            name: "v2".to_string(),
            description: Some("updated".to_string()),
            version: Some("2.0.0".to_string()),
            dependencies: vec![VaultDependency {
                identity: "dep".to_string(),
                dep_type: "github".to_string(),
                url: "https://github.com/org/dep".to_string(),
            }],
        };
        store.save(&path, &v2).unwrap();

        let loaded = store.load(&path).unwrap();
        assert_eq!(loaded.name, "v2");
        assert_eq!(loaded.dependencies.len(), 1);
    }
}