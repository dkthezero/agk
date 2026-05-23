use crate::app::bootstrap;
use crate::app::ports::{ConfigStorePort, ProviderPort};
use crate::cli::entry::{Cli, Commands, PackTarget, ScopeArg};
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::config::ConfigFile;
use crate::domain::identity::AssetIdentity;
use crate::domain::scope::Scope;
use anyhow::{Context, Result};

// ---------------------------------------------------------------------------
// Output formatting
// ---------------------------------------------------------------------------

pub enum OutputMode {
    Quiet,
    Normal,
    Verbose,
    Json,
}

impl OutputMode {
    fn from_cli(cli: &Cli) -> Self {
        if cli.json {
            OutputMode::Json
        } else if cli.quiet {
            OutputMode::Quiet
        } else if cli.verbose {
            OutputMode::Verbose
        } else {
            OutputMode::Normal
        }
    }
}

fn println_if_not_quiet(mode: &OutputMode, msg: &str) {
    match mode {
        OutputMode::Quiet => {}
        _ => println!("{}", msg),
    }
}

fn eprintln_if_not_quiet(mode: &OutputMode, msg: &str) {
    match mode {
        OutputMode::Quiet => {}
        _ => eprintln!("{}", msg),
    }
}

fn print_json<T: serde::Serialize>(mode: &OutputMode, value: &T) -> Result<()> {
    if matches!(mode, OutputMode::Json) {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

fn telemetry_to_csv(config: &crate::domain::telemetry::AnalyticsConfig) -> String {
    let mut lines = vec!["skill,invocations,last_used,providers".to_string()];
    for (name, analytics) in &config.skills {
        let last = analytics.last_used.as_deref().unwrap_or("never");
        let providers = analytics.providers().join("; ");
        lines.push(format!(
            "\"{}\",{},\"{}\",\"{}\"",
            name.replace('"', "\"\""),
            analytics.total_invocations,
            last,
            providers.replace('"', "\"\""),
        ));
    }
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Common helpers
// ---------------------------------------------------------------------------

fn resolve_scope(scope_arg: Option<ScopeArg>) -> Scope {
    scope_arg
        .map(|s| s.to_domain_scope())
        .unwrap_or(Scope::Workspace)
}

fn active_providers_from_config<'a>(
    registry: &'a crate::app::registry::Registry,
    config: &ConfigFile,
) -> Vec<&'a dyn ProviderPort> {
    registry
        .providers
        .iter()
        .filter(|p| config.providers.contains(&p.id().to_string()))
        .map(|p| p.as_ref())
        .collect()
}

fn find_package_by_full_identity(
    registry: &crate::app::registry::Registry,
    identity_str: &str,
) -> Result<Option<ScannedPackage>> {
    let parts: Vec<&str> = identity_str.split('/').collect();
    let (vault_hint, name_part) = if parts.len() == 2 {
        (Some(parts[0]), parts[1])
    } else {
        (None, identity_str)
    };

    let name = name_part.split(':').next().unwrap_or(name_part);

    for vault in &registry.vaults {
        if let Some(hint) = vault_hint {
            if vault.id() != hint {
                continue;
            }
        }
        for feature in &registry.feature_sets {
            let pkgs = vault.list_packages(feature.as_ref())?;
            for pkg in pkgs {
                if pkg.identity.name == name {
                    return Ok(Some(pkg));
                }
            }
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// Exit codes
// ---------------------------------------------------------------------------

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_GENERAL_FAILURE: i32 = 1;
pub const EXIT_VALIDATION_FAILURE: i32 = 2;
pub const EXIT_PARTIAL_SUCCESS: i32 = 3;

// ---------------------------------------------------------------------------
// Command: sync
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
struct SyncResult {
    updated: Vec<String>,
    skipped: Vec<String>,
    errors: Vec<String>,
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

    let (registry, _scan, store) = bootstrap::build(workspace.to_path_buf())?;
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
                &identity,
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
                &identity,
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
    workspace: &std::path::Path,
) -> Result<i32> {
    let mode = OutputMode::from_cli(cli);
    let scope = resolve_scope(scope_arg);

    let (registry, _scan, store) = bootstrap::build(workspace.to_path_buf())?;
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
        Some(p) => p,
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

    let (registry, _scan, store) = bootstrap::build(workspace.to_path_buf())?;
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

    let (registry, _scan, _store) = bootstrap::build(workspace.to_path_buf())?;
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

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn run(cli: Cli, workspace: &std::path::Path) -> Result<i32> {
    match cli.command {
        Some(Commands::Clean { global }) => {
            let dir = if global {
                crate::domain::paths::global_config_root()
            } else {
                workspace.join(".agk")
            };

            if dir.exists() {
                if !cli.quiet {
                    println!(
                        "This will securely remove all configuration in: {}",
                        dir.display()
                    );
                    println!("Are you sure you want to proceed? [y/N]");
                }
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if input.trim().eq_ignore_ascii_case("y") {
                    std::fs::remove_dir_all(&dir)?;
                    if !cli.quiet {
                        println!("Cleaned up {}", dir.display());
                    }
                } else if !cli.quiet {
                    println!("Operation cancelled");
                }
            } else if !cli.quiet {
                println!("Nothing to clean at {}", dir.display());
            }
            Ok(EXIT_SUCCESS)
        }

        Some(Commands::Sync { global, dry_run }) => cmd_sync(&cli, global, dry_run, workspace),

        Some(Commands::Install {
            ref identity,
            scope,
            dry_run,
            ref provider,
        }) => cmd_install(
            &cli,
            identity,
            scope,
            dry_run,
            provider.as_deref(),
            workspace,
        ),

        Some(Commands::Validate { scope }) => cmd_validate(&cli, scope, workspace),

        Some(Commands::Pack {
            ref identity,
            target,
            stdout,
        }) => cmd_pack(&cli, identity, target, stdout, workspace),

        Some(Commands::Mcp { ref command }) => match command {
            crate::cli::entry::McpCommands::Add {
                ref name,
                ref command,
                ref args,
                ref env,
                ref transport,
                ref description,
                no_test,
            } => {
                let mode = OutputMode::from_cli(&cli);
                match crate::infra::mcp::register(
                    name,
                    command,
                    args.as_deref(),
                    env.as_deref(),
                    transport,
                    description.as_deref(),
                ) {
                    Ok(server) => {
                        println_if_not_quiet(
                            &mode,
                            &format!(
                                "Registered MCP server '{}' ({})",
                                server.name, server.command
                            ),
                        );
                        if !no_test {
                            println_if_not_quiet(&mode, "Testing connection...");
                            let rt = tokio::runtime::Runtime::new()?;
                            match rt.block_on(crate::infra::mcp::test_server(name)) {
                                Ok(()) => println_if_not_quiet(
                                    &mode,
                                    &format!("MCP server '{}' tested successfully", name),
                                ),
                                Err(e) => {
                                    eprintln_if_not_quiet(
                                        &mode,
                                        &format!("MCP test failed: {}", e),
                                    );
                                    return Ok(EXIT_GENERAL_FAILURE);
                                }
                            }
                        }
                        Ok(EXIT_SUCCESS)
                    }
                    Err(e) => {
                        eprintln_if_not_quiet(&mode, &format!("Failed to register: {}", e));
                        Ok(EXIT_GENERAL_FAILURE)
                    }
                }
            }
            crate::cli::entry::McpCommands::Enable {
                ref name,
                ref provider,
                scope,
            } => {
                let mode = OutputMode::from_cli(&cli);
                let scope = resolve_scope(*scope);
                let providers = crate::infra::mcp::build_mcp_providers(&workspace);
                match crate::infra::mcp::enable(name, provider, scope, &providers) {
                    Ok(()) => {
                        println_if_not_quiet(
                            &mode,
                            &format!("Enabled MCP server '{}' for {}", name, provider),
                        );
                        Ok(EXIT_SUCCESS)
                    }
                    Err(e) => {
                        eprintln_if_not_quiet(&mode, &format!("Enable failed: {}", e));
                        Ok(EXIT_GENERAL_FAILURE)
                    }
                }
            }
            crate::cli::entry::McpCommands::Disable {
                ref name,
                ref provider,
                scope,
            } => {
                let mode = OutputMode::from_cli(&cli);
                let scope = resolve_scope(*scope);
                let providers = crate::infra::mcp::build_mcp_providers(&workspace);
                match crate::infra::mcp::disable(name, provider, scope, &providers) {
                    Ok(()) => {
                        println_if_not_quiet(
                            &mode,
                            &format!("Disabled MCP server '{}' for {}", name, provider),
                        );
                        Ok(EXIT_SUCCESS)
                    }
                    Err(e) => {
                        eprintln_if_not_quiet(&mode, &format!("Disable failed: {}", e));
                        Ok(EXIT_GENERAL_FAILURE)
                    }
                }
            }
            crate::cli::entry::McpCommands::List { provider: _ } => {
                let mode = OutputMode::from_cli(&cli);
                let path = crate::domain::paths::mcp_path();
                let registry = crate::domain::mcp::McpRegistry::load(&path).unwrap_or_default();
                let mut items: Vec<&crate::domain::mcp::McpServer> =
                    registry.servers.values().collect();
                items.sort_by(|a, b| a.name.cmp(&b.name));

                if matches!(mode, OutputMode::Json) {
                    let json: Vec<serde_json::Value> = items
                        .iter()
                        .map(|s| {
                            serde_json::json!({
                                "name": s.name,
                                "command": s.command,
                                "transport": match s.transport {
                                    crate::domain::mcp::McpTransport::Stdio => "stdio",
                                    crate::domain::mcp::McpTransport::Sse { .. } => "sse",
                                },
                                "tested": s.tested,
                                "tested_at": s.tested_at,
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&json)?);
                } else {
                    if items.is_empty() {
                        println_if_not_quiet(&mode, "No MCP servers registered.");
                    } else {
                        for s in items {
                            let tested = if s.tested { "[✓]" } else { "[ ]" };
                            println_if_not_quiet(
                                &mode,
                                &format!("{} {} ({:?})", tested, s.name, s.transport),
                            );
                        }
                    }
                }
                Ok(EXIT_SUCCESS)
            }
            crate::cli::entry::McpCommands::Test { ref name } => {
                let mode = OutputMode::from_cli(&cli);
                println_if_not_quiet(&mode, &format!("Testing MCP server '{}'...", name));
                let rt = tokio::runtime::Runtime::new()?;
                match rt.block_on(crate::infra::mcp::test_server(name)) {
                    Ok(()) => {
                        println_if_not_quiet(&mode, &format!("MCP server '{}' is healthy", name));
                        Ok(EXIT_SUCCESS)
                    }
                    Err(e) => {
                        eprintln_if_not_quiet(&mode, &format!("Test failed: {}", e));
                        Ok(EXIT_GENERAL_FAILURE)
                    }
                }
            }
        },

        Some(Commands::Telemetry { ref command }) => match command {
            crate::cli::entry::TelemetryCommands::Enable => {
                let path = crate::domain::paths::analytics_path();
                let mut scanner = crate::infra::telemetry::scanner::Scanner::new(path);
                scanner.enable();
                println_if_not_quiet(
                    &OutputMode::from_cli(&cli),
                    "Telemetry enabled. Background scanner started.",
                );
                Ok(EXIT_SUCCESS)
            }
            crate::cli::entry::TelemetryCommands::Disable => {
                let path = crate::domain::paths::analytics_path();
                let mut scanner = crate::infra::telemetry::scanner::Scanner::new(path);
                scanner.disable();
                println_if_not_quiet(&OutputMode::from_cli(&cli), "Telemetry disabled.");
                Ok(EXIT_SUCCESS)
            }
            crate::cli::entry::TelemetryCommands::Status => {
                let path = crate::domain::paths::analytics_path();
                let scanner = crate::infra::telemetry::scanner::Scanner::new(path);
                let status = scanner.status();
                let mode = OutputMode::from_cli(&cli);
                if matches!(mode, OutputMode::Json) {
                    println!("{}", serde_json::to_string_pretty(&status)?);
                } else {
                    println_if_not_quiet(
                        &mode,
                        &format!(
                            "Telemetry: {} | Skills tracked: {} | Last scan: {}",
                            if status.enabled {
                                "enabled"
                            } else {
                                "disabled"
                            },
                            status.skills_tracked,
                            status.last_scan.as_deref().unwrap_or("never")
                        ),
                    );
                }
                Ok(EXIT_SUCCESS)
            }
            crate::cli::entry::TelemetryCommands::Export { format, output } => {
                let path = crate::domain::paths::analytics_path();
                let config = crate::domain::telemetry::AnalyticsConfig::load(&path)?;
                let mode = OutputMode::from_cli(&cli);

                let content = match format {
                    crate::cli::entry::ExportFormat::Json => serde_json::to_string_pretty(&config)?,
                    crate::cli::entry::ExportFormat::Csv => telemetry_to_csv(&config),
                };

                if let Some(file_path) = output {
                    std::fs::write(&file_path, &content)?;
                    println_if_not_quiet(&mode, &format!("Telemetry exported to {}", file_path));
                } else {
                    println!("{}", content);
                }
                Ok(EXIT_SUCCESS)
            }
        },

        None => {
            // No subcommand — fall through to TUI in main.rs
            Ok(EXIT_SUCCESS)
        }
    }
}
