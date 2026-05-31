use crate::app::ports::ConfigStorePort;
use crate::app::registry::Registry;

pub fn build_with_store(
    workspace_root: std::path::PathBuf,
    store: crate::infra::config::toml_store::TomlConfigStore,
) -> anyhow::Result<(
    Registry,
    super::ScanResult,
    crate::infra::config::toml_store::TomlConfigStore,
    crate::domain::config::ConfigFile,
    crate::domain::config::ConfigFile,
)> {
    let mut registry = Registry::new();

    // Feature sets — order defines tab order
    registry.register_feature_set(Box::new(crate::infra::feature::skill::SkillFeatureSet));
    registry.register_feature_set(Box::new(crate::infra::feature::mcp::McpFeatureSet));
    registry.register_feature_set(Box::new(
        crate::infra::feature::instruction::InstructionFeatureSet,
    ));
    registry.register_feature_set(Box::new(crate::infra::feature::stub::StubFeatureSet::new(
        "provider",
        "Providers",
        "",
    )));
    registry.register_feature_set(Box::new(crate::infra::feature::profile::ProfileFeatureSet));
    registry.register_feature_set(Box::new(crate::infra::feature::stub::StubFeatureSet::new(
        "vault", "Vaults", "",
    )));

    let mut global_config =
        ConfigStorePort::load(&store, crate::domain::scope::Scope::Global).unwrap_or_default();

    if !global_config.vault_defs.contains_key("clawhub") {
        global_config.vault_defs.insert(
            "clawhub".to_string(),
            crate::domain::config::VaultSection {
                vault: Some(crate::domain::config::VaultConfig::Clawhub(
                    crate::domain::config::ClawHubVaultSource {},
                )),
                skills: None,
                instructions: None,
                mcps: None,
                profiles: None,
            },
        );
        let _ = ConfigStorePort::save(&store, crate::domain::scope::Scope::Global, &global_config);
    }

    let workspace_config =
        ConfigStorePort::load(&store, crate::domain::scope::Scope::Workspace).unwrap_or_default();

    registry.register_provider(Box::new(
        crate::infra::provider::github::GithubProvider::new(workspace_root.clone()),
    ));
    registry.register_provider(Box::new(
        crate::infra::provider::firebender::FirebenderProvider::new(workspace_root.clone()),
    ));
    registry.register_provider(Box::new(crate::infra::provider::letta::LettaProvider::new(
        workspace_root.clone(),
    )));
    registry.register_provider(Box::new(
        crate::infra::provider::snowflake::SnowflakeProvider::new(workspace_root.clone()),
    ));
    registry.register_provider(Box::new(
        crate::infra::provider::gemini::GeminiProvider::new(workspace_root.clone()),
    ));
    registry.register_provider(Box::new(crate::infra::provider::amp::AmpProvider::new(
        workspace_root.clone(),
    )));
    registry.register_provider(Box::new(
        crate::infra::provider::claude_code::ClaudeCodeProvider::new(workspace_root.clone()),
    ));
    registry.register_provider(Box::new(
        crate::infra::provider::opencode::OpenCodeProvider::new(workspace_root.clone()),
    ));

    let active_vaults = super::build_vaults(&global_config, &workspace_root);
    for vault in active_vaults {
        registry.register_vault(vault);
    }

    // Scan is deferred to the first async TriggerReload so the TUI renders
    // instantly instead of blocking on filesystem I/O before entering alternate
    // screen.  The empty ScanResult is populated later via ReloadComplete.
    let scan_result = super::ScanResult {
        packages_by_tab: std::iter::repeat_with(Vec::new)
            .take(registry.feature_sets.len())
            .collect(),
    };

    Ok((
        registry,
        scan_result,
        store,
        global_config,
        workspace_config,
    ))
}

pub fn build(
    workspace_root: std::path::PathBuf,
) -> anyhow::Result<(
    Registry,
    super::ScanResult,
    crate::infra::config::toml_store::TomlConfigStore,
    crate::domain::config::ConfigFile,
    crate::domain::config::ConfigFile,
)> {
    let store = crate::infra::config::toml_store::TomlConfigStore::standard(&workspace_root);
    build_with_store(workspace_root, store)
}
