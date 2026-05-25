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
    /// Source path or URL for this vault (PR #5)
    pub source_path: String,
}

impl VaultEntry {
    pub fn counts_label(&self) -> String {
        format!(
            "{}/{}s  {}/{}i",
            self.installed_skills,
            self.available_skills,
            self.installed_instructions,
            self.available_instructions,
        )
    }
}

/// Display-only struct for the Providers tab.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderEntry {
    pub id: String,
    pub name: String,
    pub active: bool,
}

/// Display-only struct for the Profiles tab.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileEntry {
    pub name: String,
    pub provider_id: String,
    pub skills: Vec<String>,
    pub mcps: Vec<String>,
}
