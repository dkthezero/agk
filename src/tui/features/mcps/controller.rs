use crate::tui::app::{AppState, ListMode};
use crate::tui::event::{AppEvent, ControlFlow, EventContext};
use anyhow::Result;
use crossterm::event::KeyCode;

pub fn handle_space_mcp(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    let items = state.mcp_state.servers_list();
    let Some((id, server)) = items.get(state.selected_index).copied() else {
        return Ok(());
    };
    let name = id.clone();

    let mcp_providers = crate::infra::mcp::build_mcp_providers(&ctx.workspace_root);
    let supported_ids: std::collections::HashSet<&str> =
        mcp_providers.iter().map(|p| p.provider_id()).collect();

    let target_providers: Vec<String> = state
        .provider_entries
        .iter()
        .filter(|p| p.active && supported_ids.contains(p.id.as_str()))
        .map(|p| p.id.clone())
        .collect();

    if target_providers.is_empty() {
        state.status_line =
            "No MCP-capable providers active. Activate Claude Code or OpenCode in Providers tab."
                .to_string();
        return Ok(());
    }

    let is_enabled = target_providers.iter().any(|pid| {
        server
            .activation
            .get(pid)
            .map(|a| match state.active_scope {
                crate::domain::scope::Scope::Global => a.global,
                crate::domain::scope::Scope::Workspace => a.workspace,
            })
            .unwrap_or(false)
    });

    let scope = state.active_scope;
    let tx = ctx.tx.clone();
    let ws = ctx.workspace_root.clone();

    let task_id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    tokio::task::spawn_blocking(move || {
        let action = if is_enabled { "Disabling" } else { "Enabling" };
        let _ = tx.send(AppEvent::TaskStarted {
            id: task_id,
            name: format!("{} MCP server '{}'", action, name),
        });

        let providers = crate::infra::mcp::build_mcp_providers(&ws);

        let mut success = true;
        for pid in &target_providers {
            let result = if is_enabled {
                crate::infra::mcp::disable(&name, pid, scope, &providers)
            } else {
                crate::infra::mcp::enable(&name, pid, scope, &providers)
            };
            if let Err(e) = result {
                let _ = tx.send(AppEvent::TaskFailed {
                    id: task_id,
                    error: format!("Failed for {}: {}", pid, e),
                });
                success = false;
                break;
            }
        }

        let _ = tx.send(AppEvent::TaskProgress {
            id: task_id,
            percent: 100,
        });
        let _ = tx.send(AppEvent::TriggerReload);
        if success {
            let done = if is_enabled { "Disabled" } else { "Enabled" };
            let _ = tx.send(AppEvent::TaskCompleted {
                id: task_id,
                message: format!("{} MCP server '{}'", done, name),
            });
        }
    });

    Ok(())
}

pub fn handle_register_mcp_input(
    state: &mut AppState,
    _ctx: &EventContext,
    code: &KeyCode,
) -> Result<()> {
    match code {
        KeyCode::Char(c) => {
            state.prompt_buffer.push(*c);
        }
        KeyCode::Backspace => {
            state.prompt_buffer.pop();
        }
        KeyCode::Enter => match state.list_mode {
            ListMode::RegisterMcpStepName => {
                state.pending_mcp_name =
                    std::mem::take(&mut state.prompt_buffer).trim().to_string();
                if state.pending_mcp_name.is_empty() {
                    state.list_mode = ListMode::Normal;
                    state.status_line = "Cancelled — name required".to_string();
                } else {
                    state.list_mode = ListMode::RegisterMcpStepCommand;
                    state.status_line.clear();
                }
            }
            ListMode::RegisterMcpStepCommand => {
                state.pending_mcp_command =
                    std::mem::take(&mut state.prompt_buffer).trim().to_string();
                if state.pending_mcp_command.is_empty() {
                    state.status_line =
                        "Command cannot be empty. Enter a command (e.g. npx, python):".to_string();
                } else {
                    state.list_mode = ListMode::RegisterMcpStepArgs;
                    state.status_line.clear();
                }
            }
            ListMode::RegisterMcpStepArgs => {
                state.pending_mcp_args = std::mem::take(&mut state.prompt_buffer);
                state.list_mode = ListMode::RegisterMcpStepTransport;
                state.prompt_buffer = "stdio".to_string();
                state.status_line.clear();
            }
            ListMode::RegisterMcpStepTransport => {
                let transport = std::mem::take(&mut state.prompt_buffer).trim().to_string();
                state.pending_mcp_transport = if transport.is_empty() {
                    "stdio".to_string()
                } else {
                    transport
                };
                state.list_mode = ListMode::RegisterMcpStepDescription;
                state.status_line.clear();
            }
            ListMode::RegisterMcpStepDescription => {
                state.pending_mcp_description = std::mem::take(&mut state.prompt_buffer);
                state.list_mode = ListMode::ConfirmMcpTest;
                state.status_line.clear();
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

pub fn handle_mcp_register_confirm(
    state: &mut AppState,
    ctx: &EventContext,
) -> Result<ControlFlow> {
    state.list_mode = ListMode::Normal;
    let name = state.pending_mcp_name.clone();
    let command = state.pending_mcp_command.clone();
    let args = state.pending_mcp_args.clone();
    let transport = state.pending_mcp_transport.clone();
    let description = state.pending_mcp_description.clone();

    let tx = ctx.tx.clone();
    let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    tokio::task::spawn_blocking(move || {
        let _ = tx.send(AppEvent::TaskStarted {
            id,
            name: format!("Registering MCP server '{}'", name),
        });
        match crate::infra::mcp::register(
            &name,
            &command,
            if args.is_empty() { None } else { Some(&args) },
            None,
            &transport,
            if description.is_empty() {
                None
            } else {
                Some(&description)
            },
        ) {
            Ok(_) => {
                let _ = tx.send(AppEvent::TaskProgress { id, percent: 50 });
                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(crate::infra::mcp::test_server(&name)) {
                    Ok(()) => {
                        let _ = tx.send(AppEvent::TaskProgress { id, percent: 100 });
                        let _ = tx.send(AppEvent::TriggerReload);
                        let _ = tx.send(AppEvent::TaskCompleted {
                            id,
                            message: format!("MCP server '{}' registered and tested", name),
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::TaskFailed {
                            id,
                            error: format!("MCP test failed: {}", e),
                        });
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(AppEvent::TaskFailed {
                    id,
                    error: format!("Registration failed: {}", e),
                });
            }
        }
    });

    let name = state.pending_mcp_name.clone();
    state.status_line = format!("Registering MCP server '{}'...", name);
    Ok(ControlFlow::Continue)
}
