use crate::app::ports::McpRegistryPort;
use crate::domain::mcp::{McpRegistry, McpServer, McpTransport};
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
        crate::infra::mcp::register(name, command, args, env, transport, description)
    }

    fn test_server(&self, name: &str) -> Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(crate::infra::mcp::test_server(name))
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
}
