//! Vault-workspace asset install/remove path for `asset::dispatch`.
//!
//! When the workspace root is a vault source repository (it contains
//! `.agk/vault.toml`), assets are copied into the vault's own
//! `skills/` / `instructions/` / `mcps/` / `profiles/` folders rather than
//! installed via a provider. Extracted from `dispatch_helpers.rs` so that
//! file stays under the 300-LOC ADR-001 §6.4 limit.

use crate::app::core::AgkCore;
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::domain::asset::AssetKind;

/// Returns true when the workspace root contains `.agk/vault.toml`, meaning
/// this workspace is a vault source repository. Skills are installed into the
/// vault's own `skills/` folder rather than any provider-specific directory.
pub(super) fn is_vault_workspace(core: &AgkCore) -> bool {
    core.workspace_root.join(".agk").join("vault.toml").exists()
}

fn vault_kind_folder(kind: &AssetKind) -> &'static str {
    match kind {
        AssetKind::Skill => "skills",
        AssetKind::Instruction => "instructions",
        AssetKind::McpServer => "mcps",
        AssetKind::Profile => "profiles",
    }
}

fn vault_copy_dir(src: &std::path::Path, dest: &std::path::Path) -> anyhow::Result<()> {
    if dest.exists() {
        std::fs::remove_dir_all(dest)?;
    }
    std::fs::create_dir_all(dest)?;
    for entry in walkdir::WalkDir::new(src).min_depth(1).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_symlink() {
            continue;
        }
        let rel = entry.path().strip_prefix(src)?;
        let target = dest.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

pub(super) fn install_asset(
    identity: &str,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let pkg = match core.registry.find_package_by_identity(identity) {
        Ok(Some(p)) => p,
        Ok(None) => {
            // Emit TaskFailed so the TUI clears its install spinner, then
            // return Err so CLI/TUI control flow detects the failure
            // (TaskFailed alone leaves the command's exit code at success).
            let error = format!("Asset '{}' not found in any vault", identity);
            sink.on_event(CoreEvent::TaskFailed {
                id: 0,
                error: error.clone(),
            });
            return Err(anyhow::anyhow!(error));
        }
        Err(e) => {
            let error = format!("Lookup failed: {}", e);
            sink.on_event(CoreEvent::TaskFailed {
                id: 0,
                error: error.clone(),
            });
            return Err(anyhow::anyhow!(error));
        }
    };

    let folder = vault_kind_folder(&pkg.kind);
    let dest = core.workspace_root.join(folder).join(&pkg.identity.name);
    if let Err(e) = vault_copy_dir(&pkg.path, &dest) {
        let error = format!("Failed to copy '{}' to vault: {}", identity, e);
        sink.on_event(CoreEvent::TaskFailed {
            id: 0,
            error: error.clone(),
        });
        return Err(anyhow::anyhow!(error));
    }

    sink.on_event(CoreEvent::TaskCompleted {
        id: 0,
        message: format!("'{}' added to vault {}/", pkg.identity.name, folder),
    });
    Ok(CoreOutcome::Ok)
}

pub(super) fn remove_asset(
    identity: &str,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let pkg = match core.registry.find_package_by_identity(identity) {
        Ok(Some(p)) => p,
        Ok(None) => {
            let error = format!("Asset '{}' not found", identity);
            sink.on_event(CoreEvent::TaskFailed {
                id: 0,
                error: error.clone(),
            });
            return Err(anyhow::anyhow!(error));
        }
        Err(e) => {
            let error = format!("Lookup failed: {}", e);
            sink.on_event(CoreEvent::TaskFailed {
                id: 0,
                error: error.clone(),
            });
            return Err(anyhow::anyhow!(error));
        }
    };

    let folder = vault_kind_folder(&pkg.kind);
    let dest = core.workspace_root.join(folder).join(&pkg.identity.name);
    if dest.exists() {
        if let Err(e) = std::fs::remove_dir_all(&dest) {
            let error = format!("Failed to remove '{}' from vault: {}", identity, e);
            sink.on_event(CoreEvent::TaskFailed {
                id: 0,
                error: error.clone(),
            });
            return Err(anyhow::anyhow!(error));
        }
    }

    sink.on_event(CoreEvent::TaskCompleted {
        id: 0,
        message: format!("'{}' removed from vault {}/", pkg.identity.name, folder),
    });
    Ok(CoreOutcome::Ok)
}
