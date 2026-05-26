/// Lightweight declarative configuration for `agk apply`.
///
/// This is the serialisable intent that a team config file or CLI
/// argument produces.  It is **not** the full runtime state — it is a
/// diff-like request that the `apply_config` use-case merges into the
/// current [`crate::domain::config::ConfigFile`].
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyConfig {
    pub source: String,
    pub vaults: Vec<ApplyVault>,
    pub providers: Vec<String>,
    pub profiles: Vec<crate::domain::profile::Profile>,
}

impl Default for ApplyConfig {
    fn default() -> Self {
        Self {
            source: String::new(),
            vaults: Vec::new(),
            providers: Vec::new(),
            profiles: Vec::new(),
        }
    }
}

/// A vault to attach as part of `apply`.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyVault {
    pub id: String,
    pub config: crate::domain::config::VaultConfig,
}

impl ApplyConfig {
    pub fn from_source(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            ..Default::default()
        }
    }

    pub fn with_vault(
        mut self,
        id: impl Into<String>,
        config: crate::domain::config::VaultConfig,
    ) -> Self {
        self.vaults.push(ApplyVault {
            id: id.into(),
            config,
        });
        self
    }

    pub fn with_provider(mut self, id: impl Into<String>) -> Self {
        self.providers.push(id.into());
        self
    }

    pub fn with_profile(mut self, profile: crate::domain::profile::Profile) -> Self {
        self.profiles.push(profile);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_adds_vault_and_provider() {
        let cfg = ApplyConfig::from_source("https://example.com/team.yaml")
            .with_vault(
                "team-skills",
                crate::domain::config::VaultConfig::Local(
                    crate::domain::config::LocalVaultSource {
                        path: "/tmp".into(),
                    },
                ),
            )
            .with_provider("opencode");

        assert_eq!(cfg.source, "https://example.com/team.yaml");
        assert_eq!(cfg.vaults.len(), 1);
        assert_eq!(cfg.providers, vec!["opencode"]);
    }

    #[test]
    fn default_is_empty() {
        let cfg = ApplyConfig::default();
        assert!(cfg.vaults.is_empty());
        assert!(cfg.providers.is_empty());
        assert!(cfg.profiles.is_empty());
    }
}
