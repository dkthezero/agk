use crate::app::ports::{McpProvider, ProviderPort};
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::identity::AssetIdentity;
use crate::domain::mcp::McpServer;
use crate::domain::scope::Scope;
use crate::infra::provider::common;
use crate::infra::provider::common::copy_dir;
use anyhow::Result;
use std::path::PathBuf;

pub struct GeminiProvider {
    workspace_root: PathBuf,
}

impl GeminiProvider {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    fn provider_root(
        &self,
        scope: &Scope,
        config: Option<&crate::domain::config::ConfigFile>,
    ) -> PathBuf {
        // provider_roots is workspace-only; global always uses the hardcoded default
        match scope {
            Scope::Global => dirs_next::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".gemini"),
            Scope::Workspace => {
                let folder = config
                    .and_then(|c| c.provider_roots.get(self.id()))
                    .map(|s| s.as_str())
                    .unwrap_or(".gemini");
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
        self.provider_root(scope, None).join("settings.json")
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

impl ProviderPort for GeminiProvider {
    fn id(&self) -> &str {
        "gemini-cli"
    }

    fn name(&self) -> &str {
        "Gemini CLI"
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
            (".gemini".to_string(), "Gemini native folder".to_string()),
            (".ai".to_string(), "Legacy .ai folder".to_string()),
        ]
    }

    fn supports_mcp(&self) -> bool {
        true
    }
}

impl McpProvider for GeminiProvider {
    fn provider_id(&self) -> &str {
        "gemini-cli"
    }

    fn supports_mcp(&self) -> bool {
        true
    }

    fn mcp_config_path(&self, scope: Scope) -> Option<PathBuf> {
        match scope {
            Scope::Global => Some(self.provider_root(&scope, None).join("settings.json")),
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
            anyhow::anyhow!(".gemini/settings.json 'mcpServers' key is not an object")
        })?;

        let entry = serde_json::json!({
            "command": server.command,
            "args": server.args,
            "env": server.env,
            "trust": true,
            "includeTools": ["*"],
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
    use crate::domain::config::ConfigFile;
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

    fn provider_in_temp() -> GeminiProvider {
        GeminiProvider::new(tempfile::tempdir().unwrap().path().to_path_buf())
    }

    #[test]
    fn gemini_provider_root_uses_config_override() {
        let dir = tempfile::tempdir().unwrap();
        let provider = GeminiProvider::new(dir.path().to_path_buf());
        let mut config = ConfigFile::default();
        config
            .provider_roots
            .insert("gemini-cli".to_string(), ".ai".to_string());
        let root = provider.provider_root(&Scope::Workspace, Some(&config));
        assert_eq!(root, dir.path().join(".ai"));
    }

    #[test]
    fn write_read_roundtrip_persists_server_with_gemini_schema() {
        let provider = provider_in_temp();
        provider
            .write_mcp_server(&sample_server("filesystem"), Scope::Workspace)
            .unwrap();

        let path = provider.mcp_json_path(&Scope::Workspace);
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let entry = &written["mcpServers"]["filesystem"];
        assert_eq!(entry["command"], "npx");
        assert_eq!(entry["args"][1], "@modelcontextprotocol/server-fs");
        assert_eq!(entry["env"]["API_KEY"], "secret");
        assert_eq!(entry["trust"], true);
        assert_eq!(entry["includeTools"][0], "*");
    }

    #[test]
    fn write_then_remove_clears_entry() {
        let provider = provider_in_temp();
        provider
            .write_mcp_server(&sample_server("filesystem"), Scope::Workspace)
            .unwrap();
        provider
            .remove_mcp_server("filesystem", Scope::Workspace)
            .unwrap();

        let path = provider.mcp_json_path(&Scope::Workspace);
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(written["mcpServers"].get("filesystem").is_none());
    }

    #[test]
    fn preserves_existing_settings_on_write() {
        let provider = provider_in_temp();
        let path = provider.mcp_json_path(&Scope::Workspace);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{"mcpServers":{"existing":{"command":"foo","args":[],"env":{},"trust":true,"includeTools":["*"]}},"otherKey":7}"#,
        ).unwrap();

        provider
            .write_mcp_server(&sample_server("filesystem"), Scope::Workspace)
            .unwrap();

        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(written["mcpServers"]["existing"]["command"], "foo");
        assert_eq!(written["otherKey"], 7);
        assert_eq!(written["mcpServers"]["filesystem"]["command"], "npx");
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

        let path = provider.mcp_json_path(&Scope::Workspace);
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let entry = &written["mcpServers"]["filesystem"];
        assert_eq!(entry["command"], "node");
        assert_eq!(entry["args"][0], "server.js");
    }

    #[test]
    fn supports_mcp_true_for_gemini() {
        use crate::app::ports::ProviderPort;
        assert!(ProviderPort::supports_mcp(&provider_in_temp()));
    }
}
