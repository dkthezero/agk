use crate::domain::identity::AssetIdentity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VaultKind {
    Local,
    Github,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalVaultSource {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GithubVaultSource {
    pub repo: String,
    pub r#ref: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClawHubVaultSource {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum VaultConfig {
    Local(LocalVaultSource),
    Github(GithubVaultSource),
    Clawhub(ClawHubVaultSource),
}

/// Key for tracking checked/installed items in AppState.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetKey {
    pub name: String,
    pub vault_id: String,
}

impl AssetKey {
    #[allow(dead_code)]
    pub fn new(name: impl Into<String>, vault_id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            vault_id: vault_id.into(),
        }
    }
}

fn default_version() -> u32 {
    1
}

/// Full config.toml schema — one instance per scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub vaults: Vec<String>,
    #[serde(default)]
    pub providers: Vec<String>,
    /// Vault definitions keyed by vault id, stored as `[<id>.vault]`
    #[serde(default, flatten)]
    pub vault_defs: HashMap<String, VaultSection>,
    #[serde(default)]
    pub provider_roots: HashMap<String, String>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            version: 1,
            vaults: Vec::new(),
            providers: Vec::new(),
            vault_defs: HashMap::new(),
            provider_roots: HashMap::new(),
        }
    }
}

/// Intermediate serde type for `[<id>.vault]` and `[<id>.skills]` / `[<id>.instructions]`
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct VaultSection {
    pub vault: Option<VaultConfig>,
    pub skills: Option<AssetBucket>,
    pub instructions: Option<AssetBucket>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AssetBucket {
    pub items: Vec<String>, // "[name:version:sha10]" strings
}

impl ConfigFile {
    pub fn installed_skills(&self, vault_id: &str) -> Vec<AssetIdentity> {
        self.vault_defs
            .get(vault_id)
            .and_then(|s| s.skills.as_ref())
            .map(|b| b.items.iter().filter_map(|s| parse_identity(s)).collect())
            .unwrap_or_default()
    }

    pub fn installed_instructions(&self, vault_id: &str) -> Vec<AssetIdentity> {
        self.vault_defs
            .get(vault_id)
            .and_then(|s| s.instructions.as_ref())
            .map(|b| b.items.iter().filter_map(|s| parse_identity(s)).collect())
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

    pub fn has_installed_assets(&self, vault_id: &str) -> bool {
        if let Some(section) = self.vault_defs.get(vault_id) {
            let s_count = section.skills.as_ref().map(|b| b.items.len()).unwrap_or(0);
            let i_count = section
                .instructions
                .as_ref()
                .map(|b| b.items.len())
                .unwrap_or(0);
            s_count + i_count > 0
        } else {
            false
        }
    }

    /// Validate the configuration for common errors, such as stray keys in vault_defs
    /// resulting from serde(flatten).
    pub fn validate(&self) -> anyhow::Result<()> {
        for (id, section) in &self.vault_defs {
            if section.vault.is_none() && section.skills.is_none() && section.instructions.is_none()
            {
                anyhow::bail!(
                    "Unknown top-level field or empty vault definition in config: '{}'",
                    id
                );
            }
        }
        Ok(())
    }
}

/// Parse "[name:version:sha10]" into AssetIdentity. Returns None on malformed input.
pub fn parse_identity(s: &str) -> Option<AssetIdentity> {
    let inner = s.strip_prefix('[')?.strip_suffix(']')?;
    let parts: Vec<&str> = inner.splitn(3, ':').collect();
    if parts.len() != 3 {
        return None;
    }
    let version = if parts[1] == "--" {
        None
    } else {
        Some(parts[1].to_string())
    };
    Some(AssetIdentity::new(parts[0], version, parts[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_key_eq_and_hash() {
        let a = AssetKey::new("my-skill", "workspace");
        let b = AssetKey::new("my-skill", "workspace");
        assert_eq!(a, b);
        let mut set = std::collections::HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn parse_identity_with_version() {
        let id = parse_identity("[web-tool:1.2.0:a13c9ef042]").unwrap();
        assert_eq!(id.name, "web-tool");
        assert_eq!(id.version, Some("1.2.0".to_string()));
        assert_eq!(id.sha10, "a13c9ef042");
    }

    #[test]
    fn parse_identity_without_version() {
        let id = parse_identity("[local-script:--:9ac00ff113]").unwrap();
        assert_eq!(id.name, "local-script");
        assert!(id.version.is_none());
    }

    #[test]
    fn parse_identity_malformed_returns_none() {
        assert!(parse_identity("bad-input").is_none());
        assert!(parse_identity("[only:two]").is_none());
    }

    #[test]
    fn config_file_default_is_empty() {
        let c = ConfigFile::default();
        assert!(c.vaults.is_empty());
        assert!(c.providers.is_empty());
    }

    #[test]
    fn clawhub_vault_config_round_trip() {
        let toml_str = r#"
version = 1
vaults = ["clawhub"]

[clawhub.vault]
type = "clawhub"
"#;
        let config: ConfigFile = toml::from_str(toml_str).unwrap();
        assert!(config.vaults.contains(&"clawhub".to_string()));
        let section = config.vault_defs.get("clawhub").unwrap();
        assert!(matches!(
            section.vault,
            Some(VaultConfig::Clawhub(ClawHubVaultSource {}))
        ));
        let serialized = toml::to_string(&config).unwrap();
        assert!(serialized.contains("type = \"clawhub\""));
    }

    #[test]
    fn is_skill_installed_true_when_present() {
        let mut config = ConfigFile::default();
        config.vault_defs.insert(
            "workspace".to_string(),
            VaultSection {
                vault: None,
                skills: Some(AssetBucket {
                    items: vec!["[my-skill:--:0000000000]".to_string()],
                }),
                instructions: None,
            },
        );
        assert!(config.is_skill_installed("workspace", "my-skill"));
        assert!(!config.is_skill_installed("workspace", "other-skill"));
    }

    #[test]
    fn provider_roots_toml_round_trip() {
        let mut config = ConfigFile::default();
        config
            .provider_roots
            .insert("opencode".to_string(), ".agents".to_string());
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("provider_roots"));
        assert!(toml_str.contains("opencode"));
        assert!(toml_str.contains(".agents"));

        let loaded: ConfigFile = toml::from_str(&toml_str).unwrap();
        assert_eq!(
            loaded.provider_roots.get("opencode"),
            Some(&".agents".to_string())
        );
    }
}
