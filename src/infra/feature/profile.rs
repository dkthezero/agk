use crate::app::ports::FeatureSetPort;
use crate::domain::asset::AssetKind;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct ProfileFeatureSet;

impl FeatureSetPort for ProfileFeatureSet {
    fn kind_name(&self) -> &str {
        "profile"
    }
    fn display_name(&self) -> &str {
        "Profiles"
    }
    fn scan_root(&self) -> &str {
        "profiles"
    }
    fn asset_kind(&self) -> AssetKind {
        AssetKind::Profile
    }

    fn is_package(&self, path: &Path) -> bool {
        path.join("PROFILE.md").exists() || path.join("profile.toml").exists()
    }

    fn hash_files(&self, path: &Path) -> Vec<PathBuf> {
        WalkDir::new(path)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect()
    }

    fn extract_version(&self, path: &Path) -> Option<String> {
        let profile_toml = std::fs::read_to_string(path.join("profile.toml")).ok()?;
        super::extract_frontmatter_version(&profile_toml)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::FeatureSetPort;

    #[test]
    fn profile_feature_set_kind_name() {
        assert_eq!(ProfileFeatureSet.kind_name(), "profile");
        assert_eq!(ProfileFeatureSet.display_name(), "Profiles");
    }

    #[test]
    fn profile_feature_set_detects_profile_md() {
        let dir = tempfile::tempdir().unwrap();
        let pkg_dir = dir.path().join("my-profile");
        std::fs::create_dir(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join("PROFILE.md"), "# My Profile").unwrap();
        assert!(ProfileFeatureSet.is_package(&pkg_dir));
    }

    #[test]
    fn profile_feature_set_detects_profile_toml() {
        let dir = tempfile::tempdir().unwrap();
        let pkg_dir = dir.path().join("my-profile");
        std::fs::create_dir(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join("profile.toml"), "name = 'my-profile'").unwrap();
        assert!(ProfileFeatureSet.is_package(&pkg_dir));
    }

    #[test]
    fn profile_feature_set_rejects_non_package() {
        let dir = tempfile::tempdir().unwrap();
        let other_dir = dir.path().join("not-a-profile");
        std::fs::create_dir(&other_dir).unwrap();
        std::fs::write(other_dir.join("README.md"), "nothing").unwrap();
        assert!(!ProfileFeatureSet.is_package(&other_dir));
    }

    #[test]
    fn profile_feature_set_hash_files_includes_all_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("PROFILE.md"), "profile").unwrap();
        std::fs::write(dir.path().join("notes.md"), "notes").unwrap();
        let files = ProfileFeatureSet.hash_files(dir.path());
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn profile_feature_set_is_not_stub() {
        assert!(!ProfileFeatureSet.is_stub());
    }

    #[test]
    fn profile_asset_kind_is_profile() {
        assert_eq!(ProfileFeatureSet.asset_kind(), AssetKind::Profile);
    }

    #[test]
    fn extract_version_from_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let profile_dir = dir.path().join("my-profile");
        std::fs::create_dir_all(&profile_dir).unwrap();
        std::fs::write(
            profile_dir.join("profile.toml"),
            "---\nname: my-profile\nversion: 1.0.0\n---\nname = 'my-profile'\n",
        )
        .unwrap();
        let version = ProfileFeatureSet.extract_version(&profile_dir);
        assert_eq!(version, Some("1.0.0".to_string()));
    }

    #[test]
    fn extract_version_none_when_no_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let profile_dir = dir.path().join("my-profile");
        std::fs::create_dir_all(&profile_dir).unwrap();
        std::fs::write(profile_dir.join("profile.toml"), "name = 'my-profile'\n").unwrap();
        let version = ProfileFeatureSet.extract_version(&profile_dir);
        assert!(version.is_none());
    }
}
