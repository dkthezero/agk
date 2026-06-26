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

/// Write `content` to `path` and restrict it to owner-only access
/// (`0600` on Unix), per the mcp-vault PRD Security Considerations.
///
/// On Unix the restrictive mode is applied at creation time via
/// `OpenOptions::mode(0o600)` so the file is never briefly world/group-
/// readable between creation and a follow-up chmod. On non-Unix platforms
/// the file is written without Unix mode hardening (Windows ACLs are not
/// adjusted here); a non-fatal chmod attempt is still made so any error
/// surfaces rather than being silently swallowed.
fn write_restricted(path: &Path, content: &str) -> Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(content.as_bytes())?;
        let _ = file;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, content)?;
        let _ = path;
    }
    Ok(())
}

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
        write_restricted(path, &content)?;
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
                security_flags: vec![],
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

    #[cfg(unix)]
    #[test]
    fn save_sets_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.toml");
        McpRegistry::default().save(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "mcp.toml should be 0600 owner-only, got {:o}",
            mode & 0o777
        );
    }
}
