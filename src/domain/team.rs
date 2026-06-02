use serde::{Deserialize, Serialize};
use crate::domain::asset::AssetKind;

fn default_branch() -> String {
    "main".to_string()
}

fn default_kind() -> AssetKind {
    AssetKind::Skill
}

/// Team membership configuration stored in .agk/team.toml
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TeamConfig {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default)]
    pub vaults: Vec<TeamVault>,
    #[serde(default)]
    pub requirements: Vec<TeamRequirement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamVault {
    pub identity: String,
    #[serde(rename = "type")]
    pub vault_type: String,
    pub url: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamRequirement {
    pub identity: String,
    pub vault: String,
    #[serde(default = "default_kind")]
    pub kind: AssetKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_constraint: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_config_round_trip() {
        let config = TeamConfig {
            name: "my-team".to_string(),
            source: Some("https://github.com/org/team-repo".to_string()),
            branch: Some("develop".to_string()),
            vaults: vec![TeamVault {
                identity: "shared-vault".to_string(),
                vault_type: "github".to_string(),
                url: "https://github.com/org/shared-skills".to_string(),
                branch: "main".to_string(),
                path: Some("skills/".to_string()),
            }],
            requirements: vec![TeamRequirement {
                identity: "core-skill".to_string(),
                vault: "shared-vault".to_string(),
                kind: AssetKind::Skill,
                version_constraint: Some(">=1.0.0".to_string()),
            }],
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let loaded: TeamConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(loaded, config);
    }

    #[test]
    fn team_config_defaults() {
        let toml_str = r#"
name = "test"
"#;
        let config: TeamConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.name, "test");
        assert!(config.source.is_none());
        assert!(config.branch.is_none());
        assert!(config.vaults.is_empty());
        assert!(config.requirements.is_empty());
    }

    #[test]
    fn team_config_with_vaults() {
        let toml_str = r#"
name = "enterprise-team"
source = "https://github.com/acme/team-config"

[[vaults]]
identity = "shared-lib"
type = "github"
url = "https://github.com/acme/shared-lib"
branch = "release"

[[vaults]]
identity = "internal"
type = "github"
url = "https://github.com/acme/internal-skills"

[[requirements]]
identity = "security-scan"
vault = "shared-lib"
kind = "skill"

[[requirements]]
identity = "compliance-check"
vault = "shared-lib"
kind = "instruction"
version_constraint = ">=2.0.0"
"#;
        let config: TeamConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.name, "enterprise-team");
        assert_eq!(config.source.as_deref(), Some("https://github.com/acme/team-config"));
        assert_eq!(config.vaults.len(), 2);

        assert_eq!(config.vaults[0].identity, "shared-lib");
        assert_eq!(config.vaults[0].vault_type, "github");
        assert_eq!(config.vaults[0].branch, "release");

        assert_eq!(config.vaults[1].identity, "internal");
        assert_eq!(config.vaults[1].branch, "main"); // default

        assert_eq!(config.requirements.len(), 2);
        assert_eq!(config.requirements[0].identity, "security-scan");
        assert_eq!(config.requirements[0].kind, AssetKind::Skill);
        assert!(config.requirements[0].version_constraint.is_none());

        assert_eq!(config.requirements[1].identity, "compliance-check");
        assert_eq!(config.requirements[1].kind, AssetKind::Instruction);
        assert_eq!(
            config.requirements[1].version_constraint.as_deref(),
            Some(">=2.0.0")
        );
    }
}