use crate::app::ports::McpProvider;
use crate::domain::mcp::{McpServer, McpTransport};
use crate::domain::scope::Scope;
use crate::infra::provider::opencode::OpenCodeProvider;
use anyhow::Result;
use std::path::PathBuf;

impl McpProvider for OpenCodeProvider {
    fn provider_id(&self) -> &str {
        "opencode"
    }

    fn supports_mcp(&self) -> bool {
        true
    }

    fn mcp_config_path(&self, scope: Scope) -> Option<PathBuf> {
        Some(self.config_path(&scope))
    }

    fn write_mcp_server(&self, server: &McpServer, scope: Scope) -> Result<()> {
        let path = self.config_path(&scope);
        let mut config: serde_json::Value = if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let cleaned = super::util::strip_jsonc_comments(&content);
            serde_json::from_str(&cleaned)?
        } else {
            serde_json::json!({})
        };

        if !config.is_object() {
            config = serde_json::json!({});
        }
        if config.get("mcp").is_none() {
            config["mcp"] = serde_json::json!({});
        }
        let mcp = config["mcp"]
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("opencode.json 'mcp' key is not an object"))?;

        // Migrate: drop old nested "servers" key if present so we don't leave a
        // stale empty object that OpenCode rejects.
        mcp.remove("servers");

        let entry = match &server.transport {
            McpTransport::Stdio => serde_json::json!({
                "type": "local",
                "command": server.command,
                "args": server.args,
                "env": server.env,
                "enabled": true,
            }),
            McpTransport::Sse { url } => serde_json::json!({
                "type": "remote",
                "url": url,
                "enabled": true,
            }),
        };
        mcp.insert(server.name.clone(), entry);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    fn remove_mcp_server(&self, name: &str, scope: Scope) -> Result<()> {
        let path = self.config_path(&scope);
        if !path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(&path)?;
        let cleaned = super::util::strip_jsonc_comments(&content);
        let mut config: serde_json::Value = serde_json::from_str(&cleaned)?;
        if let Some(mcp) = config.get_mut("mcp").and_then(|m| m.as_object_mut()) {
            mcp.remove(name);
            // If mcp is now empty, drop it entirely.
            if mcp.is_empty() {
                config.as_object_mut().unwrap().remove("mcp");
            }
        }
        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
