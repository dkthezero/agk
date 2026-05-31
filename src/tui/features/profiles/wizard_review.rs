use crate::domain::profile::validate_profile_id;
use crate::tui::app::AppState;
use crate::tui::event::{AppEvent, EventContext};
use crate::tui::list_mode::ListMode;
use crossterm::event::KeyCode;

/// Map provider_id to the CLI command used to create a profile session.
/// Only providers that support interactive profile creation are listed.
fn provider_command(provider_id: &str) -> &'static str {
    match provider_id {
        "claude-code" => "claude",
        _ => "opencode",
    }
}

pub fn handle_review_step(
    state: &mut AppState,
    ctx: &EventContext,
    key: &crossterm::event::KeyEvent,
) {
    let ws = match state.wizard_state.as_mut() {
        Some(s) => s,
        None => return,
    };
    match key.code {
        KeyCode::Enter => {
            let name = ws.name.clone();
            // Validate profile name before using it in filesystem paths.
            if let Err(e) = validate_profile_id(&crate::domain::profile::ProfileId::new(&name)) {
                state.wizard_state = None;
                state.list_mode = ListMode::Normal;
                state.status_line = format!("Invalid profile name: {}", e);
                return;
            }
            let skills = ws.skills.clone();
            let mcps = ws.mcps.clone();
            let provider_id = ws.provider_id.clone();
            let desc = ws.composed_description();
            let selected_tools = ws.selected_tools.clone();
            let permission_mode = ws.selected_permission_mode.clone();
            let scope = ws.scope.unwrap_or(state.active_scope);
            state.wizard_state = None;
            state.list_mode = ListMode::Normal;
            state.status_line.clear();

            let mut new_config = state.active_config().clone();
            new_config.profiles.push(crate::domain::config::Profile {
                name: name.clone(),
                provider_id,
                scope: scope.to_string().to_lowercase(),
                skills: skills
                    .iter()
                    .map(|s| crate::domain::profile::ProfileAssetRef::new(s.clone(), "auto"))
                    .collect(),
                mcps: mcps
                    .iter()
                    .map(|m| crate::domain::profile::ProfileAssetRef::new(m.clone(), "auto"))
                    .collect(),
                instructions: vec![],
                tool_refs: selected_tools,
                permission_mode,
                prompt_overlay_path: None,
            });

            let store = ctx.core.store.clone();
            let tx = ctx.tx.clone();
            let id = crate::tui::progress::NEXT_TASK_ID
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let workspace_root = ctx.workspace_root.clone();
            let provider_cmd = provider_command(&new_config.profiles.last().unwrap().provider_id);
            let profile_dir = workspace_root.join(".agk").join("profiles").join(&name);
            let _ = std::fs::create_dir_all(&profile_dir);

            tokio::task::spawn_blocking(move || {
                let _ = tx.send(AppEvent::TaskStarted {
                    id,
                    name: format!("Creating profile '{}'", name),
                });
                match store.save(scope, &new_config) {
                    Ok(()) => {
                        let _ = tx.send(AppEvent::TriggerReload);
                        let _ = tx.send(AppEvent::RunInteractiveProcess {
                            command: provider_cmd.into(),
                            args: vec![
                                "agent".into(),
                                "create".into(),
                                "--path".into(),
                                profile_dir.display().to_string(),
                                "--mode".into(),
                                "primary".into(),
                                "--description".into(),
                                desc,
                            ],
                            current_dir: workspace_root,
                            profile_name: Some(name),
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::TaskFailed {
                            id,
                            error: format!("Failed to save profile: {}", e),
                        });
                    }
                }
            });
        }
        KeyCode::Esc => {
            if ws.step_index > 0 {
                ws.step_index -= 1;
            } else {
                state.wizard_state = None;
                state.list_mode = ListMode::Normal;
                state.status_line = "Cancelled profile creation".to_string();
            }
        }
        KeyCode::Up if ws.scroll_offset > 0 => {
            ws.scroll_offset -= 1;
        }
        KeyCode::Down => {
            ws.scroll_offset = ws.scroll_offset.saturating_add(1);
        }
        _ => {}
    }
}
