use crate::app::ports::WizardStep;
use crate::domain::scope::Scope;

/// Accumulator + UI state for the active profile-creation wizard.
#[derive(Clone, Debug, PartialEq)]
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
}

impl WizardState {
    /// Get the indices of options that match the current filter query.
    pub fn filtered_indices(&self) -> Vec<usize> {
        let option_count = match self.steps.get(self.step_index) {
            Some(WizardStep::Checklist { options, .. }) => Some(options.len()),
            Some(WizardStep::ToolSelect { tools, .. }) => Some(tools.len()),
            Some(WizardStep::PermissionSelect { modes, .. }) => Some(modes.len()),
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
