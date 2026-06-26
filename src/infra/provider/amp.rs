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
            AssetKind::Profile => PathBuf::new(),
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

    fn supports_mcp(&self) -> bool {
        true
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
            amp.insert("mcpServers".to_string(), serde_json::json!({}));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mcp::McpTransport;
    use std::collections::HashMap;

    fn sample_server(name: &str) -> McpServer {
        let mut env = HashMap::new();
        env.insert("API_KEY".to_string(), "secret".to_string());
        McpServer {
            name: name.to_string(),
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-fs".to_string(),
            ],
            env,
            transport: McpTransport::Stdio,
            description: Some("Test FS server".to_string()),
            tested: false,
            tested_at: None,
            activation: HashMap::new(),
            security_flags: Vec::new(),
        }
    }

    fn provider_in_temp() -> AmpProvider {
        AmpProvider::new(tempfile::tempdir().unwrap().path().to_path_buf())
    }

    #[test]
    fn write_read_roundtrip_persists_server_under_amp_key() {
        let provider = provider_in_temp();
        provider
            .write_mcp_server(&sample_server("filesystem"), Scope::Workspace)
            .unwrap();

        let path = provider.mcp_config_path(&Scope::Workspace);
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let entry = &written["amp"]["mcpServers"]["filesystem"];
        assert_eq!(entry["command"], "npx");
        assert_eq!(entry["args"][1], "@modelcontextprotocol/server-fs");
        assert_eq!(entry["env"]["API_KEY"], "secret");
    }

    #[test]
    fn write_then_remove_clears_entry_and_prunes_empty_bucket() {
        let provider = provider_in_temp();
        provider
            .write_mcp_server(&sample_server("filesystem"), Scope::Workspace)
            .unwrap();
        provider
            .remove_mcp_server("filesystem", Scope::Workspace)
            .unwrap();

        let path = provider.mcp_config_path(&Scope::Workspace);
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(written["amp"].get("mcpServers").is_none());
    }

    #[test]
    fn preserves_existing_settings_on_write() {
        let provider = provider_in_temp();
        let path = provider.mcp_config_path(&Scope::Workspace);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{"amp":{"mcpServers":{"existing":{"command":"foo","args":[],"env":{}}},"otherKey":1},"topLevel":true}"#,
        ).unwrap();

        provider
            .write_mcp_server(&sample_server("filesystem"), Scope::Workspace)
            .unwrap();

        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(written["amp"]["mcpServers"]["existing"]["command"], "foo");
        assert_eq!(written["amp"]["otherKey"], 1);
        assert_eq!(written["topLevel"], true);
        assert_eq!(written["amp"]["mcpServers"]["filesystem"]["command"], "npx");
    }

    #[test]
    fn overwrite_existing_server_updates_fields() {
        let provider = provider_in_temp();
        let mut server = sample_server("filesystem");
        provider
            .write_mcp_server(&server, Scope::Workspace)
            .unwrap();
        server.command = "node".to_string();
        server.args = vec!["server.js".to_string()];
        provider
            .write_mcp_server(&server, Scope::Workspace)
            .unwrap();

        let path = provider.mcp_config_path(&Scope::Workspace);
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let entry = &written["amp"]["mcpServers"]["filesystem"];
        assert_eq!(entry["command"], "node");
        assert_eq!(entry["args"][0], "server.js");
    }

    #[test]
    fn supports_mcp_true_for_amp() {
        use crate::app::ports::ProviderPort;
        assert!(ProviderPort::supports_mcp(&provider_in_temp()));
    }
}
