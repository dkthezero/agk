#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP server configuration stored in agk's global registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub transport: McpTransport,
    pub description: Option<String>,
    #[serde(default)]
    pub tested: bool,
    pub tested_at: Option<String>,
    /// Provider activation state: provider_id → { global: bool, workspace: bool }
    #[serde(default)]
    pub activation: HashMap<String, McpActivation>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpActivation {
    #[serde(default)]
    pub global: bool,
    #[serde(default)]
    pub workspace: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum McpTransport {
    #[default]
    Stdio,
    Sse {
        url: String,
    },
}

/// Full MCP registry stored in ~/.config/agk/mcp.toml
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpRegistry {
    #[serde(default)]
    pub servers: HashMap<String, McpServer>,
}

impl McpRegistry {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let registry: Self = toml::from_str(&content)?;
        Ok(registry)
    }

    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_registry_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.toml");

        let mut registry = McpRegistry::default();
        registry.servers.insert(
            "fs".to_string(),
            McpServer {
                name: "fs".to_string(),
                command: "npx".to_string(),
                args: vec![
                    "@modelcontextprotocol/server-filesystem".to_string(),
                    "/tmp".to_string(),
                ],
                env: HashMap::new(),
                transport: McpTransport::Stdio,
                description: Some("Filesystem access".to_string()),
                tested: true,
                tested_at: Some("2026-05-01T00:00:00Z".to_string()),
                activation: HashMap::new(),
            },
        );
        registry.save(&path).unwrap();

        let loaded = McpRegistry::load(&path).unwrap();
        assert!(loaded.servers.contains_key("fs"));
        let fs = loaded.servers.get("fs").unwrap();
        assert_eq!(fs.command, "npx");
        assert!(fs.tested);
    }
}
