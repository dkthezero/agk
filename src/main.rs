mod app;
mod cli;
mod domain;
mod infra;
mod tui;

use anyhow::Result;
use app::ports::ConfigStorePort;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::entry::parse();
    let workspace = std::env::current_dir()?;

    // If a subcommand other than Clean is provided, run headless CLI
    if cli.command.is_some() {
        let code = cli::commands::run(cli, &workspace)?;
        if code != cli::commands::EXIT_SUCCESS {
            std::process::exit(code);
        }
        return Ok(());
    }

    // No subcommand — launch TUI
    let (registry, scan, store) = app::bootstrap::build(workspace)?;

    let workspace_for_ctx = std::env::current_dir()?;

    let tab_names: Vec<String> = registry
        .feature_sets
        .iter()
        .map(|f| f.display_name().to_string())
        .collect();

    let tab_live: Vec<bool> = registry.feature_sets.iter().map(|f| !f.is_stub()).collect();

    // Build display entries before consuming scan data
    let global_config = store.load(domain::scope::Scope::Global).unwrap_or_default();
    let active_config_for_entries = store
        .load(domain::scope::Scope::Workspace)
        .unwrap_or_default();
    let vault_entries = app::bootstrap::build_vault_entries(
        &global_config,
        &active_config_for_entries,
        &scan,
        &registry,
        &workspace_for_ctx,
    );
    let provider_entries =
        app::bootstrap::build_provider_entries(&active_config_for_entries, &registry);
    let profile_entries = app::bootstrap::build_profile_entries(&active_config_for_entries);
    let tab_kinds = app::bootstrap::build_tab_kinds(&registry);

    let packages: HashMap<usize, Vec<_>> = scan.packages_by_tab.into_iter().enumerate().collect();

    let mut state = tui::app::AppState::new(tab_names, tab_live, packages);
    state.tab_kinds = tab_kinds;
    state.vault_entries = vault_entries;
    state.provider_entries = provider_entries;
    state.profile_entries = profile_entries;

    // Load both scope configs into AppState
    if let Ok(global_config) = store.load(domain::scope::Scope::Global) {
        state
            .configs
            .insert(domain::scope::Scope::Global, global_config);
    }
    if let Ok(workspace_config) = store.load(domain::scope::Scope::Workspace) {
        state
            .configs
            .insert(domain::scope::Scope::Workspace, workspace_config);
    }

    // Wrap in Arc for background tasks
    let registry = Arc::new(registry);
    let store = Arc::new(store) as Arc<dyn ConfigStorePort>;

    let (tx, mut rx) = mpsc::unbounded_channel::<tui::event::AppEvent>();

    // Input thread
    let tx_in = tx.clone();
    tokio::spawn(async move {
        let mut reader = crossterm::event::EventStream::new();
        while let Some(Ok(evt)) = reader.next().await {
            let _ = tx_in.send(tui::event::AppEvent::Input(evt));
        }
    });

    // Tick timer for scroll animation
    let tx_tick = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(300));
        loop {
            interval.tick().await;
            if tx_tick.send(tui::event::AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    let ctx = tui::event::EventContext {
        store,
        registry,
        tx,
        workspace_root: workspace_for_ctx,
    };

    // Auto-pull on boot
    let tx_boot = ctx.tx.clone();
    let registry_boot = ctx.registry.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let _ = tui::event::refresh_all_vaults(registry_boot, tx_boot, "Auto-").await;
    });

    // Terminal setup
    enable_raw_mode()?;
    let setup_result = async {
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = run_loop(&mut terminal, &mut state, &ctx, &mut rx).await;

        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        result
    }
    .await;
    disable_raw_mode()?;

    let code = if setup_result.is_ok() { 0 } else { 1 };
    std::process::exit(code);
}

async fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut tui::app::AppState,
    ctx: &tui::event::EventContext,
    rx: &mut mpsc::UnboundedReceiver<tui::event::AppEvent>,
) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    terminal.draw(|frame| tui::render::draw(frame, state))?;

    while let Some(event) = rx.recv().await {
        match event {
            tui::event::AppEvent::Input(evt) => match tui::event::handle(state, ctx, evt)? {
                tui::event::ControlFlow::Quit => break,
                tui::event::ControlFlow::Continue => {}
            },
            tui::event::AppEvent::TaskStarted { id, name } => {
                state.latest_task_id = Some(id);
                state.active_tasks.insert(
                    id,
                    crate::tui::app::Progress {
                        name,
                        status: crate::tui::app::ProgressStatus::Starting,
                    },
                );
            }
            tui::event::AppEvent::TaskProgress { id, percent } => {
                if let Some(task) = state.active_tasks.get_mut(&id) {
                    task.status = crate::tui::app::ProgressStatus::Running(percent);
                }
            }
            tui::event::AppEvent::TaskCompleted { id, message } => {
                state.active_tasks.remove(&id);
                state.status_line = message;
            }
            tui::event::AppEvent::TaskFailed { id, error } => {
                state.active_tasks.remove(&id);
                state.status_line = format!("Error: {}", error);
            }
            tui::event::AppEvent::TriggerReload => {
                let active_config_for_entries =
                    ctx.store.load(state.active_scope).unwrap_or_default();
                let global_config = ctx
                    .store
                    .load(crate::domain::scope::Scope::Global)
                    .unwrap_or_default();
                let workspace_config = ctx
                    .store
                    .load(crate::domain::scope::Scope::Workspace)
                    .unwrap_or_default();

                let active_vaults =
                    crate::app::bootstrap::build_vaults(&global_config, &ctx.workspace_root);

                if let Ok(mut scan) = crate::app::bootstrap::scan(&ctx.registry, &active_vaults) {
                    let opt_workspace_config =
                        if state.active_scope == crate::domain::scope::Scope::Workspace {
                            Some(&workspace_config)
                        } else {
                            None
                        };
                    crate::app::bootstrap::filter_scan(
                        &mut scan,
                        &global_config,
                        opt_workspace_config,
                    );
                    state.vault_entries = crate::app::bootstrap::build_vault_entries(
                        &global_config,
                        &active_config_for_entries,
                        &scan,
                        &ctx.registry,
                        &ctx.workspace_root,
                    );
                    state.provider_entries = crate::app::bootstrap::build_provider_entries(
                        &active_config_for_entries,
                        &ctx.registry,
                    );
                    state.profile_entries =
                        crate::app::bootstrap::build_profile_entries(&active_config_for_entries);
                    state.packages = scan.packages_by_tab.into_iter().enumerate().collect();
                }

                state
                    .configs
                    .insert(crate::domain::scope::Scope::Global, global_config);
                state
                    .configs
                    .insert(crate::domain::scope::Scope::Workspace, workspace_config);

                state.mcp_state.refresh();
            }
            tui::event::AppEvent::ClawHubSearchResults { packages, task_id } => {
                state.remote_packages = packages;
                state.active_tasks.remove(&task_id);
                state.clawhub_search_task_id = None;
            }
            tui::event::AppEvent::Tick => {
                state.scroll_tick = state.scroll_tick.wrapping_add(1);
                if state.scroll_tick.is_multiple_of(2) {
                    state.scroll_offset = state.scroll_offset.wrapping_add(1);
                }
            }
            tui::event::AppEvent::VaultRefreshRequired {
                id: vault_id,
                config: vault_config,
            } => {
                let tx = ctx.tx.clone();
                tokio::spawn(async move {
                    let vault: Box<dyn crate::app::ports::VaultPort> = match vault_config {
                        crate::domain::config::VaultConfig::Github(g) => {
                            Box::new(crate::infra::vault::github::GithubVaultAdapter::new(
                                vault_id.clone(),
                                g.repo,
                                g.r#ref,
                                g.path,
                            ))
                        }
                        crate::domain::config::VaultConfig::Local(l) => {
                            Box::new(crate::infra::vault::local::LocalVaultAdapter::new(
                                vault_id.clone(),
                                std::path::PathBuf::from(l.path),
                            ))
                        }
                        crate::domain::config::VaultConfig::Clawhub(_) => {
                            Box::new(crate::infra::vault::clawhub::ClawHubVaultAdapter::new(
                                vault_id.clone(),
                            ))
                        }
                    };
                    let id = crate::tui::app::NEXT_TASK_ID
                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let _ = tx.send(tui::event::AppEvent::TaskStarted {
                        id,
                        name: format!("Pulling vault '{}'...", vault_id),
                    });
                    if let Err(e) = vault.refresh().await {
                        let _ = tx.send(tui::event::AppEvent::TaskFailed {
                            id,
                            error: e.to_string(),
                        });
                    } else {
                        let _ = tx.send(tui::event::AppEvent::TaskProgress { id, percent: 100 });
                        let _ = tx.send(tui::event::AppEvent::TriggerReload);
                        let _ = tx.send(tui::event::AppEvent::TaskCompleted {
                            id,
                            message: format!("Pulled vault '{}'", vault_id),
                        });
                    }
                });
            }
            tui::event::AppEvent::RunInteractiveProcess {
                command,
                args,
                current_dir,
                profile_name,
            } => {
                // Pause TUI and run the command directly in the current terminal.
                // This is synchronous in the event loop – no async races – so
                // keystrokes are never dropped the way they are when spawning a
                // background child with concurrent event processing.
                let mut out = io::stdout();
                let _ = crossterm::execute!(out, LeaveAlternateScreen);
                let _ = out.flush();
                let _ = terminal.show_cursor();
                let _ = disable_raw_mode();

                let status = std::process::Command::new(&command)
                    .current_dir(&current_dir)
                    .args(&args)
                    .stdin(std::process::Stdio::inherit())
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .status();

                let _ = enable_raw_mode();

                // Drain any crossterm events buffered while the child had stdin
                while let Ok(tui::event::AppEvent::Input(_)) = rx.try_recv() {}

                let _ = crossterm::execute!(out, EnterAlternateScreen);
                let _ = out.flush();
                let _ = terminal.hide_cursor();

                // Force a full clear/redraw now that we're back in the alternate screen
                let _ = terminal.clear();

                match &status {
                    Ok(s) if s.success() => {
                        state.status_line = format!("Finished: {} {}", command, args.join(" "));
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

                // Move freshly-created agent markdown into .agk/profiles if triggered by wizard
                if command == "opencode" && args.iter().any(|a| a == "create") {
                    let Some(name) = profile_name else {
                        continue;
                    };
                    let agents_dir = current_dir.join(".opencode").join("agents");
                    let target_dir: std::path::PathBuf =
                        current_dir.join(".agk").join("profiles").join(&name);

                    let mut moved = false;
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
                                state.status_line = format!("Failed to create profile dir: {}", e);
                            } else {
                                let dest = target_dir.join("agent.md");
                                if let Err(e) = std::fs::rename(&agent_md, &dest) {
                                    state.status_line = format!("Failed to move agent file: {}", e);
                                } else {
                                    state.status_line =
                                        format!("Profile '{}' created successfully", name);
                                    moved = true;
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
        terminal.draw(|frame| tui::render::draw(frame, state))?;
    }
    Ok(())
}
