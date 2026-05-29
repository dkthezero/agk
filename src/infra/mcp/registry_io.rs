//! File-backed MCP registry persistence.
//!
//! `McpRegistry::{load, save}` originally lived in `domain/mcp.rs` and called
//! `std::fs` directly. ADR-001 Commit 1 moves those inherent impls here so the
//! domain layer stays pure. Existing call sites continue to use the same
//! `McpRegistry::load(&path)` / `registry.save(&path)` API; resolution finds
//! this impl block instead.

use crate::domain::mcp::McpRegistry;
use anyhow::Result;
use std::path::Path;

impl McpRegistry {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let registry: Self = toml::from_str(&content)?;
        Ok(registry)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
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
    use crate::domain::mcp::{McpServer, McpTransport};
    use std::collections::HashMap;

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

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("absent.toml");
        let loaded = McpRegistry::load(&path).unwrap();
        assert!(loaded.servers.is_empty());
    }
}
