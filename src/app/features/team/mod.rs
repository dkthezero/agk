pub mod add;
pub mod diff;
pub mod init;
pub mod remove;
pub mod status;
pub mod update;

use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};

/// Dispatch team-related [`CoreCommand`] variants.
/// Returns `Some(result)` if the command was handled, `None` otherwise.
pub fn dispatch(
    cmd: &CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::TeamInit { name, dry_run } => {
            let result = init::team_init(&core.workspace_root, name, *dry_run);
            match result {
                Ok(init_result) => {
                    if init_result.created {
                        sink.on_event(crate::app::event::CoreEvent::TeamInitialized(
                            init_result.name.clone(),
                        ));
                    } else {
                        sink.on_event(crate::app::event::CoreEvent::Info(
                            init_result.message.clone(),
                        ));
                    }
                    Some(Ok(CoreOutcome::Ok))
                }
                Err(e) => {
                    sink.on_error(format!("Team init failed: {}", e));
                    Some(Ok(CoreOutcome::Ok))
                }
            }
        }
        CoreCommand::TeamAddVault {
            identity,
            vault_type,
            url,
            branch,
        } => {
            let result = add::team_add_vault(
                &core.workspace_root,
                identity,
                vault_type,
                url,
                branch,
            );
            match result {
                Ok(add_result) => {
                    sink.on_event(crate::app::event::CoreEvent::TeamVaultAdded(
                        add_result.identity.clone(),
                    ));
                    Some(Ok(CoreOutcome::Ok))
                }
                Err(e) => {
                    sink.on_error(format!("Team add-vault failed: {}", e));
                    Some(Ok(CoreOutcome::Ok))
                }
            }
        }
        CoreCommand::TeamAddRequirement {
            identity,
            vault,
            kind,
            version_constraint,
        } => {
            let result = add::team_add_requirement(
                &core.workspace_root,
                identity,
                vault,
                kind,
                version_constraint.as_deref(),
            );
            match result {
                Ok(add_result) => {
                    sink.on_event(crate::app::event::CoreEvent::TeamRequirementAdded(
                        add_result.identity.clone(),
                    ));
                    Some(Ok(CoreOutcome::Ok))
                }
                Err(e) => {
                    sink.on_error(format!("Team add failed: {}", e));
                    Some(Ok(CoreOutcome::Ok))
                }
            }
        }
        CoreCommand::TeamRemove { identity } => {
            let result = remove::team_remove_requirement(&core.workspace_root, identity);
            match result {
                Ok(remove_result) => {
                    if remove_result.removed {
                        sink.on_event(crate::app::event::CoreEvent::TeamRequirementRemoved(
                            remove_result.identity.clone(),
                        ));
                    } else {
                        sink.on_event(crate::app::event::CoreEvent::Info(
                            remove_result.message.clone(),
                        ));
                    }
                    Some(Ok(CoreOutcome::Ok))
                }
                Err(e) => {
                    sink.on_error(format!("Team remove failed: {}", e));
                    Some(Ok(CoreOutcome::Ok))
                }
            }
        }
        CoreCommand::TeamDiff => {
            let result = diff::team_diff(&core.workspace_root);
            match result {
                Ok(diff_result) => {
                    sink.on_event(crate::app::event::CoreEvent::TeamDiffResult {
                        summary: diff_result.summary(),
                    });
                    Some(Ok(CoreOutcome::Ok))
                }
                Err(e) => {
                    sink.on_error(format!("Team diff failed: {}", e));
                    Some(Ok(CoreOutcome::Ok))
                }
            }
        }
        CoreCommand::TeamStatus => {
            let result = status::team_status(&core.workspace_root);
            match result {
                Ok(status_result) => {
                    sink.on_event(crate::app::event::CoreEvent::TeamStatusResult {
                        team_name: status_result.team_name.clone(),
                        installed: status_result.installed,
                        required: status_result.required,
                        personal: status_result.personal,
                    });
                    Some(Ok(CoreOutcome::Ok))
                }
                Err(e) => {
                    sink.on_error(format!("Team status failed: {}", e));
                    Some(Ok(CoreOutcome::Ok))
                }
            }
        }
        CoreCommand::TeamUpdate => {
            let result = update::team_update(&core.workspace_root);
            match result {
                Ok(update_result) => {
                    sink.on_event(crate::app::event::CoreEvent::Info(
                        update_result.message.clone(),
                    ));
                    Some(Ok(CoreOutcome::Ok))
                }
                Err(e) => {
                    sink.on_error(format!("Team update failed: {}", e));
                    Some(Ok(CoreOutcome::Ok))
                }
            }
        }
        _ => None,
    }
}