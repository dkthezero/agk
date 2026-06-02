use crate::domain::identity::AssetIdentity;
use serde::{Deserialize, Serialize};

/// Tag indicating whether an installed asset is team-mandated or personal.
/// Stored as `source = "team"` or `source = "personal"` in config.toml.
/// When absent, defaults to "personal" (backward compatible).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AssetSource {
    #[default]
    #[serde(rename = "personal")]
    Personal,
    #[serde(rename = "team")]
    Team,
}

/// Intermediate serde type for `[<id>.vault]` and `[<id>.skills]` / `[<id>.instructions]`
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct VaultSection {
    pub vault: Option<super::VaultConfig>,
    pub skills: Option<AssetBucket>,
    pub instructions: Option<AssetBucket>,
    pub mcps: Option<AssetBucket>,
    pub profiles: Option<AssetBucket>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AssetBucket {
    pub items: Vec<String>, // "[name:version:sha10]" strings
}

impl super::ConfigFile {
    pub fn installed_skills(&self, vault_id: &str) -> Vec<AssetIdentity> {
        self.vault_defs
            .get(vault_id)
            .and_then(|s| s.skills.as_ref())
            .map(|b| {
                b.items
                    .iter()
                    .filter_map(|s| super::parse_identity(s))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn installed_instructions(&self, vault_id: &str) -> Vec<AssetIdentity> {
        self.vault_defs
            .get(vault_id)
            .and_then(|s| s.instructions.as_ref())
            .map(|b| {
                b.items
                    .iter()
                    .filter_map(|s| super::parse_identity(s))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn is_skill_installed(&self, vault_id: &str, name: &str) -> bool {
        self.installed_skills(vault_id)
            .iter()
            .any(|id| id.name == name)
    }

    pub fn is_instruction_installed(&self, vault_id: &str, name: &str) -> bool {
        self.installed_instructions(vault_id)
            .iter()
            .any(|id| id.name == name)
    }

    pub fn installed_skill_hash(&self, vault_id: &str, name: &str) -> Option<String> {
        self.installed_skills(vault_id)
            .into_iter()
            .find(|id| id.name == name)
            .map(|id| id.sha10)
    }

    pub fn installed_instruction_hash(&self, vault_id: &str, name: &str) -> Option<String> {
        self.installed_instructions(vault_id)
            .into_iter()
            .find(|id| id.name == name)
            .map(|id| id.sha10)
    }

    pub fn installed_mcps(&self, vault_id: &str) -> Vec<AssetIdentity> {
        self.vault_defs
            .get(vault_id)
            .and_then(|s| s.mcps.as_ref())
            .map(|b| {
                b.items
                    .iter()
                    .filter_map(|s| super::parse_identity(s))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn installed_profiles(&self, vault_id: &str) -> Vec<AssetIdentity> {
        self.vault_defs
            .get(vault_id)
            .and_then(|s| s.profiles.as_ref())
            .map(|b| {
                b.items
                    .iter()
                    .filter_map(|s| super::parse_identity(s))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn is_mcp_installed(&self, vault_id: &str, name: &str) -> bool {
        self.installed_mcps(vault_id)
            .iter()
            .any(|id| id.name == name)
    }

    pub fn is_profile_installed(&self, vault_id: &str, name: &str) -> bool {
        self.installed_profiles(vault_id)
            .iter()
            .any(|id| id.name == name)
    }

    pub fn installed_mcp_hash(&self, vault_id: &str, name: &str) -> Option<String> {
        self.installed_mcps(vault_id)
            .into_iter()
            .find(|id| id.name == name)
            .map(|id| id.sha10)
    }

    pub fn installed_profile_hash(&self, vault_id: &str, name: &str) -> Option<String> {
        self.installed_profiles(vault_id)
            .into_iter()
            .find(|id| id.name == name)
            .map(|id| id.sha10)
    }

    pub fn has_installed_assets(&self, vault_id: &str) -> bool {
        if let Some(section) = self.vault_defs.get(vault_id) {
            let s_count = section.skills.as_ref().map(|b| b.items.len()).unwrap_or(0);
            let i_count = section
                .instructions
                .as_ref()
                .map(|b| b.items.len())
                .unwrap_or(0);
            let m_count = section.mcps.as_ref().map(|b| b.items.len()).unwrap_or(0);
            let p_count = section
                .profiles
                .as_ref()
                .map(|b| b.items.len())
                .unwrap_or(0);
            s_count + i_count + m_count + p_count > 0
        } else {
            false
        }
    }

    pub fn find_profile(&self, name: &str) -> Option<&super::Profile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    pub fn remove_profile(&mut self, name: &str) -> bool {
        let before = self.profiles.len();
        self.profiles.retain(|p| p.name != name);
        self.profiles.len() < before
    }

    /// Remove a skill by name from the specified vault section.
    /// Returns `true` if a skill was actually removed.
    pub fn remove_skill_installed(&mut self, vault_id: &str, name: &str) -> bool {
        if let Some(section) = self.vault_defs.get_mut(vault_id) {
            if let Some(ref mut bucket) = section.skills {
                let before = bucket.items.len();
                bucket.items.retain(|item| {
                    super::parse_identity(item)
                        .map(|id| id.name != name)
                        .unwrap_or(true)
                });
                bucket.items.len() < before
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Remove an MCP by name from the specified vault section.
    /// Returns `true` if an MCP was actually removed.
    pub fn remove_mcp_installed(&mut self, vault_id: &str, name: &str) -> bool {
        if let Some(section) = self.vault_defs.get_mut(vault_id) {
            if let Some(ref mut bucket) = section.mcps {
                let before = bucket.items.len();
                bucket.items.retain(|item| {
                    super::parse_identity(item)
                        .map(|id| id.name != name)
                        .unwrap_or(true)
                });
                bucket.items.len() < before
            } else {
                false
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_source_default_is_personal() {
        let source: AssetSource = Default::default();
        assert_eq!(source, AssetSource::Personal);
    }

    #[test]
    fn asset_source_serializes_to_string() {
        assert_eq!(
            serde_json::to_string(&AssetSource::Team).unwrap(),
            "\"team\""
        );
        assert_eq!(
            serde_json::to_string(&AssetSource::Personal).unwrap(),
            "\"personal\""
        );
    }
}
