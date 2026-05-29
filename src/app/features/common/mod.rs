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
            },
        );
        config.vault_defs.insert(
            "b".to_string(),
            VaultSection {
                vault: None,
                skills: Some(AssetBucket { items: vec![] }),
                instructions: None,
            },
        );
        config.vault_defs.insert(
            "c".to_string(),
            VaultSection {
                vault: None,
                skills: None,
                instructions: Some(AssetBucket {
                    items: vec!["[i:--:0000000000]".to_string()],
                }),
            },
        );

        prune_empty_vault_defs(&mut config);

        assert!(config.vault_defs.contains_key("a"));
        assert!(!config.vault_defs.contains_key("b"));
        assert!(config.vault_defs.contains_key("c"));
    }
}
