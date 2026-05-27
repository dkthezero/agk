use crate::app::command::CreateProfileInput;
use crate::domain::profile::ProfileId;

/// Messages emitted by the pure TUI reducer (`tui/reducer.rs`).
///
/// Each variant represents a user intent — **an action they tried to perform** —
/// not a side-effect command.  No variant contains I/O handles, process
/// spawns, or network calls.
#[derive(Debug, Clone, PartialEq)]
pub enum UiIntent {
    // -----------------------------------------------------------------------
    // Navigation
    // -----------------------------------------------------------------------
    SwitchTab(usize),
    NavigateUp,
    NavigateDown,
    ToggleScope,

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------
    StartSearch,
    AppendSearchChar(char),
    ClearSearch,

    // -----------------------------------------------------------------------
    // Modals / Wizards
    // -----------------------------------------------------------------------
    OpenAttachVaultWizard,
    OpenRegisterMcpWizard,
    OpenCreateProfileWizard,
    CloseModal,

    // -----------------------------------------------------------------------
    // Profile wizard steps
    // -----------------------------------------------------------------------
    UpdateProfileName(String),
    ToggleSkillInProfile(String),
    ToggleMcpInProfile(String),
    ConfirmProfileCreation(CreateProfileInput),

    // -----------------------------------------------------------------------
    // Asset actions
    // -----------------------------------------------------------------------
    InstallAsset(String),
    RemoveAsset(String),
    UpdateAsset(String),

    // -----------------------------------------------------------------------
    // Provider actions
    // -----------------------------------------------------------------------
    ActivateProvider(String),
    DeactivateProvider(String),

    // -----------------------------------------------------------------------
    // Profile actions
    // -----------------------------------------------------------------------
    StartProfile(ProfileId),
    DeleteProfile(ProfileId),

    // -----------------------------------------------------------------------
    // Vault actions
    // -----------------------------------------------------------------------
    AttachVault(String),
    DetachVault(String),

    // -----------------------------------------------------------------------
    // Apply config
    // -----------------------------------------------------------------------
    ApplyConfig(String),

    // -----------------------------------------------------------------------
    // System
    // -----------------------------------------------------------------------
    RequestQuit,
    RequestReload,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_intent_equality() {
        let a = UiIntent::NavigateUp;
        let b = UiIntent::NavigateUp;
        assert_eq!(a, b);
    }

    #[test]
    fn create_profile_intent_holds_input() {
        let input =
            CreateProfileInput::new("test", "opencode", crate::domain::scope::Scope::Workspace);
        let intent = UiIntent::ConfirmProfileCreation(input);
        assert!(matches!(intent, UiIntent::ConfirmProfileCreation(_)));
    }
}
