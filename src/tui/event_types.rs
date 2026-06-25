//! TUI event/value types.
//!
//! Split out of `event.rs` so the key-dispatch logic there stays under the
//! 300-LOC ADR-001 §6.4 limit. Re-exported by `event.rs` so consumer paths
//! (`crate::tui::event::AppEvent`, …) remain unchanged.

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::event::CoreEvent;
use crate::tui::reload::ReloadSnapshot;
use std::sync::Arc;

pub enum ControlFlow {
    Continue,
    Quit,
}

#[derive(Debug)]
pub enum AppEvent {
    /// Keyboard events from `crossterm` forwarded by `main.rs` into the async
    /// channel consumed by `runtime_loop::run_loop`.
    Input(crossterm::event::Event),
    TaskStarted {
        id: usize,
        name: String,
    },
    TaskProgress {
        id: usize,
        percent: u8,
    },
    TaskCompleted {
        id: usize,
        message: String,
    },
    TaskFailed {
        id: usize,
        error: String,
    },
    TriggerReload,
    ClawHubSearchResults {
        packages: Vec<crate::domain::asset::ScannedPackage>,
        task_id: usize,
    },
    Tick,
    /// Background reload finished atomically so the UI never freezes.
    ReloadComplete(ReloadSnapshot),
    /// then resume TUI. The child runs interactively (user can type/respond).
    RunInteractiveProcess {
        command: String,
        args: Vec<String>,
        current_dir: std::path::PathBuf,
        profile_name: Option<String>,
    },
    /// Execute a [`CoreCommand`] through [`AgkCore`] in a blocking task.
    ExecuteCommand(CoreCommand),
    /// A [`CoreEvent`] emitted by [`AgkCore`] back to the TUI.
    CoreEvent(CoreEvent),
}

pub struct EventContext {
    pub tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
    pub workspace_root: std::path::PathBuf,
    pub file_opener: Arc<dyn crate::app::ports::FileOpenerPort>,
    /// Reference to the shared [`AgkCore`] façade so controllers can dispatch
    /// commands and the runtime loop can execute them.
    pub core: Arc<AgkCore>,
}
