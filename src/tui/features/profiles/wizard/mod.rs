//! Profile-creation wizard input handling.
//!
//! The wizard is a sequence of [`WizardStep`]s driven entirely by key events.
//! This module owns the entry point [`handle_profile_wizard_input`], which does
//! the common setup (bounds check, current-step lookup) and then dispatches to
//! a per-step-family handler. The handlers live in sibling submodules so each
//! file stays focused and within the ADR-001 §6.4 file-size budget:
//!
//! * [`text_steps`]   — free-form text entry (`TextInput`, `QuestionAnswer`,
//!   `Textarea`, `ModelInput`, `AgentDescription`).
//! * [`select_steps`] — single-select lists (`TemplateSelect`, `ScopeSelect`,
//!   `ProviderSelect`, `LlmProviderSelect`).
//! * [`pick_steps`]   — checklists (`Checklist`, `ToolSelect`,
//!   `PermissionSelect`, `SkillsPick`).
//! * [`review_steps`] — `Review` and `ReviewFinal`.

mod pick_steps;
mod review_steps;
mod select_steps;
mod text_steps;

use crate::app::ports::WizardStep;
use crate::tui::app::AppState;
use crate::tui::event::EventContext;
use crate::tui::list_mode::ListMode;
use anyhow::Result;
use crossterm::event::KeyEvent;

pub fn handle_profile_wizard_input(
    state: &mut AppState,
    ctx: &EventContext,
    key: &KeyEvent,
) -> Result<()> {
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
        | WizardStep::Textarea { .. } => text_steps::handle_text_input(state, key, current_step),
        WizardStep::ModelInput { .. } => text_steps::handle_model_input(state, key),
        WizardStep::AgentDescription { .. } => text_steps::handle_agent_description(state, key),
        WizardStep::TemplateSelect { .. } => {
            select_steps::handle_template_select(state, key, &current_step)
        }
        WizardStep::ScopeSelect { .. } => select_steps::handle_scope_select(state, key),
        WizardStep::ProviderSelect { .. } => {
            select_steps::handle_provider_select(state, key, &current_step)
        }
        WizardStep::LlmProviderSelect { .. } => {
            select_steps::handle_llm_provider_select(state, key, &current_step)
        }
        WizardStep::Checklist { .. }
        | WizardStep::ToolSelect { .. }
        | WizardStep::PermissionSelect { .. } => {
            pick_steps::handle_checklist(state, key, &current_step)
        }
        WizardStep::SkillsPick { .. } => pick_steps::handle_skills_pick(state, key, &current_step),
        WizardStep::Review { .. } => review_steps::handle_review(state, ctx, key),
        WizardStep::ReviewFinal { .. } => review_steps::handle_review_final(state, key),
        WizardStep::Interactive { .. } => Ok(()),
    }
}
