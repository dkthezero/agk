use crate::app::ports::{McpProvider, ProviderPort};
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::identity::AssetIdentity;
use crate::domain::mcp::McpServer;
use crate::domain::scope::Scope;
use crate::infra::provider::common;
use crate::infra::provider::common::copy_dir;
use anyhow::Result;
use std::path::PathBuf;

pub struct AmpProvider {
    workspace_root: PathBuf,
}

impl AmpProvider {
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
                .join(".amp"),
            Scope::Workspace => self.workspace_root.join(".amp"),
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

    fn mcp_config_path(&self, scope: &Scope) -> PathBuf {
        match scope {
            Scope::Global => dirs_next::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
                .join("amp")
                .join("settings.json"),
            Scope::Workspace => self.workspace_root.join(".amp").join("settings.json"),
        }
    }

    fn load_mcp_config(&self, scope: &Scope) -> Result<serde_json::Value> {
        let path = self.mcp_config_path(scope);
        if !path.exists() {
            return Ok(serde_json::json!({}));
        }
        let content = std::fs::read_to_string(&path)?;
        let config: serde_json::Value = serde_json::from_str(&content)?;
        Ok(config)
    }

    fn save_mcp_config(&self, scope: &Scope, config: &serde_json::Value) -> Result<()> {
        let path = self.mcp_config_path(scope);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(config)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

impl ProviderPort for AmpProvider {
    fn id(&self) -> &str {
        "amp"
    }

    fn name(&self) -> &str {
        "AMP Code"
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
        scope: Scope,
    ) -> Option<PathBuf> {
        if *kind == AssetKind::McpServer {
            return None;
        }
        Some(self.asset_dir(&scope, kind, &_identity.name, None))
    }
}

impl McpProvider for AmpProvider {
    fn provider_id(&self) -> &str {
        "amp"
    }

    fn supports_mcp(&self) -> bool {
        true
    }

    fn mcp_config_path(&self, scope: Scope) -> Option<PathBuf> {
        Some(self.mcp_config_path(&scope))
    }

    fn write_mcp_server(&self, server: &McpServer, scope: Scope) -> Result<()> {
        let mut config = self.load_mcp_config(&scope)?;
        if !config.is_object() {
            config = serde_json::json!({});
        }
        if config.get("amp").is_none() {
            config["amp"] = serde_json::json!({});
        }
        let amp = config["amp"]
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("AMP settings.json 'amp' key is not an object"))?;

        if amp.get("mcpServers").is_none() {
            amp["mcpServers"] = serde_json::json!({});
        }
        let mcp_servers = amp["mcpServers"].as_object_mut().ok_or_else(|| {
            anyhow::anyhow!("AMP settings.json 'amp.mcpServers' key is not an object")
        })?;

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
        if let Some(amp) = config.get_mut("amp").and_then(|v| v.as_object_mut()) {
            if let Some(servers) = amp.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
                servers.remove(name);
                if servers.is_empty() {
                    amp.remove("mcpServers");
                }
            }
        }
        self.save_mcp_config(&scope, &config)
    }
}
