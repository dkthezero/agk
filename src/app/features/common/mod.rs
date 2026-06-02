use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::domain::config::ConfigFile;

/// Remove empty vault sections / asset buckets so the TOML stays clean.
pub fn prune_empty_vault_defs(config: &mut ConfigFile) {
    config.vault_defs.retain(|_id, section| {
        let has_vault = section.vault.is_some();
        let has_skills = section
            .skills
            .as_ref()
            .map(|b| !b.items.is_empty())
            .unwrap_or(false);
        let has_instructions = section
            .instructions
            .as_ref()
            .map(|b| !b.items.is_empty())
            .unwrap_or(false);
        has_vault || has_skills || has_instructions
    });
}

/// Dispatch common/workspace-related [`CoreCommand`] variants.
/// Returns `Some(result)` if the command was handled, `None` otherwise.
pub fn dispatch(
    cmd: &CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::CleanWorkspace { global } => {
            let dir = if *global {
                crate::domain::paths::global_config_root()
            } else {
                core.workspace_root.join(".agk")
            };
            if dir.exists() {
                if let Err(e) = std::fs::remove_dir_all(&dir) {
                    return Some(Err(e.into()));
                }
                sink.on_event(crate::app::event::CoreEvent::Info(format!(
                    "Cleaned up {}",
                    dir.display()
                )));
            } else {
                sink.on_event(crate::app::event::CoreEvent::Info(format!(
                    "Nothing to clean at {}",
                    dir.display()
                )));
            }
            Some(Ok(CoreOutcome::Ok))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{AssetBucket, VaultSection};

    #[test]
    fn prune_empty_vault_defs_keeps_nonempty() {
        let mut config = ConfigFile::default();
        config.vault_defs.insert(
            "a".to_string(),
            VaultSection {
                vault: Some(crate::domain::config::VaultConfig::Local(
                    crate::domain::config::LocalVaultSource {
                        path: "/tmp".into(),
                    },
                )),
                skills: None,
                instructions: None,
                mcps: None,
                profiles: None,
            },
        );
        config.vault_defs.insert(
            "b".to_string(),
            VaultSection {
                vault: None,
                skills: Some(AssetBucket { items: vec![], source: None }),
                instructions: None,
                mcps: None,
                profiles: None,
            },
        );
        config.vault_defs.insert(
            "c".to_string(),
            VaultSection {
                vault: None,
                skills: None,
                instructions: Some(AssetBucket {
                    items: vec!["[i:--:0000000000]".to_string()],
                    source: None,
                }),
                mcps: None,
                profiles: None,
            },
        );

        prune_empty_vault_defs(&mut config);

        assert!(config.vault_defs.contains_key("a"));
        assert!(!config.vault_defs.contains_key("b"));
        assert!(config.vault_defs.contains_key("c"));
    }
}
