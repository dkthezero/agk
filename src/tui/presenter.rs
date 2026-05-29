use crate::app::event::CoreEvent;
use crate::app::outcome::CoreEventSink;
use crate::tui::event::AppEvent;
use tokio::sync::mpsc::UnboundedSender;

/// TUI presenter: bridges [`CoreEventSink`] into the async event loop.
///
/// Every `on_event` / `on_error` call sends an [`AppEvent::CoreEvent`] back
/// into the same channel consumed by [`crate::tui::runtime_loop::run_loop`].
/// This keeps the TUI single-threaded for state mutations while allowing
/// [`AgkCore`] to run in `spawn_blocking`.
pub struct TuiPresenter {
    tx: UnboundedSender<AppEvent>,
}

impl TuiPresenter {
    pub fn new(tx: UnboundedSender<AppEvent>) -> Self {
        Self { tx }
    }
}

impl CoreEventSink for TuiPresenter {
    fn on_event(&mut self, event: CoreEvent) {
        let _ = self.tx.send(AppEvent::CoreEvent(event));
    }

    fn on_error(&mut self, error: String) {
        let _ = self.tx.send(AppEvent::CoreEvent(CoreEvent::Error(error)));
    }
}
