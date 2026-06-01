use crate::app::ports::WizardStep;
use crate::tui::app::AppState;
use crate::tui::event::EventContext;
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
        WizardStep::TextInput { .. }
        | WizardStep::QuestionAnswer { .. }
        | WizardStep::Textarea { .. } => {
            match code {
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
                        // TextInput requires a value; Textarea/QuestionAnswer allow empty
                        if let WizardStep::TextInput { .. } = current_step {
                            state.status_line = "Cancelled — input required".to_string();
                            state.wizard_state = None;
                            state.list_mode = ListMode::Normal;
                            return Ok(());
                        }
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
                            WizardStep::Textarea { key, .. } if !val.is_empty() => {
                                ws.structured_answers.insert(key, val);
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
            }
        }
        WizardStep::TemplateSelect { ref templates, .. } => match code {
            KeyCode::Up if ws.selected > 0 => {
                ws.selected -= 1;
            }
            KeyCode::Down if ws.selected + 1 < templates.len() => {
                ws.selected += 1;
            }
            KeyCode::Enter => {
                if let Some(tmpl) = templates.get(ws.selected) {
                    ws.selected_template = Some(tmpl.id.clone());
                    ws.structured_answers = tmpl.defaults.clone();
                    ws.selected_tools = tmpl.default_tools.clone();
                    ws.selected_permission_mode = tmpl.default_permission_mode.clone();
                    // Track template selection in telemetry
                    state.analytics_config.increment_template_selection(&tmpl.id);
                    let _ = crate::domain::telemetry::AnalyticsConfig::save(
                        &state.analytics_config,
                        &crate::domain::paths::analytics_path(),
                    );
                }
                ws.step_index += 1;
                ws.selected = 0;
                ws.sync_checklist_state();
            }
            KeyCode::Esc => {
                state.wizard_state = None;
                state.list_mode = ListMode::Normal;
                state.status_line = "Cancelled profile creation".to_string();
            }
            _ => {}
        },
        WizardStep::ScopeSelect { .. } => {
            let options = ["workspace", "global"];
            match code {
                KeyCode::Up if ws.selected > 0 => {
                    ws.selected -= 1;
                }
                KeyCode::Down if ws.selected + 1 < options.len() => {
                    ws.selected += 1;
                }
                KeyCode::Enter => {
                    ws.scope = Some(match options[ws.selected] {
                        "global" => crate::domain::scope::Scope::Global,
                        _ => crate::domain::scope::Scope::Workspace,
                    });
                    ws.step_index += 1;
                    ws.selected = 0;
                    ws.sync_checklist_state();
                }
                KeyCode::Esc => {
                    state.wizard_state = None;
                    state.list_mode = ListMode::Normal;
                    state.status_line = "Cancelled profile creation".to_string();
                }
                _ => {}
            }
        }
        WizardStep::Checklist { .. }
        | WizardStep::ToolSelect { .. }
        | WizardStep::PermissionSelect { .. } => {
            let is_permission = matches!(current_step, WizardStep::PermissionSelect { .. });
            let is_tool = matches!(current_step, WizardStep::ToolSelect { .. });
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
                        if is_permission {
                            // Single-select: uncheck all, then check selected
                            for c in &mut ws.checked {
                                *c = false;
                            }
                            if let Some(c) = ws.checked.get_mut(original_idx) {
                                *c = true;
                            }
                        } else if let Some(c) = ws.checked.get_mut(original_idx) {
                            *c = !*c;
                        }
                    }
                }
                KeyCode::Char(c) if key.modifiers.is_empty() => {
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
                    if is_tool {
                        let tool_ids: Vec<String> = match &current_step {
                            WizardStep::ToolSelect { tools, .. } => tools
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| ws.checked.get(*i) == Some(&true))
                                .map(|(_, (id, _, _))| id.clone())
                                .collect(),
                            _ => vec![],
                        };
                        ws.selected_tools = tool_ids;
                    } else if is_permission {
                        let selected_mode: Option<String> = match &current_step {
                            WizardStep::PermissionSelect { modes, .. } => modes
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| ws.checked.get(*i) == Some(&true))
                                .map(|(_, (id, _))| id.clone())
                                .next(),
                            _ => None,
                        };
                        ws.selected_permission_mode = selected_mode;
                    } else if let WizardStep::Checklist { title, .. } = &current_step {
                        let options = match &current_step {
                            WizardStep::Checklist { options, .. } => options,
                            _ => &vec![],
                        };
                        let selected_items: Vec<String> = options
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| ws.checked.get(*i) == Some(&true))
                            .map(|(_, name)| name.clone())
                            .collect();
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
        WizardStep::Review { .. } => {
            crate::tui::features::profiles::wizard_review::handle_review_step(state, ctx, key);
        }
        WizardStep::Interactive { .. } => {}
    }
    Ok(())
}
