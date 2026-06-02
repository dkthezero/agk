use crate::domain::vault_manifest::VaultManifest;
use anyhow::Result;
use std::path::Path;

/// Port for reading/writing vault manifest (.agk/vault.toml).
/// Concrete implementation: VaultManifestTomlStore in infra/config/vault_manifest_store.rs
pub trait VaultManifestStorePort: Send + Sync {
    fn load(&self, path: &Path) -> Result<VaultManifest>;
    fn save(&self, path: &Path, manifest: &VaultManifest) -> Result<()>;
}
