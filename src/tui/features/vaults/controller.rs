use crate::tui::app::{AppState, ListMode};
use crate::tui::event::{AppEvent, ControlFlow, EventContext};
use crate::tui::features::common::actions::parse_github_url;
use anyhow::Result;
use crossterm::event::KeyCode;

pub fn dispatch_clawhub_search(state: &mut AppState, ctx: &EventContext) {
    let query = state.search_query.clone();
    let tx = ctx.tx.clone();
    let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    state.clawhub_search_task_id = Some(id);
    let _ = ctx.tx.send(AppEvent::TaskStarted {
        id,
        name: format!("Searching ClawHub '{}'", query),
    });
    tokio::task::spawn_blocking(move || {
        let packages = crate::infra::vault::clawhub::cli_search(&query).unwrap_or_default();
        let _ = tx.send(AppEvent::ClawHubSearchResults {
            packages,
            task_id: id,
        });
    });
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
                    state.status_line = "Cancelled \u{2014} empty path".to_string();
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
        let store = ctx.store.clone();
        let tx = ctx.tx.clone();
        let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::task::spawn_blocking(move || {
            let _ = tx.send(AppEvent::TaskStarted {
                id,
                name: format!("Detaching vault '{}'", vault_id),
            });
            match crate::app::actions::detach_vault(&vault_id, store.as_ref()) {
                Ok(()) => {
                    let _ = tx.send(AppEvent::TaskProgress { id, percent: 100 });
                    let _ = tx.send(AppEvent::TriggerReload);
                    let _ = tx.send(AppEvent::TaskCompleted {
                        id,
                        message: format!("Detached vault '{}'", vault_id),
                    });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::TaskFailed {
                        id,
                        error: format!("Detach failed: {}", e),
                    });
                }
            }
        });
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

        let is_attached = if let Ok(config) = ctx.store.load(crate::domain::scope::Scope::Global) {
            config.vaults.contains(&vault_id)
        } else {
            false
        };

        if is_attached {
            state.list_mode = ListMode::ConfirmDetachVault;
            state.pending_detach_vault = Some(vault_id.clone());
            state.status_line.clear();
        } else if vault.kind == "clawhub" {
            if crate::infra::vault::clawhub::is_cli_available() {
                let store = ctx.store.clone();
                let tx = ctx.tx.clone();
                let id =
                    crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                tokio::task::spawn_blocking(move || {
                    let _ = tx.send(AppEvent::TaskStarted {
                        id,
                        name: format!("Attaching vault '{}'", vault_id),
                    });
                    if let Ok(mut config) = store.load(crate::domain::scope::Scope::Global) {
                        config.vaults.push(vault_id.clone());
                        let _ = store.save(crate::domain::scope::Scope::Global, &config);
                    }
                    let _ = tx.send(AppEvent::TaskProgress { id, percent: 100 });
                    let _ = tx.send(AppEvent::TriggerReload);
                    let _ = tx.send(AppEvent::TaskCompleted {
                        id,
                        message: format!("Attached vault '{}'", vault_id),
                    });
                });
            } else if crate::infra::vault::clawhub::is_homebrew_available() {
                state.list_mode = ListMode::ConfirmClawHubInstall;
                state.status_line.clear();
            } else {
                state.status_line =
                    "ClawHub CLI not found. Install manually from https://clawhub.ai".to_string();
            }
        } else {
            let store = ctx.store.clone();
            let tx = ctx.tx.clone();
            let id =
                crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            tokio::task::spawn_blocking(move || {
                let _ = tx.send(AppEvent::TaskStarted {
                    id,
                    name: format!("Attaching vault '{}'", vault_id),
                });
                if let Ok(mut config) = store.load(crate::domain::scope::Scope::Global) {
                    config.vaults.push(vault_id.clone());
                    let _ = store.save(crate::domain::scope::Scope::Global, &config);
                }
                let _ = tx.send(AppEvent::TaskProgress { id, percent: 100 });
                let _ = tx.send(AppEvent::TriggerReload);
                let _ = tx.send(AppEvent::TaskCompleted {
                    id,
                    message: format!("Attached vault '{}'", vault_id),
                });
            });
        }
    }
    Ok(())
}

pub fn execute_attach_vault(
    ctx: &EventContext,
    vault_id: String,
    vault_config: crate::domain::config::VaultConfig,
) {
    let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let _ = ctx.tx.send(AppEvent::TaskStarted {
        id,
        name: format!("Attaching vault '{}'", vault_id),
    });

    let store = ctx.store.clone();
    let tx = ctx.tx.clone();

    tokio::task::spawn_blocking(move || {
        let vault_config_clone = vault_config.clone();
        match crate::app::actions::attach_vault(vault_id.clone(), vault_config, store.as_ref()) {
            Ok(()) => {
                let _ = tx.send(AppEvent::TaskProgress { id, percent: 100 });
                let _ = tx.send(AppEvent::TriggerReload);
                let _ = tx.send(AppEvent::TaskCompleted {
                    id,
                    message: format!("Attached vault '{}'", vault_id),
                });
                let _ = tx.send(AppEvent::VaultRefreshRequired {
                    id: vault_id,
                    config: vault_config_clone,
                });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::TaskFailed {
                    id,
                    error: format!("Failed to attach: {}", e),
                });
            }
        }
    });
}

pub fn handle_clawhub_install_confirm(
    state: &mut AppState,
    ctx: &EventContext,
) -> Result<ControlFlow> {
    state.list_mode = ListMode::Normal;
    state.status_line = "Installing ClawHub CLI via Homebrew...".to_string();

    let store = ctx.store.clone();
    let tx = ctx.tx.clone();
    let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    tokio::task::spawn_blocking(move || {
        let _ = tx.send(AppEvent::TaskStarted {
            id,
            name: "Installing ClawHub CLI via Homebrew".into(),
        });
        match crate::infra::vault::clawhub::install_cli_via_homebrew() {
            Ok(()) => {
                if let Ok(mut config) = store.load(crate::domain::scope::Scope::Global) {
                    config.vaults.push("clawhub".to_string());
                    let _ = store.save(crate::domain::scope::Scope::Global, &config);
                }
                let _ = tx.send(AppEvent::TaskProgress { id, percent: 100 });
                let _ = tx.send(AppEvent::TriggerReload);
                let _ = tx.send(AppEvent::TaskCompleted {
                    id,
                    message: "Installed ClawHub CLI and activated vault".into(),
                });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::TaskFailed {
                    id,
                    error: format!("Failed to install ClawHub CLI: {}", e),
                });
            }
        }
    });

    Ok(ControlFlow::Continue)
}
