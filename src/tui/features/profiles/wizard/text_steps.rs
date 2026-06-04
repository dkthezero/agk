//! Text-editing wizard steps: free-form `TextInput` / `QuestionAnswer` /
//! `Textarea`, plus the v0.4 `ModelInput` and `AgentDescription` steps. They
//! share the same character/cursor editing semantics over `prompt_buffer`.

use crate::app::ports::WizardStep;
use crate::tui::app::AppState;
use crate::tui::list_mode::ListMode;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

/// Handle `TextInput` / `QuestionAnswer` / `Textarea` steps.
pub(super) fn handle_text_input(
    state: &mut AppState,
    key: &KeyEvent,
    current_step: WizardStep,
) -> Result<()> {
    let code = &key.code;
    let ws = match state.wizard_state.as_mut() {
        Some(s) => s,
        None => return Ok(()),
    };
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
        KeyCode::Right if ws.cursor_pos < ws.prompt_buffer.chars().count() => ws.cursor_pos += 1,
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
    Ok(())
}

/// Handle the `ModelInput` step — same key handling as `TextInput` but writes
/// to `model_string`.
pub(super) fn handle_model_input(state: &mut AppState, key: &KeyEvent) -> Result<()> {
    let code = &key.code;
    let ws = match state.wizard_state.as_mut() {
        Some(s) => s,
        None => return Ok(()),
    };
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
            ws.model_string = val;
            ws.step_index += 1;
            ws.sync_checklist_state();
        }
        KeyCode::Left if ws.cursor_pos > 0 => ws.cursor_pos -= 1,
        KeyCode::Right if ws.cursor_pos < ws.prompt_buffer.chars().count() => ws.cursor_pos += 1,
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
    Ok(())
}

/// Handle the `AgentDescription` step — multi-line free-form description.
/// Enter inserts a newline; an empty Enter finishes the step.
pub(super) fn handle_agent_description(state: &mut AppState, key: &KeyEvent) -> Result<()> {
    let code = &key.code;
    let ws = match state.wizard_state.as_mut() {
        Some(s) => s,
        None => return Ok(()),
    };
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
            // Empty Enter on the AgentDescription step finishes it.
            if ws.prompt_buffer.is_empty() {
                ws.agent_description = String::new();
                ws.step_index += 1;
                ws.sync_checklist_state();
                return Ok(());
            }
            let byte_idx = ws
                .prompt_buffer
                .char_indices()
                .nth(ws.cursor_pos)
                .map(|(i, _)| i)
                .unwrap_or(ws.prompt_buffer.len());
            ws.prompt_buffer.insert(byte_idx, '\n');
            ws.cursor_pos += 1;
        }
        KeyCode::Left if ws.cursor_pos > 0 => ws.cursor_pos -= 1,
        KeyCode::Right if ws.cursor_pos < ws.prompt_buffer.chars().count() => ws.cursor_pos += 1,
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
    Ok(())
}
