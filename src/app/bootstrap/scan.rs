use crate::app::registry::Registry;
use crate::domain::asset::ScannedPackage;
use crate::domain::config::ConfigFile;
use anyhow::Result;
use rayon::prelude::*;

/// Result of scanning all vaults for packages across all feature tabs.
pub struct ScanResult {
    /// Packages grouped by tab (one Vec per feature set).
    pub packages_by_tab: Vec<Vec<ScannedPackage>>,
    /// Errors encountered during vault scanning, keyed by vault ID.
    pub scan_errors: Vec<ScanError>,
}

/// A non-fatal error from scanning a single vault.
pub struct ScanError {
    /// The vault that produced the error.
    pub vault_id: String,
    /// The error message.
    pub error: String,
}

/// Scan all vaults for packages across all feature tabs.
///
/// Each tab's vaults are scanned in parallel using rayon. Errors from
/// individual vaults are collected into `scan_errors` rather than
/// causing the entire scan to fail — the caller can decide how to
/// surface them.
pub fn scan(
    registry: &Registry,
    vaults: &[Box<dyn crate::app::ports::VaultPort>],
) -> Result<ScanResult> {
    let mut packages_by_tab = Vec::new();
    let mut all_errors = Vec::new();

    for feature in &registry.feature_sets {
        if feature.is_stub() {
            packages_by_tab.push(Vec::new());
            continue;
        }

        // Scan all vaults in parallel for this tab.
        let results: Vec<(String, Result<Vec<ScannedPackage>>)> = vaults
            .par_iter()
            .map(|vault| {
                let id = vault.id().to_string();
                let result = vault.list_packages(feature.as_ref());
                (id, result)
            })
            .collect();

        let mut tab_packages = Vec::new();
        for (vault_id, result) in results {
            match result {
                Ok(pkgs) => tab_packages.extend(pkgs),
                Err(e) => all_errors.push(ScanError {
                    vault_id,
                    error: e.to_string(),
                }),
            }
        }
        packages_by_tab.push(tab_packages);
    }

    Ok(ScanResult {
        packages_by_tab,
        scan_errors: all_errors,
    })
}

/// Filter scanned packages to only those in enabled vaults or explicitly
/// installed in global/workspace config.
pub fn filter_scan(
    scan: &mut ScanResult,
    global_config: &ConfigFile,
    workspace_config: Option<&ConfigFile>,
) {
    let mut combined_vaults: std::collections::HashSet<_> =
        global_config.vaults.iter().cloned().collect();
    if let Some(ws) = workspace_config {
        combined_vaults.extend(ws.vaults.iter().cloned());
    }

    for tab_pkgs in &mut scan.packages_by_tab {
        tab_pkgs.retain(|pkg| {
            if combined_vaults.contains(&pkg.vault_id) {
                true
            } else {
                let is_global = match pkg.kind {
                    crate::domain::asset::AssetKind::Skill => {
                        global_config.is_skill_installed(&pkg.vault_id, &pkg.identity.name)
                    }
                    crate::domain::asset::AssetKind::Instruction => {
                        global_config.is_instruction_installed(&pkg.vault_id, &pkg.identity.name)
                    }
                    crate::domain::asset::AssetKind::McpServer => {
                        global_config.is_mcp_installed(&pkg.vault_id, &pkg.identity.name)
                    }
                    crate::domain::asset::AssetKind::Profile => {
                        global_config.is_profile_installed(&pkg.vault_id, &pkg.identity.name)
                    }
                };
                let is_ws = if let Some(ws) = workspace_config {
                    match pkg.kind {
                        crate::domain::asset::AssetKind::Skill => {
                            ws.is_skill_installed(&pkg.vault_id, &pkg.identity.name)
                        }
                        crate::domain::asset::AssetKind::Instruction => {
                            ws.is_instruction_installed(&pkg.vault_id, &pkg.identity.name)
                        }
                        crate::domain::asset::AssetKind::McpServer => {
                            ws.is_mcp_installed(&pkg.vault_id, &pkg.identity.name)
                        }
                        crate::domain::asset::AssetKind::Profile => {
                            ws.is_profile_installed(&pkg.vault_id, &pkg.identity.name)
                        }
                    }
                } else {
                    false
                };
                is_global || is_ws
            }
        });
    }
}

pub fn build_vaults(
    config: &ConfigFile,
    workspace_root: &std::path::Path,
) -> Vec<Box<dyn crate::app::ports::VaultPort>> {
    let mut vaults: Vec<Box<dyn crate::app::ports::VaultPort>> = Vec::new();
    let mut keys: Vec<_> = config.vault_defs.keys().collect();
    keys.sort();

    for vault_id in keys {
        if let Some(section) = config.vault_defs.get(vault_id) {
            if let Some(vault_conf) = &section.vault {
                match vault_conf {
                    crate::domain::config::VaultConfig::Local(local) => {
                        let mut p = std::path::PathBuf::from(&local.path);
                        if p.is_relative() {
                            p = workspace_root.join(p);
                        }
                        vaults.push(Box::new(
                            crate::infra::vault::local::LocalVaultAdapter::new(vault_id, p),
                        ));
                    }
                    crate::domain::config::VaultConfig::Github(github) => {
                        vaults.push(Box::new(
                            crate::infra::vault::github::GithubVaultAdapter::new(
                                vault_id,
                                &github.repo,
                                &github.r#ref,
                                &github.path,
                            ),
                        ));
                    }
                    crate::domain::config::VaultConfig::Clawhub(_) => {
                        vaults.push(Box::new(
                            crate::infra::vault::clawhub::ClawHubVaultAdapter::new(vault_id),
                        ));
                    }
                }
            }
        }
    }
    vaults
}
