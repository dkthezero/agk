use crate::app::ports::ConfigStorePort;
use crate::cli::commands::{
    active_providers_from_config, eprintln_if_not_quiet, find_package_by_full_identity, OutputMode,
};
use crate::cli::entry::Cli;
use crate::domain::asset::AssetKind;
use crate::domain::identity::AssetIdentity;
use crate::domain::scope::Scope;
use anyhow::{Context, Result};

use crate::cli::commands::{print_json, EXIT_GENERAL_FAILURE, EXIT_PARTIAL_SUCCESS, EXIT_SUCCESS};

#[derive(Debug, serde::Serialize)]
pub struct SyncResult {
    pub updated: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

pub fn cmd_sync(
    cli: &Cli,
    global: bool,
    dry_run: bool,
    workspace: &std::path::Path,
) -> Result<i32> {
    let mode = OutputMode::from_cli(cli);
    let scope = if global {
        Scope::Global
    } else {
        Scope::Workspace
    };

    let (registry, _scan, store) = crate::app::bootstrap::build(workspace.to_path_buf())?;
    let config = store.load(scope)?;

    let providers = active_providers_from_config(&registry, &config);
    if providers.is_empty() {
        eprintln_if_not_quiet(
            &mode,
            "No active providers configured. Use the TUI to enable providers.",
        );
        return Ok(EXIT_GENERAL_FAILURE);
    }

    let mut result = SyncResult {
        updated: vec![],
        skipped: vec![],
        errors: vec![],
    };

    let all_vault_ids: Vec<String> = config.vault_defs.keys().cloned().collect();

    for vault_id in &all_vault_ids {
        let skills = config.installed_skills(vault_id);
        let instructions = config.installed_instructions(vault_id);

        for identity in &skills {
            if dry_run {
                result.skipped.push(format!("{} (dry-run)", identity.name));
                continue;
            }
            match sync_single_asset(
                scope,
                identity,
                &AssetKind::Skill,
                vault_id,
                &registry,
                &store,
                &providers,
            ) {
                Ok(action) => match action {
                    SyncAction::Updated => result.updated.push(identity.name.clone()),
                    SyncAction::UpToDate => result.skipped.push(identity.name.clone()),
                },
                Err(e) => result.errors.push(format!("{}: {}", identity.name, e)),
            }
        }

        for identity in &instructions {
            if dry_run {
                result.skipped.push(format!("{} (dry-run)", identity.name));
                continue;
            }
            match sync_single_asset(
                scope,
                identity,
                &AssetKind::Instruction,
                vault_id,
                &registry,
                &store,
                &providers,
            ) {
                Ok(action) => match action {
                    SyncAction::Updated => result.updated.push(identity.name.clone()),
                    SyncAction::UpToDate => result.skipped.push(identity.name.clone()),
                },
                Err(e) => result.errors.push(format!("{}: {}", identity.name, e)),
            }
        }
    }

    let exit_code = if result.errors.is_empty() {
        EXIT_SUCCESS
    } else if result.updated.is_empty() && result.skipped.is_empty() {
        EXIT_GENERAL_FAILURE
    } else {
        EXIT_PARTIAL_SUCCESS
    };

    match mode {
        OutputMode::Json => {
            print_json(&mode, &result)?;
        }
        OutputMode::Quiet => {}
        _ => {
            println!("Sync complete:");
            println!("  Updated:   {}", result.updated.len());
            println!("  Skipped:   {}", result.skipped.len());
            println!("  Errors:    {}", result.errors.len());
            if !result.errors.is_empty() {
                for e in &result.errors {
                    eprintln!("    - {}", e);
                }
            }
        }
    }

    Ok(exit_code)
}

#[derive(Debug)]
pub enum SyncAction {
    Updated,
    UpToDate,
}

pub fn sync_single_asset(
    scope: Scope,
    identity: &AssetIdentity,
    _kind: &AssetKind,
    _vault_id: &str,
    registry: &crate::app::registry::Registry,
    store: &dyn crate::app::ports::ConfigStorePort,
    providers: &[&dyn crate::app::ports::ProviderPort],
) -> Result<SyncAction> {
    let latest_pkg = find_package_by_full_identity(registry, &identity.name)?;

    if let Some(pkg) = latest_pkg {
        if pkg.identity.sha10 != identity.sha10 {
            for provider in providers {
                crate::app::features::asset::update::update_asset(scope, &pkg, store, *provider)
                    .with_context(|| format!("update via {}", provider.name()))?;
            }
            Ok(SyncAction::Updated)
        } else {
            Ok(SyncAction::UpToDate)
        }
    } else {
        Ok(SyncAction::UpToDate)
    }
}
