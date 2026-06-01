use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::CoreEventSink;
use crate::cli::entry::{Cli, Commands, McpCommands, ProfileCommands, TelemetryCommands};
use crate::cli::presenter::CliPresenter;
use crate::domain::profile::ProfileId;
use crate::domain::scope::Scope;
use anyhow::Context;

/// Phase 4+4.5: routes all CLI commands through [`AgkCore`] instead of
/// calling inline handlers directly.  This makes the CLI and TUI share the
/// exact same use-case implementations.
pub fn dispatch(cli: &Cli, workspace: &std::path::Path, core: &AgkCore) -> anyhow::Result<i32> {
    let mut presenter = CliPresenter::new(cli.json, cli.quiet);

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
        Commands::Profile { command } => match command {
            ProfileCommands::Start { name, dry_run } => Ok(CoreCommand::StartProfile {
                id: ProfileId::new(name.clone()),
                scope: Scope::Workspace,
                dry_run: *dry_run,
            }),
            ProfileCommands::Create {
                name,
                provider,
                skills,
                mcps,
                description,
                description_file,
                scope,
                dry_run: _,
            } => {
                let desc = if let Some(path) = description_file {
                    std::fs::read_to_string(path)
                        .with_context(|| format!("Failed to read description file: {}", path))?
                } else {
                    description.clone().unwrap_or_default()
                };
                Ok(CoreCommand::CreateProfile {
                    input: crate::app::features::profile::command::CreateProfileInput {
                        id: crate::domain::profile::ProfileId::new(name.clone()),
                        provider_id: crate::domain::profile::ProviderId::new(provider.clone()),
                        skill_refs: skills
                            .iter()
                            .map(|s| {
                                crate::domain::profile::ProfileAssetRef::new(s.clone(), "auto")
                            })
                            .collect(),
                        mcp_refs: mcps
                            .iter()
                            .map(|m| {
                                crate::domain::profile::ProfileAssetRef::new(m.clone(), "auto")
                            })
                            .collect(),
                        instruction_refs: vec![],
                        description: desc,
                        scope: scope.into_domain_scope(),
                    },
                })
            }
            ProfileCommands::Export {
                name,
                file,
                resolve_vaults,
                scope,
            } => Ok(CoreCommand::ExportProfile {
                profile_id: ProfileId::new(name.clone()),
                scope: scope.into_domain_scope(),
                file_path: Some(file.clone()),
                resolve_vaults: *resolve_vaults,
            }),
            ProfileCommands::Import {
                file_path,
                name,
                scope,
            } => Ok(CoreCommand::ImportProfile {
                file_path: file_path.clone(),
                target_name: name.clone(),
                scope: scope.into_domain_scope(),
            }),
            ProfileCommands::Diff { name, scope } => Ok(CoreCommand::DiffProfile {
                id: ProfileId::new(name.clone()),
                scope: scope.into_domain_scope(),
            }),
        },
        Commands::Mcp { command } => match command {
            McpCommands::Add {
                name,
                command,
                args,
                env,
                transport,
                description,
                no_test,
            } => Ok(CoreCommand::RegisterMcp {
                input: crate::app::features::mcp::command::RegisterMcpInput {
                    name: name.clone(),
                    command: command.clone(),
                    args: args
                        .clone()
                        .map(|s| s.split_whitespace().map(|a| a.to_string()).collect())
                        .unwrap_or_default(),
                    env: env
                        .clone()
                        .map(|s| {
                            s.split(',')
                                .filter_map(|pair| {
                                    let mut parts = pair.splitn(2, '=');
                                    let key = parts.next()?.trim().to_string();
                                    let val = parts.next()?.trim().to_string();
                                    Some((key, val))
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    transport: match transport.as_str() {
                        "sse" => crate::domain::mcp::McpTransport::Sse {
                            url: "http://localhost:3000".to_string(),
                        },
                        _ => crate::domain::mcp::McpTransport::Stdio,
                    },
                    description: description.clone(),
                    test_after: !no_test,
                },
            }),
            McpCommands::Enable {
                name,
                provider,
                scope,
            } => Ok(CoreCommand::EnableMcp {
                name: name.clone(),
                provider_id: provider.clone(),
                scope: scope
                    .map(|s| s.into_domain_scope())
                    .unwrap_or(Scope::Workspace),
            }),
            McpCommands::Disable {
                name,
                provider,
                scope,
            } => Ok(CoreCommand::DisableMcp {
                name: name.clone(),
                provider_id: provider.clone(),
                scope: scope
                    .map(|s| s.into_domain_scope())
                    .unwrap_or(Scope::Workspace),
            }),
            McpCommands::List { provider: _ } => Ok(CoreCommand::ListMcp),
            McpCommands::Test { name } => Ok(CoreCommand::TestMcp { name: name.clone() }),
        },
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
            crate::cli::entry::ContextCommands::Switch { name, dry_run } => {
                Ok(CoreCommand::SwitchContext {
                    id: crate::domain::context::ContextId::new(name.clone()),
                    dry_run: *dry_run,
                })
            }
            crate::cli::entry::ContextCommands::List => Ok(CoreCommand::ListContexts),
            crate::cli::entry::ContextCommands::Create {
                name,
                display_name: _display_name,
            } => Ok(CoreCommand::ApplyConfig {
                input: crate::app::features::apply::command::ApplyConfigInput::from_url(format!(
                    "context://{}",
                    name
                )),
                scope: crate::domain::scope::Scope::Global,
                environment: None,
                context: Some(crate::domain::context::ContextId::new(name.clone())),
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
                .unwrap_or(crate::domain::scope::Scope::Workspace),
        }),
        Commands::Pack {
            identity,
            target,
            stdout,
        } => Ok(CoreCommand::PackAsset {
            identity: identity.clone(),
            target: match target {
                crate::cli::entry::PackTarget::ClaudeDesktop => {
                    crate::domain::asset::PackTarget::ClaudeDesktop
                }
                crate::cli::entry::PackTarget::Firebender => {
                    crate::domain::asset::PackTarget::Firebender
                }
                crate::cli::entry::PackTarget::Tarball => crate::domain::asset::PackTarget::Tarball,
            },
            stdout: *stdout,
            scope: crate::domain::scope::Scope::Workspace,
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
            crate::cli::entry::DebugCommands::Tasks => Ok(CoreCommand::DebugListTasks),
            crate::cli::entry::DebugCommands::Hangs => Ok(CoreCommand::DebugDetectHangs),
            crate::cli::entry::DebugCommands::Trace => Ok(CoreCommand::DebugDumpTrace),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::entry::ScopeArg;

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
