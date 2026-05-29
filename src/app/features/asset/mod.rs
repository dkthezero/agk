pub mod install;
pub mod remove;
pub mod search_remote;
pub mod sync;
pub mod update;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::{CoreEventSink, CoreResult};

/// Dispatch asset-related [`CoreCommand`] variants.
/// Returns `Some(result)` if the command was handled, `None` otherwise.
pub fn dispatch(
    cmd: &CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::SearchRemoteVault { vault_id, query } => {
            Some(search_remote::run(
                vault_id.clone(),
                query.clone(),
                core.vault_search.as_ref(),
                sink,
            ))
        }
        _ => None,
    }
}
