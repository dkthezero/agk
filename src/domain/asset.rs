use crate::domain::identity::AssetIdentity;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum AssetKind {
    Skill,
    Instruction,
    McpServer,
    Profile,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackTarget {
    ClaudeDesktop,
    Firebender,
    Tarball,
}

/// Metadata from ClawHub for remote packages.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RemoteMetadata {
    pub owner: String,
    pub summary: String,
    pub downloads: u64,
    pub stars: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScannedPackage {
    pub identity: AssetIdentity,
    pub path: PathBuf,
    pub vault_id: String,
    pub kind: AssetKind,
    pub is_remote: bool,
    pub remote_meta: Option<RemoteMetadata>,
    /// Dependencies declared in SKILL.md frontmatter (meta-skill support)
    pub requires: Vec<String>,
    /// Optional dependencies that don't fail if missing
    pub requires_optional: Vec<String>,
    /// Parsed frontmatter metadata for display in detail panels
    pub author: Option<String>,
    pub description: Option<String>,
    /// When `true`, the `evals` sub-folder is copied during install.
    /// TUI keeps this `false`; headless CLI sets it via `--evals`.
    pub include_evals: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_kind_clone() {
        let k = AssetKind::Skill;
        assert_eq!(k.clone(), AssetKind::Skill);
    }

    #[test]
    fn asset_kind_eq() {
        assert_ne!(AssetKind::Skill, AssetKind::Instruction);
    }

    #[test]
    fn scanned_package_name_via_identity() {
        let pkg = ScannedPackage {
            identity: AssetIdentity::new("my-skill", None, "abc1234567"),
            path: PathBuf::from("/skills/my-skill"),
            vault_id: "workspace".to_string(),
            kind: AssetKind::Skill,
            is_remote: false,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
            include_evals: false,
        };
        assert_eq!(pkg.identity.name, "my-skill");
        assert_eq!(pkg.vault_id, "workspace");
    }

    #[test]
    fn scanned_package_default_not_remote() {
        let pkg = ScannedPackage {
            identity: AssetIdentity::new("my-skill", None, "abc1234567"),
            path: PathBuf::from("/skills/my-skill"),
            vault_id: "workspace".to_string(),
            kind: AssetKind::Skill,
            is_remote: false,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
            include_evals: false,
        };
        assert!(!pkg.is_remote);
    }

    #[test]
    fn scanned_package_remote_flag() {
        let pkg = ScannedPackage {
            identity: AssetIdentity::new("remote-skill", None, "0000000000"),
            path: PathBuf::new(),
            vault_id: "clawhub".to_string(),
            kind: AssetKind::Skill,
            is_remote: true,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
            include_evals: false,
        };
        assert!(pkg.is_remote);
    }
}
