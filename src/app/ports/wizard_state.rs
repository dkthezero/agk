use crate::app::ports::WizardStep;
use crate::domain::scope::Scope;
use serde::{Deserialize, Serialize};

/// Accumulator + UI state for the active profile-creation wizard.
///
/// `Serialize`/`Deserialize` are derived so the wizard can be snapshotted
/// to disk between steps and resumed mid-flow. New fields added in v0.4
/// use `#[serde(default)]` so older snapshots still deserialize cleanly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WizardState {
    pub steps: Vec<WizardStep>,
    pub step_index: usize,
    /// Profile name collected in step 0.
    pub name: String,
    /// (question, answer) pairs from Q&A steps (legacy path).
    pub description_parts: Vec<(String, String)>,
    pub skills: Vec<String>,
    pub mcps: Vec<String>,
    pub skill_options: Vec<String>,
    pub mcp_options: Vec<String>,
    /// Shared text buffer for TextInput / QuestionAnswer / Textarea steps.
    pub prompt_buffer: String,
    /// Shared checklist state for Checklist / ToolSelect steps.
    pub checked: Vec<bool>,
    pub selected: usize,
    /// Cursor position tracked in **character indices** (not bytes) so
    /// multi-byte UTF-8 characters are handled correctly.
    pub cursor_pos: usize,
    /// Provider id that produced this wizard.
    pub provider_id: String,
    /// Tracks which step_index `checked` was last initialized for, so
    /// entering a different checklist step always resets state even if
    /// option counts happen to match.
    pub checked_step_index: Option<usize>,
    /// Vertical scroll offset for the Review step (wrapped lines).
    pub scroll_offset: usize,
    /// Search query for Checklist steps (filtered options).
    pub filter_query: String,
    /// Selected archetype template ID (if any).
    pub selected_template: Option<String>,
    /// Scope selection (workspace / global).
    pub scope: Option<Scope>,
    /// Structured answers: key -> value.
    pub structured_answers: std::collections::HashMap<String, String>,
    /// Selected tool IDs.
    pub selected_tools: Vec<String>,
    /// Selected permission mode.
    pub selected_permission_mode: Option<String>,
    /// Provider id picked on the ProviderSelect step.
    #[serde(default)]
    pub provider_id_choice: String,
    /// LLM provider id picked on the LlmProviderSelect step.
    #[serde(default)]
    pub llm_provider_id: String,
    /// Free-form model string captured on the ModelInput step.
    #[serde(default)]
    pub model_string: String,
    /// Multi-line agent description captured on the AgentDescription step.
    #[serde(default)]
    pub agent_description: String,
}

impl WizardState {
    /// Get the indices of options that match the current filter query.
    pub fn filtered_indices(&self) -> Vec<usize> {
        let option_count = match self.steps.get(self.step_index) {
            Some(WizardStep::Checklist { options, .. }) => Some(options.len()),
            Some(WizardStep::ToolSelect { tools, .. }) => Some(tools.len()),
            Some(WizardStep::PermissionSelect { modes, .. }) => Some(modes.len()),
            Some(WizardStep::SkillsPick { options, .. }) => Some(options.len()),
            _ => None,
        };
        if let Some(count) = option_count {
            let q = self.filter_query.to_lowercase();
            if q.is_empty() {
                (0..count).collect()
            } else {
                let options: Vec<String> = match self.steps.get(self.step_index) {
                    Some(WizardStep::Checklist { options, .. }) => options.clone(),
                    Some(WizardStep::ToolSelect { tools, .. }) => {
                        tools.iter().map(|(id, _, _)| id.clone()).collect()
                    }
                    Some(WizardStep::PermissionSelect { modes, .. }) => {
                        modes.iter().map(|(id, _)| id.clone()).collect()
                    }
                    Some(WizardStep::SkillsPick { options, .. }) => options.clone(),
                    _ => vec![],
                };
                options
                    .into_iter()
                    .enumerate()
                    .filter(|(_, opt)| opt.to_lowercase().contains(&q))
                    .map(|(i, _)| i)
                    .collect()
            }
        } else {
            vec![]
        }
    }

    /// Get the currently selected option index (in the original options array),
    /// accounting for filtered view.
    pub fn selected_original_index(&self) -> Option<usize> {
        let filtered = self.filtered_indices();
        filtered.get(self.selected).copied()
    }
}

impl WizardState {
    pub fn new(steps: Vec<WizardStep>, provider_id: String) -> Self {
        let mut ws = Self {
            steps,
            step_index: 0,
            name: String::new(),
            description_parts: Vec::new(),
            skills: Vec::new(),
            mcps: Vec::new(),
            skill_options: Vec::new(),
            mcp_options: Vec::new(),
            prompt_buffer: String::new(),
            checked: vec![],
            selected: 0,
            cursor_pos: 0,
            provider_id,
            checked_step_index: None,
            scroll_offset: 0,
            filter_query: String::new(),
            selected_template: None,
            scope: None,
            structured_answers: std::collections::HashMap::new(),
            selected_tools: Vec::new(),
            selected_permission_mode: None,
            provider_id_choice: String::new(),
            llm_provider_id: String::new(),
            model_string: String::new(),
            agent_description: String::new(),
        };
        ws.sync_checklist_state();
        ws
    }

    /// Resize `checked` and reset `selected` when the current step is a Checklist.
    /// Always resets when entering a new checklist step to prevent state leakage.
    /// Pre-checks items that match `selected_tools` or `selected_permission_mode`.
    pub fn sync_checklist_state(&mut self) {
        let (options_len, pre_checked) = match self.steps.get(self.step_index) {
            Some(WizardStep::Checklist { options, .. }) => (Some(options.len()), None),
            Some(WizardStep::ToolSelect { tools, .. }) => {
                let pre: Vec<usize> = tools
                    .iter()
                    .enumerate()
                    .filter(|(_, (id, _, _))| self.selected_tools.contains(id))
                    .map(|(i, _)| i)
                    .collect();
                (Some(tools.len()), Some(pre))
            }
            Some(WizardStep::PermissionSelect { modes, .. }) => {
                let pre: Vec<usize> = if let Some(ref mode) = self.selected_permission_mode {
                    modes
                        .iter()
                        .enumerate()
                        .filter(|(_, (id, _))| id == mode)
                        .map(|(i, _)| i)
                        .collect()
                } else {
                    vec![]
                };
                (Some(modes.len()), Some(pre))
            }
            Some(WizardStep::SkillsPick { options, .. }) => {
                // Pre-check items already present in `skills` so the user
                // sees their previous selection when re-entering the step.
                let pre: Vec<usize> = options
                    .iter()
                    .enumerate()
                    .filter(|(_, name)| self.skills.iter().any(|s| s == *name))
                    .map(|(i, _)| i)
                    .collect();
                (Some(options.len()), Some(pre))
            }
            _ => (None, None),
        };
        if let Some(len) = options_len {
            if self.checked_step_index != Some(self.step_index) {
                self.checked = vec![false; len];
                self.selected = self.selected.min(len.saturating_sub(1));
                self.checked_step_index = Some(self.step_index);
                if let Some(indices) = pre_checked {
                    for i in indices {
                        if let Some(c) = self.checked.get_mut(i) {
                            *c = true;
                        }
                    }
                }
            }
        }
    }

    /// Compose the full description string.
    ///
    /// If structured answers exist (v0.3+ path), generates canonical markdown.
    /// Otherwise falls back to legacy Q&A concatenation.
    pub fn composed_description(&self) -> String {
        if self.structured_answers.is_empty() {
            // Legacy Q&A path
            let mut lines: Vec<String> = Vec::new();
            for (q, a) in &self.description_parts {
                lines.push(format!("Q: {}", q));
                lines.push(format!("A: {}", a));
                lines.push(String::new());
            }
            lines.join("\n")
        } else {
            crate::app::features::profile::wizard_description::compose_description(
                &self.structured_answers,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_steps() -> Vec<WizardStep> {
        vec![
            WizardStep::Checklist {
                title: "Skills".into(),
                options: vec!["rust".into(), "python".into()],
            },
            WizardStep::ToolSelect {
                title: "Tools".into(),
                tools: vec![
                    ("Read".into(), "Read".into(), false),
                    ("Grep".into(), "Grep".into(), false),
                ],
            },
            WizardStep::PermissionSelect {
                title: "Permissions".into(),
                modes: vec![
                    ("auto".into(), "Auto-approve".into()),
                    ("plan".into(), "Plan only".into()),
                ],
            },
        ]
    }

    #[test]
    fn sync_checklist_state_resets_on_new_step() {
        let mut ws = WizardState::new(simple_steps(), "opencode".into());
        // Step 0: Checklist with 2 options
        assert_eq!(ws.checked.len(), 2);
        assert_eq!(ws.checked, vec![false, false]);

        // Manually check an item, then advance to a different step type
        ws.checked[0] = true;
        ws.step_index = 1;
        ws.sync_checklist_state();
        // ToolSelect step has 2 tools — checked should reset to all false
        assert_eq!(ws.checked.len(), 2);
        assert_eq!(ws.checked, vec![false, false]);
    }

    #[test]
    fn sync_checklist_state_pre_checks_tools() {
        let mut ws = WizardState::new(simple_steps(), "opencode".into());
        ws.selected_tools = vec!["Grep".into()];
        ws.step_index = 1; // ToolSelect
        ws.checked_step_index = None; // force re-sync
        ws.sync_checklist_state();
        assert_eq!(ws.checked, vec![false, true]); // Grep is pre-checked
    }

    #[test]
    fn sync_checklist_state_pre_checks_permission_mode() {
        let mut ws = WizardState::new(simple_steps(), "opencode".into());
        ws.selected_permission_mode = Some("plan".into());
        ws.step_index = 2; // PermissionSelect
        ws.checked_step_index = None; // force re-sync
        ws.sync_checklist_state();
        assert_eq!(ws.checked, vec![false, true]); // "plan" is at index 1
    }

    #[test]
    fn sync_checklist_state_noop_on_non_checklist_step() {
        let ws = WizardState::new(
            vec![WizardStep::TextInput {
                title: "Name".into(),
                placeholder: "foo".into(),
            }],
            "opencode".into(),
        );
        // TextInput is not a checklist step — checked should stay empty
        assert!(ws.checked.is_empty());
    }

    #[test]
    fn filtered_indices_matches_filter() {
        let mut ws = WizardState::new(simple_steps(), "opencode".into());
        ws.step_index = 1; // ToolSelect step
        ws.filter_query = "grep".into(); // matches only "Grep"
        let indices = ws.filtered_indices();
        assert_eq!(indices, vec![1]); // Grep is at index 1
    }

    #[test]
    fn composed_description_uses_structured_answers() {
        let mut ws = WizardState::new(vec![], "opencode".into());
        ws.structured_answers
            .insert("role".into(), "Rust dev".into());
        let desc = ws.composed_description();
        assert!(desc.contains("Rust dev"));
    }

    #[test]
    fn new_wizard_variants_construct() {
        let _ = WizardStep::ProviderSelect {
            title: "Pick agent provider".into(),
            providers: vec![
                ("claude-code".into(), "Claude Code".into()),
                ("opencode".into(), "OpenCode".into()),
            ],
        };
        let _ = WizardStep::LlmProviderSelect {
            title: "Pick LLM provider".into(),
            providers: vec![("local-ollama".into(), "Ollama (local)".into())],
        };
        let _ = WizardStep::ModelInput {
            title: "Model string".into(),
            placeholder: "e.g. claude-sonnet-4-5 or llama3.2".into(),
        };
        let _ = WizardStep::AgentDescription {
            title: "Describe this agent".into(),
            placeholder: "Used as the agent's `description` frontmatter".into(),
            rows: 5,
        };
        let _ = WizardStep::SkillsPick {
            title: "Pick skills".into(),
            options: vec!["code-review".into()],
        };
        let _ = WizardStep::ReviewFinal {
            title: "Review and confirm".into(),
        };
    }
}
