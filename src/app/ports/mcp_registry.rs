use crate::domain::mcp::McpServer;
use crate::domain::scope::Scope;
use anyhow::Result;
use std::path::PathBuf;

/// Port for MCP registry operations.
pub trait McpRegistryPort: Send + Sync {
    fn register(
        &self,
        name: &str,
        command: &str,
        args: Option<&str>,
        env: Option<&str>,
        transport: &str,
        description: Option<&str>,
    ) -> Result<McpServer>;

    fn list(&self) -> Result<Vec<McpServer>>;
    fn test_server(&self, name: &str) -> Result<()>;
    fn build_providers(&self, workspace_root: &std::path::Path) -> Vec<Box<dyn McpProvider>>;
    fn enable(&self, name: &str, provider_id: &str, scope: Scope) -> Result<()>;
    fn disable(&self, name: &str, provider_id: &str, scope: Scope) -> Result<()>;
}

/// Extension trait for providers that support MCP configuration.
pub trait McpProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn supports_mcp(&self) -> bool;
    fn mcp_config_path(&self, scope: Scope) -> Option<PathBuf>;
    fn write_mcp_server(&self, server: &McpServer, scope: Scope) -> Result<()>;
    fn remove_mcp_server(&self, name: &str, scope: Scope) -> Result<()>;
}
