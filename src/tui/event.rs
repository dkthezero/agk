use crate::tui::app::{AppState, ListMode};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};

pub enum ControlFlow {
    Continue,
    Quit,
}

use std::sync::Arc;

pub enum AppEvent {
    Input(crossterm::event::Event),
    TaskStarted {
        id: usize,
        name: String,
    },
    TaskProgress {
        id: usize,
        percent: u8,
    },
    TaskCompleted {
        id: usize,
        message: String,
    },
    TaskFailed {
        id: usize,
        error: String,
    },
    TriggerReload,
    VaultRefreshRequired {
        id: String,
        config: crate::domain::config::VaultConfig,
    },
    ClawHubSearchResults {
        packages: Vec<crate::domain::asset::ScannedPackage>,
        task_id: usize,
    },
    Tick,
}

pub struct EventContext {
    pub store: Arc<dyn crate::app::ports::ConfigStorePort>,
    pub registry: Arc<crate::app::registry::Registry>,
    pub tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
    pub workspace_root: std::path::PathBuf,
}

fn is_clawhub_active(ctx: &EventContext) -> bool {
    ctx.store
        .load(crate::domain::scope::Scope::Global)
        .map(|c| c.vaults.contains(&"clawhub".to_string()))
        .unwrap_or(false)
}

fn dispatch_clawhub_search(state: &mut AppState, ctx: &EventContext) {
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

pub fn handle(
    state: &mut AppState,
    ctx: &EventContext,
    evt: crossterm::event::Event,
) -> Result<ControlFlow> {
    if let crossterm::event::Event::Key(key) = evt {
        // Ctrl+C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Ok(ControlFlow::Quit);
        }

        if key.code != KeyCode::Esc {
            state.esc_pressed_once = false;
        }

        // Short-circuit when in SelectProviderRoot modal
        if matches!(state.list_mode, ListMode::SelectProviderRoot { .. }) {
            return handle_select_provider_root(state, ctx, &key.code);
        }

        // Confirmation modals share Enter alias
        let in_confirm = matches!(
            state.list_mode,
            ListMode::ConfirmMcpTest
                | ListMode::ConfirmClawHubInstall
                | ListMode::ConfirmDetachVault
                | ListMode::ConfirmDeactivateLastProvider
                | ListMode::ConfirmDeleteProfile
        );

        match &key.code {
            KeyCode::Enter if in_confirm => match state.list_mode {
                ListMode::ConfirmMcpTest => return handle_mcp_register_confirm(state, ctx),
                ListMode::ConfirmClawHubInstall => {
                    return handle_clawhub_install_confirm(state, ctx)
                }
                ListMode::ConfirmDetachVault => return handle_detach_confirm(state, ctx),
                ListMode::ConfirmDeactivateLastProvider => {
                    return handle_deactivate_last_provider_confirm(state, ctx)
                }
                ListMode::ConfirmDeleteProfile => return handle_delete_profile_confirm(state, ctx),
                _ => {}
            },
            KeyCode::Esc if state.list_mode == ListMode::ConfirmMcpTest => {
                state.list_mode = ListMode::Normal;
                state.status_line = "Cancelled MCP registration".to_string();
                return Ok(ControlFlow::Continue);
            }
            KeyCode::Esc if state.list_mode == ListMode::ConfirmClawHubInstall => {
                state.list_mode = ListMode::Normal;
                state.status_line = "Cancelled ClawHub CLI install".to_string();
                return Ok(ControlFlow::Continue);
            }
            KeyCode::Esc if state.list_mode == ListMode::ConfirmDetachVault => {
                return handle_detach_cancel(state);
            }
            KeyCode::Esc if state.list_mode == ListMode::ConfirmDeactivateLastProvider => {
                return handle_deactivate_last_provider_cancel(state);
            }
            KeyCode::Esc if state.list_mode == ListMode::ConfirmDeleteProfile => {
                state.list_mode = ListMode::Normal;
                state.pending_delete_profile = None;
                state.status_line = "Cancelled profile deletion".to_string();
                return Ok(ControlFlow::Continue);
            }
            KeyCode::Char('0') if state.list_mode == ListMode::Normal => {
                // [0] is always the right-aligned Vault tab (last index)
                let vault_idx = state.tab_names.len().saturating_sub(1);
                apply_tab_switch(state, vault_idx, state.tab_names.len());
            }
            KeyCode::Char(c @ '1'..='5') if state.list_mode == ListMode::Normal => {
                let idx = (*c as usize) - ('1' as usize);
                apply_tab_switch(state, idx, state.tab_names.len());
            }
            KeyCode::Up | KeyCode::Down => {
                handle_navigation(state, &key.code);
            }
            KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Enter
                if state.is_attach_vault_mode()
                    || state.is_register_mcp_mode()
                    || state.is_profile_wizard_mode() =>
            {
                if state.is_attach_vault_mode() {
                    handle_attach_vault_input(state, ctx, &key.code)?;
                } else if state.is_register_mcp_mode() {
                    handle_register_mcp_input(state, ctx, &key.code)?;
                } else {
                    handle_profile_wizard_input(state, ctx, &key.code)?;
                }
            }
            KeyCode::Esc => {
                return handle_esc(state);
            }
            KeyCode::Backspace => {
                if state.is_attach_vault_mode()
                    || state.is_register_mcp_mode()
                    || state.is_profile_wizard_mode()
                {
                    state.prompt_buffer.pop();
                } else {
                    handle_backspace(state);
                }
            }
            KeyCode::Char(' ')
                if state.list_mode == ListMode::Normal
                    || state.list_mode == ListMode::Searching =>
            {
                handle_space(state, ctx)?;
            }
            KeyCode::Enter if state.list_mode == ListMode::Normal => {
                handle_enter(state, ctx)?;
            }
            KeyCode::F(5) | KeyCode::F(4) | KeyCode::F(2)
                if state.list_mode == ListMode::Normal =>
            {
                handle_f_keys(state, ctx, &key.code)?;
            }
            KeyCode::Tab if state.list_mode == ListMode::Normal => {
                apply_scope_toggle(state);
                let _ = ctx.tx.send(AppEvent::TriggerReload);
            }
            KeyCode::Delete if state.list_mode == ListMode::Normal => {
                let active_kind = state.tab_kinds.get(state.active_tab).copied();
                if active_kind == Some(crate::tui::app::TabKind::Profile) {
                    handle_delete_profile(state, ctx)?;
                }
            }
            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                handle_open_location(state, ctx, false)?;
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                handle_open_location(state, ctx, true)?;
            }
            KeyCode::Char(c) => {
                if state.is_profile_wizard_mode() {
                    state.list_mode = ListMode::Normal;
                    state.prompt_buffer.clear();
                    state.status_line = "Cancelled profile creation".to_string();
                    return Ok(ControlFlow::Continue);
                }

                let active_kind = state.tab_kinds.get(state.active_tab).copied();
                if active_kind != Some(crate::tui::app::TabKind::Vault) {
                    apply_search_char(state, *c);
                    if active_kind == Some(crate::tui::app::TabKind::Asset)
                        && is_clawhub_active(ctx)
                        && !state.search_query.is_empty()
                    {
                        dispatch_clawhub_search(state, ctx);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(ControlFlow::Continue)
}

fn handle_clawhub_install_confirm(state: &mut AppState, ctx: &EventContext) -> Result<ControlFlow> {
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
                // Now activate the vault
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

fn handle_detach_confirm(state: &mut AppState, ctx: &EventContext) -> Result<ControlFlow> {
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

fn handle_detach_cancel(state: &mut AppState) -> Result<ControlFlow> {
    state.list_mode = ListMode::Normal;
    state.status_line = "Cancelled detach".to_string();
    state.pending_detach_vault = None;
    Ok(ControlFlow::Continue)
}

fn handle_deactivate_last_provider_confirm(
    state: &mut AppState,
    ctx: &EventContext,
) -> Result<ControlFlow> {
    let provider_id = std::mem::take(&mut state.pending_deactivate_provider_id);
    state.list_mode = ListMode::Normal;

    let scope = state.active_scope;
    let store = ctx.store.clone();
    let tx = ctx.tx.clone();
    let registry = ctx.registry.clone();

    let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    tokio::task::spawn_blocking(move || {
        let _ = tx.send(AppEvent::TaskStarted {
            id,
            name: format!("Deactivating '{}'", provider_id),
        });

        let mut config = store.load(scope).unwrap_or_default();
        if !config.providers.contains(&provider_id) {
            let _ = tx.send(AppEvent::TaskFailed {
                id,
                error: "Provider already deactivated".into(),
            });
            return;
        }

        if let Ok(provider) = registry.get_provider(&provider_id) {
            config.providers.retain(|p| p != &provider_id);

            // Remove provider root so re-activation shows the config-folder popup again
            config.provider_roots.remove(&provider_id);

            // Remove installed assets from the provider's filesystem
            for section in config.vault_defs.values() {
                if let Some(ref skills) = section.skills {
                    for item in &skills.items {
                        if let Some(identity) = crate::domain::config::parse_identity(item) {
                            let _ = provider.remove(
                                &identity,
                                &crate::domain::asset::AssetKind::Skill,
                                scope,
                                Some(&config),
                            );
                        }
                    }
                }
                if let Some(ref instructions) = section.instructions {
                    for item in &instructions.items {
                        if let Some(identity) = crate::domain::config::parse_identity(item) {
                            let _ = provider.remove(
                                &identity,
                                &crate::domain::asset::AssetKind::Instruction,
                                scope,
                                Some(&config),
                            );
                        }
                    }
                }
            }

            // Clear all installed assets
            for section in config.vault_defs.values_mut() {
                if let Some(ref mut b) = section.skills {
                    b.items.clear();
                }
                if let Some(ref mut b) = section.instructions {
                    b.items.clear();
                }
            }
            crate::app::actions::prune_empty_vault_defs(&mut config);

            // If the entire config is now default (empty), delete the file
            if config == crate::domain::config::ConfigFile::default() {
                if let Err(e) = store.delete_file(scope) {
                    let _ = tx.send(AppEvent::TaskFailed {
                        id,
                        error: format!("Failed to delete empty config file: {}", e),
                    });
                    return;
                }
            } else {
                let _ = store.save(scope, &config);
            }
        }

        let _ = tx.send(AppEvent::TriggerReload);
        let _ = tx.send(AppEvent::TaskCompleted {
            id,
            message: format!("Deactivated '{}'", provider_id),
        });
    });

    Ok(ControlFlow::Continue)
}

fn handle_deactivate_last_provider_cancel(state: &mut AppState) -> Result<ControlFlow> {
    state.list_mode = ListMode::Normal;
    state.status_line = "Cancelled provider deactivation".to_string();
    state.pending_deactivate_provider_id.clear();
    Ok(ControlFlow::Continue)
}

fn handle_select_provider_root(
    state: &mut AppState,
    ctx: &EventContext,
    code: &KeyCode,
) -> Result<ControlFlow> {
    match code {
        KeyCode::Up => {
            if let ListMode::SelectProviderRoot { selected, .. } = &mut state.list_mode {
                *selected = selected.saturating_sub(1);
            }
            Ok(ControlFlow::Continue)
        }
        KeyCode::Down => {
            if let ListMode::SelectProviderRoot {
                selected, options, ..
            } = &mut state.list_mode
            {
                if *selected + 1 < options.len() {
                    *selected += 1;
                }
            }
            Ok(ControlFlow::Continue)
        }
        KeyCode::Enter => {
            if let ListMode::SelectProviderRoot {
                provider_id,
                options,
                selected,
            } = &state.list_mode
            {
                let chosen = options[*selected].0.clone();
                let mut config = state.active_config().clone();
                config.provider_roots.insert(provider_id.clone(), chosen);
                let scope = state.active_scope;
                match ctx.store.save(scope, &config) {
                    Ok(()) => {
                        state.configs.insert(scope, config);
                        state.list_mode = ListMode::Normal;
                        return toggle_provider(state, ctx);
                    }
                    Err(e) => {
                        state.status_line = format!("Failed to save config: {}", e);
                        return Ok(ControlFlow::Continue);
                    }
                }
            }
            Ok(ControlFlow::Continue)
        }
        KeyCode::Esc => {
            state.list_mode = ListMode::Normal;
            state.status_line = "Provider enable cancelled".to_string();
            Ok(ControlFlow::Continue)
        }
        _ => Ok(ControlFlow::Continue),
    }
}

fn handle_navigation(state: &mut AppState, code: &KeyCode) {
    match code {
        KeyCode::Up => {
            if state.selected_index > 0 {
                state.selected_index -= 1;
                state.scroll_offset = 0;
                state.scroll_tick = 0;
            }
        }
        KeyCode::Down => {
            let count = state.list_length();
            if state.selected_index + 1 < count {
                state.selected_index += 1;
                state.scroll_offset = 0;
                state.scroll_tick = 0;
            }
        }
        _ => {}
    }
}

fn handle_attach_vault_input(
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

// ---------------------------------------------------------------------------
// MCP registration modal input handler
// ---------------------------------------------------------------------------

fn handle_register_mcp_input(
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
                // Security confirmation before registering
                state.list_mode = ListMode::ConfirmMcpTest;
                state.status_line.clear();
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

fn handle_profile_wizard_input(
    state: &mut AppState,
    ctx: &EventContext,
    code: &KeyCode,
) -> Result<()> {
    match code {
        KeyCode::Char(c) => {
            state.prompt_buffer.push(*c);
        }
        KeyCode::Backspace => {
            state.prompt_buffer.pop();
        }
        KeyCode::Enter => {
            let name = std::mem::take(&mut state.prompt_buffer).trim().to_string();
            if name.is_empty() {
                state.list_mode = ListMode::Normal;
                state.status_line = "Cancelled — name required".to_string();
                return Ok(());
            }

            // Check if profile already exists
            let config = state.active_config().clone();
            if config.find_profile(&name).is_some() {
                state.list_mode = ListMode::Normal;
                state.status_line = format!("Profile '{}' already exists", name);
                return Ok(());
            }

            // Create new profile with defaults (opencode provider)
            let mut new_config = config.clone();
            new_config.profiles.push(crate::domain::config::Profile {
                name: name.clone(),
                provider_id: "opencode".to_string(),
                skills: vec![],
                mcps: vec![],
            });

            let scope = state.active_scope;
            let store = ctx.store.clone();
            let tx = ctx.tx.clone();
            let id =
                crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            tokio::task::spawn_blocking(move || {
                let _ = tx.send(AppEvent::TaskStarted {
                    id,
                    name: format!("Creating profile '{}'", name),
                });
                match store.save(scope, &new_config) {
                    Ok(()) => {
                        let _ = tx.send(AppEvent::TriggerReload);
                        let _ = tx.send(AppEvent::TaskCompleted {
                            id,
                            message: format!(
                                "Profile '{}' created. Run `opencode agent create` to generate agent file.",
                                name
                            ),
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::TaskFailed {
                            id,
                            error: format!("Failed to save profile: {}", e),
                        });
                    }
                }
            });

            state.list_mode = ListMode::Normal;
            state.status_line.clear();
        }
        _ => {}
    }
    Ok(())
}

fn handle_mcp_register_confirm(state: &mut AppState, ctx: &EventContext) -> Result<ControlFlow> {
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
                // Run test
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

fn handle_esc(state: &mut AppState) -> Result<ControlFlow> {
    if state.is_attach_vault_mode() {
        state.list_mode = ListMode::Normal;
        state.prompt_buffer.clear();
        state.status_line = "Cancelled".to_string();
        return Ok(ControlFlow::Continue);
    }

    if state.is_register_mcp_mode() || state.list_mode == ListMode::ConfirmMcpTest {
        state.list_mode = ListMode::Normal;
        state.prompt_buffer.clear();
        state.status_line = "Cancelled MCP registration".to_string();
        return Ok(ControlFlow::Continue);
    }

    let active_kind = state.tab_kinds.get(state.active_tab).copied();
    if state.list_mode == ListMode::Normal && state.search_query.is_empty() {
        if state.esc_pressed_once {
            return Ok(ControlFlow::Quit);
        } else {
            state.esc_pressed_once = true;
            state.status_line = "Press ESC again to quit".to_string();
        }
    } else if active_kind != Some(crate::tui::app::TabKind::Vault) {
        apply_esc(state);
    }
    Ok(ControlFlow::Continue)
}

fn handle_backspace(state: &mut AppState) {
    let active_kind = state.tab_kinds.get(state.active_tab).copied();
    if state.is_attach_vault_mode() || state.is_register_mcp_mode() {
        state.prompt_buffer.pop();
    } else if active_kind != Some(crate::tui::app::TabKind::Vault) {
        state.search_query.pop();
        if state.search_query.is_empty() {
            state.list_mode = ListMode::Normal;
            state.remote_packages.clear();
            if let Some(id) = state.clawhub_search_task_id.take() {
                state.active_tasks.remove(&id);
            }
        }
        state.selected_index = 0;
    }
}

fn handle_space(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    let active_kind = state.tab_kinds.get(state.active_tab).cloned();
    match active_kind {
        Some(crate::tui::app::TabKind::Provider) => handle_space_provider(state, ctx),
        Some(crate::tui::app::TabKind::Vault) => handle_space_vault(state, ctx),
        Some(crate::tui::app::TabKind::Mcp) => handle_space_mcp(state, ctx),
        Some(crate::tui::app::TabKind::Asset) => {
            if !state.active_scope_has_provider() {
                let providers_idx = state
                    .tab_names
                    .iter()
                    .position(|n| n == "Providers")
                    .unwrap_or(3);
                apply_space_no_provider(state, providers_idx);
                Ok(())
            } else {
                handle_space_asset(state, ctx)
            }
        }
        _ => Ok(()),
    }
}

fn handle_space_provider(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    if let Some(entry) = state.provider_entries.get(state.selected_index) {
        let provider = ctx.registry.providers.iter().find(|p| p.id() == entry.id);
        if let Some(p) = provider {
            if !entry.active && state.active_scope == crate::domain::scope::Scope::Workspace {
                let roots = p.available_config_roots();
                let already_selected = state.active_config().provider_roots.contains_key(&entry.id);
                if roots.len() > 1 && !already_selected {
                    state.list_mode = ListMode::SelectProviderRoot {
                        provider_id: entry.id.clone(),
                        options: roots,
                        selected: 0,
                    };
                    return Ok(());
                }
            }
        }
    }
    toggle_provider(state, ctx).map(|_| ())
}

fn toggle_provider(state: &mut AppState, ctx: &EventContext) -> Result<ControlFlow> {
    if let Some(p) = state.provider_entries.get(state.selected_index) {
        let provider_id = p.id.clone();
        let scope = state.active_scope;
        let store = ctx.store.clone();
        let tx = ctx.tx.clone();
        let registry = ctx.registry.clone();

        let mut installed_pkgs = Vec::new();
        for tab_pkgs in state.packages.values() {
            for pkg in tab_pkgs {
                if state.is_installed(&pkg.vault_id, &pkg.identity.name, &pkg.kind) {
                    installed_pkgs.push(pkg.clone());
                }
            }
        }

        // Check if deactivating the last provider with installed assets
        let config = store.load(scope).unwrap_or_default();
        let is_last_provider =
            config.providers.len() == 1 && config.providers.contains(&provider_id);
        let has_installed_assets = config.vault_defs.values().any(|section| {
            section
                .skills
                .as_ref()
                .map(|b| !b.items.is_empty())
                .unwrap_or(false)
                || section
                    .instructions
                    .as_ref()
                    .map(|b| !b.items.is_empty())
                    .unwrap_or(false)
        });

        if is_last_provider && has_installed_assets {
            state.list_mode = ListMode::ConfirmDeactivateLastProvider;
            state.pending_deactivate_provider_id = provider_id;
            state.status_line.clear();
            return Ok(ControlFlow::Continue);
        }

        let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::task::spawn_blocking(move || {
            let mut config = store.load(scope).unwrap_or_default();
            if config.providers.contains(&provider_id) {
                let _ = tx.send(AppEvent::TaskStarted {
                    id,
                    name: "Deactivating Provider".into(),
                });
                let total = installed_pkgs.len();
                if let Ok(provider) = registry.get_provider(&provider_id) {
                    config.providers.retain(|p| p != &provider_id);

                    // Also remove the provider root selection so re-activation shows the popup again
                    config.provider_roots.remove(&provider_id);

                    for (i, pkg) in installed_pkgs.iter().enumerate() {
                        let _ = provider.remove(&pkg.identity, &pkg.kind, scope, Some(&config));
                        let percent = (((i + 1) as f32 / total.max(1) as f32) * 100.0) as u8;
                        let _ = tx.send(AppEvent::TaskProgress { id, percent });
                    }

                    // If no providers remain, clear all installed assets from config
                    if config.providers.is_empty() {
                        for section in config.vault_defs.values_mut() {
                            if let Some(ref mut b) = section.skills {
                                b.items.clear();
                            }
                            if let Some(ref mut b) = section.instructions {
                                b.items.clear();
                            }
                        }
                        crate::app::actions::prune_empty_vault_defs(&mut config);

                        if config == crate::domain::config::ConfigFile::default() {
                            if let Err(e) = store.delete_file(scope) {
                                let _ = tx.send(AppEvent::TaskFailed {
                                    id,
                                    error: format!("Failed to delete empty config file: {}", e),
                                });
                                return;
                            }
                        } else {
                            let _ = store.save(scope, &config);
                        }
                    } else {
                        let _ = store.save(scope, &config);
                    }
                }
                let _ = tx.send(AppEvent::TriggerReload);
                let _ = tx.send(AppEvent::TaskCompleted {
                    id,
                    message: format!("Deactivated '{}'", provider_id),
                });
            } else {
                let _ = tx.send(AppEvent::TaskStarted {
                    id,
                    name: "Activating Provider".into(),
                });
                let total = installed_pkgs.len();
                if let Ok(provider) = registry.get_provider(&provider_id) {
                    config.providers.push(provider_id.clone());
                    let _ = store.save(scope, &config);

                    for (i, pkg) in installed_pkgs.iter().enumerate() {
                        let _ = crate::app::actions::install_asset(
                            scope,
                            pkg,
                            store.as_ref(),
                            provider,
                        );
                        let percent = (((i + 1) as f32 / total.max(1) as f32) * 100.0) as u8;
                        let _ = tx.send(AppEvent::TaskProgress { id, percent });
                    }
                }
                let _ = tx.send(AppEvent::TriggerReload);
                let _ = tx.send(AppEvent::TaskCompleted {
                    id,
                    message: format!("Activated '{}'", provider_id),
                });
            }
        });
    }
    Ok(ControlFlow::Continue)
}

fn handle_space_vault(state: &mut AppState, ctx: &EventContext) -> Result<()> {
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
                // Activate directly
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

fn handle_space_mcp(state: &mut AppState, ctx: &EventContext) -> Result<()> {
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
                message: format!("{} MCP server '{}", done, name),
            });
        }
    });

    Ok(())
}

fn handle_space_asset(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    let pkg_opt = {
        let filtered = state.filtered_packages();
        filtered.get(state.selected_index).copied().cloned()
    };
    if let Some(pkg) = pkg_opt {
        if pkg.is_remote {
            return handle_install_remote_clawhub(state, ctx, &pkg);
        }
        let is_installed = state.is_installed(&pkg.vault_id, &pkg.identity.name, &pkg.kind);
        let store = ctx.store.clone();
        let active_scope = state.active_scope;
        let tx = ctx.tx.clone();
        let registry = ctx.registry.clone();

        let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::task::spawn_blocking(move || {
            let action = if is_installed {
                "Uninstalling"
            } else {
                "Installing"
            };
            let _ = tx.send(AppEvent::TaskStarted {
                id,
                name: format!("{} '{}'", action, pkg.identity.name),
            });

            let config = store.load(active_scope).unwrap_or_default();
            let providers = active_providers(&registry, &config);

            if providers.is_empty() {
                let _ = tx.send(AppEvent::TaskFailed {
                    id,
                    error: "No active providers to install to".into(),
                });
                return;
            }

            let mut success = true;
            for provider in providers {
                if is_installed {
                    if crate::app::actions::remove_asset(
                        active_scope,
                        &pkg.identity,
                        &pkg.kind,
                        &pkg.vault_id,
                        store.as_ref(),
                        provider,
                    )
                    .is_err()
                    {
                        success = false;
                    }
                } else if crate::app::actions::install_asset(
                    active_scope,
                    &pkg,
                    store.as_ref(),
                    provider,
                )
                .is_err()
                {
                    success = false;
                }
            }
            let _ = tx.send(AppEvent::TaskProgress { id, percent: 100 });
            let _ = tx.send(AppEvent::TriggerReload);
            if success {
                let done = if is_installed {
                    "Uninstalled"
                } else {
                    "Installed"
                };
                let _ = tx.send(AppEvent::TaskCompleted {
                    id,
                    message: format!("{} '{}'", done, pkg.identity.name),
                });
            } else {
                let _ = tx.send(AppEvent::TaskFailed {
                    id,
                    error: format!(
                        "Failed to {} '{}'",
                        action.to_lowercase(),
                        pkg.identity.name
                    ),
                });
            }
        });
    }
    Ok(())
}

fn handle_install_remote_clawhub(
    state: &mut AppState,
    ctx: &EventContext,
    pkg: &crate::domain::asset::ScannedPackage,
) -> Result<()> {
    let slug = pkg.identity.name.clone();
    let store = ctx.store.clone();
    let tx = ctx.tx.clone();
    let registry = ctx.registry.clone();
    let active_scope = state.active_scope;

    let fetch_id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let install_id =
        crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    let _ = tx.send(AppEvent::TaskStarted {
        id: fetch_id,
        name: format!("Fetching '{}' from ClawHub", slug),
    });
    let _ = tx.send(AppEvent::TaskStarted {
        id: install_id,
        name: format!("Installing '{}' to {:?}", slug, active_scope),
    });

    tokio::task::spawn_blocking(move || {
        match crate::infra::vault::clawhub::cli_install(&slug) {
            Ok(()) => {
                let _ = tx.send(AppEvent::TaskProgress {
                    id: fetch_id,
                    percent: 100,
                });
                let _ = tx.send(AppEvent::TaskCompleted {
                    id: fetch_id,
                    message: format!("Fetched '{}' from ClawHub", slug),
                });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::TaskFailed {
                    id: fetch_id,
                    error: format!("Failed to fetch '{}': {}", slug, e),
                });
                let _ = tx.send(AppEvent::TaskFailed {
                    id: install_id,
                    error: "Cancelled — fetch failed".into(),
                });
                return;
            }
        }

        let cache_dir = crate::domain::paths::clawhub_cache_dir();
        let local = crate::infra::vault::local::LocalVaultAdapter::new("clawhub", cache_dir);
        let feature = crate::infra::feature::skill::SkillFeatureSet;
        use crate::app::ports::VaultPort;
        let cached_pkgs = match local.list_packages(&feature) {
            Ok(pkgs) => pkgs,
            Err(e) => {
                let _ = tx.send(AppEvent::TaskFailed {
                    id: install_id,
                    error: format!("Failed to scan cached package: {}", e),
                });
                return;
            }
        };

        let cached_pkg = cached_pkgs.iter().find(|p| p.identity.name == slug);
        if let Some(pkg) = cached_pkg {
            let config = store.load(active_scope).unwrap_or_default();
            let providers = active_providers(&registry, &config);
            if providers.is_empty() {
                let _ = tx.send(AppEvent::TaskFailed {
                    id: install_id,
                    error: "No active providers to install to".into(),
                });
                return;
            }
            let mut success = true;
            for provider in providers {
                if crate::app::actions::install_asset(active_scope, pkg, store.as_ref(), provider)
                    .is_err()
                {
                    success = false;
                }
            }
            let _ = tx.send(AppEvent::TaskProgress {
                id: install_id,
                percent: 100,
            });
            let _ = tx.send(AppEvent::TriggerReload);
            if success {
                let _ = tx.send(AppEvent::TaskCompleted {
                    id: install_id,
                    message: format!("Installed '{}' to {:?}", slug, active_scope),
                });
            } else {
                let _ = tx.send(AppEvent::TaskFailed {
                    id: install_id,
                    error: format!("Failed to install '{}'", slug),
                });
            }
        } else {
            let _ = tx.send(AppEvent::TaskFailed {
                id: install_id,
                error: format!("Skill '{}' not found in ClawHub cache after fetch", slug),
            });
        }
    });
    Ok(())
}

fn handle_enter(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    let active_kind = state
        .tab_kinds
        .get(state.active_tab)
        .cloned()
        .unwrap_or(crate::tui::app::TabKind::Asset);
    if active_kind != crate::tui::app::TabKind::Asset {
        state.status_line = "Update only applies to Skills/Instructions tabs".to_string();
    } else if !state.active_scope_has_provider() {
        let providers_idx = state
            .tab_names
            .iter()
            .position(|n| n == "Providers")
            .unwrap_or(3);
        apply_space_no_provider(state, providers_idx);
    } else {
        let pkg_clone = {
            let filtered = state.filtered_packages();
            filtered.get(state.selected_index).map(|p| (*p).clone())
        };
        if let Some(pkg) = pkg_clone {
            let is_installed = state.is_installed(&pkg.vault_id, &pkg.identity.name, &pkg.kind);
            if !is_installed {
                state.status_line =
                    "Item not installed \u{2014} use Space to install first".to_string();
            } else {
                let providers = if let Ok(config) = ctx.store.load(state.active_scope) {
                    active_providers(&ctx.registry, &config)
                } else {
                    vec![]
                };

                if providers.is_empty() {
                    state.status_line = "No active providers to update to".to_string();
                } else {
                    let mut success = true;
                    for provider in providers {
                        if let Err(e) = crate::app::actions::update_asset(
                            state.active_scope,
                            &pkg,
                            ctx.store.as_ref(),
                            provider,
                        ) {
                            state.status_line =
                                format!("Update failed for {}: {}", provider.name(), e);
                            success = false;
                            break;
                        }
                    }
                    if success {
                        if let Ok(config) = ctx.store.load(state.active_scope) {
                            state.configs.insert(state.active_scope, config);
                        }
                        state.status_line = format!("Updated '{}'", pkg.identity.name);
                    }
                }
            }
        }
    }
    Ok(())
}

fn handle_f_keys(state: &mut AppState, ctx: &EventContext, code: &KeyCode) -> Result<()> {
    match code {
        KeyCode::F(5) => handle_f5_update_all(state, ctx),
        KeyCode::F(4) => {
            let tx = ctx.tx.clone();
            let registry = ctx.registry.clone();
            tokio::spawn(async move {
                let _ = refresh_all_vaults(registry, tx, "").await;
            });
            Ok(())
        }
        KeyCode::F(2) => {
            let vaults_idx = state
                .tab_names
                .iter()
                .position(|n| n == "Vaults")
                .unwrap_or(0);
            let mcp_idx = state
                .tab_names
                .iter()
                .position(|n| n == "MCP Servers")
                .unwrap_or(1);
            let profile_idx = state
                .tab_names
                .iter()
                .position(|n| n == "Profiles")
                .unwrap_or(4);
            if state.active_tab == vaults_idx {
                apply_enter_attach_vault(state);
            } else if state.active_tab == mcp_idx {
                apply_enter_register_mcp(state);
            } else if state.active_tab == profile_idx {
                apply_enter_add_profile(state);
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn handle_f5_update_all(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    let mut pkgs_to_update = Vec::new();
    for pkg_list in state.packages.values() {
        for pkg in pkg_list {
            if state.is_installed(&pkg.vault_id, &pkg.identity.name, &pkg.kind) {
                pkgs_to_update.push(pkg.clone());
            }
        }
    }

    if pkgs_to_update.is_empty() {
        state.status_line = "No installed items to update".into();
        return Ok(());
    }

    let tx = ctx.tx.clone();
    let store = ctx.store.clone();
    let registry = ctx.registry.clone();
    let scope = state.active_scope;
    let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    tokio::task::spawn_blocking(move || {
        let _ = tx.send(AppEvent::TaskStarted {
            id,
            name: format!("Updating {} items...", pkgs_to_update.len()),
        });

        let providers = if let Ok(config) = store.load(scope) {
            active_providers(&registry, &config)
        } else {
            vec![]
        };

        if providers.is_empty() {
            let _ = tx.send(AppEvent::TaskFailed {
                id,
                error: "No active providers for update".into(),
            });
            return;
        }

        let mut success = 0;
        for pkg in pkgs_to_update {
            for provider in &providers {
                if crate::app::actions::update_asset(scope, &pkg, store.as_ref(), *provider).is_ok()
                {
                    success += 1;
                }
            }
        }
        let _ = tx.send(AppEvent::TaskProgress { id, percent: 100 });
        let _ = tx.send(AppEvent::TriggerReload);
        let _ = tx.send(AppEvent::TaskCompleted {
            id,
            message: format!("Updated {} items successfully", success),
        });
    });

    state.checked_items.clear();
    Ok(())
}

pub fn apply_tab_switch(state: &mut AppState, idx: usize, tab_count: usize) {
    if idx < tab_count {
        state.active_tab = idx;
        state.selected_index = 0;
        state.search_query.clear();
        state.status_line.clear();
        state.list_mode = ListMode::Normal;
    }
}

pub fn apply_search_char(state: &mut AppState, c: char) {
    state.search_query.push(c);
    state.list_mode = ListMode::Searching;
    state.selected_index = 0;
    state.status_line.clear();
}

pub fn apply_esc(state: &mut AppState) {
    state.search_query.clear();
    state.list_mode = ListMode::Normal;
    state.selected_index = 0;
    state.remote_packages.clear();
    if let Some(id) = state.clawhub_search_task_id.take() {
        state.active_tasks.remove(&id);
    }
}

pub fn apply_scope_toggle(state: &mut AppState) {
    state.toggle_scope();
    state.status_line = format!("Scope: {}", state.scope_label());
}

pub fn apply_space_no_provider(state: &mut AppState, providers_tab_idx: usize) {
    apply_tab_switch(state, providers_tab_idx, state.tab_names.len());
    state.status_line = "No provider configured \u{2014} please select one".to_string();
}

pub fn apply_enter_attach_vault(state: &mut AppState) {
    state.list_mode = ListMode::AttachVault;
    state.prompt_buffer = String::new();
    state.status_line.clear();
}

pub fn apply_enter_register_mcp(state: &mut AppState) {
    state.list_mode = ListMode::RegisterMcpStepName;
    state.prompt_buffer = String::new();
    state.pending_mcp_name.clear();
    state.pending_mcp_command.clear();
    state.pending_mcp_args.clear();
    state.pending_mcp_transport = "stdio".to_string();
    state.pending_mcp_description.clear();
    state.status_line.clear();
}

pub fn apply_enter_add_profile(state: &mut AppState) {
    state.list_mode = ListMode::ProfileWizardName;
    state.prompt_buffer = String::new();
    state.status_line = "Profile name: ".to_string();
}

fn handle_delete_profile(state: &mut AppState, _ctx: &EventContext) -> Result<ControlFlow> {
    let filtered_len = state.profile_entries.len();
    if filtered_len == 0 || state.selected_index >= filtered_len {
        state.status_line = "No profile selected to delete".to_string();
        return Ok(ControlFlow::Continue);
    }
    let name = state.profile_entries[state.selected_index].name.clone();
    state.pending_delete_profile = Some(name.clone());
    state.list_mode = ListMode::ConfirmDeleteProfile;
    Ok(ControlFlow::Continue)
}

fn handle_delete_profile_confirm(state: &mut AppState, ctx: &EventContext) -> Result<ControlFlow> {
    let name = match state.pending_delete_profile.take() {
        Some(n) => n,
        None => {
            state.list_mode = ListMode::Normal;
            return Ok(ControlFlow::Continue);
        }
    };

    let scope = state.active_scope;
    let store = ctx.store.clone();
    let tx = ctx.tx.clone();
    let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    tokio::task::spawn_blocking(move || {
        let _ = tx.send(AppEvent::TaskStarted {
            id,
            name: format!("Deleting profile '{}'", &name),
        });
        match store.load(scope) {
            Ok(mut config) => {
                if config.remove_profile(&name) {
                    match store.save(scope, &config) {
                        Ok(()) => {
                            let _ = tx.send(AppEvent::TriggerReload);
                            let _ = tx.send(AppEvent::TaskCompleted {
                                id,
                                message: format!("Profile '{}' deleted", &name),
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::TaskFailed {
                                id,
                                error: format!("Failed to save config: {}", e),
                            });
                        }
                    }
                } else {
                    let _ = tx.send(AppEvent::TaskFailed {
                        id,
                        error: format!("Profile '{}' not found", &name),
                    });
                }
            }
            Err(e) => {
                let _ = tx.send(AppEvent::TaskFailed {
                    id,
                    error: format!("Failed to load config: {}", e),
                });
            }
        }
    });

    state.list_mode = ListMode::Normal;
    Ok(ControlFlow::Continue)
}

fn handle_open_location(
    state: &mut AppState,
    _ctx: &EventContext,
    in_terminal: bool,
) -> Result<()> {
    let active_kind = state
        .tab_kinds
        .get(state.active_tab)
        .copied()
        .unwrap_or(crate::tui::app::TabKind::Asset);

    if active_kind != crate::tui::app::TabKind::Asset {
        state.status_line = "Ctrl+O/T only works in Skills/Instructions tab".to_string();
        return Ok(());
    }

    let filtered = state.filtered_packages();
    let Some(pkg) = filtered.get(state.selected_index).copied() else {
        state.status_line = "No item selected".to_string();
        return Ok(());
    };

    if pkg.is_remote {
        state.status_line = "Only local packages can be opened".to_string();
        return Ok(());
    }
    if pkg.path.as_os_str().is_empty() {
        state.status_line = "Selected package has no local path".to_string();
        return Ok(());
    }

    let path = &pkg.path;
    let result = if in_terminal {
        crate::domain::paths::open_terminal(path)
    } else {
        crate::domain::paths::open_file_manager(path)
    };

    match result {
        Ok(()) => {
            state.status_line = if in_terminal {
                format!("Opening terminal at {}", path.display())
            } else {
                format!("Opening {} in file manager", path.display())
            };
        }
        Err(e) => {
            state.status_line = format!("Failed to open: {}", e);
        }
    }
    Ok(())
}

fn parse_github_url(url: &str) -> Option<(String, String)> {
    let url = url.trim();
    let url = url.strip_suffix(".git").unwrap_or(url);

    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("github.com/"));

    if let Some(path) = path {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            let repo = format!("{}/{}", parts[0], parts[1]);
            let id = parts[1].to_string();
            return Some((id, repo));
        }
    }
    None
}

pub async fn refresh_all_vaults(
    registry: Arc<crate::app::registry::Registry>,
    tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
    message_prefix: &str,
) -> Result<()> {
    let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let _ = tx.send(AppEvent::TaskStarted {
        id,
        name: format!("{}refreshing vaults...", message_prefix),
    });
    let mut errs = Vec::new();
    let total = registry.vaults.len();
    for (i, vault) in registry.vaults.iter().enumerate() {
        if let Err(e) = vault.refresh().await {
            errs.push(format!("{}: {}", vault.id(), e));
        }
        let percent = (((i + 1) as f32 / total.max(1) as f32) * 100.0) as u8;
        let _ = tx.send(AppEvent::TaskProgress { id, percent });
    }
    let _ = tx.send(AppEvent::TriggerReload);
    if errs.is_empty() {
        let _ = tx.send(AppEvent::TaskCompleted {
            id,
            message: format!("{}refreshed successfully", message_prefix),
        });
    } else {
        let _ = tx.send(AppEvent::TaskFailed {
            id,
            error: format!("{}refresh issues: {}", message_prefix, errs.join(", ")),
        });
    }
    Ok(())
}

fn active_providers<'a>(
    registry: &'a crate::app::registry::Registry,
    config: &crate::domain::config::ConfigFile,
) -> Vec<&'a dyn crate::app::ports::ProviderPort> {
    registry
        .providers
        .iter()
        .filter(|p| config.providers.contains(&p.id().to_string()))
        .map(|p| p.as_ref())
        .collect()
}

fn execute_attach_vault(
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

                // Signal that we need a refresh (async work handled elsewhere)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::{AppState, ListMode};
    use std::collections::HashMap;

    fn empty_state(tab_count: usize) -> AppState {
        AppState::new(
            (0..tab_count).map(|i| format!("Tab{}", i)).collect(),
            vec![true; tab_count],
            HashMap::new(),
        )
    }

    #[test]
    fn switch_tab_updates_active_tab() {
        let mut state = empty_state(4);
        apply_tab_switch(&mut state, 2, 4);
        assert_eq!(state.active_tab, 2);
    }

    #[test]
    fn switch_tab_resets_selection_and_search() {
        let mut state = empty_state(4);
        state.selected_index = 3;
        state.search_query = "foo".to_string();
        apply_tab_switch(&mut state, 1, 4);
        assert_eq!(state.selected_index, 0);
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn switch_tab_ignores_out_of_range() {
        let mut state = empty_state(4);
        apply_tab_switch(&mut state, 9, 4);
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn search_query_appends_char() {
        let mut state = empty_state(1);
        apply_search_char(&mut state, 'a');
        apply_search_char(&mut state, 'b');
        assert_eq!(state.search_query, "ab");
        assert_eq!(state.list_mode, ListMode::Searching);
    }

    #[test]
    fn esc_clears_search() {
        let mut state = empty_state(1);
        state.search_query = "hello".to_string();
        state.list_mode = ListMode::Searching;
        apply_esc(&mut state);
        assert!(state.search_query.is_empty());
        assert_eq!(state.list_mode, ListMode::Normal);
    }

    #[test]
    fn space_redirects_to_providers_tab_when_no_provider() {
        let mut state = empty_state(5);
        apply_space_no_provider(&mut state, 3); // Providers is now tab 3
        assert_eq!(state.active_tab, 3);
        assert!(!state.status_line.is_empty());
    }

    #[test]
    fn a_key_on_vaults_tab_enters_attach_mode() {
        let mut state = empty_state(5);
        state.active_tab = 0; // Vaults tab is now index 0
        apply_enter_attach_vault(&mut state);
        assert_eq!(state.list_mode, ListMode::AttachVault);
    }

    #[test]
    fn s_key_toggles_scope() {
        let mut state = empty_state(4);
        use crate::domain::scope::Scope;
        assert_eq!(state.active_scope, Scope::Workspace);
        apply_scope_toggle(&mut state);
        assert_eq!(state.active_scope, Scope::Global);
    }

    #[test]
    fn parse_github_url_works() {
        assert_eq!(
            parse_github_url("https://github.com/obra/superpowers"),
            Some(("superpowers".to_string(), "obra/superpowers".to_string()))
        );
        assert_eq!(
            parse_github_url("https://github.com/obra/superpowers.git"),
            Some(("superpowers".to_string(), "obra/superpowers".to_string()))
        );
        assert_eq!(
            parse_github_url("github.com/obra/superpowers"),
            Some(("superpowers".to_string(), "obra/superpowers".to_string()))
        );
        assert_eq!(
            parse_github_url("https://github.com/obra/superpowers/tree/main"),
            Some(("superpowers".to_string(), "obra/superpowers".to_string()))
        );
        assert!(parse_github_url("/local/path").is_none());
    }

    #[test]
    fn test_handle_navigation_down() {
        let mut state = empty_state(1);
        state.packages.insert(
            0,
            vec![
                crate::domain::asset::ScannedPackage {
                    identity: crate::domain::identity::AssetIdentity::new("a", None, "hash"),
                    path: std::path::PathBuf::from("a"),
                    vault_id: "v".into(),
                    kind: crate::domain::asset::AssetKind::Skill,
                    is_remote: false,
                    remote_meta: None,
                    requires: vec![],
                    requires_optional: vec![],
                    author: None,
                    description: None,
                },
                crate::domain::asset::ScannedPackage {
                    identity: crate::domain::identity::AssetIdentity::new("b", None, "hash"),
                    path: std::path::PathBuf::from("b"),
                    vault_id: "v".into(),
                    kind: crate::domain::asset::AssetKind::Skill,
                    is_remote: false,
                    remote_meta: None,
                    requires: vec![],
                    requires_optional: vec![],
                    author: None,
                    description: None,
                },
            ],
        );
        state.tab_kinds = vec![crate::tui::app::TabKind::Asset];

        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let registry = Arc::new(crate::app::registry::Registry::new());
        let store = Arc::new(crate::infra::config::toml_store::TomlConfigStore::new(
            std::path::PathBuf::from("g"),
            std::path::PathBuf::from("w"),
        ));
        let ctx = EventContext {
            store,
            registry,
            tx,
            workspace_root: std::path::PathBuf::from("."),
        };

        let event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::empty(),
        ));
        handle(&mut state, &ctx, event).unwrap();
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn test_handle_esc_quit() {
        let mut state = empty_state(1);
        state.esc_pressed_once = true;
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let registry = Arc::new(crate::app::registry::Registry::new());
        let store = Arc::new(crate::infra::config::toml_store::TomlConfigStore::new(
            std::path::PathBuf::from("g"),
            std::path::PathBuf::from("w"),
        ));
        let ctx = EventContext {
            store,
            registry,
            tx,
            workspace_root: std::path::PathBuf::from("."),
        };

        let event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::empty(),
        ));
        let res = handle(&mut state, &ctx, event).unwrap();
        assert!(matches!(res, ControlFlow::Quit));
    }

    use crate::app::ports::{ConfigStorePort, ProviderPort};
    use crate::domain::asset::AssetKind;
    use crate::domain::config::ConfigFile;
    use crate::domain::identity::AssetIdentity;
    use crate::domain::scope::Scope;

    struct FakeStore {
        config: std::sync::Mutex<ConfigFile>,
    }
    impl FakeStore {
        fn new(config: ConfigFile) -> Self {
            Self {
                config: std::sync::Mutex::new(config),
            }
        }
    }
    impl ConfigStorePort for FakeStore {
        fn load(&self, _scope: Scope) -> Result<ConfigFile> {
            Ok(self.config.lock().unwrap().clone())
        }
        fn save(&self, _scope: Scope, config: &ConfigFile) -> Result<()> {
            *self.config.lock().unwrap() = config.clone();
            Ok(())
        }
    }

    struct FakeProvider {
        id: String,
    }
    impl ProviderPort for FakeProvider {
        fn id(&self) -> &str {
            &self.id
        }
        fn name(&self) -> &str {
            &self.id
        }
        fn install(
            &self,
            _pkg: &crate::domain::asset::ScannedPackage,
            _scope: Scope,
            _config: Option<&crate::domain::config::ConfigFile>,
        ) -> Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            _identity: &AssetIdentity,
            _kind: &AssetKind,
            _scope: Scope,
            _config: Option<&crate::domain::config::ConfigFile>,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn toggle_provider_shows_confirm_popup_when_last_provider_has_assets() {
        let mut state = empty_state(5);
        state.tab_kinds = vec![
            crate::tui::app::TabKind::Asset,    // Skills
            crate::tui::app::TabKind::Mcp,      // MCP
            crate::tui::app::TabKind::Asset,    // Instructions
            crate::tui::app::TabKind::Provider, // Providers
            crate::tui::app::TabKind::Vault,    // Vaults
        ];
        state.active_tab = 3; // Providers tab
        state.provider_entries = vec![crate::domain::asset::ProviderEntry {
            id: "fake".to_string(),
            name: "Fake".to_string(),
            active: true,
        }];

        let mut config = ConfigFile::default();
        config.providers = vec!["fake".to_string()];
        config.vault_defs.insert(
            "workspace".to_string(),
            crate::domain::config::VaultSection {
                vault: None,
                skills: Some(crate::domain::config::AssetBucket {
                    items: vec!["[my-skill:--:0000000000]".to_string()],
                }),
                instructions: None,
            },
        );
        state.configs.insert(Scope::Workspace, config.clone());

        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let mut registry = crate::app::registry::Registry::new();
        registry.register_provider(Box::new(FakeProvider { id: "fake".into() }));
        let registry = Arc::new(registry);

        let store = Arc::new(FakeStore::new(config));
        let ctx = EventContext {
            store,
            registry,
            tx,
            workspace_root: std::path::PathBuf::from("."),
        };

        let event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::empty(),
        ));
        let res = handle(&mut state, &ctx, event).unwrap();

        assert!(matches!(res, ControlFlow::Continue));
        assert_eq!(
            state.list_mode,
            ListMode::ConfirmDeactivateLastProvider,
            "Expected confirm popup when deactivating last provider with installed assets"
        );
        assert_eq!(state.pending_deactivate_provider_id, "fake");
    }

    #[tokio::test]
    async fn test_handle_space_install() {
        let mut state = empty_state(1);
        let pkg = crate::domain::asset::ScannedPackage {
            identity: crate::domain::identity::AssetIdentity::new("my-skill", None, "hash"),
            path: std::path::PathBuf::from("a"),
            vault_id: "v".into(),
            kind: crate::domain::asset::AssetKind::Skill,
            is_remote: false,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
        };
        state.packages.insert(0, vec![pkg.clone()]);
        state.tab_kinds = vec![crate::tui::app::TabKind::Asset];
        state.active_tab = 0;
        state.selected_index = 0;

        let mut config = ConfigFile::default();
        config.providers.push("fake".into());
        state.configs.insert(Scope::Workspace, config.clone());

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut registry = crate::app::registry::Registry::new();
        registry.register_provider(Box::new(FakeProvider { id: "fake".into() }));
        let registry = Arc::new(registry);

        let store = Arc::new(FakeStore::new(config));
        let ctx = EventContext {
            store,
            registry,
            tx,
            workspace_root: std::path::PathBuf::from("."),
        };

        let event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::empty(),
        ));
        handle(&mut state, &ctx, event).unwrap();

        // Use a small sleep to let the background task run (since it's spawn_blocking)
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Check events
        let mut events = Vec::new();
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }

        // Should have TaskStarted, TaskProgress, TriggerReload, TaskCompleted
        assert!(events
            .iter()
            .any(|e| matches!(e, AppEvent::TaskStarted { .. })));
        assert!(events.iter().any(|e| matches!(e, AppEvent::TriggerReload)));
        assert!(events
            .iter()
            .any(|e| matches!(e, AppEvent::TaskCompleted { .. })));
    }
}
