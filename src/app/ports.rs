use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::config::ConfigFile;
use crate::domain::identity::AssetIdentity;
use crate::domain::mcp::McpServer;
use crate::domain::scope::Scope;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub trait FeatureSetPort: Send + Sync {
    fn kind_name(&self) -> &str;
    fn display_name(&self) -> &str;
    fn scan_root(&self) -> &str;
    fn asset_kind(&self) -> AssetKind;
    fn is_package(&self, path: &Path) -> bool;
    fn hash_files(&self, path: &Path) -> Vec<PathBuf>;

    fn extract_version(&self, _path: &Path) -> Option<String> {
        None
    }

    /// Override to return `true` for placeholder tabs not yet implemented.
    fn is_stub(&self) -> bool {
        false
    }
}

#[async_trait::async_trait]
pub trait VaultPort: Send + Sync {
    fn id(&self) -> &str;
    #[allow(dead_code)]
    fn kind_name(&self) -> &str;

    async fn refresh(&self) -> Result<()> {
        Ok(())
    }

    fn list_packages(&self, feature: &dyn FeatureSetPort) -> Result<Vec<ScannedPackage>>;
}

pub trait ConfigStorePort: Send + Sync {
    fn load(&self, scope: Scope) -> Result<ConfigFile>;
    fn save(&self, scope: Scope, config: &ConfigFile) -> Result<()>;
    /// Delete the backing file for the scope if it exists.
    /// Default no-op for stores that don't have a file.
    fn delete_file(&self, _scope: Scope) -> Result<()> {
        Ok(())
    }
}

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
    #[allow(dead_code)]
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

/// Handle for a running profile session.
pub struct ProfileSession {
    pub process: std::process::Child,
    pub cleanup: Box<dyn FnOnce() -> Result<()> + Send>,
}

/// Extension trait for providers that support MCP configuration.
pub trait McpProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn supports_mcp(&self) -> bool;
    #[allow(dead_code)]
    fn mcp_config_path(&self, scope: Scope) -> Option<PathBuf>;
    fn write_mcp_server(&self, server: &McpServer, scope: Scope) -> Result<()>;
    fn remove_mcp_server(&self, name: &str, scope: Scope) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestFeatureSet;
    impl FeatureSetPort for TestFeatureSet {
        fn kind_name(&self) -> &str {
            "test"
        }
        fn display_name(&self) -> &str {
            "Test"
        }
        fn scan_root(&self) -> &str {
            "test_root"
        }
        fn asset_kind(&self) -> AssetKind {
            AssetKind::Skill
        }
        fn is_package(&self, _: &Path) -> bool {
            false
        }
        fn hash_files(&self, _: &Path) -> Vec<PathBuf> {
            vec![]
        }
    }

    #[test]
    fn feature_set_port_default_not_stub() {
        let f = TestFeatureSet;
        assert!(!f.is_stub());
    }

    #[test]
    fn feature_set_port_kind_name() {
        let f = TestFeatureSet;
        assert_eq!(f.kind_name(), "test");
    }

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
