use crate::domain::asset::AssetKind;
use std::path::{Path, PathBuf};

pub trait FeatureSetPort: Send + Sync {
    fn kind_name(&self) -> &str;
    fn display_name(&self) -> &str;
    fn scan_root(&self) -> &str;
    fn asset_kind(&self) -> AssetKind;
    fn is_package(&self, path: &Path) -> bool;
    fn hash_files(&self, path: &Path) -> Vec<PathBuf>;

    fn extract_version(&self, _path: &Path) -> Option<String> {
        None
    }

    /// Override to return `true` for placeholder tabs not yet implemented.
    fn is_stub(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestFeatureSet;
    impl FeatureSetPort for TestFeatureSet {
        fn kind_name(&self) -> &str {
            "test"
        }
        fn display_name(&self) -> &str {
            "Test"
        }
        fn scan_root(&self) -> &str {
            "test_root"
        }
        fn asset_kind(&self) -> AssetKind {
            AssetKind::Skill
        }
        fn is_package(&self, _: &Path) -> bool {
            false
        }
        fn hash_files(&self, _: &Path) -> Vec<PathBuf> {
            vec![]
        }
    }

    #[test]
    fn feature_set_port_default_not_stub() {
        let f = TestFeatureSet;
        assert!(!f.is_stub());
    }

    #[test]
    fn feature_set_port_kind_name() {
        let f = TestFeatureSet;
        assert_eq!(f.kind_name(), "test");
    }
}
