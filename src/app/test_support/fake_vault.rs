use crate::app::ports::feature_set::FeatureSetPort;
use crate::app::ports::VaultPort;
use crate::domain::asset::ScannedPackage;
use anyhow::Result;
use std::sync::Mutex;

/// In-memory [`VaultPort`] that returns a fixed list of [`ScannedPackage`]s.
///
/// Tests can seed packages before wiring the vault into [`crate::app::registry::Registry`].
#[derive(Debug)]
pub struct FakeVault {
    pub id: String,
    pub packages: Mutex<Vec<ScannedPackage>>,
}

impl FakeVault {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            packages: Mutex::new(Vec::new()),
        }
    }

    pub fn seed(&self, pkg: ScannedPackage) {
        self.packages.lock().unwrap().push(pkg);
    }
}

impl VaultPort for FakeVault {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind_name(&self) -> &str {
        "fake"
    }

    fn list_packages(&self, _feature: &dyn FeatureSetPort) -> Result<Vec<ScannedPackage>> {
        Ok(self.packages.lock().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::asset::{AssetKind, ScannedPackage};
    use crate::domain::identity::AssetIdentity;
    use std::path::PathBuf;

    struct FakeFeatureSet;
    impl FeatureSetPort for FakeFeatureSet {
        fn kind_name(&self) -> &str {
            "skill"
        }
        fn display_name(&self) -> &str {
            "Skill"
        }
        fn scan_root(&self) -> &str {
            "skills"
        }
        fn asset_kind(&self) -> AssetKind {
            AssetKind::Skill
        }
        fn is_package(&self, _: &std::path::Path) -> bool {
            true
        }
        fn hash_files(&self, _: &std::path::Path) -> Vec<PathBuf> {
            vec![]
        }
    }

    #[test]
    fn fake_vault_list_packages() {
        let vault = FakeVault::new("workspace");
        vault.seed(ScannedPackage {
            identity: AssetIdentity::new("test-skill", None, "0000000000"),
            path: PathBuf::from("skills/test-skill"),
            vault_id: "workspace".into(),
            kind: AssetKind::Skill,
            is_remote: false,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
            include_evals: false,
        });

        let pkgs = vault.list_packages(&FakeFeatureSet).unwrap();
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].identity.name, "test-skill");
    }
}
