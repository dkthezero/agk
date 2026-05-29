//! Stub for future sync helpers.
use crate::app::ports::ConfigStorePort;
use crate::domain::config::VaultConfig;
use crate::domain::scope::Scope;
use anyhow::Result;

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
