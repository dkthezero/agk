use crate::app::ports::WizardStep;
use crate::tui::app::AppState;
use crate::tui::event::{AppEvent, ControlFlow, EventContext};
use crate::tui::list_mode::ListMode;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_profile_wizard_input(
    state: &mut AppState,
    ctx: &EventContext,
    key: &KeyEvent,
) -> Result<()> {
    let code = &key.code;
    let ws = match state.wizard_state.as_mut() {
        Some(s) => s,
        None => return Ok(()),
    };

    if ws.step_index >= ws.steps.len() {
        state.wizard_state = None;
        state.list_mode = ListMode::Normal;
        return Ok(());
    }

    let current_step = ws.steps[ws.step_index].clone();

    match current_step {
        WizardStep::TextInput { .. } | WizardStep::QuestionAnswer { .. } => match code {
            KeyCode::Char(c) => {
                let byte_idx = ws
                    .prompt_buffer
                    .char_indices()
                    .nth(ws.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(ws.prompt_buffer.len());
                ws.prompt_buffer.insert(byte_idx, *c);
                ws.cursor_pos += 1;
            }
            KeyCode::Backspace if ws.cursor_pos > 0 => {
                ws.cursor_pos -= 1;
                let byte_idx = ws
                    .prompt_buffer
                    .char_indices()
                    .nth(ws.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(ws.prompt_buffer.len());
                let ch = ws.prompt_buffer[byte_idx..].chars().next().unwrap_or('\n');
                ws.prompt_buffer.drain(byte_idx..byte_idx + ch.len_utf8());
            }
            KeyCode::Backspace => {}
            KeyCode::Enter => {
                let val = std::mem::take(&mut ws.prompt_buffer).trim().to_string();
                ws.cursor_pos = 0;
                if val.is_empty() {
                    state.status_line = "Cancelled — input required".to_string();
                    state.wizard_state = None;
                    state.list_mode = ListMode::Normal;
                    return Ok(());
                }
                if let WizardStep::TextInput { .. } = current_step {
                    let config = state.active_config().clone();
                    if config.find_profile(&val).is_some() {
                        state.status_line = format!("Profile '{}' already exists", val);
                        state.wizard_state = None;
                        state.list_mode = ListMode::Normal;
                        return Ok(());
                    }
                }
                if let Some(ref mut ws) = state.wizard_state {
                    match current_step {
                        WizardStep::TextInput { .. } => {
                            ws.name = val;
                        }
                        WizardStep::QuestionAnswer { question, .. } => {
                            ws.description_parts.push((question, val));
                        }
                        _ => {}
                    }
                    ws.step_index += 1;
                    ws.sync_checklist_state();
                }
            }
            KeyCode::Left if ws.cursor_pos > 0 => ws.cursor_pos -= 1,
            KeyCode::Right if ws.cursor_pos < ws.prompt_buffer.chars().count() => {
                ws.cursor_pos += 1
            }
            KeyCode::Esc => {
                if ws.step_index == 0 {
                    state.wizard_state = None;
                    state.list_mode = ListMode::Normal;
                    state.status_line = "Cancelled profile creation".to_string();
                    return Ok(());
                }
                ws.step_index -= 1;
                ws.cursor_pos = 0;
            }
            _ => {}
        },
        WizardStep::Checklist { ref options, .. } => {
            let filtered = ws.filtered_indices();
            match code {
                KeyCode::Up if ws.selected > 0 => {
                    ws.selected -= 1;
                }
                KeyCode::Down if ws.selected + 1 < filtered.len() => {
                    ws.selected += 1;
                }
                KeyCode::Char(' ') => {
                    if let Some(original_idx) = ws.selected_original_index() {
                        if let Some(c) = ws.checked.get_mut(original_idx) {
                            *c = !*c;
                        }
                    }
                }
                KeyCode::Char(c) if key.modifiers.is_empty() => {
                    // Type to filter options (only if Ctrl/LAlt/RAlt not pressed)
                    ws.filter_query.push(*c);
                    ws.selected = 0;
                    ws.scroll_offset = 0;
                }
                KeyCode::Backspace if !ws.filter_query.is_empty() => {
                    ws.filter_query.pop();
                    ws.selected = 0;
                    ws.scroll_offset = 0;
                }
                KeyCode::Esc if !ws.filter_query.is_empty() => {
                    ws.filter_query.clear();
                    ws.selected = 0;
                    ws.scroll_offset = 0;
                }
                KeyCode::Esc => {
                    if ws.step_index == 0 {
                        state.wizard_state = None;
                        state.list_mode = ListMode::Normal;
                        state.status_line = "Cancelled profile creation".to_string();
                        return Ok(());
                    }
                    ws.step_index -= 1;
                    ws.filter_query.clear();
                    ws.scroll_offset = 0;
                    ws.sync_checklist_state();
                }
                KeyCode::Enter => {
                    let selected_items: Vec<String> = options
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| ws.checked.get(*i) == Some(&true))
                        .map(|(_, name)| name.clone())
                        .collect();

                    if let WizardStep::Checklist { ref title, .. } = current_step {
                        if title.to_lowercase().contains("skill") {
                            ws.skills = selected_items;
                        } else {
                            ws.mcps = selected_items;
                        }
                    }
                    ws.step_index += 1;
                    ws.filter_query.clear();
                    ws.scroll_offset = 0;
                    ws.sync_checklist_state();
                }
                _ => {}
            }
        }
        WizardStep::Review { .. } => match code {
            KeyCode::Enter => {
                let name = ws.name.clone();
                let skills = ws.skills.clone();
                let mcps = ws.mcps.clone();
                let provider_id = ws.provider_id.clone();
                let desc = ws.composed_description();
                state.wizard_state = None;
                state.list_mode = ListMode::Normal;
                state.status_line.clear();

                let mut new_config = state.active_config().clone();
                new_config.profiles.push(crate::domain::config::Profile {
                    name: name.clone(),
                    provider_id,
                    scope: state.active_scope.to_string().to_lowercase(),
                    skills: skills
                        .iter()
                        .map(|s| crate::domain::profile::ProfileAssetRef::new(s.clone(), "auto"))
                        .collect(),
                    mcps: mcps
                        .iter()
                        .map(|m| crate::domain::profile::ProfileAssetRef::new(m.clone(), "auto"))
                        .collect(),
                    instructions: vec![],
                    tool_refs: vec![],
                    permission_mode: None,
                    prompt_overlay_path: None,
                });

                let scope = state.active_scope;
                let store = ctx.core.store.clone();
                let tx = ctx.tx.clone();
                let id = crate::tui::progress::NEXT_TASK_ID
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let workspace_root = ctx.workspace_root.clone();
                let profile_dir = workspace_root.join(".agk").join("profiles").join(&name);
                let _ = std::fs::create_dir_all(&profile_dir);

                tokio::task::spawn_blocking(move || {
                    let _ = tx.send(AppEvent::TaskStarted {
                        id,
                        name: format!("Creating profile '{}'", name),
                    });
                    match store.save(scope, &new_config) {
                        Ok(()) => {
                            let _ = tx.send(AppEvent::TriggerReload);
                            let _ = tx.send(AppEvent::RunInteractiveProcess {
                                command: "opencode".into(),
                                args: vec![
                                    "agent".into(),
                                    "create".into(),
                                    "--path".into(),
                                    profile_dir.display().to_string(),
                                    "--mode".into(),
                                    "primary".into(),
                                    "--description".into(),
                                    desc,
                                ],
                                current_dir: workspace_root,
                                profile_name: Some(name),
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
            }
            KeyCode::Esc => {
                if ws.step_index > 0 {
                    ws.step_index -= 1;
                } else {
                    state.wizard_state = None;
                    state.list_mode = ListMode::Normal;
                    state.status_line = "Cancelled profile creation".to_string();
                }
            }
            KeyCode::Up if ws.scroll_offset > 0 => {
                ws.scroll_offset -= 1;
            }
            KeyCode::Down => {
                ws.scroll_offset = ws.scroll_offset.saturating_add(1);
            }
            _ => {}
        },
        WizardStep::Interactive { .. } => {}
    }
    Ok(())
}

// For use from common/controller handle_backspace
pub fn handle_delete_profile_no_ctx(state: &mut AppState) -> Result<ControlFlow> {
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

pub fn handle_delete_profile(state: &mut AppState, _ctx: &EventContext) -> Result<ControlFlow> {
    handle_delete_profile_no_ctx(state)
}

pub fn handle_delete_profile_confirm(
    state: &mut AppState,
    ctx: &EventContext,
) -> Result<ControlFlow> {
    let name = match state.pending_delete_profile.take() {
        Some(n) => n,
        None => {
            state.list_mode = ListMode::Normal;
            return Ok(ControlFlow::Continue);
        }
    };

    let _ = ctx.tx.send(AppEvent::ExecuteCommand(
        crate::app::command::CoreCommand::DeleteProfile {
            id: crate::domain::profile::ProfileId::new(name),
            scope: state.active_scope,
        },
    ));
    state.list_mode = ListMode::Normal;
    Ok(ControlFlow::Continue)
}
