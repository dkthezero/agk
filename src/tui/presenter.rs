use crate::app::event::{CoreEvent, TaskStatus};
use crate::app::outcome::CoreEventSink;
use tokio::sync::mpsc::UnboundedSender;

/// Bridges [`CoreEventSink`] into the TUI's [`AppEvent`] channel.
///
/// Every [`CoreEvent`] is mapped to the equivalent [`super::AppEvent`] so
/// the existing TUI event loop can render progress, errors, and reloads
/// without knowing about the core layer.
pub struct AppEventSink {
    tx: UnboundedSender<crate::tui::event::AppEvent>,
    current_task_id: Option<usize>,
}

impl AppEventSink {
    pub fn new(tx: UnboundedSender<crate::tui::event::AppEvent>) -> Self {
        Self {
            tx,
            current_task_id: None,
        }
    }

    pub fn set_task_id(&mut self, id: usize) {
        self.current_task_id = Some(id);
    }

    fn task_id(&self) -> usize {
        self.current_task_id.unwrap_or(0)
    }

    fn send(&self, event: crate::tui::event::AppEvent) {
        let _ = self.tx.send(event);
    }
}

impl CoreEventSink for AppEventSink {
    fn on_event(&mut self, event: CoreEvent) {
        match event {
            CoreEvent::TaskStarted { id, name } => {
                self.send(crate::tui::event::AppEvent::TaskStarted { id, name });
            }
            CoreEvent::TaskProgress { id, percent } => {
                self.send(crate::tui::event::AppEvent::TaskProgress { id, percent });
            }
            CoreEvent::TaskCompleted { id, message } => {
                self.send(crate::tui::event::AppEvent::TaskCompleted { id, message });
            }
            CoreEvent::TaskFailed { id, error } => {
                self.send(crate::tui::event::AppEvent::TaskFailed { id, error });
            }
            CoreEvent::WorkspaceLoaded(snapshot) => {
                let _ = snapshot;
                self.send(crate::tui::event::AppEvent::TriggerReload);
            }
            CoreEvent::ProviderDeactivated(provider_id) => {
                self.send(crate::tui::event::AppEvent::TaskCompleted {
                    id: self.task_id(),
                    message: format!("Deactivated '{}'", provider_id),
                });
                self.send(crate::tui::event::AppEvent::TriggerReload);
            }
            CoreEvent::McpRegistered(name) => {
                self.send(crate::tui::event::AppEvent::TaskCompleted {
                    id: self.task_id(),
                    message: format!("MCP server '{}' registered", name),
                });
                self.send(crate::tui::event::AppEvent::TriggerReload);
            }
            CoreEvent::RemoteVaultSearchResults { vault_id, packages } => {
                self.send(crate::tui::event::AppEvent::ClawHubSearchResults {
                    packages,
                    task_id: self.task_id(),
                });
            }
            _ => {}
        }
    }

    fn on_error(&mut self, error: String) {
        self.send(crate::tui::event::AppEvent::TaskFailed {
            id: self.task_id(),
            error,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::event::CoreEvent;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn sink_maps_task_started() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut sink = AppEventSink::new(tx);
        sink.on_event(CoreEvent::TaskStarted {
            id: 1,
            name: "Test".into(),
        });
        let evt = rx.try_recv().unwrap();
        assert!(matches!(
            evt,
            crate::tui::event::AppEvent::TaskStarted { id: 1, .. }
        ));
    }
}
