use crate::tui::app::AppState;
use crate::tui::event::EventContext;

pub fn apply_tab_switch(state: &mut AppState, idx: usize, tab_count: usize) {
    if idx < tab_count {
        state.active_tab = idx;
        state.selected_index = 0;
        state.search_query.clear();
        state.status_line.clear();
        state.list_mode = crate::tui::list_mode::ListMode::Normal;
    }
}

pub fn apply_search_char(state: &mut AppState, c: char) {
    state.search_query.push(c);
    state.list_mode = crate::tui::list_mode::ListMode::Searching;
    state.selected_index = 0;
    state.status_line.clear();
}

pub fn apply_scope_toggle(state: &mut AppState) {
    state.toggle_scope();
    state.status_line = format!("Scope: {}", state.scope_label());
}

pub fn apply_space_no_provider(state: &mut AppState, providers_tab_idx: usize) {
    apply_tab_switch(state, providers_tab_idx, state.tab_names.len());
    state.status_line = "No provider configured \u{2014} please select one".to_string();
}

pub fn apply_enter_attach_vault(state: &mut AppState) {
    state.list_mode = crate::tui::list_mode::ListMode::AttachVault;
    state.prompt_buffer = String::new();
    state.status_line.clear();
}

pub fn apply_enter_register_mcp(state: &mut AppState) {
    state.list_mode = crate::tui::list_mode::ListMode::RegisterMcpStepName;
    state.prompt_buffer = String::new();
    state.pending_mcp_name.clear();
    state.pending_mcp_command.clear();
    state.pending_mcp_args.clear();
    state.pending_mcp_transport = "stdio".to_string();
    state.pending_mcp_description.clear();
    state.status_line.clear();
}

pub fn apply_enter_add_profile(state: &mut AppState, ctx: &EventContext) {
    let (provider_id, steps) = if ctx.core.store.load(state.active_scope).is_ok() {
        ctx.core
            .registry
            .providers
            .iter()
            .find(|p| p.supports_profiles())
            .map(|p| {
                let id = p.id().to_string();
                let steps = p.profile_wizard_steps();
                (id, steps)
            })
            .unwrap_or_else(|| ("opencode".to_string(), vec![]))
    } else {
        ("opencode".to_string(), vec![])
    };

    let mut ws = crate::app::ports::WizardState::new(steps, provider_id);

    let skill_names: Vec<String> = state
        .packages
        .values()
        .flatten()
        .filter(|p| p.kind == crate::domain::asset::AssetKind::Skill)
        .map(|p| p.identity.name.clone())
        .collect();
    ws.skill_options = skill_names.clone();

    let mcp_names: Vec<String> = state
        .mcp_state
        .servers_list()
        .into_iter()
        .map(|(id, _)| id.clone())
        .collect();
    ws.mcp_options = mcp_names.clone();

    for step in &mut ws.steps {
        if let crate::app::ports::WizardStep::Checklist {
            title,
            ref mut options,
        } = step
        {
            if title.to_lowercase().contains("skill") && options.is_empty() {
                *options = skill_names.clone();
            } else if title.to_lowercase().contains("mcp") && options.is_empty() {
                *options = mcp_names.clone();
            }
        }
    }

    if let Some(crate::app::ports::WizardStep::Checklist { options, .. }) = ws.steps.first() {
        ws.checked = vec![false; options.len()];
    }

    state.wizard_state = Some(ws);
    state.list_mode = crate::tui::list_mode::ListMode::ProfileWizard;
    state.prompt_buffer = String::new();
    state.status_line = "Profile name: ".to_string();
}

pub fn parse_github_url(url: &str) -> Option<(String, String)> {
    let url = url.trim();
    let url = url.strip_suffix(".git").unwrap_or(url);

    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("github.com/"));

    if let Some(path) = path {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            let repo = format!("{}/{}", parts[0], parts[1]);
            let id = parts[1].to_string();
            return Some((id, repo));
        }
    }
    None
}

pub fn active_providers<'a>(
    registry: &'a crate::app::registry::Registry,
    config: &crate::domain::config::ConfigFile,
) -> Vec<&'a dyn crate::app::ports::ProviderPort> {
    registry
        .providers
        .iter()
        .filter(|p| config.providers.contains(&p.id().to_string()))
        .map(|p| p.as_ref())
        .collect()
}

use crate::tui::event::AppEvent;
use std::sync::Arc;

pub async fn refresh_all_vaults(
    registry: Arc<crate::app::registry::Registry>,
    tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
    message_prefix: &str,
) -> anyhow::Result<()> {
    let id = crate::tui::progress::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let _ = tx.send(AppEvent::TaskStarted {
        id,
        name: format!("{}refreshing vaults...", message_prefix),
    });
    let mut errs = Vec::new();
    let total = registry.vaults.len();
    for (i, vault) in registry.vaults.iter().enumerate() {
        if let Err(e) = vault.refresh().await {
            errs.push(format!("{}: {}", vault.id(), e));
        }
        let percent = (((i + 1) as f32 / total.max(1) as f32) * 100.0) as u8;
        let _ = tx.send(AppEvent::TaskProgress { id, percent });
    }
    let _ = tx.send(AppEvent::TriggerReload);
    if errs.is_empty() {
        let _ = tx.send(AppEvent::TaskCompleted {
            id,
            message: format!("{}refreshed successfully", message_prefix),
        });
    } else {
        let _ = tx.send(AppEvent::TaskFailed {
            id,
            error: format!("{}refresh issues: {}", message_prefix, errs.join(", ")),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::list_mode::ListMode;
    use std::collections::HashMap;

    fn empty_state(tab_count: usize) -> AppState {
        AppState::new(
            (0..tab_count).map(|i| format!("Tab{}", i)).collect(),
            vec![true; tab_count],
            HashMap::new(),
        )
    }

    #[test]
    fn switch_tab_updates_active_tab() {
        let mut state = empty_state(4);
        apply_tab_switch(&mut state, 2, 4);
        assert_eq!(state.active_tab, 2);
    }

    #[test]
    fn switch_tab_resets_selection_and_search() {
        let mut state = empty_state(4);
        state.selected_index = 3;
        state.search_query = "foo".to_string();
        apply_tab_switch(&mut state, 1, 4);
        assert_eq!(state.selected_index, 0);
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn switch_tab_ignores_out_of_range() {
        let mut state = empty_state(4);
        apply_tab_switch(&mut state, 9, 4);
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn search_query_appends_char() {
        let mut state = empty_state(1);
        apply_search_char(&mut state, 'a');
        apply_search_char(&mut state, 'b');
        assert_eq!(state.search_query, "ab");
        assert_eq!(state.list_mode, ListMode::Searching);
    }

    #[test]
    fn esc_clears_search() {
        let mut state = empty_state(1);
        state.search_query = "hello".to_string();
        state.list_mode = ListMode::Searching;
        crate::tui::features::common::controller::apply_esc(&mut state);
        assert!(state.search_query.is_empty());
        assert_eq!(state.list_mode, ListMode::Normal);
    }

    #[test]
    fn space_redirects_to_providers_tab_when_no_provider() {
        let mut state = empty_state(5);
        apply_space_no_provider(&mut state, 3); // Providers is now tab 3
        assert_eq!(state.active_tab, 3);
        assert!(!state.status_line.is_empty());
    }

    #[test]
    fn a_key_on_vaults_tab_enters_attach_mode() {
        let mut state = empty_state(5);
        state.active_tab = 0; // Vaults tab is now index 0
        apply_enter_attach_vault(&mut state);
        assert_eq!(state.list_mode, ListMode::AttachVault);
    }

    #[test]
    fn s_key_toggles_scope() {
        let mut state = empty_state(4);
        use crate::domain::scope::Scope;
        assert_eq!(state.active_scope, Scope::Workspace);
        apply_scope_toggle(&mut state);
        assert_eq!(state.active_scope, Scope::Global);
    }

    #[test]
    fn parse_github_url_works() {
        assert_eq!(
            parse_github_url("https://github.com/obra/superpowers"),
            Some(("superpowers".to_string(), "obra/superpowers".to_string()))
        );
        assert_eq!(
            parse_github_url("https://github.com/obra/superpowers.git"),
            Some(("superpowers".to_string(), "obra/superpowers".to_string()))
        );
        assert_eq!(
            parse_github_url("github.com/obra/superpowers"),
            Some(("superpowers".to_string(), "obra/superpowers".to_string()))
        );
        assert_eq!(
            parse_github_url("https://github.com/obra/superpowers/tree/main"),
            Some(("superpowers".to_string(), "obra/superpowers".to_string()))
        );
        assert!(parse_github_url("/local/path").is_none());
    }
}
