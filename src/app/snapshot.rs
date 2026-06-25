/// View models derived from the workspace configuration and scan results.
/// These types live in `app/` (not `domain/`) because they contain UI-oriented
/// data (counts, display strings) rather than pure domain identity.
/// Display-only struct for the Vaults tab.
#[derive(Debug, Clone, PartialEq)]
pub struct VaultEntry {
    pub id: String,
    pub kind: String,
    pub enabled: bool,
    pub installed_skills: usize,
    pub available_skills: usize,
    pub installed_instructions: usize,
    pub available_instructions: usize,
    pub installed_profiles: usize,
    pub available_profiles: usize,
    pub installed_mcps: usize,
    pub available_mcps: usize,
    /// Source path or URL for this vault (PR #5)
    pub source_path: String,
    /// Whether this vault connects to a GitHub Enterprise Server instance
    pub is_ghes: bool,
    /// The enterprise URL if this is a GHES vault
    pub enterprise_url: Option<String>,
}

impl VaultEntry {
    pub fn counts_label(&self) -> String {
        format!(
            "{}/{}s  {}/{}i  {}/{}p  {}/{}m",
            self.installed_skills,
            self.available_skills,
            self.installed_instructions,
            self.available_instructions,
            self.installed_profiles,
            self.available_profiles,
            self.installed_mcps,
            self.available_mcps,
        )
    }
}

/// Display-only struct for the Providers tab.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderEntry {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub supports_mcp: bool,
}

/// Display-only struct for the Profiles tab.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileEntry {
    pub name: String,
    pub provider_id: String,
    pub skills: Vec<crate::domain::profile::ProfileAssetRef>,
    pub mcps: Vec<crate::domain::profile::ProfileAssetRef>,
    /// True if this profile differs from its vault source.
    pub has_drift: bool,
}

/// Display-only struct for the context list.
/// Carries the lightweight data needed by the CLI/JSON/TUI renderers so all
/// three render `agk context list` identically.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextEntry {
    pub name: String,
    pub display_name: Option<String>,
    pub is_active: bool,
    pub environment: Option<String>,
    pub vaults: Vec<String>,
    pub profiles: Vec<String>,
    pub providers: Vec<String>,
}

/// A profile package discovered in a vault but not yet registered in config.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredProfile {
    pub name: String,
    pub vault_id: String,
    pub description: Option<String>,
}

/// An MCP server package discovered in a vault but not yet registered.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredMcp {
    pub name: String,
    pub vault_id: String,
    pub description: Option<String>,
}
