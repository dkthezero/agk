// During architectural convergence dead-code placeholders are expected.
#![allow(dead_code)]

use agk::app::core::AgkCore;
use agk::app::ports::ConfigStorePort;
use anyhow::Result;
use futures::StreamExt;
use std::sync::Arc;

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_GENERAL_FAILURE: i32 = 1;
pub const EXIT_INVALID_ARGS: i32 = 2;
pub const EXIT_NOT_FOUND: i32 = 3;

/// Composition root for AGK.
///
/// All concrete adapter wiring happens here and in `agk::app::bootstrap`.
/// CLI and TUI receive pre-built `AgkCore` — they never construct infra directly.
#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("Error: {}", e);
        std::process::exit(EXIT_GENERAL_FAILURE);
    }
}

async fn try_main() -> Result<()> {
    let cli = agk::cli::entry::parse();
    let workspace = std::env::current_dir()?;

    // If a subcommand is provided, run headless CLI
    if cli.command.is_some() {
        let code = run_headless(cli, &workspace)?;
        if code != EXIT_SUCCESS {
            std::process::exit(code);
        }
        return Ok(());
    }

    // No subcommand — launch TUI
    run_tui(workspace).await
}

/// Run the headless CLI path.
///
/// All commands are now routed through `AgkCore` via `agk::cli::core_dispatcher`.
fn run_headless(cli: agk::cli::entry::Cli, workspace: &std::path::Path) -> Result<i32> {
    let (registry, _scan, store, _global, _workspace) =
        agk::app::bootstrap::build(workspace.to_path_buf())?;
    let core = build_core(workspace, registry, store)?;
    agk::cli::core_dispatcher::dispatch(&cli, workspace, &core)
}

/// Run the interactive TUI.
async fn run_tui(workspace: std::path::PathBuf) -> Result<()> {
    use std::io::IsTerminal;

    // The TUI requires an interactive terminal. When stdin/stdout are not a
    // TTY (e.g. piped in CI, run under `agk < /dev/null`, or spawned by an
    // agent without a pty), crossterm's event reader panics with an opaque
    // "reader source not set" message. Fail fast with a clear, actionable
    // error and the documented general-failure exit code instead.
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        anyhow::bail!(
            "agk TUI requires an interactive terminal. \
             Run `agk <subcommand>` for headless/CLI usage, or attach a pty."
        );
    }

    let (registry, scan, store, global_config, workspace_config) =
        agk::app::bootstrap::build(workspace.clone())?;
    let core = build_core(&workspace, registry, store)?;

    // Load team config for accurate TUI status bar
    let team_config = core
        .team_config_store
        .load(agk::domain::scope::Scope::Workspace)
        .ok()
        .filter(|c| !c.name.is_empty());

    let mut state = agk::tui::entry::build_state(
        core.registry.as_ref(),
        scan,
        &workspace,
        global_config,
        workspace_config,
        team_config,
    );

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let file_opener: Arc<dyn agk::app::ports::FileOpenerPort> =
        Arc::new(agk::infra::process::opener::OsFileOpener);
    let core_arc = Arc::new(core);
    let ctx = agk::tui::event::EventContext {
        tx: tx.clone(),
        workspace_root: workspace.clone(),
        file_opener: file_opener.clone(),
        core: core_arc,
    };

    // Spawn a keyboard input reader that forwards crossterm events into the
    // same async channel consumed by `runtime_loop::run_loop`.
    let tx_input = tx.clone();
    tokio::spawn(async move {
        let mut reader = crossterm::event::EventStream::new();
        while let Some(Ok(evt)) = reader.next().await {
            if tx_input
                .send(agk::tui::event::AppEvent::Input(evt))
                .is_err()
            {
                break;
            }
        }
    });

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Spawn a ticker for progress visuals
    let tx_tick = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
        loop {
            interval.tick().await;
            if tx_tick.send(agk::tui::event::AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    // Kick off the initial vault scan in the background so the TUI renders
    // immediately.  The footer progress bar will show "Scanning vaults..." while
    // packages populate via the existing ReloadComplete pipeline.
    let _ = tx.send(agk::tui::event::AppEvent::TriggerReload);

    let result = agk::tui::runtime_loop::run_loop(&mut terminal, &mut state, &ctx, &mut rx).await;

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    result?;

    // ESC x2 (or Ctrl+C) lands here.  Terminate immediately so the
    // process does not hang waiting for spawned blocking tasks (e.g. an
    // in-flight asset install) or the ticker / keyboard reader.
    std::process::exit(EXIT_SUCCESS);
}

// ---------------------------------------------------------------------------
// Core construction (centralised, only place where concrete ports are wired)
// ---------------------------------------------------------------------------

/// Build `AgkCore` with all required port implementations.
fn build_core(
    workspace: &std::path::Path,
    registry: agk::app::registry::Registry,
    store: agk::infra::config::toml_store::TomlConfigStore,
) -> Result<AgkCore> {
    let store_arc: Arc<dyn ConfigStorePort> = Arc::new(store);
    let context_store = Arc::new(agk::infra::context::TomlContextStore::standard());

    let mcp_registry = Arc::new(agk::infra::mcp::adapter::InfraMcpRegistryAdapter::new(
        workspace.to_path_buf(),
    ));

    let vault_search =
        Arc::new(agk::infra::vault::search_adapters::ClawHubSearchAdapter::new("clawhub"));

    let mut runtime_ports: std::collections::HashMap<
        String,
        Arc<dyn agk::app::ports::ProfileRuntimePort>,
    > = std::collections::HashMap::new();
    // Register any concrete runtime ports here as they become available.
    let opencode_provider =
        agk::infra::provider::opencode::OpenCodeProvider::new(workspace.to_path_buf());
    runtime_ports.insert("opencode".to_string(), Arc::new(opencode_provider));

    let claude_code_provider =
        agk::infra::provider::claude_code::ClaudeCodeProvider::new(workspace.to_path_buf());
    runtime_ports.insert("claude-code".to_string(), Arc::new(claude_code_provider));

    let process_runner: Arc<dyn agk::app::ports::ProcessRunnerPort> =
        Arc::new(agk::infra::process::runner::OsProcessRunner);

    let task_tracker: Arc<dyn agk::app::ports::TaskTrackerPort> =
        Arc::new(agk::infra::task_tracker::InMemoryTaskTracker::new());

    let team_config_store: Arc<dyn agk::app::ports::TeamConfigStorePort> = Arc::new(
        agk::infra::config::team_store::TeamTomlStore::new(workspace.to_path_buf()),
    );

    let core = AgkCore::new(
        store_arc.clone(),
        context_store,
        mcp_registry,
        vault_search,
        Arc::new(registry),
        runtime_ports,
        process_runner,
        task_tracker,
        workspace.to_path_buf(),
        Arc::new(agk::infra::vault::clawhub::ClawHubAdapter),
        team_config_store,
    );

    Ok(core)
}
