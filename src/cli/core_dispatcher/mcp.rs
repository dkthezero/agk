use crate::app::command::CoreCommand;
use crate::app::features::mcp::command::RegisterMcpInput;
use crate::cli::entry::McpCommands;
use crate::domain::mcp::McpTransport;
use crate::domain::scope::Scope;

pub(super) fn to_core_command(command: &McpCommands) -> anyhow::Result<CoreCommand> {
    match command {
        McpCommands::Add {
            name,
            command,
            args,
            env,
            transport,
            description,
            no_test,
        } => Ok(CoreCommand::RegisterMcp {
            input: RegisterMcpInput {
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
                    "sse" => McpTransport::Sse {
                        url: "http://localhost:3000".to_string(),
                    },
                    _ => McpTransport::Stdio,
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
    }
}
