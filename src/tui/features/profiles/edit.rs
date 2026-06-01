use crate::app::features::profile::token_estimate::estimate_tokens;
use crate::domain::profile::ProfileAssetRef;
use crate::tui::app::AppState;
use crate::tui::event::{AppEvent, EventContext};
use crate::tui::list_mode::ListMode;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

/// Initialize the edit-profile modal from the currently selected profile entry.
pub fn enter_edit_profile(state: &mut AppState, ctx: &EventContext) {
    if state.selected_index >= state.profile_entries.len() {
        state.status_line = "No profile selected".to_string();
        return;
    }
    let entry = &state.profile_entries[state.selected_index];

    // Gather available skill names from vault packages.
    let skills: Vec<String> = state
        .packages
        .values()
        .flatten()
        .filter(|p| p.kind == crate::domain::asset::AssetKind::Skill)
        .map(|p| p.identity.name.clone())
        .collect();

    // Determine which skills are currently enabled for this profile.
    let skills_checked: Vec<bool> = skills
        .iter()
        .map(|name| entry.skills.iter().any(|s| s.name == *name))
        .collect();

    // Gather available MCP names.
    let mcps: Vec<String> = state
        .mcp_state
        .servers_list()
        .into_iter()
        .map(|(id, _)| id.clone())
        .collect();

    // Determine which MCPs are currently enabled.
    let mcps_checked: Vec<bool> = mcps
        .iter()
        .map(|name| entry.mcps.iter().any(|m| m.name == *name))
        .collect();

    // Permission modes: resolve from the profile's provider so values match
    // what the provider actually accepts (e.g. "acceptEdits", "auto",
    // "dontAsk", "plan"). Fall back to a sensible default list.
    let permission_modes = ctx
        .core
        .registry
        .get_provider(&entry.provider_id)
        .map(|p| {
            p.available_permission_modes()
                .into_iter()
                .map(|(mode, _label)| mode)
                .collect()
        })
        .unwrap_or_else(|_| {
            vec![
                "default".into(),
                "acceptEdits".into(),
                "auto".into(),
                "dontAsk".into(),
                "plan".into(),
            ]
        });
    let current_pm = state
        .configs
        .get(&state.active_scope)
        .and_then(|cfg| {
            cfg.profiles
                .iter()
                .find(|p| p.name == entry.name)
                .and_then(|p| p.permission_mode.clone())
        })
        .unwrap_or_else(|| "default".to_string());
    let permission_index = permission_modes
        .iter()
        .position(|m| *m == current_pm)
        .unwrap_or(0);

    // Compute estimated tokens from the names of checked skills and MCPs.
    let es_preview = crate::tui::app::EditProfileState {
        profile_name: entry.name.clone(),
        field_index: 0,
        selected: 0,
        skills,
        skills_checked,
        mcps,
        mcps_checked,
        permission_modes,
        permission_index,
        estimated_tokens: 0,
    };
    let estimated_tokens = estimate_tokens(&checked_text_from(&es_preview));

    state.edit_profile_state = Some(crate::tui::app::EditProfileState {
        estimated_tokens,
        ..es_preview
    });
    state.list_mode = ListMode::EditProfile;
    state.status_line.clear();
}

/// Handle key input while the edit-profile modal is active.
pub fn handle_edit_profile_input(
    state: &mut AppState,
    ctx: &EventContext,
    key: &KeyEvent,
) -> Result<()> {
    let code = key.code;
    // Take ownership of the edit state to avoid borrow issues.
    let mut es = match state.edit_profile_state.take() {
        Some(s) => s,
        None => return Ok(()),
    };

    match code {
        KeyCode::Tab => {
            // Cycle to next field (0 -> 1 -> 2 -> 0).
            es.field_index = (es.field_index + 1) % 3;
            es.selected = 0;
        }
        KeyCode::BackTab => {
            // Cycle to previous field.
            es.field_index = (es.field_index + 2) % 3;
            es.selected = 0;
        }
        KeyCode::Esc => {
            state.list_mode = ListMode::Normal;
            state.status_line = "Cancelled profile edit".to_string();
            return Ok(());
        }
        KeyCode::Up if es.selected > 0 => {
            es.selected -= 1;
        }
        KeyCode::Down => {
            let max = current_field_len(&es);
            if es.selected + 1 < max {
                es.selected += 1;
            }
        }
        KeyCode::Char(' ') => {
            toggle_current_item(&mut es);
            recompute_tokens(&mut es);
        }
        KeyCode::Enter => {
            // Save changes.
            save_edit(state, ctx, &es);
            return Ok(());
        }
        _ => {}
    }

    state.edit_profile_state = Some(es);
    Ok(())
}

/// Number of items in the currently active field.
fn current_field_len(es: &crate::tui::app::EditProfileState) -> usize {
    match es.field_index {
        0 => es.skills.len().max(1),
        1 => es.mcps.len().max(1),
        _ => es.permission_modes.len().max(1),
    }
}

/// Toggle the item at `selected` within the active field.
fn toggle_current_item(es: &mut crate::tui::app::EditProfileState) {
    match es.field_index {
        0 => {
            if es.selected < es.skills_checked.len() {
                es.skills_checked[es.selected] = !es.skills_checked[es.selected];
            }
        }
        1 => {
            if es.selected < es.mcps_checked.len() {
                es.mcps_checked[es.selected] = !es.mcps_checked[es.selected];
            }
        }
        _ => {
            // Permission mode: cycle to next mode.
            if !es.permission_modes.is_empty() {
                es.permission_index = (es.permission_index + 1) % es.permission_modes.len();
            }
        }
    }
}

/// Collect the names of currently checked skills and MCPs into a single string.
fn checked_text_from(es: &crate::tui::app::EditProfileState) -> String {
    es.skills
        .iter()
        .zip(es.skills_checked.iter())
        .filter(|(_, &checked)| checked)
        .map(|(name, _)| name.as_str())
        .chain(
            es.mcps
                .iter()
                .zip(es.mcps_checked.iter())
                .filter(|(_, &checked)| checked)
                .map(|(name, _)| name.as_str()),
        )
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Recompute the estimated token count from the currently checked skills and MCPs.
fn recompute_tokens(es: &mut crate::tui::app::EditProfileState) {
    es.estimated_tokens = estimate_tokens(&checked_text_from(es));
}

/// Persist the edited profile back to config and trigger a reload.
fn save_edit(state: &mut AppState, ctx: &EventContext, es: &crate::tui::app::EditProfileState) {
    let mut config = state.active_config().clone();
    let profile_idx = match config
        .profiles
        .iter()
        .position(|p| p.name == es.profile_name)
    {
        Some(i) => i,
        None => {
            state.list_mode = ListMode::Normal;
            state.status_line = format!("Profile '{}' not found", es.profile_name);
            return;
        }
    };

    // Build updated skill / mcp lists.
    let updated_skills: Vec<ProfileAssetRef> = es
        .skills
        .iter()
        .zip(es.skills_checked.iter())
        .filter(|(_, &checked)| checked)
        .map(|(name, _)| ProfileAssetRef::new(name.clone(), "auto"))
        .collect();

    let updated_mcps: Vec<ProfileAssetRef> = es
        .mcps
        .iter()
        .zip(es.mcps_checked.iter())
        .filter(|(_, &checked)| checked)
        .map(|(name, _)| ProfileAssetRef::new(name.clone(), "auto"))
        .collect();

    let permission_mode = es
        .permission_modes
        .get(es.permission_index)
        .cloned()
        .filter(|m| m != "default");

    // Retain fields that the editor does not touch.
    let old = &config.profiles[profile_idx];
    config.profiles[profile_idx] = crate::domain::config::Profile {
        name: old.name.clone(),
        provider_id: old.provider_id.clone(),
        scope: old.scope.clone(),
        skills: updated_skills,
        mcps: updated_mcps,
        instructions: old.instructions.clone(),
        tool_refs: old.tool_refs.clone(),
        permission_mode,
        prompt_overlay_path: old.prompt_overlay_path.clone(),
    };

    let store = ctx.core.store.clone();
    let tx = ctx.tx.clone();
    let scope = state.active_scope;
    let profile_name = es.profile_name.clone();

    tokio::task::spawn_blocking(move || {
        let _ = tx.send(AppEvent::TaskStarted {
            id: 0,
            name: format!("Saving profile '{}'…", profile_name),
        });
        match store.save(scope, &config) {
            Ok(()) => {
                let _ = tx.send(AppEvent::TriggerReload);
            }
            Err(e) => {
                let _ = tx.send(AppEvent::TaskFailed {
                    id: 0,
                    error: format!("Failed to save profile: {}", e),
                });
            }
        }
    });

    state.list_mode = ListMode::Normal;
    state.status_line = format!("Profile '{}' updated", es.profile_name);
}
