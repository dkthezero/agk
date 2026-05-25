use crate::app::ports::{McpProvider, ProviderPort};
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::identity::AssetIdentity;
use crate::domain::mcp::McpServer;
use crate::domain::scope::Scope;
use crate::infra::provider::common;
use crate::infra::provider::common::copy_dir;
use anyhow::Result;
use std::path::PathBuf;

pub struct GithubProvider {
    workspace_root: PathBuf,
}

impl GithubProvider {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    fn provider_root(
        &self,
        scope: &Scope,
        _config: Option<&crate::domain::config::ConfigFile>,
    ) -> PathBuf {
        match scope {
            Scope::Global => dirs_next::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".copilot"),
            Scope::Workspace => self.workspace_root.join(".github"),
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
        }
    }

    fn mcp_json_path(&self, scope: &Scope) -> PathBuf {
        self.provider_root(scope, None).join("mcp-config.json")
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

impl ProviderPort for GithubProvider {
    fn id(&self) -> &str {
        "github-copilot"
    }

    fn name(&self) -> &str {
        "GitHub Copilot"
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
        _identity: &AssetIdentity,
        kind: &AssetKind,
        _scope: Scope,
    ) -> Option<PathBuf> {
        if *kind == AssetKind::McpServer {
            return None;
        }
        None
    }
}

impl McpProvider for GithubProvider {
    fn provider_id(&self) -> &str {
        "github-copilot"
    }

    fn supports_mcp(&self) -> bool {
        true
    }

    fn mcp_config_path(&self, scope: Scope) -> Option<PathBuf> {
        match scope {
            Scope::Global => Some(self.mcp_json_path(&scope)),
            Scope::Workspace => None,
        }
    }

    fn write_mcp_server(&self, server: &McpServer, scope: Scope) -> Result<()> {
        let mut config = self.load_mcp_config(&scope)?;
        if !config.is_object() {
            config = serde_json::json!({});
        }
        if config.get("mcpServers").is_none() {
            config["mcpServers"] = serde_json::json!({});
        }
        let mcp_servers = config["mcpServers"].as_object_mut().ok_or_else(|| {
            anyhow::anyhow!(".copilot/mcp-config.json 'mcpServers' key is not an object")
        })?;

        let entry = serde_json::json!({
            "type": "local",
            "command": server.command,
            "args": server.args,
            "env": server.env,
            "tools": ["*"],
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
