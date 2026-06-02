use crate::app::ports::ConfigStorePort;
use crate::app::ports::TeamConfigStorePort;
use crate::domain::config::ConfigFile;
use crate::domain::scope::Scope;
use crate::domain::team::TeamConfig;
use crate::infra::config::team_store::TeamTomlStore;
use crate::infra::config::toml_store::TomlConfigStore;
use anyhow::Result;
use std::path::Path;

/// A single diff entry comparing team requirements against installed state.
#[derive(Debug, Clone, PartialEq)]
pub enum DiffEntry {
    /// Required by team but not installed locally.
    Missing {
        identity: String,
        vault: String,
        kind: String,
    },
    /// Installed locally but not in team requirements.
    Extra {
        identity: String,
        vault: String,
        kind: String,
    },
    /// Installed but with a different version constraint.
    Outdated {
        identity: String,
        vault: String,
        kind: String,
        expected: String,
        actual: String,
    },
}

pub struct TeamDiffResult {
    pub entries: Vec<DiffEntry>,
}

impl TeamDiffResult {
    pub fn summary(&self) -> String {
        if self.entries.is_empty() {
            return "Team configuration is in sync — no differences found.".to_string();
        }
        let mut lines = Vec::new();
        for entry in &self.entries {
            match entry {
                DiffEntry::Missing {
                    identity,
                    vault,
                    kind,
                } => {
                    lines.push(format!(
                        "  MISSING  {} (kind: {}, vault: {})",
                        identity, kind, vault
                    ));
                }
                DiffEntry::Extra {
                    identity,
                    vault,
                    kind,
                } => {
                    lines.push(format!(
                        "  EXTRA    {} (kind: {}, vault: {})",
                        identity, kind, vault
                    ));
                }
                DiffEntry::Outdated {
                    identity,
                    vault,
                    kind,
                    expected,
                    actual,
                } => {
                    lines.push(format!(
                        "  OUTDATED {} (kind: {}, vault: {}) expected: {}, actual: {}",
                        identity, kind, vault, expected, actual
                    ));
                }
            }
        }
        format!(
            "Team diff ({} differences):\n{}",
            self.entries.len(),
            lines.join("\n")
        )
    }
}

/// Compare team requirements against the currently installed assets in `config.toml`.
///
/// - Missing: required by team but not installed
/// - Extra: installed from a team vault but not in requirements
/// - Outdated: installed but with a different version
pub fn team_diff(workspace_root: &Path) -> Result<TeamDiffResult> {
    let team_store = TeamTomlStore::new(workspace_root.to_path_buf());
    let team_config = team_store.load(Scope::Workspace)?;

    // If team config is empty/default, there's nothing to diff
    if team_config.name.is_empty() && team_config.requirements.is_empty() {
        return Ok(TeamDiffResult { entries: vec![] });
    }

    // Load the installed config
    let config_store = TomlConfigStore::standard(workspace_root);
    let installed_config = config_store.load(Scope::Workspace).unwrap_or_default();

    let entries = compute_diff(&team_config, &installed_config);

    Ok(TeamDiffResult { entries })
}

fn compute_diff(team: &TeamConfig, installed: &ConfigFile) -> Vec<DiffEntry> {
    let mut entries = Vec::new();

    // Collect all installed asset identities from team vaults
    let team_vault_ids: Vec<String> = team.vaults.iter().map(|v| v.identity.clone()).collect();
    let installed_from_team_vaults = collect_installed_from_vaults(installed, &team_vault_ids);

    // Check for missing requirements
    for req in &team.requirements {
        let installed_entry = installed_from_team_vaults
            .iter()
            .find(|(id, _)| *id == req.identity);
        if let Some((_, info)) = installed_entry {
            // Check version constraint mismatch
            if let (Some(expected), Some(actual)) = (&req.version_constraint, &info.version) {
                if expected != actual {
                    entries.push(DiffEntry::Outdated {
                        identity: req.identity.clone(),
                        vault: req.vault.clone(),
                        kind: format!("{:?}", req.kind).to_lowercase(),
                        expected: expected.clone(),
                        actual: actual.clone(),
                    });
                }
            }
        } else {
            entries.push(DiffEntry::Missing {
                identity: req.identity.clone(),
                vault: req.vault.clone(),
                kind: format!("{:?}", req.kind).to_lowercase(),
            });
        }
    }

    // Check for extra installed assets from team vaults not in requirements
    let required_identities: Vec<String> = team
        .requirements
        .iter()
        .map(|r| r.identity.clone())
        .collect();
    for (id, info) in &installed_from_team_vaults {
        if !required_identities.contains(id) {
            entries.push(DiffEntry::Extra {
                identity: id.clone(),
                vault: info.vault.clone(),
                kind: info.kind.clone(),
            });
        }
    }

    entries
}

struct InstalledAssetInfo {
    vault: String,
    kind: String,
    version: Option<String>,
}

fn collect_installed_from_vaults(
    config: &ConfigFile,
    vault_ids: &[String],
) -> Vec<(String, InstalledAssetInfo)> {
    let mut result = Vec::new();

    for vault_id in vault_ids {
        if let Some(section) = config.vault_defs.get(vault_id) {
            if let Some(ref bucket) = section.skills {
                for item in &bucket.items {
                    if let Some(identity) = parse_identity_from_item(item) {
                        result.push((
                            identity,
                            InstalledAssetInfo {
                                vault: vault_id.clone(),
                                kind: "skill".to_string(),
                                version: parse_version_from_item(item),
                            },
                        ));
                    }
                }
            }
            if let Some(ref bucket) = section.instructions {
                for item in &bucket.items {
                    if let Some(identity) = parse_identity_from_item(item) {
                        result.push((
                            identity,
                            InstalledAssetInfo {
                                vault: vault_id.clone(),
                                kind: "instruction".to_string(),
                                version: parse_version_from_item(item),
                            },
                        ));
                    }
                }
            }
            if let Some(ref bucket) = section.mcps {
                for item in &bucket.items {
                    if let Some(identity) = parse_identity_from_item(item) {
                        result.push((
                            identity,
                            InstalledAssetInfo {
                                vault: vault_id.clone(),
                                kind: "mcp".to_string(),
                                version: parse_version_from_item(item),
                            },
                        ));
                    }
                }
            }
            if let Some(ref bucket) = section.profiles {
                for item in &bucket.items {
                    if let Some(identity) = parse_identity_from_item(item) {
                        result.push((
                            identity,
                            InstalledAssetInfo {
                                vault: vault_id.clone(),
                                kind: "profile".to_string(),
                                version: parse_version_from_item(item),
                            },
                        ));
                    }
                }
            }
        }
    }

    result
}

/// Parse "[name:version:sha10]" format → extract name (identity).
fn parse_identity_from_item(item: &str) -> Option<String> {
    let item = item.trim_start_matches('[').trim_end_matches(']');
    let parts: Vec<&str> = item.split(':').collect();
    if parts.is_empty() {
        return None;
    }
    Some(parts[0].to_string())
}

/// Parse "[name:version:sha10]" format → extract version.
fn parse_version_from_item(item: &str) -> Option<String> {
    let item = item.trim_start_matches('[').trim_end_matches(']');
    let parts: Vec<&str> = item.split(':').collect();
    if parts.len() >= 2 && !parts[1].is_empty() {
        Some(parts[1].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::asset::AssetKind;
    use crate::domain::team::{TeamRequirement, TeamVault};

    fn make_team_config() -> TeamConfig {
        TeamConfig {
            name: "test-team".to_string(),
            source: None,
            branch: Some("main".to_string()),
            vaults: vec![TeamVault {
                identity: "shared".to_string(),
                vault_type: "github".to_string(),
                url: "https://github.com/org/skills".to_string(),
                branch: "main".to_string(),
                path: None,
            }],
            requirements: vec![
                TeamRequirement {
                    identity: "react-conventions".to_string(),
                    vault: "shared".to_string(),
                    kind: AssetKind::Skill,
                    version_constraint: None,
                },
                TeamRequirement {
                    identity: "security-scan".to_string(),
                    vault: "shared".to_string(),
                    kind: AssetKind::Instruction,
                    version_constraint: Some(">=2.0.0".to_string()),
                },
            ],
        }
    }

    #[test]
    fn diff_missing_requirements() {
        let team = make_team_config();
        let installed = ConfigFile::default(); // nothing installed

        let result = compute_diff(&team, &installed);
        assert!(result.len() >= 2); // both requirements are missing

        let missing: Vec<&DiffEntry> = result
            .iter()
            .filter(|e| matches!(e, DiffEntry::Missing { .. }))
            .collect();
        assert_eq!(missing.len(), 2);
    }

    #[test]
    fn diff_with_installed_matching() {
        let team = make_team_config();
        let mut installed = ConfigFile::default();
        // Add a matching installed skill
        let section = crate::domain::config::VaultSection {
            skills: Some(crate::domain::config::AssetBucket {
                items: vec!["react-conventions:1.0.0:abc123".to_string()],
                source: None,
            }),
            ..crate::domain::config::VaultSection::default()
        };
        installed.vault_defs.insert("shared".to_string(), section);

        let result = compute_diff(&team, &installed);
        // react-conventions is installed, security-scan is still missing
        let missing: Vec<&DiffEntry> = result
            .iter()
            .filter(|e| matches!(e, DiffEntry::Missing { .. }))
            .collect();
        assert_eq!(missing.len(), 1);
        assert!(
            matches!(&missing[0], DiffEntry::Missing { identity, .. } if identity == "security-scan")
        );
    }

    #[test]
    fn diff_empty_team_no_diffs() {
        let team = TeamConfig::default();
        let installed = ConfigFile::default();

        let result = compute_diff(&team, &installed);
        assert!(result.is_empty());
    }

    #[test]
    fn parse_identity_from_item_works() {
        assert_eq!(
            parse_identity_from_item("[my-skill:1.0.0:abc123]"),
            Some("my-skill".to_string())
        );
        assert_eq!(
            parse_identity_from_item("[my-skill::abc123]"),
            Some("my-skill".to_string())
        );
        assert_eq!(
            parse_identity_from_item("plain-name"),
            Some("plain-name".to_string())
        );
    }

    #[test]
    fn parse_version_from_item_works() {
        assert_eq!(
            parse_version_from_item("[my-skill:1.0.0:abc123]"),
            Some("1.0.0".to_string())
        );
        assert_eq!(parse_version_from_item("[my-skill::abc123]"), None);
    }

    #[test]
    fn diff_summary_empty() {
        let result = TeamDiffResult { entries: vec![] };
        assert!(result.summary().contains("in sync"));
    }

    #[test]
    fn diff_summary_with_entries() {
        let result = TeamDiffResult {
            entries: vec![DiffEntry::Missing {
                identity: "my-skill".to_string(),
                vault: "shared".to_string(),
                kind: "skill".to_string(),
            }],
        };
        assert!(result.summary().contains("MISSING"));
        assert!(result.summary().contains("1 differences"));
    }
}
