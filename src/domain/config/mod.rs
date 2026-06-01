pub mod vault_section;

use crate::domain::identity::AssetIdentity;
use crate::domain::profile::ProfileAssetRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export vault_section types so external callers don't need to change imports
pub use vault_section::{AssetBucket, VaultSection};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enterprise_url: Option<String>,
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

/// Profile definition stored in config.toml.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Profile {
    pub name: String,
    pub provider_id: String,
    #[serde(default)]
    pub scope: String,
    /// Vault-aware skill references. Backward-compatible: deserializes from
    /// both `skills = ["name"]` (legacy) and `[[profiles.skills]]` tables.
    #[serde(default)]
    pub skills: Vec<ProfileAssetRef>,
    /// Vault-aware MCP references. Backward-compatible: deserializes from
    /// both `mcps = ["name"]` (legacy) and `[[profiles.mcps]]` tables.
    #[serde(default)]
    pub mcps: Vec<ProfileAssetRef>,
    /// Vault-aware instruction references.
    #[serde(default)]
    pub instructions: Vec<ProfileAssetRef>,
    /// Optional tool references for providers that support tool selection.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_refs: Vec<String>,
    /// Optional permission mode for providers that support it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// Optional path to a prompt-overlay / agent markdown file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_overlay_path: Option<String>,
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
    #[serde(default)]
    pub profiles: Vec<Profile>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            version: 1,
            vaults: Vec::new(),
            providers: Vec::new(),
            vault_defs: HashMap::new(),
            provider_roots: HashMap::new(),
            profiles: Vec::new(),
        }
    }
}

impl ConfigFile {
    pub fn validate(&self) -> anyhow::Result<()> {
        for (id, section) in &self.vault_defs {
            if section.vault.is_none()
                && section.skills.is_none()
                && section.instructions.is_none()
                && section.mcps.is_none()
                && section.profiles.is_none()
            {
                anyhow::bail!(
                    "Unknown top-level field or empty vault definition in config: '{}'",
                    id
                );
            }
        }

        for profile in &self.profiles {
            if profile.name.is_empty()
                || profile.name.contains('/')
                || profile.name.contains('\\')
                || profile.name.contains('\u{0000}')
                || profile.name.contains(':')
                || profile.name == "."
                || profile.name == ".."
                || profile.name.starts_with("..")
            {
                anyhow::bail!(
                    "Profile '{}' contains invalid filesystem characters",
                    profile.name
                );
            }
        }

        Ok(())
    }

    /// Normalize profiles so that every `ProfileAssetRef` has a non-empty `vault`.
    ///
    /// Old configs may contain `skills = ["name"]` which deserializes with
    /// `vault: "auto"` via the custom deserializer, but profiles created
    /// programmatically might have empty vault strings. This method ensures
    /// all refs use `"auto"` as the default, making the serialized output
    /// consistently use the structured `[[profiles.skills]]` format.
    ///
    /// This is idempotent — already-migrated configs are unchanged.
    pub fn migrate_profiles(&mut self) {
        for profile in &mut self.profiles {
            for skill in &mut profile.skills {
                if skill.vault.is_empty() {
                    skill.vault = "auto".to_string();
                }
            }
            for mcp in &mut profile.mcps {
                if mcp.vault.is_empty() {
                    mcp.vault = "auto".to_string();
                }
            }
            for instr in &mut profile.instructions {
                if instr.vault.is_empty() {
                    instr.vault = "auto".to_string();
                }
            }
        }
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
                mcps: None,
                profiles: None,
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

    #[test]
    fn profile_toml_round_trip() {
        let mut config = ConfigFile::default();
        config.profiles.push(Profile {
            name: "opencode-dev".to_string(),
            provider_id: "opencode".to_string(),
            scope: "workspace".to_string(),
            skills: vec![
                ProfileAssetRef::new("skill-a", "auto"),
                ProfileAssetRef::new("skill-b", "clawhub"),
            ],
            mcps: vec![ProfileAssetRef::new("mcp-server", "auto")],
            instructions: vec![],
            tool_refs: vec![],
            permission_mode: None,
            prompt_overlay_path: None,
        });
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("opencode-dev"));
        assert!(toml_str.contains("opencode"));

        let loaded: ConfigFile = toml::from_str(&toml_str).unwrap();
        assert_eq!(loaded.profiles.len(), 1);
        let p = &loaded.profiles[0];
        assert_eq!(p.name, "opencode-dev");
        assert_eq!(p.provider_id, "opencode");
        assert_eq!(p.skills[0].name, "skill-a");
        assert_eq!(p.skills[1].vault, "clawhub");
        assert_eq!(p.mcps[0].name, "mcp-server");
    }

    #[test]
    fn profile_backward_compatible_flat_skills() {
        let toml_str = r#"
[[profiles]]
name = "legacy"
provider_id = "opencode"
skills = ["rust-patterns", "docker"]
mcps = ["filesystem"]
"#;
        let loaded: ConfigFile = toml::from_str(toml_str).unwrap();
        assert_eq!(loaded.profiles.len(), 1);
        let p = &loaded.profiles[0];
        assert_eq!(p.name, "legacy");
        assert_eq!(p.skills.len(), 2);
        assert_eq!(p.skills[0].name, "rust-patterns");
        assert_eq!(p.skills[0].vault, "auto");
        assert_eq!(p.skills[1].name, "docker");
        assert_eq!(p.mcps.len(), 1);
        assert_eq!(p.mcps[0].name, "filesystem");
        assert_eq!(p.mcps[0].vault, "auto");
    }

    #[test]
    fn github_vault_source_round_trip_without_enterprise_url() {
        let toml_str = r#"
version = 1
vaults = ["gh-vault"]

[gh-vault.vault]
type = "github"
repo = "owner/repo"
ref = "main"
path = "skills/"
"#;
        let config: ConfigFile = toml::from_str(toml_str).unwrap();
        let section = config.vault_defs.get("gh-vault").unwrap();
        match &section.vault {
            Some(VaultConfig::Github(src)) => {
                assert_eq!(src.repo, "owner/repo");
                assert_eq!(src.r#ref, "main");
                assert_eq!(src.path, "skills/");
                assert!(src.enterprise_url.is_none());
            }
            _ => panic!("expected Github vault"),
        }
        // Serializing should NOT emit enterprise_url when None
        let serialized = toml::to_string(&config).unwrap();
        assert!(!serialized.contains("enterprise_url"));
    }

    #[test]
    fn github_vault_source_round_trip_with_enterprise_url() {
        let toml_str = r#"
version = 1
vaults = ["ghes-vault"]

[ghes-vault.vault]
type = "github"
repo = "owner/repo"
ref = "main"
path = "skills/"
enterprise_url = "https://github.example.com"
"#;
        let config: ConfigFile = toml::from_str(toml_str).unwrap();
        let section = config.vault_defs.get("ghes-vault").unwrap();
        match &section.vault {
            Some(VaultConfig::Github(src)) => {
                assert_eq!(src.repo, "owner/repo");
                assert_eq!(src.r#ref, "main");
                assert_eq!(src.path, "skills/");
                assert_eq!(
                    src.enterprise_url,
                    Some("https://github.example.com".to_string())
                );
            }
            _ => panic!("expected Github vault"),
        }
        // Serializing should preserve enterprise_url
        let serialized = toml::to_string(&config).unwrap();
        assert!(serialized.contains("enterprise_url"));
        let roundtripped: ConfigFile = toml::from_str(&serialized).unwrap();
        let section2 = roundtripped.vault_defs.get("ghes-vault").unwrap();
        assert_eq!(section2.vault, section.vault);
    }

    #[test]
    fn find_profile_returns_some() {
        let mut config = ConfigFile::default();
        config.profiles.push(Profile {
            name: "test".to_string(),
            provider_id: "opencode".to_string(),
            scope: "workspace".to_string(),
            skills: vec![],
            mcps: vec![],
            instructions: vec![],
            tool_refs: vec![],
            permission_mode: None,
            prompt_overlay_path: None,
        });
        assert!(config.find_profile("test").is_some());
        assert!(config.find_profile("missing").is_none());
    }

    #[test]
    fn remove_profile_deletes() {
        let mut config = ConfigFile::default();
        config.profiles.push(Profile {
            name: "a".to_string(),
            provider_id: "opencode".to_string(),
            scope: "workspace".to_string(),
            skills: vec![],
            mcps: vec![],
            instructions: vec![],
            tool_refs: vec![],
            permission_mode: None,
            prompt_overlay_path: None,
        });
        assert!(config.remove_profile("a"));
        assert!(!config.remove_profile("a"));
        assert!(config.profiles.is_empty());
    }

    #[test]
    fn migrate_old_flat_skills_to_profile_asset_ref() {
        let old_toml = r#"
version = 1
vaults = ["workspace"]
providers = ["claude-code"]

[[profiles]]
name = "web-app"
provider_id = "opencode"
skills = ["rust-patterns", "docker"]
mcps = ["filesystem"]
"#;
        let mut config: ConfigFile = toml::from_str(old_toml).unwrap();
        assert_eq!(config.profiles[0].skills.len(), 2);
        assert_eq!(config.profiles[0].skills[0].name, "rust-patterns");
        assert_eq!(config.profiles[0].skills[0].vault, "auto");

        config.migrate_profiles();

        let new_toml = toml::to_string_pretty(&config).unwrap();
        assert!(
            new_toml.contains("[[profiles.skills]]"),
            "Expected structured skills format in:\n{}",
            new_toml
        );
        assert!(
            new_toml.contains("vault = \"auto\""),
            "Expected vault field in:\n{}",
            new_toml
        );
        let reloaded: ConfigFile = toml::from_str(&new_toml).unwrap();
        assert_eq!(reloaded.profiles[0].skills.len(), 2);
        assert_eq!(reloaded.profiles[0].skills[0].name, "rust-patterns");
    }

    #[test]
    fn migrate_profiles_is_idempotent() {
        let mut config = ConfigFile::default();
        config.profiles.push(Profile {
            name: "dev".to_string(),
            provider_id: "opencode".to_string(),
            scope: "workspace".to_string(),
            skills: vec![
                ProfileAssetRef::new("rust-patterns", "auto"),
                ProfileAssetRef::new("docker", "clawhub"),
            ],
            mcps: vec![ProfileAssetRef::new("filesystem", "auto")],
            instructions: vec![],
            tool_refs: vec![],
            permission_mode: None,
            prompt_overlay_path: None,
        });
        let before = config.clone();
        config.migrate_profiles();
        assert_eq!(
            config, before,
            "migrate_profiles should be idempotent for already-migrated configs"
        );
    }

    #[test]
    fn migrate_empty_vault_defaults_to_auto() {
        let mut config = ConfigFile::default();
        config.profiles.push(Profile {
            name: "old".to_string(),
            provider_id: "claude-code".to_string(),
            scope: String::new(),
            skills: vec![ProfileAssetRef::new("skill-a", "")],
            mcps: vec![],
            instructions: vec![],
            tool_refs: vec![],
            permission_mode: None,
            prompt_overlay_path: None,
        });
        config.migrate_profiles();
        assert_eq!(
            config.profiles[0].skills[0].vault, "auto",
            "empty vault should be migrated to 'auto'"
        );
    }
}
