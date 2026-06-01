use crate::app::ports::ProviderPort;
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::identity::AssetIdentity;
use crate::domain::scope::Scope;
use crate::infra::provider::common;
use crate::infra::provider::common::{copy_dir, copy_dir_filtered};
use crate::infra::provider::opencode::OpenCodeProvider;
use anyhow::Result;
use std::path::PathBuf;

impl ProviderPort for OpenCodeProvider {
    fn id(&self) -> &str {
        "opencode"
    }

    fn name(&self) -> &str {
        "OpenCode"
    }

    fn install_path_for(
        &self,
        identity: &AssetIdentity,
        kind: &AssetKind,
        scope: Scope,
    ) -> Option<PathBuf> {
        if *kind == AssetKind::McpServer {
            return None;
        }
        Some(self.asset_dir(&scope, kind, &identity.name, None))
    }

    fn install(
        &self,
        pkg: &ScannedPackage,
        scope: Scope,
        config: Option<&crate::domain::config::ConfigFile>,
        include_evals: bool,
    ) -> Result<()> {
        let dest = self.asset_dir(&scope, &pkg.kind, &pkg.identity.name, config);
        if include_evals {
            copy_dir(&pkg.path, &dest)?;
        } else {
            copy_dir_filtered(&pkg.path, &dest, common::is_not_evals)?;
        }

        // OpenCode does NOT accept a "skills" key in opencode.json.
        // Skills are auto-discovered from the .opencode/skills directory.
        // Self-heal: strip any stale "skills" array left by older agk versions
        // so users upgrading from the buggy build get a working config.
        self.drop_stale_skills_array(&scope)?;
        Ok(())
    }

    fn remove(
        &self,
        identity: &AssetIdentity,
        kind: &AssetKind,
        scope: Scope,
        config: Option<&crate::domain::config::ConfigFile>,
    ) -> Result<()> {
        let dest = self.asset_dir(&scope, kind, &identity.name, config);
        common::remove_dir_and_prune_empty_parents(&dest, 2)?;

        // Also remove any stale "skills" array that agk may have written in an
        // earlier version.  OpenCode rejects this key, so we quietly strip it.
        self.drop_stale_skills_array(&scope)?;
        Ok(())
    }

    fn available_config_roots(&self) -> Vec<(String, String)> {
        vec![
            (
                ".opencode".to_string(),
                "OpenCode native folder".to_string(),
            ),
            (
                ".agents".to_string(),
                "Shared agents folder (Claude-compatible)".to_string(),
            ),
        ]
    }

    fn supports_profiles(&self) -> bool {
        true
    }

    fn supports_mcp(&self) -> bool {
        true
    }

    fn start_profile_session(
        &self,
        profile: &crate::domain::config::Profile,
        session_key: &str,
        _workspace_root: &std::path::Path,
    ) -> anyhow::Result<crate::app::ports::ProfileSession> {
        let session = self.start_opencode_session(profile, session_key)?;
        Ok(session)
    }

    fn profile_wizard_steps(&self) -> Vec<crate::app::ports::WizardStep> {
        use crate::app::ports::WizardStep;
        vec![
            WizardStep::TemplateSelect {
                title: "Select Archetype".into(),
                templates: crate::app::features::profile::template::TEMPLATES.to_vec(),
            },
            WizardStep::TextInput {
                title: "Profile name".into(),
                placeholder: "e.g. opencode-dev".into(),
            },
            WizardStep::ScopeSelect {
                title: "Select Scope".into(),
            },
            WizardStep::Textarea {
                title: "Role".into(),
                placeholder: "e.g. Senior Rust engineer".into(),
                rows: 2,
                key: "role".into(),
            },
            WizardStep::Textarea {
                title: "Domain / Specialty".into(),
                placeholder: "e.g. async CLI tooling".into(),
                rows: 2,
                key: "domain".into(),
            },
            WizardStep::Textarea {
                title: "Collaboration Style".into(),
                placeholder: "e.g. Direct and thorough".into(),
                rows: 3,
                key: "style".into(),
            },
            WizardStep::Textarea {
                title: "Scope Boundaries".into(),
                placeholder: "IN SCOPE:\n...\n\nOUT OF SCOPE:\n...".into(),
                rows: 5,
                key: "boundaries".into(),
            },
            WizardStep::Textarea {
                title: "Activation Triggers".into(),
                placeholder: "e.g. After code changes, on explicit request".into(),
                rows: 3,
                key: "triggers".into(),
            },
            WizardStep::Textarea {
                title: "Constraints".into(),
                placeholder: "e.g. Always run cargo fmt before finishing".into(),
                rows: 3,
                key: "constraints".into(),
            },
            WizardStep::Textarea {
                title: "Output Format".into(),
                placeholder: "e.g. Concise bullets, max 5 items".into(),
                rows: 2,
                key: "format".into(),
            },
            WizardStep::Textarea {
                title: "Core Responsibilities".into(),
                placeholder: "e.g. Review PRs, suggest idioms, catch regressions".into(),
                rows: 4,
                key: "responsibilities".into(),
            },
            WizardStep::ToolSelect {
                title: "Select Tools".into(),
                tools: self
                    .available_profile_tools()
                    .into_iter()
                    .map(|t| (t.clone(), t, false))
                    .collect(),
            },
            WizardStep::PermissionSelect {
                title: "Select Permission Mode".into(),
                modes: self.available_permission_modes(),
            },
            WizardStep::Checklist {
                title: "Select Skills".into(),
                options: vec![],
            },
            WizardStep::Checklist {
                title: "Select MCP Servers".into(),
                options: vec![],
            },
            WizardStep::Review {
                title: "Review & Confirm".into(),
            },
        ]
    }

    fn available_profile_tools(&self) -> Vec<String> {
        vec![
            "Read".into(),
            "Glob".into(),
            "Grep".into(),
            "Bash".into(),
            "Write".into(),
            "Edit".into(),
        ]
    }

    fn available_permission_modes(&self) -> Vec<(String, String)> {
        vec![
            ("default".into(), "Ask for confirmation on edits".into()),
            ("acceptEdits".into(), "Accept edits automatically".into()),
            ("auto".into(), "Auto-approve safe operations".into()),
            ("dontAsk".into(), "Never ask for confirmation".into()),
            ("plan".into(), "Plan mode — suggest only".into()),
        ]
    }
}
