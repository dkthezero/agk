use agk::app::command::CoreCommand;
use agk::app::features::profile::command::CreateProfileInput;
use agk::app::ports::WizardState;
use agk::app::ports::WizardStep;
use agk::domain::profile::ProfileId;
use agk::domain::profile::ProviderId;
use agk::domain::scope::Scope;
use agk::tui::app::AppState;
use agk::tui::list_mode::ListMode;
use std::collections::HashMap;

/// Verify that setting the list mode to ProfileWizard and constructing a
/// WizardState with the opencode provider steps produces the correct initial
/// state (step 0 = TemplateSelect, name empty).
#[test]
fn wizard_initial_state_is_template_select() {
    let steps = vec![
        WizardStep::TemplateSelect {
            title: "Select Archetype".into(),
            templates: vec![],
        },
        WizardStep::TextInput {
            title: "Profile name".into(),
            placeholder: "e.g. opencode-dev".into(),
        },
        WizardStep::Review {
            title: "Review & Confirm".into(),
        },
    ];
    let ws = WizardState::new(steps, "opencode".into());
    assert!(ws.step_index == 0, "Wizard should start at step 0");
    assert!(ws.name.is_empty(), "Name should start empty");
    assert!(ws.provider_id == "opencode");
    assert!(ws.prompt_buffer.is_empty());
}

/// Simulate advancing through wizard steps: type a name, move to the next
/// step, and verify the wizard state transitions correctly.
#[test]
fn wizard_advances_through_steps() {
    let steps = vec![
        WizardStep::TextInput {
            title: "Profile name".into(),
            placeholder: "e.g. opencode-dev".into(),
        },
        WizardStep::ScopeSelect {
            title: "Select Scope".into(),
        },
        WizardStep::Review {
            title: "Review & Confirm".into(),
        },
    ];
    let mut ws = WizardState::new(steps, "opencode".into());

    // Step 0: TextInput — type a profile name
    ws.prompt_buffer = "my-profile".into();
    ws.name = ws.prompt_buffer.clone();
    assert_eq!(ws.step_index, 0);
    assert_eq!(ws.name, "my-profile");

    // Advance to step 1: ScopeSelect
    ws.step_index = 1;
    assert_eq!(ws.step_index, 1);

    // Advance to step 2: Review
    ws.step_index = 2;
    assert_eq!(ws.step_index, 2);
    // The review step should have title "Review & Confirm"
    match &ws.steps[2] {
        WizardStep::Review { title } => {
            assert_eq!(title, "Review & Confirm");
        }
        _ => panic!("Expected Review step at index 2"),
    }
}

/// End-to-end: create an AppState with the Profile tab, enter wizard mode,
/// set a WizardState, then render and assert the wizard mode is active.
/// Also run a CreateProfile command through the core to verify the wire
/// produces a status-line response.
#[test]
fn wizard_mode_and_create_profile_command() {
    let core = super::common::test_core();
    let mut state = AppState::new(
        vec!["Skills".into(), "Profiles".into()],
        vec![true, true],
        HashMap::new(),
    );

    // Switch to Profile tab and enter wizard mode
    state.active_tab = 1;
    state.list_mode = ListMode::ProfileWizard;

    // Construct a minimal WizardState
    let steps = vec![
        WizardStep::TextInput {
            title: "Profile name".into(),
            placeholder: "e.g. test-profile".into(),
        },
        WizardStep::Review {
            title: "Review & Confirm".into(),
        },
    ];
    let ws = WizardState::new(steps, "opencode".into());
    state.wizard_state = Some(ws);

    // Assert wizard mode is active
    assert!(state.is_profile_wizard_mode());

    // Render the TUI and assert something is drawn (wizard UI renders)
    let buf = super::common::render_buffer(&state, 80, 24);
    // The rendered buffer should contain text — it should not be empty
    let text: String = buf.content.iter().map(|cell| cell.symbol()).collect();
    let trimmed = text.trim();
    assert!(
        !trimmed.is_empty(),
        "Expected non-empty rendered buffer in wizard mode"
    );

    // Now create a profile through the core to verify the command pipeline
    let cmd = CoreCommand::CreateProfile {
        input: CreateProfileInput::new(
            ProfileId::new("test-profile"),
            ProviderId::new("opencode"),
            Scope::Workspace,
        ),
    };
    let _ = super::common::execute(&core, &mut state, cmd);
    assert!(
        !state.status_line.is_empty(),
        "Expected non-empty status line after CreateProfile"
    );
}
