use crate::app::ports::ConfigStorePort;
use crate::cli::commands::{
    active_providers_from_config, find_package_by_full_identity, print_json, OutputMode,
};
use crate::cli::entry::{Cli, ScopeArg};
use crate::domain::asset::AssetKind;
use anyhow::Result;

#[derive(Debug, serde::Serialize)]
pub struct ValidateResult {
    pub passed: bool,
    pub assets: Vec<AssetValidation>,
}

#[derive(Debug, serde::Serialize)]
pub struct AssetValidation {
    pub name: String,
    pub vault_id: String,
    pub sha10_match: bool,
    pub parse_ok: bool,
    pub provider_check: Vec<ProviderCheck>,
}

#[derive(Debug, serde::Serialize)]
pub struct ProviderCheck {
    pub provider: String,
    pub path_exists: bool,
}

pub fn cmd_validate(
    cli: &Cli,
    scope_arg: Option<ScopeArg>,
    workspace: &std::path::Path,
) -> Result<i32> {
    use crate::cli::commands::{resolve_scope, EXIT_SUCCESS, EXIT_VALIDATION_FAILURE};

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
