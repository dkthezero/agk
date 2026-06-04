//! Assembles the wizard step list for a profile-create flow.
//!
//! Order:
//!   1. TextInput  (profile name)
//!   2. ProviderSelect  (always)
//!   3. LlmProviderSelect  (only if any LLM providers are configured)
//!   4. ModelInput  (always; claude-code always wants a model string)
//!   5. AgentDescription  (always; goes into agent markdown frontmatter)
//!   6. SkillsPick  (always; uses vault-discovered skills)
//!   7. ReviewFinal  (always; final confirmation)
//!
//! Provider-specific steps from `profile_wizard_steps()` are spliced in
//! BEFORE `ReviewFinal`.

use crate::app::ports::provider::ProviderPort;
use crate::app::ports::WizardStep;

pub fn build_step_list(
    provider: &dyn ProviderPort,
    configured_llm_provider_ids: &[String],
) -> Vec<WizardStep> {
    let mut steps: Vec<WizardStep> = vec![
        WizardStep::TextInput {
            title: "Profile name".into(),
            placeholder: "e.g. reviewer, docs-writer, swe-bench".into(),
        },
        WizardStep::ProviderSelect {
            title: "Pick the agent provider".into(),
            providers: vec![
                ("claude-code".into(), "Claude Code".into()),
                ("opencode".into(), "OpenCode".into()),
            ],
        },
    ];
    if !configured_llm_provider_ids.is_empty() {
        let providers: Vec<(String, String)> = configured_llm_provider_ids
            .iter()
            .map(|id| (id.clone(), id.clone()))
            .collect();
        steps.push(WizardStep::LlmProviderSelect {
            title: "Pick the LLM provider".into(),
            providers,
        });
    }
    steps.push(WizardStep::ModelInput {
        title: "Model".into(),
        placeholder: "e.g. claude-sonnet-4-5 or llama3.2:8b".into(),
    });
    steps.push(WizardStep::AgentDescription {
        title: "Describe what this agent does".into(),
        placeholder: "Used as the agent's `description` frontmatter".into(),
        rows: 5,
    });
    steps.push(WizardStep::SkillsPick {
        title: "Pick skills to attach".into(),
        options: vec![],
    });
    for step in provider.profile_wizard_steps() {
        steps.push(step);
    }
    steps.push(WizardStep::ReviewFinal {
        title: "Review and create".into(),
    });
    steps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::provider::ProviderPort;
    use crate::app::ports::WizardStep;
    use crate::domain::asset::AssetKind;
    use crate::domain::config::ConfigFile;
    use crate::domain::scope::Scope;

    struct StubProvider;
    impl ProviderPort for StubProvider {
        fn id(&self) -> &str {
            "claude-code"
        }
        fn name(&self) -> &str {
            "Claude Code"
        }
        fn install(
            &self,
            _: &crate::domain::asset::ScannedPackage,
            _: Scope,
            _: Option<&ConfigFile>,
            _: bool,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            _: &crate::domain::identity::AssetIdentity,
            _: &AssetKind,
            _: Scope,
            _: Option<&ConfigFile>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn supports_profiles(&self) -> bool {
            true
        }
        fn profile_wizard_steps(&self) -> Vec<WizardStep> {
            vec![]
        }
    }

    #[test]
    fn build_step_list_includes_provider_select_for_claude_code() {
        let steps = build_step_list(&StubProvider, &[]);
        assert!(matches!(steps[0], WizardStep::TextInput { .. }));
        assert!(matches!(steps[1], WizardStep::ProviderSelect { .. }));
    }

    #[test]
    fn build_step_list_includes_llm_select_when_providers_configured() {
        let steps = build_step_list(&StubProvider, &["local-ollama".to_string()]);
        assert!(steps
            .iter()
            .any(|s| matches!(s, WizardStep::LlmProviderSelect { .. })));
        assert!(steps
            .iter()
            .any(|s| matches!(s, WizardStep::ModelInput { .. })));
    }

    #[test]
    fn build_step_list_omits_llm_select_when_no_providers() {
        let steps = build_step_list(&StubProvider, &[]);
        assert!(!steps
            .iter()
            .any(|s| matches!(s, WizardStep::LlmProviderSelect { .. })));
    }

    #[test]
    fn build_step_list_always_ends_with_review_final() {
        let steps = build_step_list(&StubProvider, &[]);
        assert!(matches!(steps.last(), Some(WizardStep::ReviewFinal { .. })));
    }
}
