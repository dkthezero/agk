use crate::tui::app::{AppState, ListMode};
use crate::tui::layout;
use crate::tui::widgets::{detail, list, mcp, modal, status, tabs};
use ratatui::{
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Paragraph,
    Frame,
};

pub fn draw(frame: &mut Frame, state: &AppState) {
    let layout = layout::compute(frame.area());

    // Header
    let search_hint = if state.search_query.is_empty() {
        String::new()
    } else {
        format!("  [ Search: {} ]", state.search_query)
    };
    let header_text = format!("agk v0.2.6{}", search_hint);
    frame.render_widget(
        Paragraph::new(Line::from(header_text)).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        layout.header,
    );

    // Tab bar
    tabs::render(frame, layout.tabs, &state.tab_names, state.active_tab);

    // Content — dispatch by tab kind
    use crate::tui::app::TabKind;

    let is_live = state.is_active_tab_live();
    let active_kind = state
        .tab_kinds
        .get(state.active_tab)
        .cloned()
        .unwrap_or(TabKind::Asset);

    match active_kind {
        TabKind::Asset => {
            let filtered = state.filtered_packages();
            let selected_pkg = filtered.get(state.selected_index).copied();
            list::render(
                frame,
                layout.list,
                &filtered,
                state.selected_index,
                !is_live,
                state.active_config(),
                state.scroll_offset,
            );
            detail::render(
                frame,
                layout.detail,
                selected_pkg,
                !is_live,
                &state.vault_entries,
            );
        }
        TabKind::Vault => {
            list::render_vaults(
                frame,
                layout.list,
                &state.vault_entries,
                state.selected_index,
            );
            let selected_vault = state.vault_entries.get(state.selected_index);
            detail::render_vault_detail(frame, layout.detail, selected_vault);
        }
        TabKind::Provider => {
            list::render_providers(
                frame,
                layout.list,
                &state.provider_entries,
                state.selected_index,
            );
            let selected_provider = state.provider_entries.get(state.selected_index);
            detail::render_provider_detail(
                frame,
                layout.detail,
                selected_provider,
                state.active_scope,
            );
        }
        TabKind::Mcp => {
            let list_area = layout.list;
            let detail_area = layout.detail;
            // Build active provider list for checkbox rendering
            let active_providers: Vec<crate::domain::asset::ProviderEntry> = state
                .provider_entries
                .iter()
                .filter(|p| p.active)
                .cloned()
                .collect();
            mcp::render::render(
                frame,
                list_area,
                &state.mcp_state,
                state.selected_index,
                state.active_scope,
                &active_providers,
            );
            mcp::render::render_detail(frame, detail_area, &state.mcp_state, state.selected_index);
        }
        TabKind::Profile => {
            list::render_profiles(
                frame,
                layout.list,
                &state.profile_entries,
                state.selected_index,
            );
            let selected_profile = state.profile_entries.get(state.selected_index);
            detail::render_profile_detail(frame, layout.detail, selected_profile);
        }
        TabKind::Analytics => {
            // Telemetry tab is hidden from the UI but the data structure still exists
            // so the match stays exhaustive. Nothing renders.
        }
    }
    let keybinds = if matches!(state.list_mode, ListMode::SelectProviderRoot { .. }) {
        "[↑/↓] Move  [Enter] Confirm  [Esc] Cancel"
    } else if state.is_attach_vault_mode() || state.is_register_mcp_mode() {
        "[Enter] Confirm  [Esc] Cancel"
    } else if matches!(
        state.list_mode,
        ListMode::ConfirmMcpTest
            | ListMode::ConfirmClawHubInstall
            | ListMode::ConfirmDetachVault
            | ListMode::ConfirmDeactivateLastProvider
    ) {
        ""
    } else {
        match active_kind {
            TabKind::Asset => {
                "[↑/↓] Move  [Space] Toggle  [Enter] Update  [F5] Update All  [F4] Refresh  [type] Search  [Esc]x2 Quit"
            }
            TabKind::Provider => {
                "[↑/↓] Move  [Space] Toggle  [Enter] Update  [F4] Refresh  [Esc]x2 Quit"
            }
            TabKind::Mcp => {
                "[↑/↓] Move  [F2] Add MCP  [Space] Enable  [Enter] Test  [Esc]x2 Quit"
            }
            TabKind::Vault => {
                "[↑/↓] Move  [F2] Attach New  [Space] Toggle  [F4] Refresh  [Esc]x2 Quit"
            }
            TabKind::Profile => {
                "[↑/↓] Move  [F2] Add Profile  [Delete] Remove  [Esc]x2 Quit"
            }
            TabKind::Analytics => "",
        }
    };

    status::render(
        frame,
        layout.footer,
        &state.status_line,
        &state.search_query,
        keybinds,
        state.scope_label(),
        state.progress_summary().as_deref(),
    );

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
            );
        }
        ListMode::AttachVaultBranch => {
            modal::render_input_modal(
                frame,
                "Attach Vault",
                "Branch (default: main):",
                &state.prompt_buffer,
            );
        }
        ListMode::AttachVaultPath => {
            modal::render_input_modal(
                frame,
                "Attach Vault",
                "Subfolder (default: skills/):",
                &state.prompt_buffer,
            );
        }
        ListMode::AttachVaultName => {
            modal::render_input_modal(frame, "Attach Vault", "Vault name:", &state.prompt_buffer);
        }
        ListMode::RegisterMcpStepName => {
            modal::render_input_modal(frame, "Register MCP Server", "Name:", &state.prompt_buffer);
        }
        ListMode::RegisterMcpStepCommand => {
            modal::render_input_modal(
                frame,
                "Register MCP Server",
                "Command to run (e.g. npx, python):",
                &state.prompt_buffer,
            );
        }
        ListMode::RegisterMcpStepArgs => {
            modal::render_input_modal(
                frame,
                "Register MCP Server",
                "Arguments (space-separated, optional):",
                &state.prompt_buffer,
            );
        }
        ListMode::RegisterMcpStepTransport => {
            modal::render_input_modal(
                frame,
                "Register MCP Server",
                "Transport (stdio/sse), default stdio:",
                &state.prompt_buffer,
            );
        }
        ListMode::RegisterMcpStepDescription => {
            modal::render_input_modal(
                frame,
                "Register MCP Server",
                "Description (optional):",
                &state.prompt_buffer,
            );
        }
        ListMode::ProfileWizardName => {
            modal::render_input_modal(
                frame,
                "Add Profile",
                "Profile name (e.g. opencode-dev):",
                &state.prompt_buffer,
            );
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
