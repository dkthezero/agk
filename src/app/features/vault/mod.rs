pub mod attach;
pub mod command;
pub mod detach;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};

/// Dispatch vault-related [`CoreCommand`] variants.
/// Returns `Some(result)` if the command was handled, `None` otherwise.
pub fn dispatch(
    cmd: &CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::AttachVault { input } => Some(
            attach::run(
                input.vault_id.clone(),
                input.config.clone(),
                core.store.as_ref(),
                core.clawhub.as_ref(),
                sink,
            )
            .map(|_| CoreOutcome::Ok),
        ),
        CoreCommand::DetachVault { vault_id, .. } => {
            Some(detach::detach_vault(vault_id, core.store.as_ref()).map(|_| CoreOutcome::Ok))
        }
        CoreCommand::AttachBareVault { vault_id, scope } => {
            let result = core.store.load(*scope).and_then(|mut config| {
                if !config.vaults.contains(vault_id) {
                    config.vaults.push(vault_id.clone());
                }
                core.store.save(*scope, &config)
            });
            match result {
                Ok(()) => {
                    sink.on_event(crate::app::event::CoreEvent::VaultAttached(
                        vault_id.clone(),
                    ));
                    Some(Ok(CoreOutcome::Ok))
                }
                Err(e) => {
                    sink.on_error(format!("Failed to attach vault '{}': {}", vault_id, e));
                    Some(Ok(CoreOutcome::Ok))
                }
            }
        }
        CoreCommand::RefreshAllVaults => {
            let mut errs = Vec::new();
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    sink.on_error(format!("Failed to create runtime for vault refresh: {}", e));
                    return Some(Ok(CoreOutcome::Ok));
                }
            };
            for vault in &core.registry.vaults {
                if let Err(e) = rt.block_on(vault.refresh()) {
                    errs.push(format!("{}: {}", vault.id(), e));
                }
            }
            if errs.is_empty() {
                sink.on_event(crate::app::event::CoreEvent::Info(
                    "All vaults refreshed".into(),
                ));
            } else {
                sink.on_error(format!("Vault refresh issues: {}", errs.join(", ")));
            }
            Some(Ok(CoreOutcome::Ok))
        }
        _ => None,
    }
}
