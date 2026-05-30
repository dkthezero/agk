mod hangs;
mod tasks;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::{CoreEventSink, CoreResult};

/// Dispatch debug-related [`CoreCommand`] variants.
/// Returns `Some(result)` if the command was handled, `None` otherwise.
pub fn dispatch(
    cmd: &CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::DebugListTasks => Some(tasks::run(core, sink)),
        CoreCommand::DebugDetectHangs => Some(hangs::run(core, sink)),
        CoreCommand::DebugDumpTrace => {
            // Tracing dump is a no-op unless the observability feature is enabled.
            // The actual trace subscription lives outside AgkCore.
            sink.on_event(crate::app::event::CoreEvent::Info(
                "Trace dump: enable observability feature to capture spans.".into(),
            ));
            Some(Ok(crate::app::outcome::CoreOutcome::Ok))
        }
        _ => None,
    }
}
