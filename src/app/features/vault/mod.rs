pub mod attach;
pub mod command;
pub mod detach;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::{CoreEventSink, CoreResult, CoreOutcome};

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
                sink,
            )
            .map(|_| CoreOutcome::Ok),
        ),
        _ => None,
    }
}
