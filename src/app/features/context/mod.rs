pub mod list;
pub mod switch;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::{CoreEventSink, CoreResult};

/// Dispatch context-related [`CoreCommand`] variants.
/// Returns `Some(result)` if the command was handled, `None` otherwise.
pub fn dispatch(
    cmd: &CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::SwitchContext { id, dry_run } => Some(switch::run(
            id,
            *dry_run,
            core.context_store.as_ref(),
            sink,
            core.store.as_ref(),
        )),
        CoreCommand::ListContexts => Some(list::run(core.context_store.as_ref(), sink)),
        _ => None,
    }
}
