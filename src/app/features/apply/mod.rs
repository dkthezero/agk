pub mod command;
pub mod run;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::{CoreEventSink, CoreResult};

/// Dispatch apply-related [`CoreCommand`] variants.
/// Returns `Some(result)` if the command was handled, `None` otherwise.
pub fn dispatch(
    cmd: &CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::ApplyConfig {
            input,
            scope,
            environment,
            context,
            dry_run,
        } => {
            Some(run::run(
                input.clone(),
                *scope,
                *environment,
                context.clone(),
                *dry_run,
                core.store.as_ref(),
                core.context_store.as_ref(),
                core.registry.providers.iter().map(|p| p.id().to_string()).collect(),
                sink,
            ))
        }
        _ => None,
    }
}
