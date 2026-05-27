// During architectural convergence dead-code placeholders are expected.
#![allow(dead_code)]

mod app;
mod cli;
mod domain;
mod infra;
mod tui;

use anyhow::Result;
use app::core::AgkCore;
use app::ports::ConfigStorePort;
use futures::StreamExt;
use std::sync::Arc;

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_GENERAL_FAILURE: i32 = 1;
pub const EXIT_INVALID_ARGS: i32 = 2;
pub const EXIT_NOT_FOUND: i32 = 3;

/// Composition root for AGK.
///
/// All concrete adapter wiring happens here and in `app::bootstrap`.
/// CLI and TUI receive pre-built `AgkCore` — they never construct infra directly.
#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("Error: {}", e);
        std::process::exit(EXIT_GENERAL_FAILURE);
    }
}

async fn try_main() -> Result<()> {
    let cli = cli::entry::parse();
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
/// Phase B: Routes through `cli::core_dispatcher::dispatch` first.  If the
/// dispatcher returns an error signalling "not yet wired", falls back to the
/// legacy `cli::commands::run` path.  This preserves behaviour while
/// incrementally wiring commands through `AgkCore`.
fn run_headless(cli: cli::entry::Cli, workspace: &std::path::Path) -> Result<i32> {
    let (registry, _scan, store) = app::bootstrap::build(workspace.to_path_buf())?;
    let core = build_core(workspace, registry, store)?;

    match cli::core_dispatcher::dispatch(&cli, workspace, &core) {
        Ok(exit_code) => Ok(exit_code),
        Err(e) => {
            let msg = format!("{}", e);
            if msg.contains("not yet wired") || msg.contains("not yet") {
                eprintln!(
                    "[Phase B fallback] Command not yet routed through AgkCore; using legacy path."
                );
                cli::commands::run(cli, workspace)
            } else {
                Err(e)
            }
        }
    }
}

/// Run the interactive TUI.
async fn run_tui(workspace: std::path::PathBuf) -> Result<()> {
    let (registry, scan, store) = app::bootstrap::build(workspace.clone())?;
    let registry_arc = Arc::new(registry);
    let store_arc: Arc<dyn ConfigStorePort> = Arc::new(store);

    let mut state = tui::entry::build_state(&registry_arc, store_arc.as_ref(), scan, &workspace);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let ctx = tui::event::EventContext {
        store: store_arc.clone(),
        registry: registry_arc.clone(),
        tx: tx.clone(),
        workspace_root: workspace.clone(),
    };

    // Spawn a keyboard input reader that forwards crossterm events into the
    // same async channel consumed by `runtime_loop::run_loop`.
    let tx_input = tx.clone();
    tokio::spawn(async move {
        let mut reader = crossterm::event::EventStream::new();
        while let Some(Ok(evt)) = reader.next().await {
            if tx_input.send(tui::event::AppEvent::Input(evt)).is_err() {
                break;
            }
        }
    });

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Spawn a ticker for progress visuals
    let tx_tick = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
        loop {
            interval.tick().await;
            if tx_tick.send(tui::event::AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    let result = tui::runtime_loop::run_loop(&mut terminal, &mut state, &ctx, &mut rx).await;

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

// ---------------------------------------------------------------------------
// Core construction (centralised, only place where concrete ports are wired)
// ---------------------------------------------------------------------------

/// Build `AgkCore` with all required port implementations.
fn build_core(
    workspace: &std::path::Path,
    registry: app::registry::Registry,
    store: infra::config::toml_store::TomlConfigStore,
) -> Result<AgkCore> {
    let store_arc: Arc<dyn ConfigStorePort> = Arc::new(store);
    let context_store = Arc::new(infra::context::TomlContextStore::standard());

    let mcp_registry = Arc::new(infra::mcp::adapter::InfraMcpRegistryAdapter::new(
        workspace.to_path_buf(),
    ));

    let vault_search = Arc::new(infra::vault::search_adapters::ClawHubSearchAdapter::new(
        "clawhub",
    ));

    let mut runtime_ports: std::collections::HashMap<
        String,
        Arc<dyn app::ports::ProfileRuntimePort>,
    > = std::collections::HashMap::new();
    // Register any concrete runtime ports here as they become available.
    // Phase 5 has OpenCodeProvider via infra/provider/opencode.rs.
    let opencode_provider =
        crate::infra::provider::opencode::OpenCodeProvider::new(workspace.to_path_buf());
    runtime_ports.insert("opencode".to_string(), Arc::new(opencode_provider));

    let core = AgkCore::new(
        store_arc.clone(),
        context_store,
        mcp_registry,
        vault_search,
        Arc::new(registry),
        runtime_ports,
    );

    Ok(core)
}
