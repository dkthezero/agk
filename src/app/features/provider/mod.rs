pub mod activate;
pub mod deactivate;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};

/// Dispatch provider-related [`CoreCommand`] variants.
/// Returns `Some(result)` if the command was handled, `None` otherwise.
pub fn dispatch(
    cmd: &CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::ActivateProvider { id, scope } => {
            match core.registry.get_provider(id) {
                Ok(_provider) => {
                    Some(activate::run(id.clone(), *scope, core.store.as_ref(), sink))
                }
                Err(e) => {
                    sink.on_error(format!("Provider '{}' not found: {}", id, e));
                    Some(Ok(CoreOutcome::Ok))
                }
            }
        }
        CoreCommand::DeactivateProvider { id, scope } => {
            match core.registry.get_provider(id) {
                Ok(provider) => {
                    Some(deactivate::run(
                        id.clone(),
                        *scope,
                        core.store.as_ref(),
                        provider,
                        sink,
                    ))
                }
                Err(e) => {
                    sink.on_error(format!("Provider '{}' not found: {}", id, e));
                    Some(Ok(CoreOutcome::Ok))
                }
            }
        }
        _ => None,
    }
}
