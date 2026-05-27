use crate::app::ports::ConfigStorePort;
use crate::app::registry::Registry;

pub fn build_with_store(
    workspace_root: std::path::PathBuf,
    store: crate::infra::config::toml_store::TomlConfigStore,
) -> anyhow::Result<(
    Registry,
    super::ScanResult,
    crate::infra::config::toml_store::TomlConfigStore,
)> {
    let mut registry = Registry::new();

    // Feature sets — order defines tab order
    registry.register_feature_set(Box::new(crate::infra::feature::skill::SkillFeatureSet));
    registry.register_feature_set(Box::new(crate::infra::feature::stub::StubFeatureSet::new(
        "mcp",
        "MCP Servers",
        "",
    )));
    registry.register_feature_set(Box::new(
        crate::infra::feature::instruction::InstructionFeatureSet,
    ));
    registry.register_feature_set(Box::new(crate::infra::feature::stub::StubFeatureSet::new(
        "provider",
        "Providers",
        "",
    )));
    registry.register_feature_set(Box::new(crate::infra::feature::stub::StubFeatureSet::new(
        "profile", "Profiles", "",
    )));
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
    let mut scan_result = super::scan(&registry, &registry.vaults)?;
    super::filter_scan(&mut scan_result, &global_config, Some(&workspace_config));

    Ok((registry, scan_result, store))
}

pub fn build(
    workspace_root: std::path::PathBuf,
) -> anyhow::Result<(
    Registry,
    super::ScanResult,
    crate::infra::config::toml_store::TomlConfigStore,
)> {
    let store = crate::infra::config::toml_store::TomlConfigStore::standard(&workspace_root);
    build_with_store(workspace_root, store)
}
