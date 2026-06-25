use crate::app::command::CoreCommand;
use crate::tui::app::AppState;
use crate::tui::event::{AppEvent, ControlFlow, EventContext};
use crate::tui::features::common::actions::parse_github_url;
use crate::tui::list_mode::ListMode;
use anyhow::Result;
use crossterm::event::KeyCode;

pub fn dispatch_clawhub_search(state: &mut AppState, ctx: &EventContext) {
    let query = state.search_query.clone();
    let _ = ctx
        .tx
        .send(AppEvent::ExecuteCommand(CoreCommand::SearchRemoteVault {
            vault_id: "clawhub".to_string(),
            query,
        }));
}

pub fn handle_attach_vault_input(
    state: &mut AppState,
    ctx: &EventContext,
    code: &KeyCode,
) -> Result<()> {
    match code {
        KeyCode::Char(c) => {
            state.prompt_buffer.push(*c);
        }
        KeyCode::Backspace if state.is_attach_vault_mode() || state.is_register_mcp_mode() => {
            state.prompt_buffer.pop();
        }
        KeyCode::Enter => match state.list_mode {
            ListMode::AttachVault => {
                let input = std::mem::take(&mut state.prompt_buffer);
                if input.is_empty() {
                    state.list_mode = ListMode::Normal;
                    state.status_line = "Cancelled — empty path".to_string();
                } else if let Some((id, repo)) = parse_github_url(&input) {
                    state.pending_vault_id = id;
                    state.pending_vault_repo = repo;
                    state.pending_vault_url = input;
                    state.list_mode = ListMode::AttachVaultBranch;
                    state.prompt_buffer = "main".to_string();
                } else {
                    state.pending_vault_local_path = input.clone();
                    let id = std::path::Path::new(&input)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    state.pending_vault_id = id;
                    state.list_mode = ListMode::AttachVaultName;
                    state.prompt_buffer.clone_from(&state.pending_vault_id);
                }
            }
            ListMode::AttachVaultBranch => {
                let branch = std::mem::take(&mut state.prompt_buffer);
                state.pending_vault_ref = if branch.trim().is_empty() {
                    "main".to_string()
                } else {
                    branch
                };
                state.list_mode = ListMode::AttachVaultPath;
                state.prompt_buffer = "skills/".to_string();
            }
            ListMode::AttachVaultPath => {
                let subfolder = std::mem::take(&mut state.prompt_buffer);
                state.pending_vault_path = if subfolder.trim().is_empty() {
                    "skills/".to_string()
                } else {
                    subfolder
                };
                state.list_mode = ListMode::AttachVaultName;
                state.prompt_buffer.clone_from(&state.pending_vault_id);
            }
            ListMode::AttachVaultName => {
                let name = std::mem::take(&mut state.prompt_buffer).trim().to_string();
                if name.is_empty() {
                    state.list_mode = ListMode::Normal;
                    state.status_line = "Cancelled — empty vault name".to_string();
                    state.pending_vault_local_path.clear();
                } else {
                    state.pending_vault_id = name.clone();
                    state.list_mode = ListMode::Normal;
                    if !state.pending_vault_local_path.is_empty() {
                        let vault_config = crate::domain::config::VaultConfig::Local(
                            crate::domain::config::LocalVaultSource {
                                path: state.pending_vault_local_path.clone(),
                            },
                        );
                        execute_attach_vault(ctx, name, vault_config);
                        state.pending_vault_local_path.clear();
                    } else {
                        let vault_config = crate::domain::config::VaultConfig::Github(
                            crate::domain::config::GithubVaultSource {
                                repo: state.pending_vault_repo.clone(),
                                r#ref: state.pending_vault_ref.clone(),
                                path: state.pending_vault_path.clone(),
                                enterprise_url: None,
                            },
                        );
                        execute_attach_vault(ctx, name, vault_config);
                    }
                }
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

pub fn handle_detach_confirm(state: &mut AppState, ctx: &EventContext) -> Result<ControlFlow> {
    if let Some(vault_id) = state.pending_detach_vault.take() {
        let _ = ctx
            .tx
            .send(AppEvent::ExecuteCommand(CoreCommand::DetachVault {
                vault_id,
                scope: crate::domain::scope::Scope::Global,
            }));
    }
    state.list_mode = ListMode::Normal;
    state.status_line.clear();
    Ok(ControlFlow::Continue)
}

pub fn handle_detach_cancel(state: &mut AppState) -> Result<ControlFlow> {
    state.list_mode = ListMode::Normal;
    state.status_line = "Cancelled detach".to_string();
    state.pending_detach_vault = None;
    Ok(ControlFlow::Continue)
}

pub fn handle_space_vault(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    if let Some(vault) = state.vault_entries.get(state.selected_index) {
        let vault_id = vault.id.clone();

        let is_attached =
            if let Ok(config) = ctx.core.store.load(crate::domain::scope::Scope::Global) {
                config.vaults.contains(&vault_id)
            } else {
                false
            };

        if is_attached {
            state.list_mode = ListMode::ConfirmDetachVault;
            state.pending_detach_vault = Some(vault_id.clone());
            state.status_line.clear();
        } else if vault.kind == "clawhub" {
            // ClawHub attachment is handled by AgkCore, which auto-installs the CLI if needed.
            let vault_config = crate::domain::config::VaultConfig::Clawhub(
                crate::domain::config::ClawHubVaultSource {},
            );
            execute_attach_vault(ctx, vault_id, vault_config);
        } else {
            let _ = ctx
                .tx
                .send(AppEvent::ExecuteCommand(CoreCommand::AttachBareVault {
                    vault_id,
                    scope: crate::domain::scope::Scope::Global,
                }));
        }
    }
    Ok(())
}

pub fn execute_attach_vault(
    ctx: &EventContext,
    vault_id: String,
    vault_config: crate::domain::config::VaultConfig,
) {
    let input = crate::app::features::vault::command::AttachVaultInput {
        vault_id,
        config: vault_config,
        scope: crate::domain::scope::Scope::Global,
    };
    let _ = ctx
        .tx
        .send(AppEvent::ExecuteCommand(CoreCommand::AttachVault { input }));
}

pub fn handle_clawhub_install_confirm(
    state: &mut AppState,
    _ctx: &EventContext,
) -> Result<ControlFlow> {
    // The ClawHub auto-install is now handled inside AgkCore::AttachVault.
    // This confirm handler simply cancels the modal since the user already
    // triggered attachment via the vault list.
    state.list_mode = ListMode::Normal;
    state.status_line = "ClawHub attachment handled by AgkCore.".to_string();
    Ok(ControlFlow::Continue)
}

/// Enter the vault-init confirmation modal.
/// Stores the workspace folder name in `pending_vault_local_path` for the modal to display.
pub fn enter_vault_init(state: &mut AppState, ctx: &EventContext) {
    let name = ctx
        .workspace_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "vault".to_string());
    state.pending_vault_local_path = name;
    state.list_mode = ListMode::ConfirmVaultInit;
    state.status_line.clear();
}

pub fn handle_vault_init_confirm(state: &mut AppState, ctx: &EventContext) -> Result<ControlFlow> {
    let _ = ctx.tx.send(AppEvent::ExecuteCommand(
        crate::app::command::CoreCommand::VaultInit {
            name: None,
            dry_run: false,
        },
    ));
    state.list_mode = ListMode::Normal;
    state.pending_vault_local_path.clear();
    Ok(ControlFlow::Continue)
}

pub fn handle_vault_init_cancel(state: &mut AppState) -> Result<ControlFlow> {
    state.list_mode = ListMode::Normal;
    state.pending_vault_local_path.clear();
    state.status_line = "Cancelled vault init".to_string();
    Ok(ControlFlow::Continue)
}
