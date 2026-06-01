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
    auth_token: Option<String>,
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
            auth_token: None,
            cache_root: None,
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
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

    /// Build the clone/pull URL without any embedded credentials.
    ///
    /// Authentication is handled via a temporary GIT_ASKPASS script that
    /// provides the token on demand, keeping it out of process args,
    /// git config, and log files.
    fn build_remote_url(&self) -> String {
        let base = &self.base_url;

        if base.starts_with("file://") {
            return format!("{}/{}", base, self.repo);
        }

        // Parse host from base_url, stripping any trailing slashes
        let host = base
            .strip_prefix("https://")
            .or_else(|| base.strip_prefix("http://"))
            .unwrap_or(base)
            .trim_end_matches('/');

        format!("https://{}/{}.git", host, self.repo)
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
        let url = self.build_remote_url();

        // Set up secure credential delivery via GIT_ASKPASS when we have a token.
        // This avoids embedding the token in the URL (visible in ps, git config, logs).
        let askpass_script =
            crate::infra::vault::askpass::create_askpass_script(&self.id, &self.auth_token)?;

        let log_file_path = crate::domain::paths::global_config_root().join("git.log");
        if let Some(parent) = log_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let log_file_out = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)?;

        let result = async {
            if dir.exists() && dir.join(".git").exists() {
                let mut cmd = Command::new("git");
                cmd.args([
                    "-C",
                    dir.to_str().unwrap(),
                    "pull",
                    "origin",
                    &self.target_ref,
                ]);
                if let Some(ref script) = askpass_script {
                    cmd.env(
                        "GIT_ASKPASS",
                        crate::infra::vault::askpass::askpass_executable_path(script),
                    );
                    cmd.env("GIT_TERMINAL_PROMPT", "0");
                }
                let status = cmd
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

                let mut cmd = Command::new("git");
                cmd.args([
                    "clone",
                    "--filter=blob:none",
                    "--sparse",
                    "--depth",
                    "1",
                    "-b",
                    &self.target_ref,
                    &url,
                    dir.to_str().unwrap(),
                ]);
                if let Some(ref script) = askpass_script {
                    cmd.env(
                        "GIT_ASKPASS",
                        crate::infra::vault::askpass::askpass_executable_path(script),
                    );
                    cmd.env("GIT_TERMINAL_PROMPT", "0");
                }
                let clone_status = cmd
                    .stdout(log_file_out.try_clone().map(std::process::Stdio::from)?)
                    .stderr(std::process::Stdio::from(log_file_out.try_clone()?))
                    .status()
                    .await?;

                if !clone_status.success() {
                    bail!("Failed to clone github repo for vault '{}'", self.id);
                }

                if !self.folder_path.is_empty() && self.folder_path != "/" {
                    let mut sparse_cmd = Command::new("git");
                    sparse_cmd.args([
                        "-C",
                        dir.to_str().unwrap(),
                        "sparse-checkout",
                        "set",
                        &self.folder_path,
                    ]);
                    // sparse-checkout doesn't need auth, but set env for consistency
                    if let Some(ref script) = askpass_script {
                        sparse_cmd.env(
                            "GIT_ASKPASS",
                            crate::infra::vault::askpass::askpass_executable_path(script),
                        );
                        sparse_cmd.env("GIT_TERMINAL_PROMPT", "0");
                    }
                    let sparse_status = sparse_cmd
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
        .await;

        // Always clean up the askpass script, even on error
        if let Some(ref script_path) = askpass_script {
            crate::infra::vault::askpass::cleanup_askpass(script_path);
        }

        result
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

    #[test]
    fn test_build_remote_url_default() {
        let adapter = GithubVaultAdapter::new("test", "owner/repo", "main", "skills/");
        assert_eq!(
            adapter.build_remote_url(),
            "https://github.com/owner/repo.git"
        );
    }

    #[test]
    fn test_build_remote_url_no_token_in_url() {
        // Token must NOT appear in the URL — it's delivered via GIT_ASKPASS
        let adapter = GithubVaultAdapter::new("test", "owner/repo", "main", "skills/")
            .with_auth_token("ghp_abcdef123");
        let url = adapter.build_remote_url();
        assert_eq!(url, "https://github.com/owner/repo.git");
        assert!(
            !url.contains("ghp_abcdef123"),
            "token must not appear in URL"
        );
        assert!(
            !url.contains("x-access-token"),
            "credentials must not appear in URL"
        );
    }

    #[test]
    fn test_build_remote_url_ghes_no_token_in_url() {
        let adapter = GithubVaultAdapter::new("test", "owner/repo", "main", "skills/")
            .with_base_url("https://github.example.com")
            .with_auth_token("ghp_abcdef123");
        let url = adapter.build_remote_url();
        assert_eq!(url, "https://github.example.com/owner/repo.git");
        assert!(
            !url.contains("ghp_abcdef123"),
            "token must not appear in URL"
        );
    }

    #[test]
    fn test_build_remote_url_file_protocol() {
        let adapter = GithubVaultAdapter::new("test", "test/repo", "main", "skills/")
            .with_base_url("file:///tmp/repos");
        assert_eq!(adapter.build_remote_url(), "file:///tmp/repos/test/repo");
    }

    #[test]
    fn test_build_remote_url_strips_trailing_slash() {
        let adapter = GithubVaultAdapter::new("test", "owner/repo", "main", "skills/")
            .with_base_url("https://github.example.com/");
        assert_eq!(
            adapter.build_remote_url(),
            "https://github.example.com/owner/repo.git"
        );
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
