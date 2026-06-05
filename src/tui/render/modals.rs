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
                "Arguments (space-separated, optional) — for SSE this is the URL:",
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
            crate::tui::render::profile_wizard::draw_profile_wizard(frame, state);
        }
        ListMode::EditProfile => {
            if let Some(ref es) = state.edit_profile_state {
                crate::tui::widgets::edit_profile_modal::render_edit_profile_modal(frame, es);
            }
        }
        ListMode::ExportProfile => {
            crate::tui::widgets::export_profile_modal::render_export_profile_modal(frame, state);
        }
        ListMode::ImportProfile => {
            crate::tui::widgets::import_profile_modal::render_import_profile_modal(frame, state);
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
        ListMode::ConfirmVaultInit => {
            let vault_name = if state.pending_vault_local_path.is_empty() {
                "this workspace".to_string()
            } else {
                format!("'{}'", state.pending_vault_local_path)
            };
            let msg = format!(
                "Initialize {} as a vault?\n\nCreates:\n  .agk/vault.toml\n  skills/\n  instructions/\n  mcps/\n  profiles/",
                vault_name
            );
            modal::render_confirm_modal(
                frame,
                "Init as Vault",
                &msg,
                "[Enter] Confirm  [Esc] Cancel",
            );
        }
        _ => {}
    }
}
