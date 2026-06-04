//! Multi-/single-select checklist wizard steps: the legacy
//! `Checklist` / `ToolSelect` / `PermissionSelect` trio and the v0.4
//! `SkillsPick` step. All support filtering, Space to toggle, Enter to commit.

use crate::app::ports::WizardStep;
use crate::tui::app::AppState;
use crate::tui::list_mode::ListMode;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

/// Handle `Checklist` / `ToolSelect` / `PermissionSelect` steps.
pub(super) fn handle_checklist(
    state: &mut AppState,
    key: &KeyEvent,
    current_step: &WizardStep,
) -> Result<()> {
    let code = &key.code;
    let is_permission = matches!(current_step, WizardStep::PermissionSelect { .. });
    let is_tool = matches!(current_step, WizardStep::ToolSelect { .. });
    let ws = match state.wizard_state.as_mut() {
        Some(s) => s,
        None => return Ok(()),
    };
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
                let tool_ids: Vec<String> = match current_step {
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
                let selected_mode: Option<String> = match current_step {
                    WizardStep::PermissionSelect { modes, .. } => modes
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| ws.checked.get(*i) == Some(&true))
                        .map(|(_, (id, _))| id.clone())
                        .next(),
                    _ => None,
                };
                ws.selected_permission_mode = selected_mode;
            } else if let WizardStep::Checklist { title, .. } = current_step {
                let options = match current_step {
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
    Ok(())
}

/// Handle the v0.4 `SkillsPick` step — re-uses the checklist state but keys
/// the toggle off a locally computed filtered index list.
pub(super) fn handle_skills_pick(
    state: &mut AppState,
    key: &KeyEvent,
    step: &WizardStep,
) -> Result<()> {
    let WizardStep::SkillsPick { options, .. } = step else {
        return Ok(());
    };
    let code = &key.code;
    let ws = match state.wizard_state.as_mut() {
        Some(s) => s,
        None => return Ok(()),
    };
    let filtered_indices: Vec<usize> = if ws.filter_query.is_empty() {
        (0..options.len()).collect()
    } else {
        let q = ws.filter_query.to_lowercase();
        options
            .iter()
            .enumerate()
            .filter(|(_, o)| o.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect()
    };
    match code {
        KeyCode::Up if ws.selected > 0 => {
            ws.selected -= 1;
        }
        KeyCode::Down if ws.selected + 1 < filtered_indices.len() => {
            ws.selected += 1;
        }
        KeyCode::Char(' ') => {
            if let Some(orig) = filtered_indices.get(ws.selected).copied() {
                if ws.checked.len() < options.len() {
                    ws.checked.resize(options.len(), false);
                }
                if let Some(c) = ws.checked.get_mut(orig) {
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
            let selected: Vec<String> = options
                .iter()
                .enumerate()
                .filter(|(i, _)| ws.checked.get(*i) == Some(&true))
                .map(|(_, name)| name.clone())
                .collect();
            ws.skills = selected;
            ws.step_index += 1;
            ws.filter_query.clear();
            ws.scroll_offset = 0;
            ws.sync_checklist_state();
        }
        _ => {}
    }
    Ok(())
}
