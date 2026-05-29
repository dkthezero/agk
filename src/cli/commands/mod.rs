use crate::app::ports::ProviderPort;
use crate::cli::entry::{Cli, Commands, ScopeArg};
use crate::domain::asset::ScannedPackage;
use crate::domain::config::ConfigFile;
use crate::domain::scope::Scope;
use anyhow::Result;

#[cfg(test)]
mod commands_tests;

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
    pub fn from_cli(cli: &Cli) -> Self {
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

pub fn println_if_not_quiet(mode: &OutputMode, msg: &str) {
    match mode {
        OutputMode::Quiet => {}
        _ => println!("{}", msg),
    }
}

pub fn eprintln_if_not_quiet(mode: &OutputMode, msg: &str) {
    match mode {
        OutputMode::Quiet => {}
        _ => eprintln!("{}", msg),
    }
}

pub fn print_json<T: serde::Serialize>(mode: &OutputMode, value: &T) -> Result<()> {
    if matches!(mode, OutputMode::Json) {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

pub fn telemetry_to_csv(config: &crate::domain::telemetry::AnalyticsConfig) -> String {
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

pub fn resolve_scope(scope_arg: Option<ScopeArg>) -> Scope {
    scope_arg
        .map(|s| s.into_domain_scope())
        .unwrap_or(Scope::Workspace)
}

pub fn active_providers_from_config<'a>(
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

pub fn find_package_by_full_identity(
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

pub fn generate_profile_session_key() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let pid = std::process::id();
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{}-{}-{}", timestamp, pid, nonce)
}

// ---------------------------------------------------------------------------
// Exit codes
// ---------------------------------------------------------------------------

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_GENERAL_FAILURE: i32 = 1;
pub const EXIT_VALIDATION_FAILURE: i32 = 2;
pub const EXIT_PARTIAL_SUCCESS: i32 = 3;

// ---------------------------------------------------------------------------
// Feature modules
// ---------------------------------------------------------------------------

pub mod assets;
pub mod telemetry;

// ---------------------------------------------------------------------------
// Command dispatcher
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

        Some(Commands::Sync { global, dry_run }) => {
            assets::cmd_sync(&cli, global, dry_run, workspace)
        }

        Some(Commands::Install {
            ref identity,
            scope,
            dry_run,
            ref provider,
            evals,
        }) => assets::cmd_install(
            &cli,
            identity,
            scope,
            dry_run,
            provider.as_deref(),
            evals,
            workspace,
        ),

        Some(Commands::Validate { scope }) => assets::cmd_validate(&cli, scope, workspace),

        Some(Commands::Pack {
            ref identity,
            target,
            stdout,
        }) => assets::cmd_pack(&cli, identity, target, stdout, workspace),

        Some(Commands::Mcp { .. }) => {
            println!("MCP commands are wired to AgkCore; run via `cli::core_dispatcher` instead.");
            Ok(EXIT_SUCCESS)
        }

        Some(Commands::Telemetry { ref command }) => telemetry::dispatch_telemetry(&cli, command),

        Some(Commands::Profile { .. }) => {
            println!("Profile commands are wired to AgkCore; run via `cli::core_dispatcher` instead.");
            Ok(EXIT_SUCCESS)
        }

        Some(Commands::Apply {
            source: _,
            scope: _,
            context: _,
            environment: _,
            dry_run: _,
        }) => {
            println!("Apply command is wired to AgkCore; run via `cli::core_dispatcher` instead.");
            Ok(EXIT_SUCCESS)
        }

        Some(Commands::Context { command: _ }) => {
            println!("Context commands are wired to AgkCore; run via `cli::core_dispatcher`.");
            Ok(EXIT_SUCCESS)
        }

        None => {
            // No subcommand — fall through to TUI in main.rs
            Ok(EXIT_SUCCESS)
        }
    }
}
