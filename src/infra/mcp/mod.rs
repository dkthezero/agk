use crate::app::ports::McpProvider;
use crate::domain::mcp::{McpRegistry, McpServer, McpTransport};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::time::Duration;

/// Register a new MCP server in the global registry.
pub fn register(
    name: &str,
    command: &str,
    args: Option<&str>,
    env: Option<&str>,
    transport_str: &str,
    description: Option<&str>,
) -> Result<McpServer> {
    let path = crate::domain::paths::mcp_path();
    let mut registry = McpRegistry::load(&path).unwrap_or_default();

    if registry.servers.contains_key(name) {
        bail!("MCP server '{}' already exists", name);
    }

    let args_vec: Vec<String> = args
        .map(|s| s.split_whitespace().map(|a| a.to_string()).collect())
        .unwrap_or_default();

    let env_map = env
        .map(|s| {
            s.split(',')
                .filter_map(|pair| {
                    let mut parts = pair.splitn(2, '=');
                    let key = parts.next()?.trim().to_string();
                    let val = parts.next()?.trim().to_string();
                    Some((key, val))
                })
                .collect()
        })
        .unwrap_or_default();

    let transport = match transport_str {
        "sse" => McpTransport::Sse {
            url: args_vec.first().cloned().unwrap_or_default(),
        },
        _ => McpTransport::Stdio,
    };

    let server = McpServer {
        name: name.to_string(),
        command: command.to_string(),
        args: args_vec,
        env: env_map,
        transport,
        description: description.map(|s| s.to_string()),
        tested: false,
        tested_at: None,
        activation: HashMap::new(),
    };

    registry.servers.insert(name.to_string(), server.clone());
    registry.save(&path)?;

    Ok(server)
}

/// Test an MCP server by spawning it and sending an initialize request.
pub async fn test_server(name: &str) -> Result<()> {
    let path = crate::domain::paths::mcp_path();
    let mut registry = McpRegistry::load(&path)?;

    let server = registry
        .servers
        .get_mut(name)
        .ok_or_else(|| anyhow::anyhow!("MCP server '{}' not found", name))?;

    match &server.transport {
        McpTransport::Stdio => {
            let mut cmd = tokio::process::Command::new(&server.command);
            cmd.args(&server.args);
            for (k, v) in &server.env {
                cmd.env(k, v);
            }

            let mut child = cmd
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .with_context(|| format!("Failed to spawn: {}", server.command))?;

            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| anyhow::anyhow!("Failed to open stdin"))?;
            let mut stdout = child
                .stdout
                .take()
                .ok_or_else(|| anyhow::anyhow!("Failed to open stdout"))?;

            let init_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "agk", "version": env!("CARGO_PKG_VERSION") }
            }
            });

            let request_str = format!("{}\n", init_request);
            tokio::io::AsyncWriteExt::write_all(&mut stdin, request_str.as_bytes()).await?;

            let mut buf = [0u8; 4096];
            let n = tokio::time::timeout(Duration::from_secs(10), async {
                tokio::io::AsyncReadExt::read(&mut stdout, &mut buf).await
            })
            .await??;

            child.kill().await.ok();

            if n == 0 {
                bail!("MCP server closed stdout without responding");
            }

            let response = String::from_utf8_lossy(&buf[..n]);
            if !response.contains("jsonrpc") {
                bail!("Invalid MCP response: {}", response);
            }
        }
        McpTransport::Sse { url } => {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?;
            let _res =
                client.get(url).send().await.with_context(|| {
                    format!("SSE MCP server '{}' did not respond at {}", name, url)
                })?;
        }
    }

    server.tested = true;
    server.tested_at = Some(chrono::Utc::now().to_rfc3339());
    registry.save(&path)?;

    Ok(())
}

/// Enable an MCP server for a provider in a scope.
pub fn enable(
    name: &str,
    provider_id: &str,
    scope: crate::domain::scope::Scope,
    providers: &[Box<dyn McpProvider>],
) -> Result<()> {
    let mcp_path = crate::domain::paths::mcp_path();
    let mut registry = McpRegistry::load(&mcp_path)?;

    let server = registry
        .servers
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("MCP server '{}' not found", name))?;

    let provider = providers
        .iter()
        .find(|p| p.provider_id() == provider_id)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Provider '{}' not found or does not support MCP",
                provider_id
            )
        })?;

    if !provider.supports_mcp() {
        bail!("Provider '{}' does not support MCP", provider_id);
    }

    provider.write_mcp_server(server, scope)?;

    let entry = registry
        .servers
        .get_mut(name)
        .unwrap()
        .activation
        .entry(provider_id.to_string())
        .or_default();
    match scope {
        crate::domain::scope::Scope::Global => entry.global = true,
        crate::domain::scope::Scope::Workspace => entry.workspace = true,
    }
    registry.save(&mcp_path)?;

    Ok(())
}

/// Disable an MCP server for a provider in a scope.
pub fn disable(
    name: &str,
    provider_id: &str,
    scope: crate::domain::scope::Scope,
    providers: &[Box<dyn McpProvider>],
) -> Result<()> {
    let mcp_path = crate::domain::paths::mcp_path();
    let mut registry = McpRegistry::load(&mcp_path)?;

    if !registry.servers.contains_key(name) {
        bail!("MCP server '{}' not found", name);
    }

    let provider = providers
        .iter()
        .find(|p| p.provider_id() == provider_id)
        .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", provider_id))?;

    if !provider.supports_mcp() {
        bail!("Provider '{}' does not support MCP", provider_id);
    }

    provider.remove_mcp_server(name, scope)?;

    if let Some(entry) = registry
        .servers
        .get_mut(name)
        .unwrap()
        .activation
        .get_mut(provider_id)
    {
        match scope {
            crate::domain::scope::Scope::Global => entry.global = false,
            crate::domain::scope::Scope::Workspace => entry.workspace = false,
        }
    }
    registry.save(&mcp_path)?;

    Ok(())
}

pub(crate) mod adapter;

pub fn build_mcp_providers(workspace_root: &std::path::Path) -> Vec<Box<dyn McpProvider>> {
    vec![
        Box::new(
            crate::infra::provider::claude_code::ClaudeCodeProvider::new(
                workspace_root.to_path_buf(),
            ),
        ),
        Box::new(crate::infra::provider::opencode::OpenCodeProvider::new(
            workspace_root.to_path_buf(),
        )),
        Box::new(crate::infra::provider::github::GithubProvider::new(
            workspace_root.to_path_buf(),
        )),
        Box::new(crate::infra::provider::gemini::GeminiProvider::new(
            workspace_root.to_path_buf(),
        )),
        Box::new(crate::infra::provider::amp::AmpProvider::new(
            workspace_root.to_path_buf(),
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.toml");
        let _ = std::fs::create_dir_all(path.parent().unwrap());

        // TODO: Test register + load
        let _ = path;
    }
}
