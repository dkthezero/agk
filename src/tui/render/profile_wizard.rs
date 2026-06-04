use crate::app::ports::WizardStep;
use crate::tui::app::AppState;
use crate::tui::widgets::modal;
use ratatui::Frame;

pub fn draw_profile_wizard(frame: &mut Frame, state: &AppState) {
    if let Some(ref ws) = state.wizard_state {
        let idx = ws.step_index;
        if let Some(step) = ws.steps.get(idx) {
            match step {
                WizardStep::TextInput { title, placeholder } => {
                    modal::render_input_modal(
                        frame,
                        title,
                        placeholder,
                        &ws.prompt_buffer,
                        ws.cursor_pos,
                    );
                }
                WizardStep::QuestionAnswer {
                    question,
                    placeholder,
                } => {
                    modal::render_input_modal(
                        frame,
                        question,
                        placeholder,
                        &ws.prompt_buffer,
                        ws.cursor_pos,
                    );
                }
                WizardStep::Checklist { title, options } => {
                    let filtered_indices = ws.filtered_indices();
                    let filtered_options: Vec<String> = filtered_indices
                        .iter()
                        .map(|&i| options[i].clone())
                        .collect();
                    let filtered_checked: Vec<bool> =
                        filtered_indices.iter().map(|&i| ws.checked[i]).collect();
                    let selected_filtered =
                        ws.selected.min(filtered_options.len().saturating_sub(1));
                    modal::render_checklist_modal(
                        frame,
                        title,
                        &filtered_options,
                        &filtered_checked,
                        selected_filtered,
                        &ws.filter_query,
                    );
                }
                WizardStep::Review { title } => {
                    let desc = ws.composed_description();
                    let tools = &ws.selected_tools;
                    let permission = ws.selected_permission_mode.as_deref();
                    modal::render_review_modal(
                        frame,
                        title,
                        &ws.name,
                        &desc,
                        &ws.skills,
                        &ws.mcps,
                        tools,
                        permission,
                        "[Enter] Confirm Create  [Esc] Back  [↑/↓] Scroll",
                        ws.scroll_offset,
                    );
                }
                WizardStep::TemplateSelect { title, templates } => {
                    let options: Vec<(String, String)> = templates
                        .iter()
                        .map(|t| (t.name.clone(), t.description.clone()))
                        .collect();
                    modal::render_select_modal(frame, title, &options, ws.selected);
                }
                WizardStep::ScopeSelect { title } => {
                    let options = vec![
                        ("workspace".into(), "Local to this workspace".into()),
                        ("global".into(), "Available in all workspaces".into()),
                    ];
                    modal::render_select_modal(frame, title, &options, ws.selected);
                }
                WizardStep::Textarea {
                    title, placeholder, ..
                } => {
                    modal::render_input_modal(
                        frame,
                        title,
                        placeholder,
                        &ws.prompt_buffer,
                        ws.cursor_pos,
                    );
                }
                WizardStep::ToolSelect { title, tools } => {
                    let filtered_indices = ws.filtered_indices();
                    let tool_labels: Vec<String> = tools
                        .iter()
                        .map(|(id, desc, _)| format!("{} — {}", id, desc))
                        .collect();
                    let filtered_options: Vec<String> = filtered_indices
                        .iter()
                        .map(|&i| tool_labels[i].clone())
                        .collect();
                    let filtered_checked: Vec<bool> =
                        filtered_indices.iter().map(|&i| ws.checked[i]).collect();
                    let selected_filtered =
                        ws.selected.min(filtered_options.len().saturating_sub(1));
                    modal::render_checklist_modal(
                        frame,
                        title,
                        &filtered_options,
                        &filtered_checked,
                        selected_filtered,
                        &ws.filter_query,
                    );
                }
                WizardStep::PermissionSelect { title, modes } => {
                    let filtered_indices = ws.filtered_indices();
                    let mode_labels: Vec<String> = modes
                        .iter()
                        .map(|(id, desc)| format!("{} — {}", id, desc))
                        .collect();
                    let filtered_options: Vec<String> = filtered_indices
                        .iter()
                        .map(|&i| mode_labels[i].clone())
                        .collect();
                    let filtered_checked: Vec<bool> =
                        filtered_indices.iter().map(|&i| ws.checked[i]).collect();
                    let selected_filtered =
                        ws.selected.min(filtered_options.len().saturating_sub(1));
                    modal::render_checklist_modal(
                        frame,
                        title,
                        &filtered_options,
                        &filtered_checked,
                        selected_filtered,
                        &ws.filter_query,
                    );
                }
                WizardStep::Interactive { .. } => {}
                // C3: render the v0.4 wizard steps. The new step kinds re-use
                // the existing modal widgets — the per-step shape is the same
                // (text input, single-select list, multi-select checklist,
                // read-only summary), so the renderer is intentionally light.
                WizardStep::ProviderSelect { title, providers } => {
                    modal::render_select_modal(frame, title, providers, ws.selected);
                }
                WizardStep::LlmProviderSelect { title, providers } => {
                    modal::render_select_modal(frame, title, providers, ws.selected);
                }
                WizardStep::ModelInput { title, placeholder } => {
                    modal::render_input_modal(
                        frame,
                        title,
                        placeholder,
                        &ws.prompt_buffer,
                        ws.cursor_pos,
                    );
                }
                WizardStep::AgentDescription {
                    title, placeholder, ..
                } => {
                    modal::render_input_modal(
                        frame,
                        title,
                        placeholder,
                        &ws.prompt_buffer,
                        ws.cursor_pos,
                    );
                }
                WizardStep::SkillsPick { title, options } => {
                    // Re-use the filtered checklist renderer — SkillsPick uses
                    // the same checked[] / filter_query state as Checklist.
                    let filtered_indices = ws.filtered_indices();
                    let filtered_options: Vec<String> = filtered_indices
                        .iter()
                        .map(|&i| options.get(i).cloned().unwrap_or_default())
                        .collect();
                    let filtered_checked: Vec<bool> = filtered_indices
                        .iter()
                        .map(|&i| ws.checked.get(i).copied().unwrap_or(false))
                        .collect();
                    let selected_filtered =
                        ws.selected.min(filtered_options.len().saturating_sub(1));
                    modal::render_checklist_modal(
                        frame,
                        title,
                        &filtered_options,
                        &filtered_checked,
                        selected_filtered,
                        &ws.filter_query,
                    );
                }
                WizardStep::ReviewFinal { title } => {
                    // Read-only summary: re-use the review modal with the
                    // most relevant fields captured on the new steps.
                    let tools = &ws.selected_tools;
                    let permission = ws.selected_permission_mode.as_deref();
                    modal::render_review_modal(
                        frame,
                        title,
                        &ws.name,
                        &ws.agent_description,
                        &ws.skills,
                        &ws.mcps,
                        tools,
                        permission,
                        "[Enter] Confirm  [Esc] Back",
                        ws.scroll_offset,
                    );
                }
            }
        }
    }
}
