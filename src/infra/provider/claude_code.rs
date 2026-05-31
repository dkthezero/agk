use crate::app::ports::{McpProvider, ProviderPort};
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::identity::AssetIdentity;
use crate::domain::mcp::McpServer;
use crate::domain::scope::Scope;
use crate::infra::provider::common;
use crate::infra::provider::common::copy_dir;
use anyhow::Result;
use std::path::PathBuf;

pub struct ClaudeCodeProvider {
    workspace_root: PathBuf,
}

impl ClaudeCodeProvider {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    pub(crate) fn provider_root(
        &self,
        scope: &Scope,
        config: Option<&crate::domain::config::ConfigFile>,
    ) -> PathBuf {
        // provider_roots is workspace-only; global always uses the hardcoded default
        match scope {
            Scope::Global => dirs_next::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".claude"),
            Scope::Workspace => {
                let folder = config
                    .and_then(|c| c.provider_roots.get(self.id()))
                    .map(|s| s.as_str())
                    .unwrap_or(".claude");
                self.workspace_root.join(folder)
            }
        }
    }

    fn asset_dir(
        &self,
        scope: &Scope,
        kind: &AssetKind,
        name: &str,
        config: Option<&crate::domain::config::ConfigFile>,
    ) -> PathBuf {
        let root = self.provider_root(scope, config);
        match kind {
            AssetKind::Skill => root.join("skills").join(name),
            AssetKind::Instruction => root.join("instructions").join(name),
            AssetKind::McpServer => PathBuf::new(),
            AssetKind::Profile => PathBuf::new(),
        }
    }

    fn mcp_json_path(&self, scope: &Scope) -> PathBuf {
        self.provider_root(scope, None).join("mcp.json")
    }

    fn load_mcp_config(&self, scope: &Scope) -> Result<serde_json::Value> {
        let path = self.mcp_json_path(scope);
        if !path.exists() {
            return Ok(serde_json::json!({ "mcpServers": {} }));
        }
        let content = std::fs::read_to_string(&path)?;
        let config: serde_json::Value = serde_json::from_str(&content)?;
        Ok(config)
    }

    fn save_mcp_config(&self, scope: &Scope, config: &serde_json::Value) -> Result<()> {
        let path = self.mcp_json_path(scope);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(config)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

impl ProviderPort for ClaudeCodeProvider {
    fn id(&self) -> &str {
        "claude-code"
    }

    fn name(&self) -> &str {
        "Claude Code"
    }

    fn install(
        &self,
        pkg: &ScannedPackage,
        scope: Scope,
        config: Option<&crate::domain::config::ConfigFile>,
        _include_evals: bool,
    ) -> Result<()> {
        let dest = self.asset_dir(&scope, &pkg.kind, &pkg.identity.name, config);
        copy_dir(&pkg.path, &dest)
    }

    fn remove(
        &self,
        identity: &AssetIdentity,
        kind: &AssetKind,
        scope: Scope,
        config: Option<&crate::domain::config::ConfigFile>,
    ) -> Result<()> {
        let dest = self.asset_dir(&scope, kind, &identity.name, config);
        common::remove_dir_and_prune_empty_parents(&dest, 2)?;
        Ok(())
    }

    fn install_path_for(
        &self,
        identity: &AssetIdentity,
        kind: &AssetKind,
        scope: Scope,
    ) -> Option<PathBuf> {
        if *kind == AssetKind::McpServer {
            return None;
        }
        Some(self.asset_dir(&scope, kind, &identity.name, None))
    }

    fn available_config_roots(&self) -> Vec<(String, String)> {
        vec![
            (
                ".claude".to_string(),
                "Claude Code native folder".to_string(),
            ),
            (".agents".to_string(), "Shared agents folder".to_string()),
        ]
    }
}

impl McpProvider for ClaudeCodeProvider {
    fn provider_id(&self) -> &str {
        "claude-code"
    }

    fn supports_mcp(&self) -> bool {
        true
    }

    fn mcp_config_path(&self, scope: Scope) -> Option<PathBuf> {
        Some(self.mcp_json_path(&scope))
    }

    fn write_mcp_server(&self, server: &McpServer, scope: Scope) -> Result<()> {
        let mut config = self.load_mcp_config(&scope)?;
        if !config.is_object() {
            config = serde_json::json!({});
        }
        if config.get("mcpServers").is_none() {
            config["mcpServers"] = serde_json::json!({});
        }
        let mcp_servers = config["mcpServers"]
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!(".claude/mcp.json 'mcpServers' key is not an object"))?;

        let entry = serde_json::json!({
            "command": server.command,
            "args": server.args,
            "env": server.env,
        });
        mcp_servers.insert(server.name.clone(), entry);
        self.save_mcp_config(&scope, &config)
    }

    fn remove_mcp_server(&self, name: &str, scope: Scope) -> Result<()> {
        let mut config = self.load_mcp_config(&scope)?;
        if let Some(servers) = config
            .as_object_mut()
            .and_then(|obj| obj.get_mut("mcpServers"))
            .and_then(|v| v.as_object_mut())
        {
            servers.remove(name);
        }
        self.save_mcp_config(&scope, &config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::asset::AssetKind;
    use std::path::Path;

    fn make_pkg(dir: &Path, name: &str, kind: AssetKind, marker: &str) -> ScannedPackage {
        let pkg_dir = dir.join(name);
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join(marker), format!("# {}", name)).unwrap();
        ScannedPackage {
            identity: AssetIdentity::new(name, None, "0000000000"),
            path: pkg_dir,
            vault_id: "workspace".to_string(),
            kind,
            is_remote: false,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
            include_evals: false,
        }
    }

    #[test]
    fn install_skill_copies_to_workspace_claude_skills() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("source");
        std::fs::create_dir(&src_dir).unwrap();
        let pkg = make_pkg(&src_dir, "my-skill", AssetKind::Skill, "SKILL.md");
        let provider = ClaudeCodeProvider::new(dir.path().to_path_buf());
        provider
            .install(&pkg, Scope::Workspace, None, false)
            .unwrap();
        assert!(dir.path().join(".claude/skills/my-skill/SKILL.md").exists());
    }

    #[test]
    fn install_instruction_copies_to_workspace_claude_instructions() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("source");
        std::fs::create_dir(&src_dir).unwrap();
        let pkg = make_pkg(&src_dir, "my-inst", AssetKind::Instruction, "AGENTS.md");
        let provider = ClaudeCodeProvider::new(dir.path().to_path_buf());
        provider
            .install(&pkg, Scope::Workspace, None, false)
            .unwrap();
        assert!(dir
            .path()
            .join(".claude/instructions/my-inst/AGENTS.md")
            .exists());
    }

    #[test]
    fn remove_skill_deletes_directory() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join(".claude/skills/my-skill");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("SKILL.md"), "x").unwrap();
        let provider = ClaudeCodeProvider::new(dir.path().to_path_buf());
        let identity = AssetIdentity::new("my-skill", None, "0000000000");
        provider
            .remove(&identity, &AssetKind::Skill, Scope::Workspace, None)
            .unwrap();
        assert!(!dest.exists());
    }

    #[test]
    fn remove_nonexistent_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let provider = ClaudeCodeProvider::new(dir.path().to_path_buf());
        let identity = AssetIdentity::new("ghost", None, "0000000000");
        let result = provider.remove(&identity, &AssetKind::Skill, Scope::Workspace, None);
        assert!(result.is_ok());
    }

    #[test]
    fn claude_provider_root_uses_config_override() {
        let dir = tempfile::tempdir().unwrap();
        let provider = ClaudeCodeProvider::new(dir.path().to_path_buf());
        let mut config = crate::domain::config::ConfigFile::default();
        config
            .provider_roots
            .insert("claude-code".to_string(), ".agents".to_string());
        let root = provider.provider_root(&Scope::Workspace, Some(&config));
        assert_eq!(root, dir.path().join(".agents"));
    }

    #[test]
    fn claude_install_uses_agents_when_configured() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = crate::domain::config::ConfigFile::default();
        config
            .provider_roots
            .insert("claude-code".to_string(), ".agents".to_string());

        let src_dir = dir.path().join("source");
        std::fs::create_dir(&src_dir).unwrap();
        let pkg = make_pkg(&src_dir, "my-skill", AssetKind::Skill, "SKILL.md");
        let provider = ClaudeCodeProvider::new(dir.path().to_path_buf());
        provider
            .install(&pkg, Scope::Workspace, Some(&config), false)
            .unwrap();

        assert!(dir.path().join(".agents/skills/my-skill/SKILL.md").exists());
        assert!(!dir.path().join(".claude/skills/my-skill/SKILL.md").exists());
    }
}
