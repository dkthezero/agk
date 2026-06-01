use crate::app::ports::McpProvider;
use crate::domain::mcp::McpServer;
use crate::domain::scope::Scope;
use crate::infra::provider::claude_code::ClaudeCodeProvider;
use anyhow::Result;
use std::path::PathBuf;

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
