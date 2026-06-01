#[derive(Debug, Clone, PartialEq)]
pub enum ListMode {
    Normal,
    Searching,
    AttachVault,
    AttachVaultBranch,
    AttachVaultPath,
    AttachVaultName,
    ConfirmDetachVault,
    ConfirmClawHubInstall,
    ConfirmDeactivateLastProvider,
    /// MCP server registration modal sub-steps
    RegisterMcpStepName,
    RegisterMcpStepCommand,
    RegisterMcpStepArgs,
    RegisterMcpStepTransport,
    RegisterMcpStepDescription,
    ConfirmMcpTest,
    SelectProviderRoot {
        provider_id: String,
        options: Vec<(String, String)>,
        selected: usize,
    },
    /// Profile creation wizard (provider-specific step stack)
    ProfileWizard,
    ConfirmDeleteProfile,
    /// Profile editor modal (F3 on Profile tab)
    EditProfile,
}
