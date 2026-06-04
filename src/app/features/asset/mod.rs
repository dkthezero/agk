pub mod install;
#[cfg(feature = "pack")]
pub mod pack;
pub mod remove;
pub mod search_remote;
pub mod sync;
pub mod sync_team;
pub mod update;
pub mod validate;

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
        CoreCommand::SearchRemoteVault { vault_id, query } => Some(search_remote::run(
            vault_id.clone(),
            query.clone(),
            core.vault_search.as_ref(),
            sink,
        )),
        CoreCommand::ValidateAssets { scope } => Some(validate::run(
            *scope,
            core.registry.as_ref(),
            core.store.as_ref(),
            sink,
        )),
        CoreCommand::PackAsset {
            identity,
            target,
            stdout,
            scope,
        } => {
            #[cfg(feature = "pack")]
            {
                Some(pack::run(
                    identity,
                    *target,
                    *stdout,
                    *scope,
                    core.registry.as_ref(),
                    &core.workspace_root,
                    sink,
                ))
            }
            #[cfg(not(feature = "pack"))]
            {
                let _ = (identity, target, stdout, scope);
                None
            }
        }
        CoreCommand::InstallAsset {
            identity,
            scope,
            provider_filter,
            include_evals,
            dry_run,
        } => Some(install_asset_cmd(
            identity,
            *scope,
            provider_filter.as_deref(),
            *include_evals,
            *dry_run,
            core,
            sink,
        )),
        CoreCommand::RemoveAsset {
            identity,
            scope,
            provider_filter,
        } => Some(remove_asset_cmd(
            identity,
            *scope,
            provider_filter.as_deref(),
            core,
            sink,
        )),
        CoreCommand::UpdateAsset {
            identity,
            scope,
            provider_filter,
        } => Some(update_asset_cmd(
            identity,
            *scope,
            provider_filter.as_deref(),
            core,
            sink,
        )),
        CoreCommand::SyncAssets { scope, dry_run } => {
            Some(sync_assets_cmd(*scope, *dry_run, core, sink))
        }
        CoreCommand::SyncTeam => {
            // Delegated from SyncAssets when team.toml is present.
            // The actual team-sync logic runs inside sync_assets_cmd.
            Some(sync_assets_cmd(
                crate::domain::scope::Scope::Workspace,
                false,
                core,
                sink,
            ))
        }
        _ => None,
    }
}

mod dispatch_helpers;
mod dispatch_sync;
use dispatch_helpers::{install_asset_cmd, remove_asset_cmd, update_asset_cmd};
use dispatch_sync::sync_assets_cmd;
