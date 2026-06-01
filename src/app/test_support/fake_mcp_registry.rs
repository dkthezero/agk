use crate::app::ports::McpProvider;
use crate::app::ports::McpRegistryPort;
use crate::domain::mcp::McpServer;
use crate::domain::scope::Scope;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Mutex;

/// In-memory [`McpRegistryPort`] backed by a `HashMap` of servers.
///
/// Supports registration, listing, enable/disable, and provider building.
/// `test_server()` always succeeds — override behavior by replacing the
/// `on_test` closure if needed.
pub struct FakeMcpRegistry {
    servers: Mutex<HashMap<String, McpServer>>,
    #[allow(clippy::type_complexity)]
    pub on_test: Mutex<Box<dyn Fn(&str) -> Result<()> + Send>>,
}

impl std::fmt::Debug for FakeMcpRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FakeMcpRegistry")
            .field("servers", &self.servers)
            .finish_non_exhaustive()
    }
}

impl Default for FakeMcpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeMcpRegistry {
    pub fn new() -> Self {
        Self {
            servers: Mutex::new(HashMap::new()),
            on_test: Mutex::new(Box::new(|_| Ok(()))),
        }
    }

    pub fn seed(&self, server: McpServer) {
        self.servers
            .lock()
            .unwrap()
            .insert(server.name.clone(), server);
    }
}

impl McpRegistryPort for FakeMcpRegistry {
    fn register(
        &self,
        name: &str,
        command: &str,
        args: Option<&str>,
        env: Option<&str>,
        transport: &str,
        description: Option<&str>,
    ) -> Result<McpServer> {
        let parsed_args = args
            .map(|a| a.split_whitespace().map(String::from).collect())
            .unwrap_or_default();
        let parsed_env = env
            .map(|e| {
                let mut map = HashMap::new();
                for pair in e.split(',') {
                    let mut parts = pair.splitn(2, '=');
                    if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
                        map.insert(k.trim().to_string(), v.trim().to_string());
                    }
                }
                map
            })
            .unwrap_or_default();

        let server = McpServer {
            name: name.to_string(),
            command: command.to_string(),
            args: parsed_args,
            env: parsed_env,
            transport: parse_transport(transport),
            description: description.map(String::from),
            tested: false,
            tested_at: None,
            activation: HashMap::new(),
        };
        self.servers
            .lock()
            .unwrap()
            .insert(name.to_string(), server.clone());
        Ok(server)
    }

    fn list(&self) -> Result<Vec<McpServer>> {
        Ok(self.servers.lock().unwrap().values().cloned().collect())
    }

    fn test_server(&self, name: &str) -> Result<()> {
        self.on_test.lock().unwrap()(name)
    }

    fn build_providers(&self, _workspace_root: &std::path::Path) -> Vec<Box<dyn McpProvider>> {
        vec![]
    }

    fn enable(&self, name: &str, provider_id: &str, scope: Scope) -> Result<()> {
        let mut servers = self.servers.lock().unwrap();
        if let Some(server) = servers.get_mut(name) {
            let entry = server
                .activation
                .entry(provider_id.to_string())
                .or_default();
            match scope {
                Scope::Global => entry.global = true,
                Scope::Workspace => entry.workspace = true,
            }
        }
        Ok(())
    }

    fn disable(&self, name: &str, provider_id: &str, scope: Scope) -> Result<()> {
        let mut servers = self.servers.lock().unwrap();
        if let Some(server) = servers.get_mut(name) {
            if let Some(entry) = server.activation.get_mut(provider_id) {
                match scope {
                    Scope::Global => entry.global = false,
                    Scope::Workspace => entry.workspace = false,
                }
            }
        }
        Ok(())
    }

    fn unregister(&self, name: &str) -> Result<()> {
        let mut servers = self.servers.lock().unwrap();
        if servers.remove(name).is_none() {
            anyhow::bail!("MCP server '{}' not found", name);
        }
        Ok(())
    }
}

fn parse_transport(t: &str) -> crate::domain::mcp::McpTransport {
    if t.to_lowercase().starts_with("sse") {
        crate::domain::mcp::McpTransport::Sse {
            url: t
                .split_once(':')
                .map(|x| x.1)
                .unwrap_or("http://localhost")
                .to_string(),
        }
    } else {
        crate::domain::mcp::McpTransport::Stdio
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_mcp_register_and_list() {
        let reg = FakeMcpRegistry::new();
        let server = reg
            .register(
                "fs",
                "npx",
                Some("-y @modelcontextprotocol/server-filesystem"),
                None,
                "stdio",
                None,
            )
            .unwrap();
        assert_eq!(server.name, "fs");

        let list = reg.list().unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn fake_mcp_enable_disable() {
        let reg = FakeMcpRegistry::new();
        reg.register("fs", "npx", None, None, "stdio", None)
            .unwrap();
        reg.enable("fs", "claude-code", Scope::Global).unwrap();

        let list = reg.list().unwrap();
        let s = list[0].activation.get("claude-code").unwrap();
        assert!(s.global);
        assert!(!s.workspace);
    }
}
