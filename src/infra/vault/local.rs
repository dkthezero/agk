use crate::app::ports::{FeatureSetPort, VaultPort};
use crate::domain::asset::ScannedPackage;
use crate::domain::hashing::compute_sha10;
use crate::domain::identity::AssetIdentity;
use crate::infra::feature;
use anyhow::Result;
use std::path::PathBuf;

pub struct LocalVaultAdapter {
    id: String,
    root: PathBuf,
}

impl LocalVaultAdapter {
    pub fn new(id: impl Into<String>, root: PathBuf) -> Self {
        Self {
            id: id.into(),
            root,
        }
    }
}

#[async_trait::async_trait]
impl VaultPort for LocalVaultAdapter {
    fn id(&self) -> &str {
        &self.id
    }
    fn kind_name(&self) -> &str {
        "local"
    }

    fn list_packages(&self, feature: &dyn FeatureSetPort) -> Result<Vec<ScannedPackage>> {
        let scan_root = self.root.join(feature.scan_root());
        if !scan_root.exists() {
            return Ok(Vec::new());
        }

        let mut packages = Vec::new();
        for entry in std::fs::read_dir(&scan_root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if !feature.is_package(&path) {
                continue;
            }
            let files = feature.hash_files(&path);
            let hashed: Vec<(std::path::PathBuf, Vec<u8>)> = files
                .into_iter()
                .map(|p| {
                    let bytes = std::fs::read(&p).unwrap_or_default();
                    (p, bytes)
                })
                .collect();
            let sha10 = compute_sha10(&hashed)?;
            let dir_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let version = feature.extract_version(&path);

            // Parse frontmatter for dependencies and metadata (meta-skill support)
            let mut requires = Vec::new();
            let mut requires_optional = Vec::new();
            let mut author = None;
            let mut description = None;
            let mut name = dir_name;
            if feature.asset_kind() == crate::domain::asset::AssetKind::Skill {
                if let Ok(content) = std::fs::read_to_string(path.join("SKILL.md")) {
                    if let Some(fm) = feature::extract_frontmatter(&content) {
                        requires = fm.requires;
                        requires_optional = fm.requires_optional;
                        author = fm.author;
                        description = fm.description;
                        if let Some(fm_name) = fm.name {
                            name = fm_name;
                        }
                    }
                }
            }

            let identity = AssetIdentity::new(name, version, sha10);

            packages.push(ScannedPackage {
                identity,
                path,
                vault_id: self.id.clone(),
                kind: feature.asset_kind(),
                is_remote: false,
                remote_meta: None,
                requires,
                requires_optional,
                author,
                description,
                include_evals: false,
            });
        }

        packages.sort_by(|a, b| a.identity.name.cmp(&b.identity.name));
        Ok(packages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::feature::skill::SkillFeatureSet;

    fn make_skill(root: &std::path::Path, name: &str) {
        let skill_dir = root.join("skills").join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), format!("# {}", name)).unwrap();
    }

    #[test]
    fn list_packages_finds_skills() {
        let dir = tempfile::tempdir().unwrap();
        make_skill(dir.path(), "alpha-skill");
        make_skill(dir.path(), "beta-skill");
        let vault = LocalVaultAdapter::new("workspace", dir.path().to_path_buf());
        let pkgs = vault.list_packages(&SkillFeatureSet).unwrap();
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].identity.name, "alpha-skill");
        assert_eq!(pkgs[1].identity.name, "beta-skill");
    }

    #[test]
    fn list_packages_empty_when_no_skills_dir() {
        let dir = tempfile::tempdir().unwrap();
        let vault = LocalVaultAdapter::new("workspace", dir.path().to_path_buf());
        let pkgs = vault.list_packages(&SkillFeatureSet).unwrap();
        assert!(pkgs.is_empty());
    }

    #[test]
    fn list_packages_skips_dirs_without_skill_md() {
        let dir = tempfile::tempdir().unwrap();
        let not_a_skill = dir.path().join("skills").join("not-a-skill");
        std::fs::create_dir_all(&not_a_skill).unwrap();
        std::fs::write(not_a_skill.join("README.md"), "nope").unwrap();
        let vault = LocalVaultAdapter::new("workspace", dir.path().to_path_buf());
        let pkgs = vault.list_packages(&SkillFeatureSet).unwrap();
        assert!(pkgs.is_empty());
    }

    #[test]
    fn list_packages_sets_vault_id() {
        let dir = tempfile::tempdir().unwrap();
        make_skill(dir.path(), "my-skill");
        let vault = LocalVaultAdapter::new("my-vault", dir.path().to_path_buf());
        let pkgs = vault.list_packages(&SkillFeatureSet).unwrap();
        assert_eq!(pkgs[0].vault_id, "my-vault");
    }

    #[test]
    fn list_packages_sha10_is_ten_chars() {
        let dir = tempfile::tempdir().unwrap();
        make_skill(dir.path(), "some-skill");
        let vault = LocalVaultAdapter::new("workspace", dir.path().to_path_buf());
        let pkgs = vault.list_packages(&SkillFeatureSet).unwrap();
        assert_eq!(pkgs[0].identity.sha10.len(), 10);
    }

    #[test]
    fn list_packages_extracts_version_from_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("skills").join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\nversion: 1.5.0\n---\n# My Skill\n",
        )
        .unwrap();
        let vault = LocalVaultAdapter::new("workspace", dir.path().to_path_buf());
        let pkgs = vault.list_packages(&SkillFeatureSet).unwrap();
        assert_eq!(pkgs[0].identity.version, Some("1.5.0".to_string()));
    }
}
