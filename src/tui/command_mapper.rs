use crate::app::command::CoreCommand;
use crate::tui::intent::UiIntent;

/// Stateless mapper: translates a list of [`UiIntent`]s into
/// [`CoreCommand`]s that can be executed by [`crate::app::core::AgkCore`].
///
/// This is the bridge between the pure UI reducer and the application core.
/// No side-effects occur here; the caller is responsible for invoking
/// `AgkCore::execute` with the returned commands.
pub fn map_intents(intents: Vec<UiIntent>) -> Vec<CoreCommand> {
    intents.into_iter().filter_map(map_single_intent).collect()
}

fn map_single_intent(intent: UiIntent) -> Option<CoreCommand> {
    match intent {
        UiIntent::ActivateProvider(id) => Some(CoreCommand::ActivateProvider {
            id,
            scope: crate::domain::scope::Scope::Workspace,
        }),
        UiIntent::DeactivateProvider(id) => Some(CoreCommand::DeactivateProvider {
            id,
            scope: crate::domain::scope::Scope::Workspace,
        }),
        UiIntent::StartProfile(id) => Some(CoreCommand::StartProfile {
            id,
            scope: crate::domain::scope::Scope::Workspace,
            dry_run: false,
        }),
        UiIntent::DeleteProfile(id) => Some(CoreCommand::DeleteProfile {
            id,
            scope: crate::domain::scope::Scope::Workspace,
        }),
        UiIntent::RequestReload => Some(CoreCommand::LoadWorkspaceSnapshot {
            scope: crate::domain::scope::Scope::Workspace,
        }),
        UiIntent::ApplyConfig(source) => Some(CoreCommand::ApplyConfig {
            input: crate::app::command::ApplyConfigInput::from_url(source),
            scope: crate::domain::scope::Scope::Workspace,
            environment: None,
            context: None,
            dry_run: false,
        }),
        // TODO: Phase 3 – map remaining intents as use-cases are implemented.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::profile::ProfileId;
    use crate::domain::scope::Scope;

    #[test]
    fn start_profile_maps_to_command() {
        let intents = vec![UiIntent::StartProfile(ProfileId::new("dev"))];
        let commands = map_intents(intents);
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            &commands[0],
            CoreCommand::StartProfile { id, scope, dry_run }
            if id.as_str() == "dev" && *scope == Scope::Workspace && !dry_run
        ));
    }

    #[test]
    fn delete_profile_maps_to_command() {
        let intents = vec![UiIntent::DeleteProfile(ProfileId::new("old"))];
        let commands = map_intents(intents);
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            &commands[0],
            CoreCommand::DeleteProfile { id, .. }
            if id.as_str() == "old"
        ));
    }

    #[test]
    fn unmapped_intents_are_filtered() {
        let intents = vec![
            UiIntent::NavigateUp,
            UiIntent::NavigateDown,
            UiIntent::SwitchTab(0),
        ];
        let commands = map_intents(intents);
        assert!(commands.is_empty());
    }
}
