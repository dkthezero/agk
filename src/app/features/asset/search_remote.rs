use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::VaultSearchPort;

/// Search a remote vault (e.g. ClawHub) for packages matching a query.
///
/// In Phase 3 this delegates to a [`VaultPort`] implementation so the TUI
/// never calls `infra::vault::clawhub::cli_search` directly.
pub fn run(
    vault_id: String,
    query: String,
    vault_search: &dyn VaultSearchPort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    sink.on_event(CoreEvent::TaskStarted {
        id: 0,
        name: format!("Searching '{}' in {}", query, vault_id),
    });

    // Attempt to search through the port.
    // For now the search is async; we block on a Tokio runtime.
    let rt = tokio::runtime::Runtime::new()?;
    let results = rt.block_on(vault_search.search(&query));

    let packages = match results {
        Ok(pkgs) => pkgs,
        Err(e) => {
            let msg = format!("Search failed: {}", e);
            sink.on_event(CoreEvent::TaskFailed {
                id: 0,
                error: msg.clone(),
            });
            return Err(anyhow::anyhow!(msg));
        }
    };

    sink.on_event(CoreEvent::RemoteVaultSearchResults {
        vault_id: vault_id.clone(),
        packages: packages.clone(),
    });
    sink.on_event(CoreEvent::TaskCompleted {
        id: 0,
        message: format!("Found {} packages in {}", packages.len(), vault_id),
    });
    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::event::CoreEvent;
    use crate::app::outcome::CoreEventSink;
    use crate::domain::asset::ScannedPackage;

    struct CollectingSink {
        events: Vec<CoreEvent>,
    }

    impl CoreEventSink for CollectingSink {
        fn on_event(&mut self, event: CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, _error: String) {}
    }

    struct FakeVaultSearch;

    #[async_trait::async_trait]
    impl VaultSearchPort for FakeVaultSearch {
        fn vault_id(&self) -> &str {
            "clawhub"
        }

        async fn search(&self, _query: &str) -> anyhow::Result<Vec<ScannedPackage>> {
            Ok(vec![])
        }
    }

    #[test]
    fn search_emits_results_event() {
        let mut sink = CollectingSink { events: vec![] };
        let searcher = FakeVaultSearch;
        let result = run("clawhub".into(), "rust".into(), &searcher, &mut sink);
        assert!(result.is_ok());
        assert!(sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::RemoteVaultSearchResults { vault_id, .. } if vault_id == "clawhub")));
    }
}
