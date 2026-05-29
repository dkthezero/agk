use crate::app::ports::ConfigStorePort;
use crate::cli::commands::{
    active_providers_from_config, eprintln_if_not_quiet, find_package_by_full_identity, print_json,
    println_if_not_quiet, OutputMode,
};
use crate::cli::entry::{Cli, ScopeArg};
use anyhow::Result;

#[derive(Debug, serde::Serialize)]
pub struct InstallResult {
    pub installed: bool,
    pub identity: Option<String>,
    pub providers: Vec<String>,
    pub sha10: Option<String>,
    pub error: Option<String>,
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
    use crate::cli::commands::{
        resolve_scope, EXIT_GENERAL_FAILURE, EXIT_PARTIAL_SUCCESS, EXIT_SUCCESS,
    };

    let mode = OutputMode::from_cli(cli);
    let scope = resolve_scope(scope_arg);

    let (registry, _scan, store) = crate::app::bootstrap::build(workspace.to_path_buf())?;
    let config = store.load(scope)?;

    let providers: Vec<&dyn crate::app::ports::ProviderPort> = if let Some(filter) = provider_filter
    {
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
        if let Err(e) = crate::app::features::asset::install::install_asset(scope, &pkg, &store, *provider) {
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
