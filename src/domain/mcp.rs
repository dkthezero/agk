use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP server configuration stored in agk's global registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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

// File-backed `McpRegistry::{load, save}` inherent impls were moved to
// `infra/mcp/registry_io.rs` by ADR-001 Commit 1 to keep the domain pure.
// The `mcp_registry_round_trip` integration test moved alongside the impl.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_server_serialize_round_trip() {
        let mut registry = McpRegistry::default();
        registry.servers.insert(
            "fs".to_string(),
            McpServer {
                name: "fs".to_string(),
                command: "npx".to_string(),
                args: vec!["server".to_string()],
                env: HashMap::new(),
                transport: McpTransport::Stdio,
                description: None,
                tested: false,
                tested_at: None,
                activation: HashMap::new(),
            },
        );

        let toml_text = toml::to_string_pretty(&registry).unwrap();
        let parsed: McpRegistry = toml::from_str(&toml_text).unwrap();
        assert!(parsed.servers.contains_key("fs"));
        assert_eq!(parsed.servers.get("fs").unwrap().command, "npx");
    }
}
