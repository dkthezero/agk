/// Payload for [`CoreCommand::AttachVault`].
#[derive(Debug, Clone, PartialEq)]
pub struct AttachVaultInput {
    pub vault_id: String,
    pub config: crate::domain::config::VaultConfig,
    pub scope: crate::domain::scope::Scope,
}
