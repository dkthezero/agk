use anyhow::Result;
use ratatui::{backend::Backend, Terminal};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::app::outcome::CoreEventSink;
use crate::app::ports::ProcessRunnerPort;
use crate::tui::app::AppState;
use crate::tui::core_event_reducer::apply_core_event;
use crate::tui::event::{handle, AppEvent, ControlFlow, EventContext};
use crate::tui::presenter::TuiPresenter;
use crate::tui::reload::{apply_reload_snapshot, compute_reload_snapshot};

/// Run the TUI event loop until `ControlFlow::Quit` is returned.
///
/// This function is intentionally thin: it matches `AppEvent` variants and
/// delegates the actual handling to feature controllers or the pure reducer.
/// As Phase C proceeds more logic will move out of `handle()` into controllers.
pub async fn run_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
    ctx: &EventContext,
    rx: &mut UnboundedReceiver<AppEvent>,
) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    terminal.draw(|frame| crate::tui::render::draw(frame, state))?;

    while let Some(event) = rx.recv().await {
        match event {
            AppEvent::Input(evt) => match handle(state, ctx, evt)? {
                ControlFlow::Quit => break,
                ControlFlow::Continue => {}
            },
            AppEvent::TaskStarted { id, name } => {
                state.latest_task_id = Some(id);
                state.active_tasks.insert(
                    id,
                    crate::tui::progress::Progress {
                        name,
                        status: crate::tui::progress::ProgressStatus::Starting,
                    },
                );
            }
            AppEvent::TaskProgress { id, percent } => {
                if let Some(task) = state.active_tasks.get_mut(&id) {
                    task.status = crate::tui::progress::ProgressStatus::Running(percent);
                }
            }
            AppEvent::TaskCompleted { id, message } => {
                state.active_tasks.remove(&id);
                state.status_line = message;
            }
            AppEvent::TaskFailed { id, error } => {
                state.active_tasks.remove(&id);
                state.status_line = format!("Error: {}", error);
            }
            AppEvent::TriggerReload => {
                let id = crate::tui::progress::NEXT_TASK_ID
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                state.latest_task_id = Some(id);
                state.active_tasks.insert(
                    id,
                    crate::tui::progress::Progress {
                        name: "Scanning vaults...".to_string(),
                        status: crate::tui::progress::ProgressStatus::Starting,
                    },
                );

                let tx = ctx.tx.clone();
                let active_scope = state.active_scope;
                let ctx2 = EventContext {
                    tx: ctx.tx.clone(),
                    workspace_root: ctx.workspace_root.clone(),
                    file_opener: ctx.file_opener.clone(),
                    core: ctx.core.clone(),
                };
                let mut existing_mcp = state.mcp_state.clone();
                tokio::task::spawn_blocking(move || {
                    let snapshot = compute_reload_snapshot(active_scope, &ctx2, &mut existing_mcp);
                    let _ = tx.send(AppEvent::ReloadComplete(snapshot));
                    let _ = tx.send(AppEvent::TaskCompleted {
                        id,
                        message: "Scanning vaults... Done".to_string(),
                    });
                });
            }
            AppEvent::ClawHubSearchResults { packages, task_id } => {
                state.remote_packages = packages;
                state.active_tasks.remove(&task_id);
                state.clawhub_search_task_id = None;
            }
            AppEvent::Tick => {
                state.scroll_tick = state.scroll_tick.wrapping_add(1);
                if state.scroll_tick.is_multiple_of(2) {
                    state.scroll_offset = state.scroll_offset.wrapping_add(1);
                }

                // Underground hang detection — query TaskTrackerPort every tick.
                let hung = ctx
                    .core
                    .task_tracker
                    .detect_hung(std::time::Duration::from_secs(30));
                let hung_ids: std::collections::HashSet<usize> = hung
                    .iter()
                    .filter_map(|t| t.id.strip_prefix("task-").and_then(|s| s.parse().ok()))
                    .collect();

                for task in &hung {
                    if let Some(id) = task.id.strip_prefix("task-").and_then(|s| s.parse().ok()) {
                        if !state.hung_warnings_shown.contains(&id) {
                            let elapsed = task
                                .started_at
                                .map(|s| s.elapsed().as_secs())
                                .unwrap_or_else(|| task.created_at.elapsed().as_secs());
                            let _ = ctx.tx.send(AppEvent::CoreEvent(
                                crate::app::event::CoreEvent::TaskHungWarning {
                                    id,
                                    name: task.name.clone(),
                                    elapsed_sec: elapsed,
                                },
                            ));
                            state.hung_warnings_shown.insert(id);
                        }
                    }
                }
                // Remove ids that are no longer hung so they can be re-warned if they hang again.
                state.hung_warnings_shown.retain(|id| hung_ids.contains(id));
            }
            AppEvent::RunInteractiveProcess {
                command,
                args,
                current_dir,
                profile_name,
            } => {
                handle_interactive_process(
                    terminal,
                    state,
                    rx,
                    ctx.core.process_runner.clone(),
                    &command,
                    &args,
                    &current_dir,
                    profile_name,
                )
                .await?;
            }
            AppEvent::ReloadComplete(snapshot) => {
                apply_reload_snapshot(state, snapshot);
                // Config is now fresh; let the list transition from spinner
                // to [✓] / [ ] without flashing an intermediate empty state.
                state.installing_names.clear();
            }
            AppEvent::ExecuteCommand(cmd) => {
                let core = ctx.core.clone();
                let tx = ctx.tx.clone();
                tokio::task::spawn_blocking(move || {
                    let mut presenter = TuiPresenter::new(tx);
                    if let Err(e) = core.execute(cmd, &mut presenter) {
                        presenter.on_error(format!("{}", e));
                    }
                });
            }
            AppEvent::CoreEvent(evt) => {
                apply_core_event(state, &evt);
                // Asset mutations update the on-disk config; reload so the
                // UI reflects installed / removed / updated state.
                if matches!(
                    &evt,
                    crate::app::event::CoreEvent::AssetInstalled { .. }
                        | crate::app::event::CoreEvent::AssetRemoved { .. }
                        | crate::app::event::CoreEvent::AssetUpdated { .. }
                        | crate::app::event::CoreEvent::SyncComplete { .. }
                        | crate::app::event::CoreEvent::VaultAttached(_)
                        | crate::app::event::CoreEvent::VaultDetached(_)
                        | crate::app::event::CoreEvent::VaultInitialized(_)
                        | crate::app::event::CoreEvent::ProviderActivated(_)
                        | crate::app::event::CoreEvent::ProviderDeactivated(_)
                        | crate::app::event::CoreEvent::TeamInitialized(_)
                        | crate::app::event::CoreEvent::TeamVaultAdded(_)
                        | crate::app::event::CoreEvent::TeamRequirementAdded(_)
                        | crate::app::event::CoreEvent::TeamRequirementRemoved(_)
                        | crate::app::event::CoreEvent::TeamSyncComplete { .. }
                ) {
                    let _ = ctx.tx.send(AppEvent::TriggerReload);
                }
            }
        }
        terminal.draw(|frame| crate::tui::render::draw(frame, state))?;
    }
    Ok(())
}
/// Suspend the TUI, run an interactive child process, then resume.
#[allow(clippy::too_many_arguments)]
async fn handle_interactive_process<B: Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
    rx: &mut UnboundedReceiver<AppEvent>,
    runner: Arc<dyn ProcessRunnerPort>,
    command: &str,
    args: &[String],
    current_dir: &std::path::Path,
    profile_name: Option<String>,
) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    use crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use std::io::{self, Write};

    let mut out = io::stdout();
    let _ = execute!(out, LeaveAlternateScreen);
    let _ = out.flush();
    let _ = terminal.show_cursor();
    let _ = disable_raw_mode();

    // Run the blocking .status() call on a dedicated thread so the async
    // event loop is never frozen. The runner port owns the actual
    // std::process::Command call so this file stays free of process-spawn
    // primitives (ADR-001).
    let cmd = command.to_string();
    let args = args.to_vec();
    let current_dir = current_dir.to_path_buf();
    let args_for_status = args.clone();
    let current_dir_for_status = current_dir.clone();
    let status = tokio::task::spawn_blocking(move || {
        runner.run_interactive(&cmd, &args_for_status, &current_dir_for_status)
    })
    .await
    .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking panicked: {}", e)));

    let _ = enable_raw_mode();

    // Drain any crossterm events buffered while the child had stdin
    while let Ok(AppEvent::Input(_)) = rx.try_recv() {}

    let _ = execute!(out, EnterAlternateScreen);
    let _ = out.flush();
    let _ = terminal.hide_cursor();
    let _ = terminal.clear();

    match &status {
        Ok(s) if s.success() => {
            state.status_line = format!("Finished: {} {}", command, args.join(" "));

            // After opencode agent create succeeds, verify the generated agent
            // file exists.  opencode writes into <--path>/agents/<name>.md.
            if command == "opencode" && args.iter().any(|a| a == "create") {
                if let Some(name) = profile_name {
                    let profile_dir = current_dir.join(".agk").join("profiles").join(&name);
                    let agents_dir = profile_dir.join("agents");
                    let found = std::fs::read_dir(&agents_dir).ok().and_then(|entries| {
                        entries
                            .flatten()
                            .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
                            .map(|e| e.path())
                    });
                    match found {
                        Some(path) => {
                            // Also copy to the legacy agent.md path so older code
                            // and the profile_agent_path fallback both resolve.
                            let dest = profile_dir.join("agent.md");
                            let _ = std::fs::copy(&path, &dest);
                            state.status_line = format!("Profile '{}' created successfully", name);
                        }
                        None => {
                            state.status_line = format!(
                                "Profile '{}' created, but no agent .md found in {}",
                                name,
                                agents_dir.display()
                            );
                        }
                    }
                }
            }
        }
        Ok(s) => {
            state.status_line = format!(
                "'{}' exited with status {}",
                command,
                s.code().unwrap_or(-1)
            );
        }
        Err(e) => {
            state.status_line = format!("Failed to run '{}': {}", command, e);
        }
    }

    Ok(())
}
