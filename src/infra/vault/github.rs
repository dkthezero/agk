use crate::app::ports::{FeatureSetPort, VaultPort};
use crate::domain::asset::ScannedPackage;
use crate::infra::vault::local::LocalVaultAdapter;
use anyhow::{bail, Result};
use std::path::PathBuf;
use std::process::Command as StdCommand;
use tokio::process::Command;

pub struct GithubVaultAdapter {
    id: String,
    repo: String,
    target_ref: String,
    folder_path: String,
    base_url: String,
    cache_root: Option<PathBuf>,
}

impl GithubVaultAdapter {
    pub fn new(
        id: impl Into<String>,
        repo: impl Into<String>,
        target_ref: impl Into<String>,
        folder_path: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            repo: repo.into(),
            target_ref: target_ref.into(),
            folder_path: folder_path.into(),
            base_url: "https://github.com".to_string(),
            cache_root: None,
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_cache_root(mut self, root: PathBuf) -> Self {
        self.cache_root = Some(root);
        self
    }

    fn validate(&self) -> Result<()> {
        crate::domain::validation::validate_git_repo(&self.repo)?;
        crate::domain::validation::validate_git_ref(&self.target_ref)?;
        crate::domain::validation::validate_git_path(&self.folder_path)?;
        Ok(())
    }

    fn cache_dir(&self) -> PathBuf {
        self.cache_root
            .clone()
            .unwrap_or_else(crate::domain::paths::global_vaults_dir)
            .join(&self.id)
    }

    fn get_commit_hash(&self) -> Result<String> {
        let dir = self.cache_dir();
        if !dir.exists() {
            bail!("Cache directory does not exist for vault '{}'", self.id);
        }
        let output = StdCommand::new("git")
            .args([
                "-C",
                dir.to_str().unwrap(),
                "rev-parse",
                "--short=10",
                "HEAD",
            ])
            .output()?;
        if !output.status.success() {
            bail!("Failed to get commit hash for vault '{}'", self.id);
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[async_trait::async_trait]
impl VaultPort for GithubVaultAdapter {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind_name(&self) -> &str {
        "github"
    }

    async fn refresh(&self) -> Result<()> {
        self.validate()?;

        let dir = self.cache_dir();
        let url = if self.base_url.starts_with("file://") {
            format!("{}/{}", self.base_url, self.repo)
        } else {
            format!("{}/{}.git", self.base_url, self.repo)
        };

        let log_file_path = crate::domain::paths::global_config_root().join("git.log");
        if let Some(parent) = log_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let log_file_out = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)?;

        if dir.exists() && dir.join(".git").exists() {
            let status = Command::new("git")
                .args([
                    "-C",
                    dir.to_str().unwrap(),
                    "pull",
                    "origin",
                    &self.target_ref,
                ])
                .stdout(log_file_out.try_clone().map(std::process::Stdio::from)?)
                .stderr(log_file_out.try_clone().map(std::process::Stdio::from)?)
                .status()
                .await?;

            if !status.success() {
                bail!("Failed to pull github repo for vault '{}'", self.id);
            }
        } else {
            if dir.exists() {
                let _ = tokio::fs::remove_dir_all(&dir).await;
            }
            tokio::fs::create_dir_all(&dir).await?;

            let clone_status = Command::new("git")
                .args([
                    "clone",
                    "--filter=blob:none",
                    "--sparse",
                    "--depth",
                    "1",
                    "-b",
                    &self.target_ref,
                    &url,
                    dir.to_str().unwrap(),
                ])
                .stdout(log_file_out.try_clone().map(std::process::Stdio::from)?)
                .stderr(log_file_out.try_clone().map(std::process::Stdio::from)?)
                .status()
                .await?;

            if !clone_status.success() {
                bail!("Failed to clone github repo for vault '{}'", self.id);
            }

            if !self.folder_path.is_empty() && self.folder_path != "/" {
                let sparse_status = Command::new("git")
                    .args([
                        "-C",
                        dir.to_str().unwrap(),
                        "sparse-checkout",
                        "set",
                        &self.folder_path,
                    ])
                    .stdout(log_file_out.try_clone().map(std::process::Stdio::from)?)
                    .stderr(std::process::Stdio::from(log_file_out))
                    .status()
                    .await?;

                if !sparse_status.success() {
                    bail!("Failed to set sparse-checkout for vault '{}'", self.id);
                }
            }
        }
        Ok(())
    }

    fn list_packages(&self, feature: &dyn FeatureSetPort) -> Result<Vec<ScannedPackage>> {
        let dir = self.cache_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut target_root = if self.folder_path.is_empty() || self.folder_path == "/" {
            dir.clone()
        } else {
            dir.join(&self.folder_path)
        };

        // If the path specifically targets `skills` or `instructions`, the actual vault root is the parent.
        if target_root.ends_with("skills") || target_root.ends_with("instructions") {
            if let Some(parent) = target_root.parent() {
                target_root = parent.to_path_buf();
            }
        }

        let local_adapter = LocalVaultAdapter::new(&self.id, target_root);
        let mut packages = local_adapter.list_packages(feature)?;

        if let Ok(hash) = self.get_commit_hash() {
            for pkg in &mut packages {
                pkg.identity.sha10 = hash.clone();
            }
        }

        Ok(packages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid() {
        let adapter = GithubVaultAdapter::new("test", "owner/repo", "main", "skills/");
        assert!(adapter.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_repo() {
        let adapter = GithubVaultAdapter::new("test", "../repo", "main", "skills/");
        assert!(adapter.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_ref() {
        let adapter = GithubVaultAdapter::new("test", "owner/repo", "--branch", "skills/");
        assert!(adapter.validate().is_err());
    }

    #[test]
    fn test_cache_dir_contains_id() {
        let adapter = GithubVaultAdapter::new("my-id", "owner/repo", "main", "skills/");
        let dir = adapter.cache_dir();
        assert!(dir.to_string_lossy().contains("my-id"));
    }

    #[tokio::test]
    async fn test_refresh_and_list_packages_with_local_git() -> Result<()> {
        let root = tempfile::tempdir()?;
        let remote_dir = root.path().join("test").join("repo");
        std::fs::create_dir_all(&remote_dir)?;

        // Helper to run git commands
        let run = |args: &[&str], dir: &std::path::Path| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(dir)
                .status()
                .expect("git command failed");
            assert!(status.success());
        };

        // 1. Init remote repo
        run(&["init", "--initial-branch=main"], &remote_dir);
        run(&["config", "user.email", "test@example.com"], &remote_dir);
        run(&["config", "user.name", "Test User"], &remote_dir);

        // 2. Add some content in a subfolder
        let skill_dir = remote_dir.join("skills").join("my-skill");
        std::fs::create_dir_all(&skill_dir)?;
        std::fs::write(skill_dir.join("SKILL.md"), "# My Skill")?;

        run(&["add", "."], &remote_dir);
        run(&["commit", "-m", "initial commit"], &remote_dir);

        // 3. Setup adapter with file:// URL and isolated cache root
        let vault_id = "test-vault";
        let mut adapter = GithubVaultAdapter::new(vault_id, "test/repo", "main", "skills/");
        let cache_root = root.path().join("cache");
        adapter = adapter
            .with_base_url(format!("file://{}", root.path().display()))
            .with_cache_root(cache_root);

        // 4. Refresh (clones/pulls)
        adapter.refresh().await?;

        // 5. List packages
        let feature = crate::infra::feature::skill::SkillFeatureSet;
        let packages = adapter.list_packages(&feature)?;

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].identity.name, "my-skill");
        assert_eq!(packages[0].vault_id, vault_id);
        assert!(!packages[0].identity.sha10.is_empty());

        Ok(())
    }
}
