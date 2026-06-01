use crate::app::snapshot::DiscoveredMcp;
use crate::domain::mcp::{McpRegistry, McpServer};

/// MCP registry state for TUI rendering.
#[derive(Debug, Clone)]
pub struct McpState {
    pub registry: McpRegistry,
    pub discovered: Vec<DiscoveredMcp>,
}

impl Default for McpState {
    fn default() -> Self {
        let path = crate::domain::paths::mcp_path();
        let registry = McpRegistry::load(&path).unwrap_or_default();
        Self {
            registry,
            discovered: Vec::new(),
        }
    }
}

impl McpState {
    pub fn refresh(&mut self) {
        let path = crate::domain::paths::mcp_path();
        if let Ok(registry) = McpRegistry::load(&path) {
            self.registry = registry;
        }
    }

    /// Refresh discovered MCPs from scan results, filtering out ones already registered.
    pub fn refresh_with_discovered(&mut self, discovered: Vec<DiscoveredMcp>) {
        let registered: std::collections::HashSet<&str> =
            self.registry.servers.keys().map(|s| s.as_str()).collect();
        self.discovered = discovered
            .into_iter()
            .filter(|d| !registered.contains(d.name.as_str()))
            .collect();
    }

    pub fn servers_list(&self) -> Vec<(&String, &McpServer)> {
        let mut items: Vec<_> = self.registry.servers.iter().collect();
        items.sort_by(|a, b| a.0.cmp(b.0));
        items
    }
}

pub mod render;
