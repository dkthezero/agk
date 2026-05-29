use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::{CoreEventSink, CoreResult};

pub mod disable;
pub mod enable;
pub mod export;
pub mod status;

pub fn dispatch(
    command: &CoreCommand,
    _core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    let path = crate::domain::paths::analytics_path();
    match command {
        CoreCommand::EnableTelemetry => Some(enable::run(&path, sink)),
        CoreCommand::DisableTelemetry => Some(disable::run(&path, sink)),
        CoreCommand::TelemetryStatus => Some(status::run(&path, sink)),
        CoreCommand::ExportTelemetry {
            format,
            output_path,
        } => Some(export::run(&path, *format, output_path.clone(), sink)),
        _ => None,
    }
}
