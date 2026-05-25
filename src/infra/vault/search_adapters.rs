use crate::app::ports::VaultSearchPort;
use crate::domain::asset::ScannedPackage;
use anyhow::Result;

/// Concrete [`VaultSearchPort`] adapter for ClawHub.
pub struct ClawHubSearchAdapter {
    vault_id: String,
}

impl ClawHubSearchAdapter {
    pub fn new(vault_id: impl Into<String>) -> Self {
        Self {
            vault_id: vault_id.into(),
        }
    }
}

#[async_trait::async_trait]
#[async_trait::async_trait]
impl VaultSearchPort for ClawHubSearchAdapter {
    fn vault_id(&self) -> &str {
        &self.vault_id
    }

    async fn search(&self, query: &str) -> Result<Vec<ScannedPackage>> {
        let packages = crate::infra::vault::clawhub::cli_search(query).unwrap_or_default();
        Ok(packages)
    }
}
