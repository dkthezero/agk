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
            }
        }
        terminal.draw(|frame| crate::tui::render::draw(frame, state))?;
    }
    Ok(())
}

/// Suspend the TUI, run an interactive child process, then resume.
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

            // After opencode agent create succeeds, move the generated agent file into
            // .agk/profiles/<name>/agent.md
            if command == "opencode" && args.iter().any(|a| a == "create") {
                if let Some(name) = profile_name {
                    let target_dir: std::path::PathBuf =
                        current_dir.join(".agk").join("profiles").join(&name);
                    let agents_dir = current_dir.join(".opencode").join("agents");
                    let agents_subdir = agents_dir.join("agents");

                    let mut moved = false;
                    // Try .opencode/agents/<name>.md first (direct mode)
                    let direct = agents_dir.join(format!("{}.md", name));
                    if direct.exists() {
                        if let Err(e) = std::fs::create_dir_all(&target_dir) {
                            state.status_line = format!("Failed to create profile dir: {}", e);
                        } else {
                            let dest = target_dir.join("agent.md");
                            if let Err(e) = std::fs::rename(&direct, &dest) {
                                state.status_line = format!("Failed to move agent file: {}", e);
                            } else {
                                state.status_line =
                                    format!("Profile '{}' created successfully", name);
                                moved = true;
                            }
                        }
                    }

                    // Fallback: look in .opencode/agents/agents/<name>.md
                    if !moved {
                        let nested = agents_subdir.join(format!("{}.md", name));
                        if nested.exists() {
                            if let Err(e) = std::fs::create_dir_all(&target_dir) {
                                state.status_line = format!("Failed to create profile dir: {}", e);
                            } else {
                                let dest = target_dir.join("agent.md");
                                if let Err(e) = std::fs::rename(&nested, &dest) {
                                    state.status_line = format!("Failed to move agent file: {}", e);
                                } else {
                                    state.status_line =
                                        format!("Profile '{}' created successfully", name);
                                    moved = true;
                                }
                            }
                        }
                    }

                    // Last resort: scan for newest .md in agents dir
                    if !moved {
                        if let Ok(mut entries) = std::fs::read_dir(&agents_dir) {
                            let now = std::time::SystemTime::now();
                            let five_secs = std::time::Duration::from_secs(5);
                            let mut newest: Option<(std::path::PathBuf, std::fs::Metadata)> = None;

                            while let Some(Ok(entry)) = entries.next() {
                                let path = entry.path();
                                if let Some("md") = path.extension().and_then(|s| s.to_str()) {
                                    if let Ok(meta) = entry.metadata() {
                                        if let Ok(modified) = meta.modified() {
                                            if now.duration_since(modified).unwrap_or(five_secs)
                                                <= five_secs
                                            {
                                                let newer = match &newest {
                                                    None => true,
                                                    Some((_, prev_meta)) => {
                                                        match prev_meta.modified() {
                                                            Ok(prev) => modified > prev,
                                                            Err(_) => true,
                                                        }
                                                    }
                                                };
                                                if newer {
                                                    newest = Some((path, meta));
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if let Some((agent_md, _)) = newest {
                                if let Err(e) = std::fs::create_dir_all(&target_dir) {
                                    state.status_line =
                                        format!("Failed to create profile dir: {}", e);
                                } else {
                                    let dest = target_dir.join("agent.md");
                                    if let Err(e) = std::fs::rename(&agent_md, &dest) {
                                        state.status_line =
                                            format!("Failed to move agent file: {}", e);
                                    } else {
                                        state.status_line =
                                            format!("Profile '{}' created successfully", name);
                                        moved = true;
                                    }
                                }
                            }
                        }
                    }

                    if !moved {
                        state.status_line = format!(
                            "Profile '{}' created, but no agent.md found in agents/",
                            name
                        );
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
