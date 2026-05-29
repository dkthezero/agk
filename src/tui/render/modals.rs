use crate::app::ports::WizardStep;
use crate::tui::app::AppState;
use crate::tui::list_mode::ListMode;
use crate::tui::widgets::modal;
use ratatui::Frame;

pub fn draw_modals(frame: &mut Frame, state: &AppState) {
    match &state.list_mode {
        ListMode::SelectProviderRoot {
            provider_id,
            options,
            selected,
        } => {
            let name = state
                .provider_entries
                .iter()
                .find(|p| p.id == *provider_id)
                .map(|p| p.name.as_str())
                .unwrap_or(provider_id);
            let title = format!("Select config folder for {}", name);
            modal::render_select_modal(frame, &title, options, *selected);
        }
        ListMode::AttachVault => {
            modal::render_input_modal(
                frame,
                "Attach Vault",
                "Enter local path or GitHub URL:",
                &state.prompt_buffer,
                state.prompt_buffer.chars().count(),
            );
        }
        ListMode::AttachVaultBranch => {
            modal::render_input_modal(
                frame,
                "Attach Vault",
                "Branch (default: main):",
                &state.prompt_buffer,
                state.prompt_buffer.chars().count(),
            );
        }
        ListMode::AttachVaultPath => {
            modal::render_input_modal(
                frame,
                "Attach Vault",
                "Subfolder (default: skills/):",
                &state.prompt_buffer,
                state.prompt_buffer.chars().count(),
            );
        }
        ListMode::AttachVaultName => {
            modal::render_input_modal(
                frame,
                "Attach Vault",
                "Vault name:",
                &state.prompt_buffer,
                state.prompt_buffer.chars().count(),
            );
        }
        ListMode::RegisterMcpStepName => {
            modal::render_input_modal(
                frame,
                "Register MCP Server",
                "Name:",
                &state.prompt_buffer,
                state.prompt_buffer.chars().count(),
            );
        }
        ListMode::RegisterMcpStepCommand => {
            modal::render_input_modal(
                frame,
                "Register MCP Server",
                "Command to run (e.g. npx, python):",
                &state.prompt_buffer,
                state.prompt_buffer.chars().count(),
            );
        }
        ListMode::RegisterMcpStepArgs => {
            modal::render_input_modal(
                frame,
                "Register MCP Server",
                "Arguments (space-separated, optional):",
                &state.prompt_buffer,
                state.prompt_buffer.chars().count(),
            );
        }
        ListMode::RegisterMcpStepTransport => {
            modal::render_input_modal(
                frame,
                "Register MCP Server",
                "Transport (stdio/sse), default stdio:",
                &state.prompt_buffer,
                state.prompt_buffer.chars().count(),
            );
        }
        ListMode::RegisterMcpStepDescription => {
            modal::render_input_modal(
                frame,
                "Register MCP Server",
                "Description (optional):",
                &state.prompt_buffer,
                state.prompt_buffer.chars().count(),
            );
        }
        ListMode::ProfileWizard => {
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
                            modal::render_review_modal(
                                frame,
                                title,
                                &ws.name,
                                &desc,
                                &ws.skills,
                                &ws.mcps,
                                "[Enter] Confirm Create  [Esc] Back  [↑/↓] Scroll",
                                ws.scroll_offset,
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
        ListMode::ConfirmMcpTest => {
            let msg = format!(
                "WARNING: This will execute '{} {}' on your machine.\nProceed?",
                state.pending_mcp_command, state.pending_mcp_args
            );
            modal::render_confirm_modal(
                frame,
                "Confirm MCP Registration",
                &msg,
                "[Enter] Confirm  [Esc] Cancel",
            );
        }
        ListMode::ConfirmClawHubInstall => {
            modal::render_confirm_modal(
                frame,
                "Install ClawHub CLI",
                "ClawHub CLI not found. Install via Homebrew?",
                "[Enter] Confirm  [Esc] Cancel",
            );
        }
        ListMode::ConfirmDetachVault => {
            let msg = format!(
                "Detach vault '{}'?
This will hide all its uninstalled skills.",
                state.pending_detach_vault.as_deref().unwrap_or("")
            );
            modal::render_confirm_modal(
                frame,
                "Detach Vault",
                &msg,
                "[Enter] Confirm  [Esc] Cancel",
            );
        }
        ListMode::ConfirmDeactivateLastProvider => {
            let msg = format!(
                "Deactivate '{}'?
This will remove all installed skills and leave no active provider.",
                state.pending_deactivate_provider_id
            );
            modal::render_confirm_modal(
                frame,
                "Deactivate Last Provider",
                &msg,
                "[Enter] Confirm  [Esc] Cancel",
            );
        }
        ListMode::ConfirmDeleteProfile => {
            let msg = format!(
                "Delete profile '{}'?
This will remove it from the configuration.",
                state.pending_delete_profile.as_deref().unwrap_or("")
            );
            modal::render_confirm_modal(
                frame,
                "Delete Profile",
                &msg,
                "[Enter] Confirm  [Esc] Cancel",
            );
        }
        _ => {}
    }
}
