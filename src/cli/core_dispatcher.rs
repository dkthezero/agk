use crate::app::command::{CoreCommand, CreateProfileInput};
use crate::app::core::AgkCore;
use crate::app::outcome::CoreEventSink;
use crate::cli::entry::{Cli, Commands, McpCommands, ProfileCommands, ScopeArg, TelemetryCommands};
use crate::cli::presenter::CliPresenter;
use crate::domain::profile::ProfileId;
use crate::domain::scope::Scope;

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
            Ok(_) => Ok(crate::cli::commands::EXIT_SUCCESS),
            Err(e) => {
                presenter.on_error(format!("{}", e));
                Ok(crate::cli::commands::EXIT_GENERAL_FAILURE)
            }
        }
    } else {
        // No subcommand — fall through to TUI in main.rs
        Ok(crate::cli::commands::EXIT_SUCCESS)
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
                        .map_err(|e| anyhow::anyhow!("Failed to read description file: {}", e))?
                } else {
                    description.clone().unwrap_or_default()
                };
                let mut input = CreateProfileInput::new(
                    ProfileId::new(name.clone()),
                    crate::domain::profile::ProviderId::new(provider.clone()),
                    scope.into_domain_scope(),
                );
                input.description = desc;
                input.skill_refs = skills
                    .iter()
                    .map(|s| crate::domain::profile::SkillId::new(s))
                    .collect();
                input.mcp_refs = mcps
                    .iter()
                    .map(|m| crate::domain::profile::McpServerId::new(m))
                    .collect();
                Ok(CoreCommand::CreateProfile { input })
            }
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
                input: crate::app::command::RegisterMcpInput {
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
            McpCommands::List { .. } => Err(anyhow::anyhow!(
                "MCP list command not yet implemented in AgkCore"
            )),
            McpCommands::Test { .. } => Err(anyhow::anyhow!(
                "MCP test command not yet implemented in AgkCore"
            )),
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
            crate::cli::entry::ContextCommands::Create { name, display_name } => {
                Ok(CoreCommand::ApplyConfig {
                    input: crate::app::command::ApplyConfigInput::from_url(format!(
                        "context://{}",
                        name
                    )),
                    scope: crate::domain::scope::Scope::Global,
                    environment: None,
                    context: Some(crate::domain::context::ContextId::new(name.clone())),
                    dry_run: false,
                })
            }
        },
        Commands::Apply {
            source,
            scope,
            context,
            environment,
            dry_run,
        } => Ok(CoreCommand::ApplyConfig {
            input: crate::app::command::ApplyConfigInput::from_url(source.clone()),
            scope: scope.into_domain_scope(),
            environment: environment
                .as_ref()
                .map(|e| crate::domain::context::Environment::from(*e)),
            context: context
                .as_deref()
                .map(crate::domain::context::ContextId::new),
            dry_run: *dry_run,
        }),
        Commands::Clean { .. } => Err(anyhow::anyhow!(
            "Clean command not yet implemented in AgkCore"
        )),
        Commands::Validate { .. } => Err(anyhow::anyhow!(
            "Validate command not yet implemented in AgkCore"
        )),
        Commands::Pack { .. } => Err(anyhow::anyhow!(
            "Pack command not yet implemented in AgkCore"
        )),
        Commands::Telemetry { command } => match command {
            TelemetryCommands::Enable => Err(anyhow::anyhow!(
                "Telemetry enable not yet implemented in AgkCore"
            )),
            TelemetryCommands::Disable => Err(anyhow::anyhow!(
                "Telemetry disable not yet implemented in AgkCore"
            )),
            TelemetryCommands::Status => Err(anyhow::anyhow!(
                "Telemetry status not yet implemented in AgkCore"
            )),
            TelemetryCommands::Export { .. } => Err(anyhow::anyhow!(
                "Telemetry export not yet implemented in AgkCore"
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::entry::Cli;

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
        let core = to_core_command(&cmd, std::path::Path::new(".")).unwrap();
        assert!(matches!(
            core,
            CoreCommand::CreateProfile { ref input }
            if input.id.as_str() == "test" && input.provider_id.as_str() == "opencode"
        ));
    }
}
