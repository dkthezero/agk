pub mod attach_mcp;
pub mod attach_skill;
pub mod command;
pub mod create;
pub mod delete;
pub mod detach_mcp;
pub mod detach_skill;
pub mod start;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::{CoreEventSink, CoreResult};

/// Dispatch profile-related [`CoreCommand`] variants.
/// Returns `Some(result)` if the command was handled, `None` otherwise.
pub fn dispatch(
    cmd: &CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::CreateProfile { ref input } => {
            Some(create::run(input, core.store.as_ref(), sink))
        }
        CoreCommand::StartProfile { id, scope, dry_run } => {
            Some(start::run(
                id,
                *scope,
                *dry_run,
                core.store.as_ref(),
                &core.runtime_ports,
                sink,
            ))
        }
        CoreCommand::DeleteProfile { id, scope } => {
            Some(delete::run(id, *scope, core.store.as_ref(), sink))
        }
        CoreCommand::AttachSkillToProfile { profile_id, skill_id } => {
            Some(attach_skill::run(
                profile_id,
                skill_id,
                crate::domain::scope::Scope::Workspace,
                core.store.as_ref(),
                sink,
            ))
        }
        CoreCommand::DetachSkillFromProfile { profile_id, skill_id } => {
            Some(detach_skill::run(
                profile_id,
                skill_id,
                crate::domain::scope::Scope::Workspace,
                core.store.as_ref(),
                sink,
            ))
        }
        CoreCommand::AttachMcpToProfile { profile_id, mcp_id } => {
            Some(attach_mcp::run(
                profile_id,
                mcp_id,
                crate::domain::scope::Scope::Workspace,
                core.store.as_ref(),
                sink,
            ))
        }
        CoreCommand::DetachMcpFromProfile { profile_id, mcp_id } => {
            Some(detach_mcp::run(
                profile_id,
                mcp_id,
                crate::domain::scope::Scope::Workspace,
                core.store.as_ref(),
                sink,
            ))
        }
        _ => None,
    }
}
