use crate::app::ports::{ConfigStorePort, ProviderPort};
use crate::domain::asset::AssetKind;
use crate::domain::identity::AssetIdentity;
use crate::domain::scope::Scope;
use anyhow::Result;

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
    super::prune_empty_vault_defs(&mut config);
    store.save(scope, &config)
}

/// Remove a provider from the scope's config.
pub fn remove_provider(scope: Scope, provider_id: &str, store: &dyn ConfigStorePort) -> Result<()> {
    let mut config = store.load(scope)?;
    config.providers.retain(|p| p != provider_id);
    store.save(scope, &config)
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
