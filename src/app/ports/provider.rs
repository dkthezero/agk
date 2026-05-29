use crate::app::ports::profile_runtime::ProfileSession;
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::config::ConfigFile;
use crate::domain::identity::AssetIdentity;
use crate::domain::scope::Scope;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub trait ProviderPort: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn install(
        &self,
        pkg: &ScannedPackage,
        scope: Scope,
        config: Option<&ConfigFile>,
        include_evals: bool,
    ) -> Result<()>;
    fn remove(
        &self,
        identity: &AssetIdentity,
        kind: &AssetKind,
        scope: Scope,
        config: Option<&ConfigFile>,
    ) -> Result<()>;

    /// Return the expected on-disk install path for the given asset, if known.
    /// Defaults to `None` for providers where the path convention is not exposed.
    fn install_path_for(
        &self,
        _identity: &AssetIdentity,
        _kind: &AssetKind,
        _scope: Scope,
    ) -> Option<PathBuf> {
        None
    }

    /// Return a list of alternative config root folder names this provider
    /// supports. Each entry is (folder_name, description).
    /// Default empty vec means the provider has a single hardcoded root.
    fn available_config_roots(&self) -> Vec<(String, String)> {
        vec![]
    }

    /// Return true if this provider supports profile sessions.
    fn supports_profiles(&self) -> bool {
        false
    }

    /// Start a profile session. Only called if supports_profiles() is true.
    fn start_profile_session(
        &self,
        _profile: &crate::domain::config::Profile,
        _session_key: &str,
        _workspace_root: &Path,
    ) -> Result<ProfileSession> {
        anyhow::bail!("Profile sessions not supported by this provider")
    }

    /// Return wizard steps if this provider supports profile creation.
    fn profile_wizard_steps(&self) -> Vec<WizardStep> {
        vec![]
    }
}

/// A single static description of a wizard step.  Mutable UI state lives in
/// `WizardState`, not here, so the step list can be cloned/replaced freely.
#[derive(Clone, Debug, PartialEq)]
pub enum WizardStep {
    TextInput {
        title: String,
        placeholder: String,
    },
    QuestionAnswer {
        question: String,
        placeholder: String,
    },
    Checklist {
        title: String,
        options: Vec<String>,
    },
    Review {
        title: String,
    },
    /// Reserved for future providers that want to embed an external interactive
    /// command as a distinct wizard step.  Not currently used by OpenCode.
    Interactive {
        title: String,
        command: String,
        args: Vec<String>,
    },
}

/// Accumulator + UI state for the active profile-creation wizard.
#[derive(Clone, Debug, PartialEq)]
pub struct WizardState {
    pub steps: Vec<WizardStep>,
    pub step_index: usize,
    /// Profile name collected in step 0.
    pub name: String,
    /// (question, answer) pairs from Q&A steps.
    pub description_parts: Vec<(String, String)>,
    pub skills: Vec<String>,
    pub mcps: Vec<String>,
    pub skill_options: Vec<String>,
    pub mcp_options: Vec<String>,
    /// Shared text buffer for TextInput / QuestionAnswer steps.
    pub prompt_buffer: String,
    /// Shared checklist state for Checklist steps.
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
}

impl WizardState {
    /// Get the indices of options that match the current filter query.
    pub fn filtered_indices(&self) -> Vec<usize> {
        if let Some(WizardStep::Checklist { options, .. }) = self.steps.get(self.step_index) {
            let q = self.filter_query.to_lowercase();
            if q.is_empty() {
                (0..options.len()).collect()
            } else {
                options
                    .iter()
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
        };
        ws.sync_checklist_state();
        ws
    }

    /// Resize `checked` and reset `selected` when the current step is a Checklist.
    /// Always resets when entering a new checklist step to prevent state leakage.
    pub fn sync_checklist_state(&mut self) {
        if let Some(WizardStep::Checklist { options, .. }) = self.steps.get(self.step_index) {
            if self.checked_step_index != Some(self.step_index) {
                self.checked = vec![false; options.len()];
                self.selected = self.selected.min(options.len().saturating_sub(1));
                self.checked_step_index = Some(self.step_index);
            }
        }
    }

    /// Compose the full description string from Q&A pairs.
    pub fn composed_description(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        for (q, a) in &self.description_parts {
            lines.push(format!("Q: {}", q));
            lines.push(format!("A: {}", a));
            lines.push(String::new());
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyProvider;
    impl ProviderPort for DummyProvider {
        fn id(&self) -> &str {
            "dummy"
        }
        fn name(&self) -> &str {
            "Dummy"
        }
        fn install(
            &self,
            _: &ScannedPackage,
            _: Scope,
            _: Option<&ConfigFile>,
            _: bool,
        ) -> Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            _: &AssetIdentity,
            _: &AssetKind,
            _: Scope,
            _: Option<&ConfigFile>,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn provider_port_default_available_roots_empty() {
        let p = DummyProvider;
        assert!(p.available_config_roots().is_empty());
    }
}
