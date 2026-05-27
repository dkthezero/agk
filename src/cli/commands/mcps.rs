use super::*;
use crate::cli::entry::{Cli, McpCommands};
use anyhow::Result;

pub fn dispatch_mcp(cli: &Cli, command: &McpCommands, workspace: &std::path::Path) -> Result<i32> {
    match command {
        McpCommands::Add {
            ref name,
            ref command,
            ref args,
            ref env,
            ref transport,
            ref description,
            no_test,
        } => {
            let mode = OutputMode::from_cli(cli);
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
                                eprintln_if_not_quiet(&mode, &format!("MCP test failed: {}", e));
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
        McpCommands::Enable {
            ref name,
            ref provider,
            scope,
        } => {
            let mode = OutputMode::from_cli(cli);
            let scope = resolve_scope(*scope);
            let providers = crate::infra::mcp::build_mcp_providers(workspace);
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
        McpCommands::Disable {
            ref name,
            ref provider,
            scope,
        } => {
            let mode = OutputMode::from_cli(cli);
            let scope = resolve_scope(*scope);
            let providers = crate::infra::mcp::build_mcp_providers(workspace);
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
        McpCommands::List { provider: _ } => {
            let mode = OutputMode::from_cli(cli);
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
            } else if items.is_empty() {
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
            Ok(EXIT_SUCCESS)
        }
        McpCommands::Test { ref name } => {
            let mode = OutputMode::from_cli(cli);
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
    }
}
