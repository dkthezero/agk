//! Single-select wizard steps: `TemplateSelect`, `ScopeSelect`, and the v0.4
//! `ProviderSelect` / `LlmProviderSelect` steps. Each is an Up/Down/Enter list
//! with Esc to go back or cancel.

use crate::app::ports::WizardStep;
use crate::tui::app::AppState;
use crate::tui::list_mode::ListMode;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

/// Handle the `TemplateSelect` step.
pub(super) fn handle_template_select(
    state: &mut AppState,
    key: &KeyEvent,
    step: &WizardStep,
) -> Result<()> {
    let WizardStep::TemplateSelect { templates, .. } = step else {
        return Ok(());
    };
    let code = &key.code;
    let ws = match state.wizard_state.as_mut() {
        Some(s) => s,
        None => return Ok(()),
    };
    match code {
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
                state
                    .analytics_config
                    .increment_template_selection(&tmpl.id);
                let _ = crate::domain::telemetry::AnalyticsConfig::save(
                    &state.analytics_config,
                    &crate::domain::paths::analytics_path(),
                );
            }
            if let Some(ref mut ws) = state.wizard_state {
                ws.step_index += 1;
                ws.selected = 0;
                ws.sync_checklist_state();
            }
        }
        KeyCode::Esc => {
            state.wizard_state = None;
            state.list_mode = ListMode::Normal;
            state.status_line = "Cancelled profile creation".to_string();
        }
        _ => {}
    }
    Ok(())
}

/// Handle the `ScopeSelect` step (workspace vs global).
pub(super) fn handle_scope_select(state: &mut AppState, key: &KeyEvent) -> Result<()> {
    let options = ["workspace", "global"];
    let code = &key.code;
    let ws = match state.wizard_state.as_mut() {
        Some(s) => s,
        None => return Ok(()),
    };
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
    Ok(())
}

/// Handle the `ProviderSelect` step (claude-code / opencode / ...).
pub(super) fn handle_provider_select(
    state: &mut AppState,
    key: &KeyEvent,
    step: &WizardStep,
) -> Result<()> {
    let WizardStep::ProviderSelect { providers, .. } = step else {
        return Ok(());
    };
    let code = &key.code;
    let ws = match state.wizard_state.as_mut() {
        Some(s) => s,
        None => return Ok(()),
    };
    match code {
        KeyCode::Up if ws.selected > 0 => {
            ws.selected -= 1;
        }
        KeyCode::Down if ws.selected + 1 < providers.len() => {
            ws.selected += 1;
        }
        KeyCode::Enter => {
            if let Some((id, _)) = providers.get(ws.selected) {
                ws.provider_id_choice = id.clone();
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
    }
    Ok(())
}

/// Handle the `LlmProviderSelect` step.
pub(super) fn handle_llm_provider_select(
    state: &mut AppState,
    key: &KeyEvent,
    step: &WizardStep,
) -> Result<()> {
    let WizardStep::LlmProviderSelect { providers, .. } = step else {
        return Ok(());
    };
    let code = &key.code;
    let ws = match state.wizard_state.as_mut() {
        Some(s) => s,
        None => return Ok(()),
    };
    match code {
        KeyCode::Up if ws.selected > 0 => {
            ws.selected -= 1;
        }
        KeyCode::Down if ws.selected + 1 < providers.len() => {
            ws.selected += 1;
        }
        KeyCode::Enter => {
            if let Some((id, _)) = providers.get(ws.selected) {
                ws.llm_provider_id = id.clone();
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
    }
    Ok(())
}
