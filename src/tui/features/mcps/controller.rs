use crate::app::command::CoreCommand;
use crate::tui::app::AppState;
use crate::tui::event::{AppEvent, ControlFlow, EventContext};
use crate::tui::list_mode::ListMode;
use anyhow::Result;
use crossterm::event::KeyCode;

pub fn handle_space_mcp(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    let items = state.mcp_state.servers_list();
    let Some((id, _server)) = items.get(state.selected_index).copied() else {
        return Ok(());
    };
    let name = id.clone();

    let _ = ctx
        .tx
        .send(AppEvent::ExecuteCommand(CoreCommand::ToggleMcp {
            name,
            scope: state.active_scope,
        }));

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

    let input = crate::app::features::mcp::command::RegisterMcpInput {
        name: state.pending_mcp_name.clone(),
        command: state.pending_mcp_command.clone(),
        args: state
            .pending_mcp_args
            .split_whitespace()
            .map(|s| s.to_string())
            .collect(),
        env: vec![],
        transport: match state.pending_mcp_transport.as_str() {
            "sse" => crate::domain::mcp::McpTransport::Sse {
                url: "http://localhost:3000".to_string(),
            },
            _ => crate::domain::mcp::McpTransport::Stdio,
        },
        description: if state.pending_mcp_description.is_empty() {
            None
        } else {
            Some(state.pending_mcp_description.clone())
        },
        test_after: true,
    };

    let _ = ctx
        .tx
        .send(AppEvent::ExecuteCommand(CoreCommand::RegisterMcp { input }));

    let name = state.pending_mcp_name.clone();
    state.status_line = format!("Registering MCP server '{}'...", name);
    Ok(ControlFlow::Continue)
}
