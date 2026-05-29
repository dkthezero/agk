use crate::domain::config::VaultConfig;

/// A vault to attach as part of `apply`.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyVault {
    pub id: String,
    pub config: VaultConfig,
}

/// Payload for [`CoreCommand::ApplyConfig`].
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyConfigInput {
    pub source_url: String,
    pub vaults: Vec<ApplyVault>,
    pub providers: Vec<String>,
    pub profiles: Vec<crate::domain::config::Profile>,
}

impl ApplyConfigInput {
    pub fn from_url(url: impl Into<String>) -> Self {
        Self {
            source_url: url.into(),
            vaults: Vec::new(),
            providers: Vec::new(),
            profiles: Vec::new(),
        }
    }

    pub fn with_vault(mut self, id: impl Into<String>, config: VaultConfig) -> Self {
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

    pub fn with_profile(mut self, profile: crate::domain::config::Profile) -> Self {
        self.profiles.push(profile);
        self
    }
}
