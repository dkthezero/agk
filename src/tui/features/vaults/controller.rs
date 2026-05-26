use crate::tui::event::{AppEvent, EventContext};
use anyhow::Result;
use tokio::sync::mpsc;

/// Vault feature controller.
///
/// Handles side effects specific to the vaults tab: refresh, search,
/// attach/detach, and ClawHub background tasks.
pub struct VaultsController;

impl VaultsController {
    /// Initiate a background vault refresh and emit lifecycle events.
    pub fn refresh(
        vault_id: String,
        vault_config: crate::domain::config::VaultConfig,
        ctx: &EventContext,
    ) {
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
            let _ = tx.send(AppEvent::TaskStarted {
                id,
                name: format!("Pulling vault '{}'...", vault_id),
            });
            if let Err(e) = vault.refresh().await {
                let _ = tx.send(AppEvent::TaskFailed {
                    id,
                    error: e.to_string(),
                });
            } else {
                let _ = tx.send(AppEvent::TaskProgress { id, percent: 100 });
                let _ = tx.send(AppEvent::TriggerReload);
                let _ = tx.send(AppEvent::TaskCompleted {
                    id,
                    message: format!("Pulled vault '{}'", vault_id),
                });
            }
        });
    }

    /// Dispatch a ClawHub search as a background blocking task.
    pub fn search_clawhub(query: String, ctx: &EventContext) -> usize {
        let tx = ctx.tx.clone();
        let id = crate::tui::app::NEXT_TASK_ID
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let _ = ctx.tx.send(AppEvent::TaskStarted {
            id,
            name: format!("Searching ClawHub '{}'", query),
        });
        tokio::task::spawn_blocking(move || {
            let packages =
                crate::infra::vault::clawhub::cli_search(&query).unwrap_or_default();
            let _ = tx.send(AppEvent::ClawHubSearchResults {
                packages,
                task_id: id,
            });
        });
        id
    }
}
