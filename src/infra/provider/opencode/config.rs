use crate::app::ports::ProviderPort;
use crate::domain::scope::Scope;
use crate::infra::provider::opencode::OpenCodeProvider;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Validate profile name is safe for filesystem use.
pub fn validate_profile_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\u{0000}')
        || name.contains(':')
        || name == "."
        || name == ".."
        || name.starts_with("..")
    {
        anyhow::bail!("Profile name contains invalid characters: {}", name);
    }
    Ok(())
}

impl OpenCodeProvider {
    /// Build the workspace path to the profile's base agent markdown.
    /// OpenCode `agent create --path <dir>` writes into `<dir>/agents/*.md`,
    /// so we scan that subdirectory and fall back to the legacy `agent.md`.
    pub fn profile_agent_path(&self, profile_name: &str) -> PathBuf {
        let profile_dir = self
            .workspace_root
            .join(".agk")
            .join("profiles")
            .join(profile_name);
        let agents_dir = profile_dir.join("agents");
        if let Ok(entries) = std::fs::read_dir(&agents_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    return path;
                }
            }
        }
        profile_dir.join("agent.md")
    }

    /// Returns the workspace `opencode.json` path.
    fn workspace_opencode_json(&self) -> PathBuf {
        self.workspace_root.join("opencode.json")
    }

    pub fn provider_root(
        &self,
        scope: &Scope,
        config: Option<&crate::domain::config::ConfigFile>,
    ) -> PathBuf {
        // provider_roots is workspace-only; global always uses hardcoded defaults
        match scope {
            Scope::Global => dirs_next::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
                .join("opencode"),
            Scope::Workspace => {
                let folder = config
                    .and_then(|c| c.provider_roots.get(self.id()))
                    .map(|s| s.as_str())
                    .unwrap_or(".opencode");
                self.workspace_root.join(folder)
            }
        }
    }

    pub fn config_path(&self, scope: &Scope) -> PathBuf {
        match scope {
            Scope::Global => dirs_next::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
                .join("opencode")
                .join("opencode.json"),
            Scope::Workspace => self.workspace_root.join("opencode.json"),
        }
    }

    /// Read workspace `opencode.json`.
    /// Returns (parsed value, original file bytes for lossless restore).
    /// Errors on parse failure or non-object root instead of silently defaulting.
    pub fn read_workspace_config(&self) -> Result<(serde_json::Value, Option<Vec<u8>>)> {
        let path = self.workspace_opencode_json();
        if !path.exists() {
            return Ok((serde_json::json!({}), None));
        }
        let content = std::fs::read_to_string(&path)?;
        let cleaned = super::util::strip_jsonc_comments(&content);
        let value: serde_json::Value = serde_json::from_str(&cleaned).with_context(|| {
            format!(
                "Failed to parse workspace opencode.json at {}",
                path.display()
            )
        })?;
        if !value.is_object() {
            anyhow::bail!(
                "Workspace opencode.json root must be an object, got {}",
                serde_json::to_string(&value).unwrap_or_else(|_| "non-serializable".into())
            );
        }
        // Preserve original bytes for lossless restore (preserves comments/formatting)
        let original_bytes = std::fs::read(&path).ok();
        Ok((value, original_bytes))
    }

    /// Write workspace `opencode.json` or delete it if empty.
    pub fn write_workspace_config(&self, value: &serde_json::Value) -> Result<()> {
        let path = self.workspace_opencode_json();
        if let Some(obj) = value.as_object() {
            if obj.is_empty() {
                if path.exists() {
                    std::fs::remove_file(&path)?;
                }
                return Ok(());
            }
        }
        let content = serde_json::to_string_pretty(value)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
