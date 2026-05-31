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

    /// Return available profile tools for providers that support tool selection.
    fn available_profile_tools(&self) -> Vec<String> {
        vec![]
    }

    /// Return available permission modes for providers that support it.
    /// Each entry is (mode_id, description).
    fn available_permission_modes(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// A single static description of a wizard step.  Mutable UI state lives in
/// `WizardState`, not here, so the step list can be cloned/replaced freely.
/// Archetype template for pre-filling wizard fields.
#[derive(Clone, Debug, PartialEq)]
pub struct ArchetypeTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub defaults: std::collections::HashMap<String, String>,
    pub default_tools: Vec<String>,
    pub default_permission_mode: Option<String>,
}

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
    /// Select an archetype template to pre-fill structured fields.
    TemplateSelect {
        title: String,
        templates: Vec<ArchetypeTemplate>,
    },
    /// Select scope (workspace / global).
    ScopeSelect {
        title: String,
    },
    /// Multi-line text input.
    Textarea {
        title: String,
        placeholder: String,
        rows: usize,
        /// Key used to store the answer in `structured_answers`.
        key: String,
    },
    /// Tool selection checklist (injected by provider if available).
    ToolSelect {
        title: String,
        tools: Vec<(String, String, bool)>, // (id, description, default)
    },
    /// Permission mode selection (injected by provider if available).
    PermissionSelect {
        title: String,
        modes: Vec<(String, String)>, // (id, description)
    },
    /// Reserved for future providers that want to embed an external interactive
    /// command as a distinct wizard step.  Not currently used by OpenCode.
    Interactive {
        title: String,
        command: String,
        args: Vec<String>,
    },
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
