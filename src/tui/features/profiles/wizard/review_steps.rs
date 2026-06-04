//! Review wizard steps: the legacy `Review` step (delegated to
//! `wizard_review`) and the v0.4 read-only `ReviewFinal` summary.

use crate::tui::app::AppState;
use crate::tui::event::EventContext;
use crate::tui::list_mode::ListMode;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

/// Handle the legacy `Review` step by delegating to the review controller.
pub(super) fn handle_review(
    state: &mut AppState,
    ctx: &EventContext,
    key: &KeyEvent,
) -> Result<()> {
    crate::tui::features::profiles::wizard_review::handle_review_step(state, ctx, key);
    Ok(())
}

/// Handle the v0.4 `ReviewFinal` read-only summary: Enter commits, Esc steps
/// back, any other key is ignored.
pub(super) fn handle_review_final(state: &mut AppState, key: &KeyEvent) -> Result<()> {
    let ws = match state.wizard_state.as_mut() {
        Some(s) => s,
        None => return Ok(()),
    };
    match &key.code {
        KeyCode::Enter => {
            // Advance past the wizard — the controller (out of scope for C3)
            // will turn the captured fields into a real profile create call.
            ws.step_index += 1;
            state.status_line = "Final review confirmed (commit wiring lands in C4)".to_string();
        }
        KeyCode::Esc => {
            if ws.step_index == 0 {
                state.wizard_state = None;
                state.list_mode = ListMode::Normal;
                state.status_line = "Cancelled profile creation".to_string();
                return Ok(());
            }
            ws.step_index -= 1;
        }
        _ => {}
    }
    Ok(())
}
