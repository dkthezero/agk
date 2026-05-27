use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};

/// Search a remote vault (e.g. ClawHub) for packages matching a query.
///
/// In Phase 3 this delegates to a [`VaultPort`] implementation so the TUI
/// never calls `infra::vault::clawhub::cli_search` directly.
#[allow(dead_code)] // search remote vault use-case stub
pub fn run(vault_id: String, query: String, sink: &mut dyn CoreEventSink) -> CoreResult {
    // Phase 3: this will call VaultPort::search() instead
    sink.on_event(CoreEvent::TaskStarted {
        id: 0,
        name: format!("Searching '{}' in {}", query, vault_id),
    });
    // Placeholder: emit empty results
    sink.on_event(CoreEvent::RemoteVaultSearchResults {
        vault_id,
        packages: vec![],
    });
    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::event::CoreEvent;
    use crate::app::outcome::CoreEventSink;

    struct CollectingSink {
        events: Vec<CoreEvent>,
    }

    impl CoreEventSink for CollectingSink {
        fn on_event(&mut self, event: CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, _error: String) {}
    }

    #[test]
    fn search_emits_results_event() {
        let mut sink = CollectingSink { events: vec![] };
        let result = run("clawhub".into(), "rust".into(), &mut sink);
        assert!(result.is_ok());
        assert!(sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::RemoteVaultSearchResults { vault_id, .. } if vault_id == "clawhub")));
    }
}
