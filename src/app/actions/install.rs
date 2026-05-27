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
