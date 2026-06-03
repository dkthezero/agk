use crate::app::command::CoreCommand;
use crate::app::features::profile::command::CreateProfileInput;
use crate::cli::entry::ProfileCommands;
use crate::domain::profile::{ProfileAssetRef, ProfileId, ProviderId};
use crate::domain::scope::Scope;
use anyhow::Context;

pub(super) fn to_core_command(command: &ProfileCommands) -> anyhow::Result<CoreCommand> {
    match command {
        ProfileCommands::Start { name, dry_run } => Ok(CoreCommand::StartProfile {
            id: ProfileId::new(name.clone()),
            scope: Scope::Workspace,
            dry_run: *dry_run,
        }),
        ProfileCommands::Create {
            name,
            provider,
            skills,
            mcps,
            description,
            description_file,
            scope,
            dry_run: _,
        } => {
            let desc = if let Some(path) = description_file {
                std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read description file: {}", path))?
            } else {
                description.clone().unwrap_or_default()
            };
            Ok(CoreCommand::CreateProfile {
                input: CreateProfileInput {
                    id: ProfileId::new(name.clone()),
                    provider_id: ProviderId::new(provider.clone()),
                    skill_refs: skills
                        .iter()
                        .map(|s| ProfileAssetRef::new(s.clone(), "auto"))
                        .collect(),
                    mcp_refs: mcps
                        .iter()
                        .map(|m| ProfileAssetRef::new(m.clone(), "auto"))
                        .collect(),
                    instruction_refs: vec![],
                    description: desc,
                    scope: scope.into_domain_scope(),
                },
            })
        }
        ProfileCommands::Export {
            name,
            file,
            resolve_vaults,
            scope,
        } => Ok(CoreCommand::ExportProfile {
            profile_id: ProfileId::new(name.clone()),
            scope: scope.into_domain_scope(),
            file_path: Some(file.clone()),
            resolve_vaults: *resolve_vaults,
        }),
        ProfileCommands::Import {
            file_path,
            name,
            scope,
        } => Ok(CoreCommand::ImportProfile {
            file_path: file_path.clone(),
            target_name: name.clone(),
            scope: scope.into_domain_scope(),
        }),
        ProfileCommands::Diff { name, scope } => Ok(CoreCommand::DiffProfile {
            id: ProfileId::new(name.clone()),
            scope: scope.into_domain_scope(),
        }),
    }
}
