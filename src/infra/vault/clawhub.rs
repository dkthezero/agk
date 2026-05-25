use crate::app::ports::{FeatureSetPort, VaultPort};
use crate::domain::asset::ScannedPackage;
use crate::infra::vault::local::LocalVaultAdapter;
use anyhow::Result;
use std::path::PathBuf;

pub struct ClawHubVaultAdapter {
    id: String,
}

impl ClawHubVaultAdapter {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[async_trait::async_trait]
impl VaultPort for ClawHubVaultAdapter {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind_name(&self) -> &str {
        "clawhub"
    }

    fn list_packages(&self, feature: &dyn FeatureSetPort) -> Result<Vec<ScannedPackage>> {
        let cache_dir = crate::domain::paths::clawhub_cache_dir();
        if !cache_dir.exists() {
            return Ok(Vec::new());
        }
        let local = LocalVaultAdapter::new(&self.id, cache_dir);
        local.list_packages(feature)
    }
}

/// Check if the `clawhub` CLI is available on $PATH.
pub fn is_cli_available() -> bool {
    std::process::Command::new("which")
        .arg("clawhub")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if Homebrew is available (macOS).
pub fn is_homebrew_available() -> bool {
    std::process::Command::new("which")
        .arg("brew")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Install clawhub CLI via Homebrew.
pub fn install_cli_via_homebrew() -> Result<()> {
    let status = std::process::Command::new("brew")
        .args(["install", "clawhub"])
        .status()?;
    if !status.success() {
        anyhow::bail!("Failed to install clawhub via Homebrew");
    }
    Ok(())
}

/// Run `clawhub search <query>` and parse results into ScannedPackages.
/// For each result (up to 10), runs `clawhub inspect <slug> --json` to fetch metadata.
/// Output format per line: `slug  Display Name  (score)`
pub fn cli_search(query: &str) -> Result<Vec<ScannedPackage>> {
    let output = std::process::Command::new("clawhub")
        .args(["search", query])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("clawhub search failed: {}", stderr);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let slugs: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .take(10)
        .collect();

    let mut packages = Vec::new();
    for slug in slugs {
        let (name, version, meta) = match inspect_slug(slug) {
            Some(info) => info,
            None => (slug.to_string(), None, None),
        };
        packages.push(ScannedPackage {
            identity: crate::domain::identity::AssetIdentity::new(name, version, "----------"),
            path: PathBuf::new(),
            vault_id: "clawhub".to_string(),
            kind: crate::domain::asset::AssetKind::Skill,
            is_remote: true,
            remote_meta: meta,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
            include_evals: false,
        });
    }
    Ok(packages)
}

/// Run `clawhub inspect <slug> --json` and parse owner, summary, stats, and version.
/// Returns (display_name as "owner/slug", version, RemoteMetadata) or None on failure.
fn inspect_slug(
    slug: &str,
) -> Option<(
    String,
    Option<String>,
    Option<crate::domain::asset::RemoteMetadata>,
)> {
    let output = std::process::Command::new("clawhub")
        .args(["inspect", slug, "--json"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;

    let owner = json
        .pointer("/owner/handle")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let summary = json
        .pointer("/skill/summary")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let downloads = json
        .pointer("/skill/stats/downloads")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let stars = json
        .pointer("/skill/stats/stars")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let version = json
        .pointer("/latestVersion/version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let display_name = if owner.is_empty() {
        slug.to_string()
    } else {
        format!("{}/{}", owner, slug)
    };

    let meta = crate::domain::asset::RemoteMetadata {
        owner,
        summary,
        downloads,
        stars,
    };

    Some((display_name, version, Some(meta)))
}

/// Run `clawhub install <slug>` with workdir set to agk's clawhub cache.
/// Accepts `"owner/slug"` format — extracts just the slug for the CLI.
pub fn cli_install(name: &str) -> Result<()> {
    let slug = name.rsplit('/').next().unwrap_or(name);
    let cache_dir = crate::domain::paths::clawhub_cache_dir();
    std::fs::create_dir_all(&cache_dir)?;
    let status = std::process::Command::new("clawhub")
        .args(["install", slug, "--workdir", &cache_dir.to_string_lossy()])
        .status()?;
    if !status.success() {
        anyhow::bail!("clawhub install '{}' failed", slug);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::feature::skill::SkillFeatureSet;

    #[test]
    fn clawhub_vault_id() {
        let adapter = ClawHubVaultAdapter::new("clawhub");
        assert_eq!(adapter.id(), "clawhub");
    }

    #[test]
    fn clawhub_vault_kind_name() {
        let adapter = ClawHubVaultAdapter::new("clawhub");
        assert_eq!(adapter.kind_name(), "clawhub");
    }

    #[test]
    fn list_packages_empty_when_no_cache_dir() {
        let adapter = ClawHubVaultAdapter::new("clawhub");
        let pkgs = adapter.list_packages(&SkillFeatureSet).unwrap();
        assert!(pkgs.is_empty());
    }

    #[test]
    fn list_packages_finds_cached_skills() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("skills").join("my-clawhub-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# My Skill").unwrap();

        let local = LocalVaultAdapter::new("clawhub", dir.path().to_path_buf());
        let pkgs = local.list_packages(&SkillFeatureSet).unwrap();
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].identity.name, "my-clawhub-skill");
        assert_eq!(pkgs[0].vault_id, "clawhub");
    }
}
