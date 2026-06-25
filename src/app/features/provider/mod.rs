pub mod activate;
pub mod deactivate;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::{CoreEventSink, CoreResult};

/// Dispatch provider-related [`CoreCommand`] variants.
/// Returns `Some(result)` if the command was handled, `None` otherwise.
pub fn dispatch(
    cmd: &CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::ActivateProvider { id, scope } => match core.registry.get_provider(id) {
            Ok(provider) => Some(activate::run(
                id.clone(),
                *scope,
                core.store.as_ref(),
                provider,
                core.registry.as_ref(),
                sink,
            )),
            Err(e) => Some(Err(anyhow::anyhow!("Provider '{}' not found: {}", id, e))),
        },
        CoreCommand::DeactivateProvider { id, scope } => match core.registry.get_provider(id) {
            Ok(provider) => Some(deactivate::run(
                id.clone(),
                *scope,
                core.store.as_ref(),
                provider,
                sink,
            )),
            Err(e) => Some(Err(anyhow::anyhow!("Provider '{}' not found: {}", id, e))),
        },
        _ => None,
    }
}
