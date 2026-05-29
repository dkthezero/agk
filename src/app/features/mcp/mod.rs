pub mod command;
pub mod disable;
pub mod enable;
pub mod list;
pub mod register;
pub mod test;
pub mod toggle;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::{CoreEventSink, CoreResult};

/// Dispatch MCP-related [`CoreCommand`] variants.
/// Returns `Some(result)` if the command was handled, `None` otherwise.
pub fn dispatch(
    cmd: &CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::RegisterMcp { ref input } => {
            Some(register::run(input, core.mcp_registry.as_ref(), sink))
        }
        CoreCommand::EnableMcp {
            ref name,
            ref provider_id,
            scope,
        } => Some(enable::run(
            name,
            provider_id,
            *scope,
            core.mcp_registry.as_ref(),
            sink,
        )),
        CoreCommand::DisableMcp {
            ref name,
            ref provider_id,
            scope,
        } => Some(disable::run(
            name,
            provider_id,
            *scope,
            core.mcp_registry.as_ref(),
            sink,
        )),
        CoreCommand::ListMcp => Some(list::run(core.mcp_registry.as_ref(), sink)),
        CoreCommand::TestMcp { ref name } => {
            Some(test::run(name, core.mcp_registry.as_ref(), sink))
        }
        CoreCommand::ToggleMcp { ref name, scope } => Some(toggle::run(name, *scope, core, sink)),
        _ => None,
    }
}
