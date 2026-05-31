//! Batch dependency resolution and installation for profile sessions.
//!
//! When a profile references skills or MCPs that are not yet installed,
//! this module resolves them from the vault scan and installs/registers
//! them automatically. If any dependency fails, previously installed
//! items are rolled back.

use crate::app::event::CoreEvent;
use crate::app::outcome::CoreEventSink;
use crate::app::ports::{ConfigStorePort, McpRegistryPort, ProviderPort};
use crate::app::registry::Registry;
use crate::domain::asset::AssetKind;
use crate::domain::config::ConfigFile;
use crate::domain::profile::ProfileAssetRef;
use crate::domain::scope::Scope;
use anyhow::Result;

/// Tracks a dependency that was successfully installed, enabling rollback.
#[derive(Debug, Clone)]
enum InstalledItem {
    Skill { name: String, vault_id: String },
    Mcp { name: String },
}

/// Result of a batch dependency installation attempt.
#[derive(Debug)]
pub struct BatchInstallResult {
    /// Dependencies that were installed/registered successfully.
    pub succeeded: Vec<String>,
    /// Dependencies that could not be installed, with error messages.
    pub failed: Vec<(String, String)>,
    /// Items that were installed but could not be rolled back.
    pub rollback_failed: Vec<(String, String)>,
}

impl BatchInstallResult {
    /// Returns true if all dependencies were installed successfully.
    pub fn all_succeeded(&self) -> bool {
        self.failed.is_empty() && self.rollback_failed.is_empty()
    }
}

/// Resolve and install all missing dependencies for a profile.
///
/// For each skill/instruction referenced by the profile that is not yet
/// installed, look it up in the vault scan and install it. For each MCP
/// that is not yet registered, register it via the MCP registry.
///
/// If any installation fails, attempt to roll back the successfully
/// installed items.
#[allow(clippy::too_many_arguments)]
pub fn resolve_and_install_deps(
    profile_name: &str,
    skills: &[ProfileAssetRef],
    mcps: &[ProfileAssetRef],
    scope: Scope,
    config: &ConfigFile,
    store: &dyn ConfigStorePort,
    registry: &Registry,
    mcp_registry: &dyn McpRegistryPort,
    providers: &[Box<dyn ProviderPort>],
    sink: &mut dyn CoreEventSink,
) -> BatchInstallResult {
    let mut result = BatchInstallResult {
        succeeded: Vec::new(),
        failed: Vec::new(),
        rollback_failed: Vec::new(),
    };
    let mut installed: Vec<InstalledItem> = Vec::new();

    // Resolve missing skills
    for skill in skills {
        if config.is_skill_installed(&skill.vault, &skill.name) {
            continue;
        }
        match resolve_and_install_asset(
            &skill.name,
            &skill.vault,
            AssetKind::Skill,
            scope,
            store,
            registry,
            providers,
            sink,
        ) {
            Ok(()) => {
                let label = format!("skill:{}", skill.name);
                sink.on_event(CoreEvent::Info(format!(
                    "Auto-installed '{}' for profile '{}'",
                    skill.name, profile_name,
                )));
                result.succeeded.push(label);
                installed.push(InstalledItem::Skill {
                    name: skill.name.clone(),
                    vault_id: skill.vault.clone(),
                });
            }
            Err(e) => {
                result
                    .failed
                    .push((format!("skill:{}", skill.name), e.to_string()));
            }
        }
    }

    // Resolve missing instructions (treated like skills for installation)
    // Note: ProfileAssetRef doesn't have an instructions field on the config
    // Profile, so we skip this for now.

    // Resolve missing MCPs
    for mcp in mcps {
        if config.is_mcp_installed(&mcp.vault, &mcp.name) {
            continue;
        }
        match mcp_registry.register(
            &mcp.name, &mcp.name, // command defaults to name
            None, None, "stdio", None,
        ) {
            Ok(_server) => {
                let label = format!("mcp:{}", mcp.name);
                sink.on_event(CoreEvent::Info(format!(
                    "Auto-registered MCP '{}' for profile '{}'",
                    mcp.name, profile_name,
                )));
                result.succeeded.push(label);
                installed.push(InstalledItem::Mcp {
                    name: mcp.name.clone(),
                });
            }
            Err(e) => {
                result
                    .failed
                    .push((format!("mcp:{}", mcp.name), e.to_string()));
            }
        }
    }

    // If anything failed, attempt rollback
    if !result.failed.is_empty() && !installed.is_empty() {
        for item in installed {
            match &item {
                InstalledItem::Skill { name, vault_id } => {
                    if let Err(e) = rollback_skill(name, vault_id, scope, store, providers) {
                        result
                            .rollback_failed
                            .push((format!("skill:{}", name), e.to_string()));
                    }
                }
                InstalledItem::Mcp { name } => {
                    if let Err(e) = mcp_registry.unregister(name) {
                        result
                            .rollback_failed
                            .push((format!("mcp:{}", name), e.to_string()));
                    }
                }
            }
        }
    }

    result
}

/// Look up a package by name in the vault scan and install it.
#[allow(clippy::too_many_arguments)]
fn resolve_and_install_asset(
    name: &str,
    vault_id: &str,
    kind: AssetKind,
    scope: Scope,
    store: &dyn ConfigStorePort,
    registry: &Registry,
    providers: &[Box<dyn ProviderPort>],
    sink: &mut dyn CoreEventSink,
) -> Result<()> {
    // Try to find the package in the vault scan
    let pkg = match registry.find_package_by_identity(name) {
        Ok(Some(pkg)) => pkg,
        Ok(None) => {
            anyhow::bail!(
                "Asset '{}' not found in any vault — install it manually with `agk install {}`",
                name,
                name
            );
        }
        Err(e) => {
            anyhow::bail!("Error looking up '{}': {}", name, e);
        }
    };

    // Verify the kind matches
    if pkg.kind != kind {
        anyhow::bail!(
            "Expected '{}' to be a {:?}, but found {:?}",
            name,
            kind,
            pkg.kind
        );
    }

    // Verify the vault matches (if specified)
    if pkg.vault_id != vault_id && vault_id != "auto" {
        sink.on_error(format!(
            "Asset '{}' found in vault '{}' (expected '{}') — installing from available vault",
            name, pkg.vault_id, vault_id
        ));
    }

    // Install via each active provider
    let mut any_failed = false;
    for provider in providers {
        if let Err(e) = crate::app::features::asset::install::install_asset(
            scope,
            &pkg,
            store,
            provider.as_ref(),
        ) {
            sink.on_error(format!("Provider {}: {}", provider.id(), e));
            any_failed = true;
        }
    }

    if any_failed {
        anyhow::bail!("Failed to install '{}' on one or more providers", name);
    }

    Ok(())
}

/// Roll back a skill installation by removing it from config and provider.
fn rollback_skill(
    name: &str,
    _vault_id: &str,
    scope: Scope,
    store: &dyn ConfigStorePort,
    providers: &[Box<dyn ProviderPort>],
) -> Result<()> {
    let mut config = store.load(scope)?;
    for provider in providers {
        let _ = provider.remove(
            &crate::domain::identity::AssetIdentity::new(name, None, "0000000000"),
            &AssetKind::Skill,
            scope,
            Some(&config),
        );
    }
    // Remove from config
    for section in config.vault_defs.values_mut() {
        if let Some(ref mut bucket) = section.skills {
            bucket.items.retain(|item| {
                // Items are "[name:version:sha10]" — check name prefix
                !item.starts_with(&format!("[{}:", name))
            });
        }
    }
    store.save(scope, &config)?;
    Ok(())
}
