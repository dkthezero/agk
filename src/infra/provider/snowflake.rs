use crate::app::ports::ProviderPort;
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::identity::AssetIdentity;
use crate::domain::scope::Scope;
use crate::infra::provider::common;
use crate::infra::provider::common::copy_dir;
use anyhow::Result;
use std::path::PathBuf;

pub struct SnowflakeProvider {
    workspace_root: PathBuf,
}

impl SnowflakeProvider {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    fn provider_root(
        &self,
        scope: &Scope,
        _config: Option<&crate::domain::config::ConfigFile>,
    ) -> PathBuf {
        match scope {
            Scope::Global => dirs_next::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".cortex"),
            Scope::Workspace => self.workspace_root.join(".cortex"),
        }
    }

    fn asset_dir(
        &self,
        scope: &Scope,
        kind: &AssetKind,
        name: &str,
        config: Option<&crate::domain::config::ConfigFile>,
    ) -> PathBuf {
        let root = self.provider_root(scope, config);
        match kind {
            AssetKind::Skill => root.join("skills").join(name),
            AssetKind::Instruction => root.join("instructions").join(name),
            AssetKind::McpServer => PathBuf::new(),
            AssetKind::Profile => PathBuf::new(),
        }
    }
}

impl ProviderPort for SnowflakeProvider {
    fn id(&self) -> &str {
        "snowflake"
    }

    fn name(&self) -> &str {
        "Snowflake Cortex"
    }

    fn install(
        &self,
        pkg: &ScannedPackage,
        scope: Scope,
        config: Option<&crate::domain::config::ConfigFile>,
        _include_evals: bool,
    ) -> Result<()> {
        let dest = self.asset_dir(&scope, &pkg.kind, &pkg.identity.name, config);
        copy_dir(&pkg.path, &dest)
    }

    fn remove(
        &self,
        identity: &AssetIdentity,
        kind: &AssetKind,
        scope: Scope,
        config: Option<&crate::domain::config::ConfigFile>,
    ) -> Result<()> {
        let dest = self.asset_dir(&scope, kind, &identity.name, config);
        common::remove_dir_and_prune_empty_parents(&dest, 2)?;
        Ok(())
    }

    fn supports_mcp(&self) -> bool {
        false
    }
}

use crate::app::ports::McpProvider;
use crate::domain::mcp::McpServer;

impl McpProvider for SnowflakeProvider {
    fn provider_id(&self) -> &str {
        "snowflake"
    }

    fn supports_mcp(&self) -> bool {
        false
    }

    fn mcp_config_path(&self, _scope: Scope) -> Option<PathBuf> {
        None
    }

    fn write_mcp_server(&self, _server: &McpServer, _scope: Scope) -> Result<()> {
        anyhow::bail!("Snowflake does not support MCP")
    }

    fn remove_mcp_server(&self, _name: &str, _scope: Scope) -> Result<()> {
        anyhow::bail!("Snowflake does not support MCP")
    }
}
