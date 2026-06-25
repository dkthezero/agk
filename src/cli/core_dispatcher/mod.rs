mod llm;
mod mcp;
mod profile;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::CoreEventSink;
#[cfg(feature = "pack")]
use crate::cli::entry::PackTarget;
use crate::cli::entry::{
    Cli, Commands, ContextCommands, DebugCommands, TeamCommands, TelemetryCommands, VaultCommands,
};
use crate::cli::presenter::CliPresenter;
use crate::domain::context::ContextId;
use crate::domain::scope::Scope;

/// Phase 4+4.5: routes all CLI commands through [`AgkCore`] instead of
/// calling inline handlers directly.  This makes the CLI and TUI share the
/// exact same use-case implementations.
pub fn dispatch(cli: &Cli, workspace: &std::path::Path, core: &AgkCore) -> anyhow::Result<i32> {
    let mut presenter = CliPresenter::new(cli.json, cli.quiet);

    // The `agk llm` subcommand does not (yet) flow through `AgkCore::execute`
    // because the LLM ports (store/factory/health-check) are constructed on
    // demand from the workspace.  Route it first so the rest of the dispatcher
    // can stay a pure CoreCommand translator.
    if let Some(Commands::Llm { command }) = &cli.command {
        let args = crate::cli::llm::LlmArgs {
            config_dir: std::path::PathBuf::from("."),
            command: command.clone(),
        };
        let rc = match llm::dispatch(&args, workspace, &mut presenter) {
            Ok(code) => code,
            Err(e) => {
                // `llm::dispatch` returns `Err` for failures that did not emit
                // a structured event (e.g. an unknown provider id or a
                // malformed kind).  Push the error into the JSON batch (in
                // JSON mode) or render it to stderr (text mode) via
                // `on_error`, then surface a non-zero exit code — mirroring
                // the main `Err` arm below so `--json` consumers always get a
                // structured error instead of a stray `Error: ...` line from
                // `main.rs`.
                if !presenter.already_reported_task_failure() {
                    presenter.on_error(format!("{}", e));
                }
                crate::cli::EXIT_GENERAL_FAILURE
            }
        };
        presenter.finalize();
        return Ok(rc);
    }

    if let Some(command) = &cli.command {
        let cmd = to_core_command(command, workspace)?;
        let result = core.execute(cmd, &mut presenter);
        let exit = match result {
            Ok(crate::app::outcome::CoreOutcome::ValidationReport { passed: false, .. }) => {
                crate::cli::EXIT_GENERAL_FAILURE
            }
            Ok(_) => crate::cli::EXIT_SUCCESS,
            Err(e) => {
                // Some use cases emit a `TaskFailed` CoreEvent (which renders
                // `[0] Failed: ...` to stderr) AND return `Err` so the exit
                // code is non-zero.  In that case the failure has already
                // been rendered; avoid a duplicate `Error: ...` line.
                if !presenter.already_reported_task_failure() {
                    presenter.on_error(format!("{}", e));
                }
                crate::cli::EXIT_GENERAL_FAILURE
            }
        };
        // `finalize()` prints the JSON batch (if any).  It must run AFTER the
        // `Err` arm so that an `Err`-only failure (which `on_error` pushes
        // into `events` as a structured `CoreEvent::Error`) is included in the
        // batch rather than dropped.
        presenter.finalize();
        Ok(exit)
    } else {
        // No subcommand — fall through to TUI in main.rs
        Ok(crate::cli::EXIT_SUCCESS)
    }
}

fn to_core_command(cmd: &Commands, _workspace: &std::path::Path) -> anyhow::Result<CoreCommand> {
    match cmd {
        Commands::Llm { .. } => Err(anyhow::anyhow!(
            "`agk llm` is handled by core_dispatcher::llm::dispatch before this point"
        )),
        Commands::Profile { command } => profile::to_core_command(command),
        Commands::Mcp { command } => mcp::to_core_command(command),
        Commands::Sync { global, dry_run } => Ok(CoreCommand::SyncAssets {
            scope: if *global {
                Scope::Global
            } else {
                Scope::Workspace
            },
            dry_run: *dry_run,
        }),
        Commands::Install {
            identity,
            scope,
            dry_run,
            provider,
            evals,
        } => Ok(CoreCommand::InstallAsset {
            identity: identity.clone(),
            scope: scope
                .map(|s| s.into_domain_scope())
                .unwrap_or(Scope::Workspace),
            provider_filter: provider.clone(),
            include_evals: *evals,
            dry_run: *dry_run,
        }),
        Commands::Context { command } => match command {
            ContextCommands::Switch { name, dry_run } => Ok(CoreCommand::SwitchContext {
                id: ContextId::new(name.clone()),
                dry_run: *dry_run,
            }),
            ContextCommands::List => Ok(CoreCommand::ListContexts),
            ContextCommands::Create {
                name,
                display_name: _display_name,
            } => Ok(CoreCommand::ApplyConfig {
                input: crate::app::features::apply::command::ApplyConfigInput::from_url(format!(
                    "context://{}",
                    name
                )),
                scope: Scope::Global,
                environment: None,
                context: Some(ContextId::new(name.clone())),
                dry_run: false,
            }),
        },
        Commands::Apply {
            source,
            scope,
            context,
            environment,
            dry_run,
        } => Ok(CoreCommand::ApplyConfig {
            input: crate::app::features::apply::command::ApplyConfigInput::from_url(source.clone()),
            scope: scope.into_domain_scope(),
            environment: environment
                .as_ref()
                .map(|e| crate::domain::context::Environment::from(*e)),
            context: context
                .as_deref()
                .map(crate::domain::context::ContextId::new),
            dry_run: *dry_run,
        }),
        Commands::Clean { global } => Ok(CoreCommand::CleanWorkspace { global: *global }),
        Commands::Validate { scope } => Ok(CoreCommand::ValidateAssets {
            scope: scope
                .map(|s| s.into_domain_scope())
                .unwrap_or(Scope::Workspace),
        }),
        #[cfg(feature = "pack")]
        Commands::Pack {
            identity,
            target,
            stdout,
        } => Ok(CoreCommand::PackAsset {
            identity: identity.clone(),
            target: match target {
                PackTarget::ClaudeDesktop => crate::domain::asset::PackTarget::ClaudeDesktop,
                PackTarget::Firebender => crate::domain::asset::PackTarget::Firebender,
                PackTarget::Tarball => crate::domain::asset::PackTarget::Tarball,
            },
            stdout: *stdout,
            scope: Scope::Workspace,
        }),
        Commands::Telemetry { command } => match command {
            TelemetryCommands::Enable => Ok(CoreCommand::EnableTelemetry),
            TelemetryCommands::Disable => Ok(CoreCommand::DisableTelemetry),
            TelemetryCommands::Status => Ok(CoreCommand::TelemetryStatus),
            TelemetryCommands::Export { format, output } => Ok(CoreCommand::ExportTelemetry {
                format: match format {
                    crate::cli::entry::ExportFormat::Json => {
                        crate::domain::telemetry::TelemetryExportFormat::Json
                    }
                    crate::cli::entry::ExportFormat::Csv => {
                        crate::domain::telemetry::TelemetryExportFormat::Csv
                    }
                },
                output_path: output.clone(),
            }),
        },
        Commands::Debug { command } => match command {
            DebugCommands::Tasks => Ok(CoreCommand::DebugListTasks),
            DebugCommands::Hangs => Ok(CoreCommand::DebugDetectHangs),
            DebugCommands::Trace => Ok(CoreCommand::DebugDumpTrace),
        },
        Commands::Vault { command } => match command {
            VaultCommands::Init { name, dry_run } => Ok(CoreCommand::VaultInit {
                name: name.clone(),
                dry_run: *dry_run,
            }),
            // Manifest read/parse/canonicalize live in the vault use-case
            // (`attach::attach_local`) so this stays a pure translator.
            VaultCommands::Attach { path, id } => Ok(CoreCommand::AttachLocalVault {
                path: path.clone(),
                id: id.clone(),
            }),
        },
        Commands::Team { command } => match command {
            TeamCommands::Init { name, dry_run } => Ok(CoreCommand::TeamInit {
                name: name.clone(),
                dry_run: *dry_run,
            }),
            TeamCommands::AddVault {
                identity,
                vault_type,
                url,
                branch,
            } => Ok(CoreCommand::TeamAddVault {
                identity: identity.clone(),
                vault_type: vault_type.clone(),
                url: url.clone(),
                branch: branch.clone(),
            }),
            TeamCommands::Add {
                identity,
                vault,
                kind,
                version_constraint,
            } => Ok(CoreCommand::TeamAddRequirement {
                identity: identity.clone(),
                vault: vault.clone(),
                kind: kind.clone(),
                version_constraint: version_constraint.clone(),
            }),
            TeamCommands::Remove { identity } => Ok(CoreCommand::TeamRemove {
                identity: identity.clone(),
            }),
            TeamCommands::Diff => Ok(CoreCommand::TeamDiff),
            TeamCommands::Status => Ok(CoreCommand::TeamStatus),
            TeamCommands::Update => Ok(CoreCommand::TeamUpdate),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::registry::Registry;
    use crate::app::test_support::FakeStore;
    use crate::cli::entry::{Cli, ProfileCommands, ScopeArg};
    use crate::domain::config::{AssetBucket, AssetSource, ConfigFile, VaultSection};
    use crate::domain::scope::Scope;
    use std::sync::Arc;

    #[test]
    fn to_core_command_profile_start() {
        let cmd = Commands::Profile {
            command: ProfileCommands::Start {
                name: "dev".into(),
                dry_run: true,
            },
        };
        let core = to_core_command(&cmd, std::path::Path::new(".")).unwrap();
        assert!(matches!(
            core,
            CoreCommand::StartProfile { id, dry_run: true, .. }
            if id.as_str() == "dev"
        ));
    }

    #[test]
    fn to_core_command_profile_create() {
        let cmd = Commands::Profile {
            command: ProfileCommands::Create {
                name: "test".into(),
                provider: "opencode".into(),
                skills: vec!["rust".into()],
                mcps: vec![],
                description: None,
                description_file: None,
                scope: ScopeArg::Workspace,
                dry_run: false,
            },
        };
        let result = to_core_command(&cmd, std::path::Path::new(".")).unwrap();
        assert!(matches!(
            result,
            CoreCommand::CreateProfile { input } if input.id.as_str() == "test"
        ));
    }

    /// Regression: `agk validate` must exit non-zero when validation fails.
    /// Previously it printed "Validation failed" but returned exit 0 because
    /// the use case returned `Ok(CoreOutcome::Ok)`.
    fn validate_cli() -> Cli {
        Cli {
            command: Some(Commands::Validate { scope: None }),
            quiet: false,
            verbose: false,
            json: false,
        }
    }

    #[test]
    fn dispatch_validate_returns_failure_exit_on_failed_validation() {
        let store = FakeStore::new();
        let mut config = ConfigFile::default();
        config.vault_defs.insert(
            "ghost-vault".to_string(),
            VaultSection {
                vault: None,
                skills: Some(AssetBucket {
                    items: vec!["[ghost:1.0.0:deadbeef00]".to_string()],
                    source: Some(AssetSource::Personal),
                }),
                instructions: None,
                mcps: None,
                profiles: None,
            },
        );
        store.seed(Scope::Workspace, config);

        let registry = Registry::new();
        let core = crate::app::core::test_core_with(Arc::new(store), Arc::new(registry));
        let cli = validate_cli();

        let rc = dispatch(&cli, std::path::Path::new("."), &core).unwrap();
        assert_eq!(
            rc,
            crate::cli::EXIT_GENERAL_FAILURE,
            "a failed validation must surface as a non-zero exit code"
        );
    }

    #[test]
    fn dispatch_validate_returns_success_exit_on_passed_validation() {
        // Empty config -> no installed assets -> nothing fails -> passed.
        let store = FakeStore::new();
        store.seed(Scope::Workspace, ConfigFile::default());

        let registry = Registry::new();
        let core = crate::app::core::test_core_with(Arc::new(store), Arc::new(registry));
        let cli = validate_cli();

        let rc = dispatch(&cli, std::path::Path::new("."), &core).unwrap();
        assert_eq!(
            rc,
            crate::cli::EXIT_SUCCESS,
            "a passing validation must keep exit code 0"
        );
    }

    /// Regression: `agk sync` must exit non-zero when there are no active
    /// providers.  Previously the use case emitted a `TaskFailed` event
    /// (rendering `[0] Failed: No active providers`) but returned
    /// `Ok(CoreOutcome::Ok)`, so the CLI exited 0 despite the failure —
    /// the `TaskFailed`-then-`Ok` anti-pattern documented in AGENTS.md.
    fn sync_cli() -> Cli {
        Cli {
            command: Some(Commands::Sync {
                global: false,
                dry_run: false,
            }),
            quiet: false,
            verbose: false,
            json: false,
        }
    }

    #[test]
    fn dispatch_sync_returns_failure_exit_when_no_active_providers() {
        // Empty config -> no providers active -> sync must fail.
        let store = FakeStore::new();
        store.seed(Scope::Workspace, ConfigFile::default());

        let registry = Registry::new();
        let core = crate::app::core::test_core_with(Arc::new(store), Arc::new(registry));
        let cli = sync_cli();

        let rc = dispatch(&cli, std::path::Path::new("."), &core).unwrap();
        assert_eq!(
            rc,
            crate::cli::EXIT_GENERAL_FAILURE,
            "sync with no active providers must surface as a non-zero exit code"
        );
    }

    /// Regression: `agk sync --dry-run` is a *preview* — it computes what
    /// would be synced without touching providers (the per-asset loop
    /// short-circuits with `continue` before the provider update).  Enforcing
    /// the empty-providers guard on a dry-run made `agk sync --dry-run` fail
    /// with "No active providers" even though it needs none, masking the
    /// useful preview of installed/available assets.
    fn sync_dry_run_cli() -> Cli {
        Cli {
            command: Some(Commands::Sync {
                global: false,
                dry_run: true,
            }),
            quiet: false,
            verbose: false,
            json: false,
        }
    }

    #[test]
    fn dispatch_sync_dry_run_succeeds_with_no_active_providers() {
        // Empty config -> no providers active, but dry_run=true must NOT
        // trip the empty-providers guard — the preview should succeed and
        // emit a (possibly empty) SyncComplete event.
        let store = FakeStore::new();
        store.seed(Scope::Workspace, ConfigFile::default());

        let registry = Registry::new();
        let core = crate::app::core::test_core_with(Arc::new(store), Arc::new(registry));
        let cli = sync_dry_run_cli();

        let rc = dispatch(&cli, std::path::Path::new("."), &core).unwrap();
        assert_eq!(
            rc,
            crate::cli::EXIT_SUCCESS,
            "sync --dry-run is a preview and must not fail when no providers are active"
        );
    }

    /// Regression: `agk install <identity>` must exit non-zero when there
    /// are no active providers (same `TaskFailed`-then-`Ok` anti-pattern as
    /// sync).
    fn install_cli(identity: &str) -> Cli {
        Cli {
            command: Some(Commands::Install {
                identity: identity.to_string(),
                scope: None,
                dry_run: false,
                provider: None,
                evals: false,
            }),
            quiet: false,
            verbose: false,
            json: false,
        }
    }

    #[test]
    fn dispatch_install_returns_failure_exit_when_no_active_providers() {
        let store = FakeStore::new();
        store.seed(Scope::Workspace, ConfigFile::default());

        let registry = Registry::new();
        let core = crate::app::core::test_core_with(Arc::new(store), Arc::new(registry));
        let cli = install_cli("ghost:1.0.0:deadbeef00");

        let rc = dispatch(&cli, std::path::Path::new("."), &core).unwrap();
        assert_eq!(
            rc,
            crate::cli::EXIT_GENERAL_FAILURE,
            "install with no active providers must surface as a non-zero exit code"
        );
    }

    /// Regression: `agk context switch <name>` must exit non-zero when the
    /// target context does not exist.  Previously the use case called
    /// `sink.on_error` (rendering `Error: Context '...' does not exist`) but
    /// returned `Ok(CoreOutcome::Ok)`, so the CLI exited 0 despite the
    /// failure — the `on_error`-then-`Ok` anti-pattern.
    fn context_switch_cli(name: &str) -> Cli {
        Cli {
            command: Some(Commands::Context {
                command: ContextCommands::Switch {
                    name: name.to_string(),
                    dry_run: false,
                },
            }),
            quiet: false,
            verbose: false,
            json: false,
        }
    }

    #[test]
    fn dispatch_context_switch_returns_failure_exit_for_missing_context() {
        // `test_core`'s FakeCtxStore returns an empty ContextFile, so any
        // non-"default" context lookup fails — but note `default` is also
        // absent from the map (only `current_context` is set), so even
        // switching to "default" is a no-op error here.  Use an explicit
        // missing name to make the intent unambiguous.
        let store = FakeStore::new();
        store.seed(Scope::Workspace, ConfigFile::default());

        let registry = Registry::new();
        let core = crate::app::core::test_core_with(Arc::new(store), Arc::new(registry));
        let cli = context_switch_cli("nonexistent");

        let rc = dispatch(&cli, std::path::Path::new("."), &core).unwrap();
        assert_eq!(
            rc,
            crate::cli::EXIT_GENERAL_FAILURE,
            "switching to a missing context must surface as a non-zero exit code"
        );
    }

    /// Regression: `agk <cmd> --json` that fails with an `Err`-only error (no
    /// structured failure event) must still surface a non-zero exit code while
    /// the structured `CoreEvent::Error` is emitted into the JSON batch
    /// (asserted at the presenter level in
    /// `presenter::tests::on_error_in_json_mode_pushes_structured_error_event`).
    fn context_switch_json_cli(name: &str) -> Cli {
        Cli {
            command: Some(Commands::Context {
                command: ContextCommands::Switch {
                    name: name.to_string(),
                    dry_run: false,
                },
            }),
            quiet: false,
            verbose: false,
            json: true,
        }
    }

    #[test]
    fn dispatch_json_mode_failure_returns_nonzero_exit() {
        let store = FakeStore::new();
        store.seed(Scope::Workspace, ConfigFile::default());

        let registry = Registry::new();
        let core = crate::app::core::test_core_with(Arc::new(store), Arc::new(registry));
        let cli = context_switch_json_cli("nonexistent");

        let rc = dispatch(&cli, std::path::Path::new("."), &core).unwrap();
        assert_eq!(
            rc,
            crate::cli::EXIT_GENERAL_FAILURE,
            "a JSON-mode failure must still surface a non-zero exit code"
        );
    }

    /// Regression: `agk pack <identity>` must exit non-zero when the asset is
    /// not found in any vault.  Previously the use case called `sink.on_error`
    /// (rendering `Error: Asset '...' not found in any vault`) but returned
    /// `Ok(CoreOutcome::Ok)`, so the CLI exited 0 despite the failure.
    #[cfg(feature = "pack")]
    #[test]
    fn dispatch_pack_returns_failure_exit_for_missing_asset() {
        let store = FakeStore::new();
        store.seed(Scope::Workspace, ConfigFile::default());

        // Empty registry -> no package matches the identity.
        let registry = Registry::new();
        let core = crate::app::core::test_core_with(Arc::new(store), Arc::new(registry));
        let cli = Cli {
            command: Some(Commands::Pack {
                identity: "ghost-skill:1.0.0:deadbeef00".to_string(),
                target: crate::cli::entry::PackTarget::ClaudeDesktop,
                stdout: false,
            }),
            quiet: false,
            verbose: false,
            json: false,
        };

        let rc = dispatch(&cli, std::path::Path::new("."), &core).unwrap();
        assert_eq!(
            rc,
            crate::cli::EXIT_GENERAL_FAILURE,
            "packing a missing asset must surface as a non-zero exit code"
        );
    }
}
