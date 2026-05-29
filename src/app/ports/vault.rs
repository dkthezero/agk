use crate::app::ports::feature_set::FeatureSetPort;
use crate::domain::asset::ScannedPackage;
use anyhow::Result;

#[async_trait::async_trait]
pub trait VaultPort: Send + Sync {
    fn id(&self) -> &str;
    fn kind_name(&self) -> &str;

    async fn refresh(&self) -> Result<()> {
        Ok(())
    }

    fn list_packages(&self, feature: &dyn FeatureSetPort) -> Result<Vec<ScannedPackage>>;
}

/// Port for searching remote vaults (e.g. ClawHub).
#[async_trait::async_trait]
pub trait VaultSearchPort: Send + Sync {
    fn vault_id(&self) -> &str;
    async fn search(&self, query: &str) -> Result<Vec<ScannedPackage>>;
}
