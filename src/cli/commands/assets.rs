use super::*;
use crate::cli::entry::{Cli, PackTarget, ScopeArg};
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::identity::AssetIdentity;
use crate::domain::scope::Scope;
use anyhow::Result;

// ---------------------------------------------------------------------------
// Command: sync
// ---------------------------------------------------------------------------

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

enum SyncAction {
    Updated,
    UpToDate,
}

fn sync_single_asset(
    scope: Scope,
    identity: &AssetIdentity,
    _kind: &AssetKind,
    _vault_id: &str,
    registry: &crate::app::registry::Registry,
    store: &dyn ConfigStorePort,
    providers: &[&dyn ProviderPort],
) -> Result<SyncAction> {
    let latest_pkg = find_package_by_full_identity(registry, &identity.name)?;

    if let Some(pkg) = latest_pkg {
        if pkg.identity.sha10 != identity.sha10 {
            for provider in providers {
                crate::app::actions::update_asset(scope, &pkg, store, *provider)
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

// ---------------------------------------------------------------------------
// Command: install
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
struct InstallResult {
    installed: bool,
    identity: Option<String>,
    providers: Vec<String>,
    sha10: Option<String>,
    error: Option<String>,
}

pub fn cmd_install(
    cli: &Cli,
    identity_str: &str,
    scope_arg: Option<ScopeArg>,
    dry_run: bool,
    provider_filter: Option<&str>,
    include_evals: bool,
    workspace: &std::path::Path,
) -> Result<i32> {
    let mode = OutputMode::from_cli(cli);
    let scope = resolve_scope(scope_arg);

    let (registry, _scan, store) = crate::app::bootstrap::build(workspace.to_path_buf())?;
    let config = store.load(scope)?;

    let providers: Vec<&dyn ProviderPort> = if let Some(filter) = provider_filter {
        match registry.get_provider(filter) {
            Ok(p) => vec![p],
            Err(_) => {
                eprintln_if_not_quiet(&mode, &format!("Provider '{}' not found", filter));
                return Ok(EXIT_GENERAL_FAILURE);
            }
        }
    } else {
        active_providers_from_config(&registry, &config)
    };

    if providers.is_empty() {
        eprintln_if_not_quiet(
            &mode,
            "No active providers configured. Use the TUI or --provider flag.",
        );
        return Ok(EXIT_GENERAL_FAILURE);
    }

    let pkg = match find_package_by_full_identity(&registry, identity_str)? {
        Some(mut p) => {
            p.include_evals = include_evals;
            p
        }
        None => {
            eprintln_if_not_quiet(
                &mode,
                &format!("Asset '{}' not found in any vault", identity_str),
            );
            let result = InstallResult {
                installed: false,
                identity: Some(identity_str.to_string()),
                providers: vec![],
                sha10: None,
                error: Some("Asset not found in any vault".to_string()),
            };
            print_json(&mode, &result)?;
            return Ok(EXIT_GENERAL_FAILURE);
        }
    };

    if dry_run {
        let provider_names: Vec<String> = providers.iter().map(|p| p.name().to_string()).collect();
        println_if_not_quiet(
            &mode,
            &format!(
                "Would install '{}' to providers: {}",
                pkg.identity.name,
                provider_names.join(", ")
            ),
        );
        let result = InstallResult {
            installed: true,
            identity: Some(pkg.identity.to_string()),
            providers: provider_names,
            sha10: Some(pkg.identity.sha10.clone()),
            error: None,
        };
        print_json(&mode, &result)?;
        return Ok(EXIT_SUCCESS);
    }

    let mut success = true;
    let provider_names: Vec<String> = providers.iter().map(|p| p.name().to_string()).collect();

    for provider in &providers {
        if let Err(e) = crate::app::actions::install_asset(scope, &pkg, &store, *provider) {
            eprintln_if_not_quiet(
                &mode,
                &format!("Failed to install to {}: {}", provider.name(), e),
            );
            success = false;
        }
    }

    let result = InstallResult {
        installed: success,
        identity: Some(pkg.identity.to_string()),
        providers: provider_names,
        sha10: Some(pkg.identity.sha10.clone()),
        error: if success {
            None
        } else {
            Some("One or more providers failed".to_string())
        },
    };
    print_json(&mode, &result)?;

    if success {
        println_if_not_quiet(
            &mode,
            &format!("Installed '{}' successfully", pkg.identity.name),
        );
        Ok(EXIT_SUCCESS)
    } else {
        Ok(EXIT_PARTIAL_SUCCESS)
    }
}

// ---------------------------------------------------------------------------
// Command: validate
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
struct ValidateResult {
    passed: bool,
    assets: Vec<AssetValidation>,
}

#[derive(Debug, serde::Serialize)]
struct AssetValidation {
    name: String,
    vault_id: String,
    sha10_match: bool,
    parse_ok: bool,
    provider_check: Vec<ProviderCheck>,
}

#[derive(Debug, serde::Serialize)]
struct ProviderCheck {
    provider: String,
    path_exists: bool,
}

pub fn cmd_validate(
    cli: &Cli,
    scope_arg: Option<ScopeArg>,
    workspace: &std::path::Path,
) -> Result<i32> {
    let mode = OutputMode::from_cli(cli);
    let scope = resolve_scope(scope_arg);

    let (registry, _scan, store) = crate::app::bootstrap::build(workspace.to_path_buf())?;
    let config = store.load(scope)?;
    let providers = active_providers_from_config(&registry, &config);

    let mut validations = vec![];
    let mut all_passed = true;

    let all_vault_ids: Vec<String> = config.vault_defs.keys().cloned().collect();

    for vault_id in &all_vault_ids {
        for identity in config.installed_skills(vault_id) {
            let latest = find_package_by_full_identity(&registry, &identity.name)?;
            let sha10_match = latest
                .as_ref()
                .map(|p| p.identity.sha10 == identity.sha10)
                .unwrap_or(false);

            let mut provider_checks = vec![];
            for provider in &providers {
                let path = provider.install_path_for(&identity, &AssetKind::Skill, scope);
                provider_checks.push(ProviderCheck {
                    provider: provider.name().to_string(),
                    path_exists: path.as_ref().map(|p| p.exists()).unwrap_or(false),
                });
            }

            let parse_ok = latest.is_some();
            if !sha10_match || !parse_ok {
                all_passed = false;
            }

            validations.push(AssetValidation {
                name: identity.name.clone(),
                vault_id: vault_id.clone(),
                sha10_match,
                parse_ok,
                provider_check: provider_checks,
            });
        }

        for identity in config.installed_instructions(vault_id) {
            let latest = find_package_by_full_identity(&registry, &identity.name)?;
            let sha10_match = latest
                .as_ref()
                .map(|p| p.identity.sha10 == identity.sha10)
                .unwrap_or(false);

            let mut provider_checks = vec![];
            for provider in &providers {
                let path = provider.install_path_for(&identity, &AssetKind::Instruction, scope);
                provider_checks.push(ProviderCheck {
                    provider: provider.name().to_string(),
                    path_exists: path.as_ref().map(|p| p.exists()).unwrap_or(false),
                });
            }

            let parse_ok = latest.is_some();
            if !sha10_match || !parse_ok {
                all_passed = false;
            }

            validations.push(AssetValidation {
                name: identity.name.clone(),
                vault_id: vault_id.clone(),
                sha10_match,
                parse_ok,
                provider_check: provider_checks,
            });
        }
    }

    let result = ValidateResult {
        passed: all_passed,
        assets: validations,
    };

    print_json(&mode, &result)?;

    match mode {
        OutputMode::Json => {}
        OutputMode::Quiet => {}
        _ => {
            if all_passed {
                println!("All {} assets are valid.", result.assets.len());
            } else {
                println!("Validation failed for some assets:");
                for v in &result.assets {
                    if !v.sha10_match || !v.parse_ok {
                        println!(
                            "  - {}: sha10_match={}, parse_ok={}",
                            v.name, v.sha10_match, v.parse_ok
                        );
                    }
                }
            }
        }
    }

    Ok(if all_passed {
        EXIT_SUCCESS
    } else {
        EXIT_VALIDATION_FAILURE
    })
}

// ---------------------------------------------------------------------------
// Command: pack
// ---------------------------------------------------------------------------

pub fn cmd_pack(
    cli: &Cli,
    identity_str: &str,
    target: PackTarget,
    stdout_flag: bool,
    workspace: &std::path::Path,
) -> Result<i32> {
    let mode = OutputMode::from_cli(cli);

    let (registry, _scan, _store) = crate::app::bootstrap::build(workspace.to_path_buf())?;
    let pkg = match find_package_by_full_identity(&registry, identity_str)? {
        Some(p) => p,
        None => {
            eprintln_if_not_quiet(
                &mode,
                &format!("Asset '{}' not found in any vault", identity_str),
            );
            return Ok(EXIT_GENERAL_FAILURE);
        }
    };

    if pkg.kind != AssetKind::Skill {
        eprintln_if_not_quiet(
            &mode,
            "Packing is only supported for Skills (not Instructions)",
        );
        return Ok(EXIT_GENERAL_FAILURE);
    }

    match target {
        PackTarget::ClaudeDesktop => {
            pack_claude_desktop(&mode, &pkg, stdout_flag, workspace)?;
        }
        PackTarget::Firebender => {
            eprintln_if_not_quiet(
                &mode,
                "Firebender pack target not yet implemented. Use --target claude-desktop.",
            );
            return Ok(EXIT_GENERAL_FAILURE);
        }
        PackTarget::Tarball => {
            pack_tarball(&mode, &pkg, stdout_flag, workspace)?;
        }
    }

    Ok(EXIT_SUCCESS)
}

fn pack_claude_desktop(
    mode: &OutputMode,
    pkg: &ScannedPackage,
    stdout_flag: bool,
    workspace: &std::path::Path,
) -> Result<()> {
    use std::io::Write;

    let out_dir = workspace.join(".agk").join("pack");
    std::fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!("{}-claude-desktop.zip", pkg.identity.name));

    let file = std::fs::File::create(&out_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    fn add_dir_to_zip(
        zip: &mut zip::ZipWriter<std::fs::File>,
        base_path: &std::path::Path,
        prefix: &str,
        options: zip::write::SimpleFileOptions,
    ) -> Result<()> {
        for entry in walkdir::WalkDir::new(base_path) {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(base_path)?;
            let zip_path = format!("{}/{}", prefix, relative.display());

            if path.is_file() {
                zip.start_file(&zip_path, options)?;
                let content = std::fs::read(path)?;
                zip.write_all(&content)?;
            }
        }
        Ok(())
    }

    add_dir_to_zip(&mut zip, &pkg.path, &pkg.identity.name, options)?;
    zip.finish()?;

    if stdout_flag {
        let bytes = std::fs::read(&out_path)?;
        std::io::stdout().write_all(&bytes)?;
    } else {
        println_if_not_quiet(
            mode,
            &format!("Packed '{}' to {}", pkg.identity.name, out_path.display()),
        );
    }
    Ok(())
}

fn pack_tarball(
    mode: &OutputMode,
    pkg: &ScannedPackage,
    stdout_flag: bool,
    workspace: &std::path::Path,
) -> Result<()> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    use tar::Builder;

    let out_dir = workspace.join(".agk").join("pack");
    std::fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!(
        "{}-{}.tar.gz",
        pkg.identity.name, pkg.identity.sha10
    ));

    let file = std::fs::File::create(&out_path)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);
    tar.append_dir_all(&pkg.identity.name, &pkg.path)?;
    let enc = tar.into_inner()?;
    enc.finish()?;

    if stdout_flag {
        let bytes = std::fs::read(&out_path)?;
        std::io::stdout().write_all(&bytes)?;
    } else {
        println_if_not_quiet(
            mode,
            &format!("Packed '{}' to {}", pkg.identity.name, out_path.display()),
        );
    }
    Ok(())
}
