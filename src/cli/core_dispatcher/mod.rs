mod llm;
mod mcp;
mod profile;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::CoreEventSink;
use crate::cli::entry::{
    Cli, Commands, ContextCommands, DebugCommands, TeamCommands, TelemetryCommands,
    VaultCommands,
};
#[cfg(feature = "pack")]
use crate::cli::entry::PackTarget;
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
        let rc = llm::dispatch(&args, workspace, &mut presenter);
        presenter.finalize();
        return rc;
    }

    if let Some(command) = &cli.command {
        let cmd = to_core_command(command, workspace)?;
        let result = core.execute(cmd, &mut presenter);
        presenter.finalize();
        match result {
            Ok(_) => Ok(crate::cli::EXIT_SUCCESS),
            Err(e) => {
                presenter.on_error(format!("{}", e));
                Ok(crate::cli::EXIT_GENERAL_FAILURE)
            }
        }
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
    use crate::cli::entry::{ProfileCommands, ScopeArg};

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
}
