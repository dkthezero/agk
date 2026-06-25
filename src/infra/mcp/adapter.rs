use crate::app::ports::McpRegistryPort;
use crate::domain::mcp::McpServer;
use crate::domain::scope::Scope;
use anyhow::Result;

/// Concrete [`McpRegistryPort`] adapter that delegates to the existing
/// `infra::mcp` implementation.
pub struct InfraMcpRegistryAdapter {
    workspace_root: std::path::PathBuf,
}

impl InfraMcpRegistryAdapter {
    pub fn new(workspace_root: impl Into<std::path::PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }
}

impl McpRegistryPort for InfraMcpRegistryAdapter {
    fn register(
        &self,
        name: &str,
        command: &str,
        args: Option<&str>,
        env: Option<&str>,
        transport: &str,
        description: Option<&str>,
    ) -> Result<McpServer> {
        let path = crate::domain::paths::mcp_path();
        crate::infra::mcp::register(name, command, args, env, transport, description, &path)
    }

    fn list(&self) -> Result<Vec<McpServer>> {
        let path = crate::domain::paths::mcp_path();
        let registry = crate::domain::mcp::McpRegistry::load(&path).unwrap_or_default();
        let mut servers: Vec<McpServer> = registry.servers.into_values().collect();
        servers.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(servers)
    }

    fn test_server(&self, name: &str) -> Result<()> {
        // We are always inside a tokio runtime; use futures::executor::block_on
        // instead of creating a nested Runtime.
        futures::executor::block_on(crate::infra::mcp::test_server(name))
    }

    fn build_providers(
        &self,
        workspace_root: &std::path::Path,
    ) -> Vec<Box<dyn crate::app::ports::McpProvider>> {
        crate::infra::mcp::build_mcp_providers(workspace_root)
    }

    fn enable(&self, name: &str, provider_id: &str, scope: Scope) -> Result<()> {
        let providers = self.build_providers(&self.workspace_root);
        crate::infra::mcp::enable(name, provider_id, scope, &providers)
    }

    fn disable(&self, name: &str, provider_id: &str, scope: Scope) -> Result<()> {
        let providers = self.build_providers(&self.workspace_root);
        crate::infra::mcp::disable(name, provider_id, scope, &providers)
    }

    fn unregister(&self, name: &str) -> Result<()> {
        crate::infra::mcp::unregister(name)
    }
}
