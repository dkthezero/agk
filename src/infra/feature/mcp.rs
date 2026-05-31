use crate::app::ports::FeatureSetPort;
use crate::domain::asset::AssetKind;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct McpFeatureSet;

impl FeatureSetPort for McpFeatureSet {
    fn kind_name(&self) -> &str {
        "mcp"
    }
    fn display_name(&self) -> &str {
        "MCP Servers"
    }
    fn scan_root(&self) -> &str {
        "mcps"
    }
    fn asset_kind(&self) -> AssetKind {
        AssetKind::McpServer
    }

    fn is_package(&self, path: &Path) -> bool {
        path.join("MCP.md").exists()
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
        let content = std::fs::read_to_string(path.join("MCP.md")).ok()?;
        super::extract_frontmatter_version(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::FeatureSetPort;

    #[test]
    fn mcp_feature_set_kind_name() {
        assert_eq!(McpFeatureSet.kind_name(), "mcp");
        assert_eq!(McpFeatureSet.display_name(), "MCP Servers");
    }

    #[test]
    fn mcp_feature_set_detects_package() {
        let dir = tempfile::tempdir().unwrap();
        let pkg_dir = dir.path().join("my-mcp");
        std::fs::create_dir(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join("MCP.md"), "# My MCP").unwrap();
        assert!(McpFeatureSet.is_package(&pkg_dir));
    }

    #[test]
    fn mcp_feature_set_rejects_non_package() {
        let dir = tempfile::tempdir().unwrap();
        let other_dir = dir.path().join("not-an-mcp");
        std::fs::create_dir(&other_dir).unwrap();
        std::fs::write(other_dir.join("README.md"), "nothing").unwrap();
        assert!(!McpFeatureSet.is_package(&other_dir));
    }

    #[test]
    fn mcp_feature_set_hash_files_includes_all_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("MCP.md"), "mcp").unwrap();
        std::fs::write(dir.path().join("config.json"), "{}").unwrap();
        let files = McpFeatureSet.hash_files(dir.path());
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn mcp_feature_set_is_not_stub() {
        assert!(!McpFeatureSet.is_stub());
    }

    #[test]
    fn mcp_asset_kind_is_mcp_server() {
        assert_eq!(McpFeatureSet.asset_kind(), AssetKind::McpServer);
    }

    #[test]
    fn extract_version_from_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let mcp_dir = dir.path().join("my-mcp");
        std::fs::create_dir_all(&mcp_dir).unwrap();
        std::fs::write(
            mcp_dir.join("MCP.md"),
            "---\nname: my-mcp\nversion: 1.0.0\n---\n# My MCP\n",
        )
        .unwrap();
        let version = McpFeatureSet.extract_version(&mcp_dir);
        assert_eq!(version, Some("1.0.0".to_string()));
    }

    #[test]
    fn extract_version_none_when_no_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let mcp_dir = dir.path().join("my-mcp");
        std::fs::create_dir_all(&mcp_dir).unwrap();
        std::fs::write(mcp_dir.join("MCP.md"), "# My MCP\n").unwrap();
        let version = McpFeatureSet.extract_version(&mcp_dir);
        assert!(version.is_none());
    }
}
