use crate::app::tab_kind::TabKind;
use crate::tui::app::AppState;
use crate::tui::widgets::{detail, list, mcp};
use ratatui::{layout::Rect, Frame};

pub fn draw_content(frame: &mut Frame, state: &AppState, list_area: Rect, detail_area: Rect) {
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
                list_area,
                &filtered,
                state.selected_index,
                !is_live,
                state.active_config(),
                state.scroll_offset,
                &state.installing_names,
            );
            detail::render(
                frame,
                detail_area,
                selected_pkg,
                !is_live,
                &state.vault_entries,
            );
        }
        TabKind::Vault => {
            list::render_vaults(frame, list_area, &state.vault_entries, state.selected_index);
            let selected_vault = state.vault_entries.get(state.selected_index);
            detail::render_vault_detail(frame, detail_area, selected_vault);
        }
        TabKind::Provider => {
            list::render_providers(
                frame,
                list_area,
                &state.provider_entries,
                state.selected_index,
            );
            let selected_provider = state.provider_entries.get(state.selected_index);
            detail::render_provider_detail(
                frame,
                detail_area,
                selected_provider,
                state.active_scope,
            );
        }
        TabKind::Mcp => {
            let active_providers: Vec<crate::app::snapshot::ProviderEntry> = state
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
                list_area,
                &state.profile_entries,
                state.selected_index,
            );
            let selected_profile = state.profile_entries.get(state.selected_index);
            detail::render_profile_detail(frame, detail_area, selected_profile);
        }
        TabKind::Analytics => {
            // Telemetry tab is hidden from the UI but the data structure still exists
            // so the match stays exhaustive. Nothing renders.
        }
    }
}
